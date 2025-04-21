use std::{cell::RefCell, rc::Rc};

use gettextrs::gettext;
use rand::prelude::SliceRandom;
use relm4::{
    gtk::{
        self, gdk,
        prelude::{AdjustmentExt, BoxExt, ButtonExt, OrientableExt, SelectionModelExt, WidgetExt},
    },
    ComponentParts, ComponentSender, RelmWidgetExt,
};

use crate::{
    components::{
        cover::CoverOut,
        sequence_button_impl::{repeat::Repeat, shuffle::Shuffle},
    },
    factory::{
        playlist_element::PlaylistElementDragged,
        playlist_row::PlaylistUids,
        queue_song_row::{QueueSongRow, QueueUid, QueueUids},
        DragIndicatable,
    },
    gtk_helper::stack::StackExt,
    play_state::PlayState,
    player::Command,
    settings::Settings,
    subsonic::Subsonic,
    types::Droppable,
};

enum QueueStack {
    Placeholder,
    Queue,
}

impl std::fmt::Display for QueueStack {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QueueStack::Placeholder => write!(f, "Placeholder"),
            QueueStack::Queue => write!(f, "Queue"),
        }
    }
}

impl TryFrom<String> for QueueStack {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.as_ref() {
            "Placeholder" => Ok(QueueStack::Placeholder),
            "Queue" => Ok(QueueStack::Queue),
            e => Err(format!("{e} does not match a QueueStack")),
        }
    }
}

#[derive(Debug)]
pub struct Queue {
    subsonic: Rc<RefCell<Subsonic>>,
    randomized_indices: Vec<usize>,
    tracks: relm4::typed_view::list::TypedListView<QueueSongRow, gtk::MultiSelection>,
}

impl Queue {
    pub fn songs(&self) -> Vec<submarine::data::Child> {
        self.iter_tracks()
            .map(|track| track.borrow().item().clone())
            .collect()
    }

    fn tracks(&self) -> &relm4::typed_view::list::TypedListView<QueueSongRow, gtk::MultiSelection> {
        &self.tracks
    }

    pub fn selected_songs(&self) -> Vec<submarine::data::Child> {
        (0..self.tracks.len())
            .filter(|i| self.tracks.view.model().unwrap().is_selected(*i))
            .filter_map(|i| self.tracks.get(i))
            .map(|t| t.borrow().item().clone())
            .collect()
    }

    pub fn iter_tracks(&self) -> QueueRowIterator {
        QueueRowIterator {
            queue: self,
            index: 0,
        }
    }

    pub fn can_play(&self) -> bool {
        !self.tracks.is_empty()
    }

    pub fn can_play_next(&self) -> bool {
        if self.tracks.is_empty() {
            return false;
        }

        let settings = Settings::get().lock().unwrap();
        if settings.repeat != Repeat::Normal || settings.shuffle == Shuffle::Shuffle {
            return true;
        }
        drop(settings);

        match self.current() {
            Some((index, _)) if index == self.tracks.len() as usize => false,
            Some((_, _)) | None => true,
        }
    }

    pub fn can_play_previous(&self) -> bool {
        if self.tracks.is_empty() {
            return false;
        }

        let settings = Settings::get().lock().unwrap();
        if settings.repeat != Repeat::Normal || settings.shuffle == Shuffle::Shuffle {
            return true;
        }
        drop(settings);

        match self.current() {
            Some((0, _)) | None => false,
            Some((_, _)) => true,
        }
    }

    pub fn current(&self) -> Option<(usize, QueueSongRow)> {
        self.iter_tracks()
            .enumerate()
            .find(|(_, t)| t.borrow().play_state() != &PlayState::Stop)
            .map(|(i, t)| (i, t.borrow().clone()))
    }

    fn find_nearest_widget(&self, y: f64) -> Option<(f64, usize)> {
        self.iter_tracks()
            .enumerate()
            .filter_map(|(i, track)| {
                let track = track.borrow();
                let Some(widget) = track.fav_btn() else {
                    return None;
                };
                let translated_y = widget.translate_coordinates(
                    &self.tracks.view,
                    0.0,
                    widget.height() as f64 * 0.5,
                )?;
                let y_diff = y - translated_y.1;
                Some((y_diff, i))
            })
            .min_by(|(diff, _), (diff1, _)| {
                diff.abs()
                    .partial_cmp(&diff1.abs())
                    .expect("widget has no NaN")
            })
    }

    fn index_of_uid(&self, uid: usize) -> Option<(usize, QueueSongRow)> {
        self.iter_tracks()
            .enumerate()
            .find(|(_i, t)| *t.borrow().uid() == uid)
            .map(|(i, track)| (i, track.borrow().clone()))
    }
}

#[derive(Debug)]
pub enum QueueIn {
    Clear,
    Remove,
    NewState(PlayState),
    ToggleShuffle(Shuffle),
    PlayNext,
    PlayPrevious,
    Append(Droppable),
    InsertAfterCurrentlyPlayed(Droppable),
    Replace(Droppable),
    UpdateFavoriteSong(String, bool),
    UpdatePlayCountSong(String, Option<i64>),
    JumpToCurrent,
    Rerandomize,
    Cover(CoverOut),
    DragCssReset,
    Activate(u32),
    ActivateUid(usize),
    DropHover(f64, f64),
    DropMotionLeave,
    DropMove(Droppable, f64, f64),
    DropInsert(Droppable, f64, f64),
    SelectionChanged,
    SetCurrent(Option<usize>),
}

#[derive(Debug)]
pub enum QueueOut {
    Play(Box<submarine::data::Child>),
    QueueEmpty,
    QueueNotEmpty,
    Player(Command),
    CreatePlaylist,
    DisplayToast(String),
    DesktopNotification(Box<submarine::data::Child>),
    FavoriteClicked(String, bool),
    UpdateControlButtons,
}

#[relm4::component(pub)]
impl relm4::Component for Queue {
    type Init = (
        Rc<RefCell<Subsonic>>,
        Vec<submarine::data::Child>,
        Option<usize>,
    );
    type Input = QueueIn;
    type Output = QueueOut;
    type Widgets = QueueWidgets;
    type CommandOutput = ();

    fn init(
        (subsonic, songs, index): Self::Init,
        root: Self::Root,
        sender: relm4::ComponentSender<Self>,
    ) -> relm4::ComponentParts<Self> {
        let tracks =
            relm4::typed_view::list::TypedListView::<QueueSongRow, gtk::MultiSelection>::new();

        let mut model = Queue {
            subsonic,
            randomized_indices: vec![],
            tracks,
        };

        //init queue
        songs
            .iter()
            .map(|song| QueueSongRow::new(&model.subsonic, song, &sender))
            .for_each(|row| model.tracks.append(row));
        sender.input(QueueIn::SetCurrent(index));
        sender.input(QueueIn::Rerandomize);

        let widgets = view_output!();

        // connect signal SelectionChanged
        let send = sender.clone();
        model
            .tracks
            .view
            .model()
            .unwrap()
            .connect_selection_changed(move |_model, _, _| {
                send.input(QueueIn::SelectionChanged);
            });

        sender.input(QueueIn::DragCssReset);

        if model.tracks.is_empty() {
            sender.input(QueueIn::Clear);
        } else {
            widgets.clear_items.set_sensitive(true);
            widgets
                .queue_stack
                .set_visible_child_enum(&QueueStack::Queue);
        }

        sender.input(QueueIn::JumpToCurrent);
        sender.output(QueueOut::UpdateControlButtons).unwrap();
        ComponentParts { model, widgets }
    }

    view! {
        gtk::Box {
            set_widget_name: "queue",
            set_orientation: gtk::Orientation::Vertical,

            append: queue_stack = &gtk::Stack {
                add_enumed[QueueStack::Queue]: scrolled = &gtk::ScrolledWindow
                {
                    set_vexpand: true,
                    model.tracks.view.clone() {
                        set_widget_name: "queue-list",

                        connect_activate[sender] => move |_view, index| {
                            sender.input(QueueIn::Activate(index));
                        },

                        add_controller = gtk::EventControllerKey {
                            connect_key_pressed[sender] => move |_widget, key, _code, _state| {
                                if key == gtk::gdk::Key::Delete {
                                    sender.input(QueueIn::Remove);
                                }
                                gtk::glib::Propagation::Proceed
                            }
                        },

                        // moving queue song
                        add_controller = gtk::DropTarget {
                            set_actions: gdk::DragAction::MOVE,
                            set_types: &[<QueueUids as gtk::prelude::StaticType>::static_type()],

                            connect_motion[sender] => move |_controller, x, y| {
                                sender.input(QueueIn::DropHover(x, y));
                                gdk::DragAction::MOVE
                            },

                            connect_leave[sender] => move |_controller| {
                                sender.input(QueueIn::DropMotionLeave)
                            },

                            connect_drop[sender] => move |_controller, value, x, y| {
                                sender.input(QueueIn::DropMotionLeave);

                                if let Ok(drop) = value.get::<QueueUids>() {
                                    let drop = Droppable::QueueSongs(drop.0);
                                    sender.input(QueueIn::DropMove(drop, x, y));
                                    true
                                } else {
                                    false
                                }
                            }
                        },

                        // adding new songs
                        add_controller = gtk::DropTarget {
                            set_actions: gdk::DragAction::COPY,
                            set_types: &[<Droppable as gtk::prelude::StaticType>::static_type()
                                         , <PlaylistUids as gtk::prelude::StaticType>::static_type()
                                         , <PlaylistElementDragged as gtk::prelude::StaticType>::static_type(),
                            ],

                            connect_motion[sender] => move |_controller, x, y| {
                                sender.input(QueueIn::DropHover(x, y));
                                gdk::DragAction::COPY
                            },

                            connect_leave[sender] => move |_controller| {
                                sender.input(QueueIn::DropMotionLeave)
                            },

                            connect_drop[sender] => move |_controller, value, x, y| {
                                sender.input(QueueIn::DropMotionLeave);

                                if let Ok(drop) = value.get::<Droppable>() {
                                    match &drop {
                                        Droppable::QueueSongs(_) => sender.input(QueueIn::DropMove(drop, x, y)),
                                        _ => sender.input(QueueIn::DropInsert(drop, x, y)),
                                    }
                                    true
                                } else if let Ok(drop) = value.get::<PlaylistElementDragged>() {
                                    let drop = Droppable::Playlist(drop.0);
                                    sender.input(QueueIn::DropInsert(drop, x, y));
                                    true
                                } else if let Ok(drop) = value.get::<PlaylistUids>() {
                                    let drop = Droppable::PlaylistItems(drop.0);
                                    sender.input(QueueIn::DropInsert(drop, x, y));
                                    true
                                }
                                else {
                                    false
                                }
                            }
                        }
                    }
                },
                add_enumed[QueueStack::Placeholder] = &gtk::CenterBox {
                    set_orientation: gtk::Orientation::Vertical,

                    #[wrap(Some)]
                    set_center_widget = &gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,

                        gtk::Label {
                            add_css_class: granite::STYLE_CLASS_H2_LABEL,
                            set_label: &gettext("Queue is empty"),
                        },
                        gtk::Label {
                            add_css_class: granite::STYLE_CLASS_H3_LABEL,
                            set_label: &gettext("Drag music here to add it"),
                        },
                    },

                    add_controller = gtk::DropTarget {
                        set_actions: gdk::DragAction::COPY,
                        set_types: &[<Droppable as gtk::prelude::StaticType>::static_type()
                                     , <PlaylistUids as gtk::prelude::StaticType>::static_type()
                                     , <PlaylistElementDragged as gtk::prelude::StaticType>::static_type(),
                        ],

                        connect_drop[sender] => move |_target, value, _x, _y| {
                            if let Ok(drop) = value.get::<Droppable>() {
                                sender.input(QueueIn::Append(drop));
                                true
                            } else if let Ok(drop) = value.get::<PlaylistElementDragged>() {
                                let drop = Droppable::Playlist(drop.0);
                                sender.input(QueueIn::Append(drop));
                                true
                            } else if let Ok(drop) = value.get::<PlaylistUids>() {
                                let drop = Droppable::PlaylistItems(drop.0);
                                sender.input(QueueIn::Append(drop));
                                true
                            } else {
                                false
                            }
                        }
                    }
                },
            },

            gtk::ActionBar {
                pack_start = &gtk::Box {
                    append: remove_items = &gtk::Button {
                        set_icon_name: "list-remove-symbolic",
                        set_tooltip: &gettext("Remove selected songs from queue"),
                        set_sensitive: false,
                        set_focus_on_click: false,
                        connect_clicked => QueueIn::Remove,
                    },

                    append: clear_items = &gtk::Button {
                        set_margin_start: 15,
                        set_icon_name: "user-trash-symbolic",
                        set_tooltip: &gettext("Clear queue"),
                        set_sensitive: false,
                        set_focus_on_click: false,
                        connect_clicked => QueueIn::Clear,
                    },
                },

                #[wrap(Some)]
                set_center_widget = &gtk::Button {
                        set_icon_name: "view-continuous-symbolic",
                        set_tooltip: &gettext("Jump to played track in queue"),
                        set_focus_on_click: false,
                        connect_clicked => QueueIn::JumpToCurrent,
                },

                pack_end = &gtk::Button {
                    set_icon_name: "document-new-symbolic",
                    set_tooltip: &gettext("Create new playlist from queue"),
                    set_focus_on_click: false,
                    connect_clicked[sender] => move |_btn| {
                        sender.output(QueueOut::CreatePlaylist).unwrap();
                    },
                },
            }
        }
    }

    fn update_with_view(
        &mut self,
        widgets: &mut Self::Widgets,
        msg: Self::Input,
        sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match msg {
            QueueIn::Replace(drop) => {
                sender.input(QueueIn::Clear);
                sender.input(QueueIn::Append(drop));
                sender.output(QueueOut::UpdateControlButtons).unwrap();
            }
            QueueIn::Append(drop) => {
                let songs = drop.get_songs(&self.subsonic);
                for song in songs {
                    let row = QueueSongRow::new(&self.subsonic, &song, &sender);
                    self.tracks.append(row);
                }

                sender.input(QueueIn::Rerandomize);

                if !self.tracks.is_empty() {
                    sender.output(QueueOut::QueueNotEmpty).unwrap();
                }
                widgets.clear_items.set_sensitive(!self.tracks.is_empty());
                widgets
                    .queue_stack
                    .set_visible_child_enum(&QueueStack::Queue);
                sender.input(QueueIn::DragCssReset);
                sender.output(QueueOut::UpdateControlButtons).unwrap();
            }
            QueueIn::InsertAfterCurrentlyPlayed(drop) => {
                let songs = drop.get_songs(&self.subsonic);

                if let Some((index, _track)) = self.current() {
                    for song in songs.into_iter().rev() {
                        self.tracks.insert(
                            index as u32 + 1,
                            QueueSongRow::new(&self.subsonic, &song, &sender),
                        );
                    }
                } else {
                    for song in songs.into_iter() {
                        self.tracks
                            .append(QueueSongRow::new(&self.subsonic, &song, &sender));
                    }
                }
                sender.input(QueueIn::Rerandomize);

                if !self.tracks.is_empty() {
                    sender.output(QueueOut::QueueNotEmpty).unwrap();
                }
                widgets.clear_items.set_sensitive(!self.tracks.is_empty());
                widgets
                    .queue_stack
                    .set_visible_child_enum(&QueueStack::Queue);
                sender.input(QueueIn::DragCssReset);
                sender.output(QueueOut::UpdateControlButtons).unwrap();
            }
            QueueIn::Clear => {
                self.tracks.clear();
                self.randomized_indices.clear();
                widgets.clear_items.set_sensitive(!self.tracks.is_empty());
                sender.input(QueueIn::SelectionChanged);
                widgets
                    .queue_stack
                    .set_visible_child_enum(&QueueStack::Placeholder);
                sender.output(QueueOut::QueueEmpty).unwrap();
                sender.output(QueueOut::UpdateControlButtons).unwrap();
            }
            QueueIn::Remove => {
                let selected_rows: Vec<u32> = (0..self.tracks.len())
                    .filter(|i| self.tracks.view.model().unwrap().is_selected(*i))
                    .collect();

                //set new state when deleting played index
                if let Some((current, _track)) = &self.current() {
                    if selected_rows.contains(&(*current as u32)) {
                        sender.output(QueueOut::Player(Command::Stop)).unwrap();
                    }
                }

                // actual removing rows
                selected_rows
                    .iter()
                    .rev()
                    .for_each(|i| self.tracks.remove(*i));

                if self.tracks.is_empty() {
                    sender.input(QueueIn::Clear);
                }

                sender.input(QueueIn::Rerandomize);
                sender.input(QueueIn::SelectionChanged);
                sender.output(QueueOut::UpdateControlButtons).unwrap();
            }
            QueueIn::NewState(state) => {
                if self.tracks.is_empty() {
                    return;
                }

                if let Some((index, _track)) = self.current() {
                    if let Some(song) = self.tracks.get(index as u32) {
                        song.borrow_mut().set_play_state(&state);
                    }
                }
            }
            QueueIn::ToggleShuffle(shuffle) => {
                {
                    let mut settings = Settings::get().lock().unwrap();
                    settings.shuffle = shuffle.clone();
                }
                sender
                    .output(QueueOut::Player(Command::Shuffle(shuffle)))
                    .unwrap();
            }
            QueueIn::PlayNext => {
                if self.tracks.is_empty() {
                    return;
                }

                let settings = Settings::get().lock().unwrap();
                let repeat = settings.repeat.clone();
                let shuffle = settings.shuffle.clone();
                drop(settings);

                match self.current() {
                    None => self.tracks.get(0).unwrap().borrow_mut().activate(),
                    Some((index, _track)) => {
                        match index as u32 {
                            // at end of queue with repeat current song
                            i if repeat == Repeat::One => {
                                self.tracks.get(i).unwrap().borrow_mut().activate();
                            }
                            // at end of queue with repeat queue
                            i if i + 1 == self.tracks.len()
                                && shuffle != Shuffle::Shuffle
                                && repeat == Repeat::All =>
                            {
                                self.tracks.get(0).unwrap().borrow_mut().activate();
                            }
                            // shuffle ignores repeat all
                            i if shuffle == Shuffle::Shuffle => {
                                let idx = self
                                    .randomized_indices
                                    .iter()
                                    .position(|idx| i == *idx as u32)
                                    .unwrap();
                                let idx =
                                    self.randomized_indices.iter().cycle().nth(idx + 1).unwrap();
                                self.tracks
                                    .get(*idx as u32)
                                    .unwrap()
                                    .borrow_mut()
                                    .activate();
                            }
                            // at end of queue
                            i if i + 1 == self.tracks.len() => {
                                self.iter_tracks().for_each(|track| {
                                    track.borrow_mut().set_play_state(&PlayState::Stop)
                                });
                            }
                            // play next song
                            i => self.tracks.get(i + 1).unwrap().borrow_mut().activate(),
                        }
                    }
                }
                sender.output(QueueOut::UpdateControlButtons).unwrap();
            }
            QueueIn::PlayPrevious => {
                if self.tracks.is_empty() {
                    return;
                }

                let settings = Settings::get().lock().unwrap();
                let repeat = settings.repeat.clone();
                let shuffle = settings.shuffle.clone();
                drop(settings);

                if let Some((index, _track)) = self.current() {
                    match index as u32 {
                        // previous with repeat current song
                        i if repeat == Repeat::One => {
                            self.tracks.get(i).unwrap().borrow_mut().activate();
                        }
                        // at start of queue with repeat queue
                        0 if repeat == Repeat::All => {
                            self.tracks
                                .get(self.tracks.len() - 1)
                                .unwrap()
                                .borrow_mut()
                                .activate();
                        }
                        // shuffle ignores repeat all
                        i if shuffle == Shuffle::Shuffle => {
                            let idx = self
                                .randomized_indices
                                .iter()
                                .position(|idx| i == *idx as u32)
                                .unwrap();
                            if idx == 0 {
                                let idx = self.randomized_indices.last().unwrap();
                                self.tracks
                                    .get(*idx as u32)
                                    .unwrap()
                                    .borrow_mut()
                                    .activate();
                            } else {
                                let idx = self.randomized_indices.get(idx - 1).unwrap();
                                self.tracks
                                    .get(*idx as u32)
                                    .unwrap()
                                    .borrow_mut()
                                    .activate();
                            }
                        }
                        // at start of queue
                        0 => unreachable!("play next should not be active"),
                        i => self.tracks.get(i - 1).unwrap().borrow_mut().activate(),
                    }
                }
                sender.output(QueueOut::UpdateControlButtons).unwrap();
            }
            QueueIn::UpdateFavoriteSong(id, state) => {
                self.iter_tracks()
                    .filter(|track| track.borrow().item().id == id)
                    .for_each(|track| match state {
                        true => {
                            if let Some(fav) = &track.borrow().fav_btn() {
                                fav.set_icon_name("starred-symbolic");
                            }
                            track.borrow_mut().item_mut().starred =
                                Some(chrono::offset::Local::now().into());
                        }
                        false => {
                            if let Some(fav) = &track.borrow().fav_btn() {
                                fav.set_icon_name("non-starred-symbolic");
                            }
                            track.borrow_mut().item_mut().starred = None;
                        }
                    });
            }
            QueueIn::UpdatePlayCountSong(id, play_count) => {
                self.iter_tracks()
                    .filter(|track| track.borrow().item().id == id)
                    .for_each(|track| track.borrow_mut().item_mut().play_count = play_count);
            }
            QueueIn::JumpToCurrent => {
                // where the current song in the window will up end from start
                const CURRENT_POSITION: f64 = 0.4;
                if let Some((index, _track)) = self.current() {
                    let adj = widgets.scrolled.vadjustment();
                    let height = adj.upper();
                    let scroll_y = height / self.tracks.len() as f64 * index as f64
                        - f64::from(widgets.scrolled.height()) * CURRENT_POSITION;
                    adj.set_value(scroll_y);
                    widgets.scrolled.set_vadjustment(Some(&adj));
                }
            }
            QueueIn::Rerandomize => {
                self.randomized_indices = (0..self.tracks.len() as usize).collect();
                let mut rng = rand::rng();
                self.randomized_indices.shuffle(&mut rng);
            }
            QueueIn::Cover(msg) => match msg {
                CoverOut::DisplayToast(msg) => sender.output(QueueOut::DisplayToast(msg)).unwrap(),
            },
            QueueIn::DragCssReset => {
                self.iter_tracks()
                    .for_each(|track| track.borrow().reset_drag_indicators());
            }
            QueueIn::Activate(index) => {
                tracing::info!("playing index {index}");
                // needed when random is activated
                self.iter_tracks().for_each(|track| {
                    track.borrow_mut().set_play_state(&PlayState::Stop);
                });

                if let Some(track) = self.tracks.get(index) {
                    track.borrow_mut().set_play_state(&PlayState::Play);
                    sender
                        .output(QueueOut::Play(Box::new(track.borrow().item().clone())))
                        .unwrap();
                }
                sender.output(QueueOut::UpdateControlButtons).unwrap();
            }
            QueueIn::ActivateUid(uid) => {
                if let Some((index, _row)) = self.index_of_uid(uid) {
                    sender.input(QueueIn::Activate(index as u32));
                }
            }
            QueueIn::DropHover(_x, y) => {
                //reset drag indicators
                self.iter_tracks()
                    .for_each(|track| track.borrow().reset_drag_indicators());

                //finding the index which is the closest
                if let Some((diff, i)) = self.find_nearest_widget(y) {
                    if diff < 0.0 {
                        self.tracks
                            .get(i as u32)
                            .unwrap()
                            .borrow()
                            .add_drag_indicator_top();
                    } else {
                        self.tracks
                            .get(i as u32)
                            .unwrap()
                            .borrow()
                            .add_drag_indicator_bottom();
                    }
                }
            }
            QueueIn::DropMotionLeave => {
                self.iter_tracks()
                    .for_each(|track| track.borrow().reset_drag_indicators());
            }
            QueueIn::DropMove(drop, _x, y) => {
                //finding the index which is the closest to mouse pointer
                let Some((diff, i)) = self.find_nearest_widget(y) else {
                    sender
                        .output(QueueOut::DisplayToast(String::from(
                            "could not find widget to drop to",
                        )))
                        .unwrap();
                    return;
                };

                let dragged = match drop {
                    Droppable::QueueSongs(songs) => songs,
                    _ => unreachable!("can only move QueueSongs"),
                };

                // find all selected rows
                let selected_idx: Vec<u32> = (0..self.tracks.len())
                    .filter(|i| self.tracks.view.model().unwrap().is_selected(*i))
                    .collect();

                // convert uid to index and track
                let Some((dragged_index, dragged_track)) = self.index_of_uid(dragged[0].uid) else {
                    return;
                };

                let mut src_index: Vec<u32> = vec![dragged_index as u32];
                let mut src_tracks: Vec<QueueSongRow> = vec![dragged_track];
                if (selected_idx).contains(&(dragged_index as u32)) {
                    (src_index, src_tracks) = selected_idx
                        .iter()
                        .filter_map(|i| self.tracks.get(*i).map(|t| (i, t)))
                        .map(|(i, track)| (i, track.borrow().clone()))
                        .collect();
                }

                // insert new tracks
                let mut inserted_uids = vec![]; // remember uids to select them later
                let i = if diff < 0.0 { i as u32 } else { i as u32 + 1 };
                tracing::info!("moving queue index {src_index:?} to {i}");
                for track in src_tracks.iter().rev() {
                    let row = QueueSongRow::new(&self.subsonic, track.item(), &sender);
                    inserted_uids.push(*row.uid());
                    self.tracks.insert(i, row);
                    self.tracks
                        .get(i)
                        .unwrap()
                        .borrow_mut()
                        .set_play_state(track.play_state());
                }

                // remove old tracks
                src_tracks.iter().for_each(|track| {
                    if let Some((i, _row)) = self.index_of_uid(*track.uid()) {
                        self.tracks.remove(i as u32);
                    }
                });

                // //unselect rows
                self.tracks.view.model().unwrap().unselect_all();
                // reselect moved rows
                self.iter_tracks()
                    .enumerate()
                    .filter(|(_i, track)| inserted_uids.contains(track.borrow().uid()))
                    .for_each(|(i, _track)| {
                        _ = self
                            .tracks
                            .view
                            .model()
                            .unwrap()
                            .select_item(i as u32, false)
                    });

                sender.input(QueueIn::DragCssReset);
            }
            QueueIn::DropInsert(drop, _x, y) => {
                //finding the index which is the closest
                if let Some((diff, i)) = self.find_nearest_widget(y) {
                    let songs = drop.get_songs(&self.subsonic);
                    //insert songs
                    let i = if diff < 0.0 { i } else { i + 1 };
                    for song in songs.iter().rev() {
                        let row = QueueSongRow::new(&self.subsonic, song, &sender);
                        self.tracks.insert(i as u32, row);
                    }
                }

                sender.input(QueueIn::DragCssReset);
                sender.output(QueueOut::UpdateControlButtons).unwrap();
            }
            QueueIn::SelectionChanged => {
                // update content for drag and drop
                let uids: Vec<_> = (0..self.tracks.len())
                    .filter(|i| self.tracks.view.model().unwrap().is_selected(*i))
                    .filter_map(|i| self.tracks.get(i))
                    .map(|row| QueueUid {
                        uid: *row.borrow().uid(),
                        child: row.borrow().item().clone(),
                    })
                    .collect();

                //reset multiple selection
                (0..self.tracks.len())
                    .filter_map(|i| self.tracks.get(i))
                    .for_each(|track| track.borrow_mut().set_multiple_selection(vec![]));

                // set multiple selection for selected items
                (0..self.tracks.len())
                    .filter(|i| self.tracks.view.model().unwrap().is_selected(*i))
                    .filter_map(|i| self.tracks.get(i))
                    .for_each(|track| track.borrow_mut().set_multiple_selection(uids.clone()));

                // update remove item button
                widgets.remove_items.set_sensitive(!uids.is_empty());
            }
            QueueIn::SetCurrent(index) => {
                if let Some(index) = index {
                    if let Some(track) = self.tracks.get(index as u32) {
                        track.borrow_mut().set_play_state(&PlayState::Pause);
                    }
                }
            }
        }
    }
}

//TODO remove when iter method available in relm4
#[derive(Debug)]
pub struct QueueRowIterator<'a> {
    queue: &'a Queue,
    index: u32,
}

impl Iterator for QueueRowIterator<'_> {
    type Item = relm4::typed_view::TypedListItem<QueueSongRow>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index < self.queue.tracks().len() {
            let result = self.queue.tracks().get(self.index);
            self.index += 1;
            result
        } else {
            None
        }
    }
}
