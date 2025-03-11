use std::{cell::RefCell, rc::Rc};

use gettextrs::gettext;
use rand::prelude::SliceRandom;
use relm4::{
    factory::FactoryVecDeque,
    gtk::{
        self, gdk,
        prelude::{
            AdjustmentExt, BoxExt, ButtonExt, ListBoxRowExt, OrientableExt, SelectionModelExt,
            WidgetExt,
        },
    },
    prelude::DynamicIndex,
    ComponentParts, ComponentSender, RelmWidgetExt,
};

use crate::{
    components::{
        cover::CoverOut,
        sequence_button_impl::{repeat::Repeat, shuffle::Shuffle},
    },
    factory::{
        queue_song_row::QueueSongRow,
        DropHalf,
    },
    gtk_helper::stack::StackExt,
    play_state::PlayState,
    player::Command,
    settings::Settings,
    subsonic::Subsonic,
    types::Droppable,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScrollMotion {
    None,
    Down,
    Up,
}

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
    scrolled: gtk::ScrolledWindow,
    scroll_motion: Rc<RefCell<ScrollMotion>>,
    randomized_indices: Vec<usize>,
    remove_items: gtk::Button,
    clear_items: gtk::Button, //TODO change to named widget
    last_selected: Option<DynamicIndex>,
    tracks: relm4::typed_view::list::TypedListView<QueueSongRow, gtk::MultiSelection>,
}

impl Queue {
    pub fn songs(&self) -> Vec<submarine::data::Child> {
        (0..self.tracks.len())
            .filter_map(|i| self.tracks.get(i))
            .map(|track| track.borrow().item().clone())
            .collect()
    }

    pub fn can_play(&self) -> bool {
        !self.tracks.is_empty()
    }

    pub fn playing_index(&self) -> Option<u32> {
        (0..self.tracks.len())
            .filter_map(|i| self.tracks.get(i).map(|t| (i, t)))
            .find(|(_i, track)| *track.borrow().play_state() == PlayState::Play)
            .map(|(i, _t)| i)
    }

    pub fn can_play_next(&self) -> bool {
        if self.tracks.is_empty() {
            return false;
        }

        let settings = Settings::get().lock().unwrap();
        if settings.repeat != Repeat::Normal {
            return true;
        }

        if settings.shuffle == Shuffle::Shuffle {
            return true; //TODO might change later
        }
        drop(settings);

        if let Some(index) = &self.playing_index() {
            if index + 1 == self.tracks.len() {
                return false;
            }
        }

        true
    }

    pub fn can_play_previous(&self) -> bool {
        if self.tracks.is_empty() {
            return false;
        }

        let settings = Settings::get().lock().unwrap();
        if settings.repeat != Repeat::Normal {
            return true;
        }

        if settings.shuffle == Shuffle::Shuffle {
            return true;
        }
        drop(settings);

        if let Some(index) = &self.playing_index() {
            if *index == 0 {
                return false;
            }
        }

        true
    }

    pub fn current_song(&self) -> Option<submarine::data::Child> {
        match &self.playing_index() {
            None => None,
            Some(index) => self
                .tracks
                .get(*index)
                .map(|queue_song| queue_song.borrow().item().clone()),
        }
    }
}

#[derive(Debug)]
pub enum QueueIn {
    SetCurrent(Option<usize>),
    Clear,
    Remove,
    NewState(PlayState),
    SomeIsSelected(bool),
    ToggleShuffle(Shuffle),
    PlayNext,
    PlayPrevious,
    Append(Droppable),
    InsertAfterCurrentlyPlayed(Droppable),
    Replace(Droppable),
    DisplayToast(String),
    Favorite(String, bool),
    JumpToCurrent,
    Rerandomize,
    DragOverSpace,
    Cover(CoverOut),
    MoveSong {
        src: usize,
        dest: usize,
        half: DropHalf,
    },
    InsertSongs {
        dest: usize,
        drop: Droppable,
        half: DropHalf,
    },
    DraggedOverRow {
        dest: usize,
        y: f64,
    },
    DragLeaveRow,
    Activate(u32),
    SelectionChanged,
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
            scrolled: gtk::ScrolledWindow::default(),
            scroll_motion: Rc::new(RefCell::new(ScrollMotion::None)),
            randomized_indices: vec![],
            remove_items: gtk::Button::new(),
            clear_items: gtk::Button::new(),
            last_selected: None,
            tracks,
        };

        //init queue
        songs
            .iter()
            .map(|song| QueueSongRow::new(&model.subsonic, song, &sender))
            .for_each(|row| model.tracks.append(row));
        if let Some(index) = index {
            model.tracks.get(index as u32).unwrap().borrow_mut().set_play_state(&PlayState::Pause);
        }
        sender.input(QueueIn::Rerandomize);

        // let songs = model.songs.widget().clone();
        let scrolled = model.scrolled.clone();
        let scrolling = model.scroll_motion.clone();
        let (scroll_sender, receiver) = async_channel::unbounded::<bool>();
        let widgets = view_output!();

        gtk::glib::spawn_future_local(async move {
            let scrolling = scrolling.clone();

            while let Ok(_msg) = receiver.recv().await {
                let scrolled = scrolled.clone();
                let scrolling = scrolling.clone();

                gtk::glib::source::timeout_add_local(
                    core::time::Duration::from_millis(15),
                    move || {
                        const SCROLL_MOVE: f64 = 5f64;
                        match *scrolling.borrow() {
                            ScrollMotion::None => return gtk::glib::ControlFlow::Break,
                            ScrollMotion::Up => {
                                let vadj = scrolled.vadjustment();
                                vadj.set_value(vadj.value() - SCROLL_MOVE);
                                scrolled.set_vadjustment(Some(&vadj));
                            }
                            ScrollMotion::Down => {
                                let vadj = scrolled.vadjustment();
                                vadj.set_value(vadj.value() + SCROLL_MOVE);
                                scrolled.set_vadjustment(Some(&vadj));
                            }
                        }
                        gtk::glib::ControlFlow::Continue
                    },
                );
            }
        });

        let send = sender.clone();
        model.tracks.view.model().unwrap().connect_selection_changed(move|_model, _, _| {
            send.input(QueueIn::SelectionChanged);
        });

        if model.tracks.is_empty() {
            sender.input(QueueIn::Clear);
        } else {
            model.clear_items.set_sensitive(true);
            widgets.queue_stack.set_visible_child_enum(&QueueStack::Queue);
        }

        ComponentParts { model, widgets }
    }

    view! {
        gtk::Box {
            set_widget_name: "queue",
            set_orientation: gtk::Orientation::Vertical,

            append: queue_stack = &gtk::Stack {
                add_enumed[QueueStack::Queue] = &model.scrolled.clone() -> gtk::ScrolledWindow
                {
                    set_vexpand: true,
                    // model.songs.widget().clone() -> gtk::ListBox {
                    //     set_selection_mode: gtk::SelectionMode::Multiple,

                    //     connect_selected_rows_changed[sender] => move |widget| {
                    //         sender.input(QueueIn::SomeIsSelected(!widget.selected_rows().is_empty()));
                    //     },

                    //     // when hovering over the queue stop scrolling
                    //     add_controller = gtk::EventControllerMotion {
                    //         connect_motion[scrolling] => move |_self, _x, _y| {
                    //             scrolling.replace(ScrollMotion::None);
                    //         }
                    //     },

                    //     add_controller = gtk::DropControllerMotion {
                    //         connect_motion[scrolled, songs, scrolling, scroll_sender] => move |_self, x, y| {
                    //             if *scrolling.borrow() != ScrollMotion::None {
                    //                 return;
                    //             }

                    //             const SCROLL_ZONE: f32 = 60f32;

                    //             let point = gtk::graphene::Point::new(x as f32, y as f32);
                    //             let computed = songs.compute_point(&scrolled, &point).unwrap();
                    //             if computed.y() >= 0f32 && computed.y() <= SCROLL_ZONE {
                    //                 scrolling.replace(ScrollMotion::Up);
                    //                 scroll_sender.try_send(true).unwrap();
                    //             } else if computed.y() >= scrolled.height() as f32 - SCROLL_ZONE && computed.y() <= scrolled.height() as f32 {
                    //                 scrolling.replace(ScrollMotion::Down);
                    //                 scroll_sender.try_send(true).unwrap();
                    //             } else {
                    //                 scrolling.replace(ScrollMotion::None);
                    //             }
                    //         },

                    //         connect_leave[scrolling] => move |_self| {
                    //             scrolling.replace(ScrollMotion::None);
                    //         },

                    //         connect_drop_notify[scrolling] => move |_self| {
                    //             scrolling.replace(ScrollMotion::None);
                    //         }
                    //     },

                    //     add_controller = gtk::DropTarget {
                    //         set_actions: gdk::DragAction::MOVE | gdk::DragAction::COPY,
                    //         set_types: &[<Droppable as gtk::prelude::StaticType>::static_type()],

                    //         connect_drop[sender] => move |_target, value, _x, _y| {
                    //             if let Ok(drop) = value.get::<Droppable>() {
                    //                 sender.input(QueueIn::Append(drop));
                    //             }
                    //             true
                    //         },

                    //         connect_motion[sender] => move |_widget, _x, _y| {
                    //             sender.input(QueueIn::DragOverSpace);
                    //             gdk::DragAction::MOVE
                    //         }
                    //     }
                    // }
                    model.tracks.view.clone() {
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
                        }
                    }
                },
                add_enumed[QueueStack::Placeholder] = &gtk::Box {
                    set_valign: gtk::Align::Center,
                    set_orientation: gtk::Orientation::Vertical,

                    gtk::Label {
                        add_css_class: granite::STYLE_CLASS_H2_LABEL,
                        set_label: &gettext("Queue is empty"),
                    },
                    gtk::Label {
                        add_css_class: granite::STYLE_CLASS_H3_LABEL,
                        set_label: &gettext("Drag music here to add it"),
                    },
                    add_controller = gtk::DropTarget {
                        set_actions: gdk::DragAction::MOVE | gdk::DragAction::COPY,
                        set_types: &[<Droppable as gtk::prelude::StaticType>::static_type()],

                        connect_motion => move |_target, _x, _y| {
                            println!("motion");
                            gdk::DragAction::COPY
                        },

                        connect_drop[sender] => move |_target, value, _x, _y| {
                            if let Ok(drop) = value.get::<Droppable>() {
                                sender.input(QueueIn::Append(drop));
                            }
                            true
                        }
                    }
                },
            },

            gtk::ActionBar {
                pack_start = &gtk::Box {
                    model.remove_items.clone() {
                        set_icon_name: "list-remove-symbolic",
                        set_tooltip: &gettext("Remove selected songs from queue"),
                        set_sensitive: false,
                        set_focus_on_click: false,
                        connect_clicked => QueueIn::Remove,
                    },

                    model.clear_items.clone() {
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

    fn update_with_view(&mut self,
                        widgets: &mut Self::Widgets,
                        msg: Self::Input, sender: ComponentSender<Self>, _root: &Self::Root) {
        match msg {
            QueueIn::SetCurrent(None) | QueueIn::NewState(PlayState::Stop) => {
                if let Some(index) = &self.playing_index() {
                    if let Some(song) = self.tracks.get(*index) {
                        song.borrow_mut().set_play_state(&PlayState::Stop);
                    }
                }
            }
            QueueIn::SetCurrent(Some(index)) => {
                if let Some(song) = self.tracks.get(index as u32) {
                    sender.input(QueueIn::NewState(PlayState::Pause));
                }
            }
            QueueIn::Replace(drop) => {
                sender.input(QueueIn::Clear);
                sender.input(QueueIn::Append(drop));
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
                self.clear_items.set_sensitive(!self.tracks.is_empty());
                widgets.queue_stack.set_visible_child_enum(&QueueStack::Queue);
            }
            QueueIn::InsertAfterCurrentlyPlayed(drop) => {
                let songs = drop.get_songs(&self.subsonic);

                if let Some(index) = self.playing_index() {
                    for song in songs.into_iter().rev() {
                        self.tracks
                            .insert(index + 1, QueueSongRow::new(&self.subsonic, &song, &sender));
                    }
                } else {
                    for song in songs.into_iter().rev() {
                        self.tracks
                            .append(QueueSongRow::new(&self.subsonic, &song, &sender));
                    }
                }
                sender.input(QueueIn::Rerandomize);

                if !self.tracks.is_empty() {
                    sender.output(QueueOut::QueueNotEmpty).unwrap();
                }
                self.clear_items.set_sensitive(!self.tracks.is_empty());
                widgets.queue_stack.set_visible_child_enum(&QueueStack::Queue);
            }
            QueueIn::Clear => {
                self.tracks.clear();
                self.randomized_indices.clear();
                self.clear_items.set_sensitive(!self.tracks.is_empty());
                sender.input(QueueIn::SelectionChanged);
                self.last_selected = None;
                widgets.queue_stack.set_visible_child_enum(&QueueStack::Placeholder);
                sender.output(QueueOut::QueueEmpty).unwrap();
            }
            QueueIn::Remove => {
                let selected_rows: Vec<u32> = (0..self.tracks.len())
                    .filter(|i| self.tracks.view.model().unwrap().is_selected(*i))
                    .collect();
                selected_rows
                    .iter()
                    .rev()
                    .for_each(|i| self.tracks.remove(*i));

                sender.input(QueueIn::Rerandomize);

                //set new state when deleting played index
                if let Some(current) = &self.playing_index() {
                    if selected_rows.contains(&current) {
                        sender.output(QueueOut::Player(Command::Stop)).unwrap();
                        sender.input(QueueIn::SetCurrent(None));
                    }
                }

                sender.input(QueueIn::SelectionChanged);

                if self.tracks.is_empty() {
                    sender.input(QueueIn::Clear);
                }
            }
            QueueIn::NewState(state) => {
                if self.tracks.is_empty() {
                    return;
                }

                if let Some(index) = self.playing_index() {
                    if let Some(song) = self.tracks.get(index) {
                        song.borrow_mut().set_play_state(&state);
                    }
                }
            }
            QueueIn::SomeIsSelected(state) => self.remove_items.set_sensitive(state),
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

                match self.playing_index() {
                    None => self.tracks.get(0).unwrap().borrow_mut().activate(),
                    Some(index) => {
                        match index {
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
                                self.tracks
                                    .get(i)
                                    .unwrap()
                                    .borrow_mut()
                                    .set_play_state(&PlayState::Stop);
                                sender.output(QueueOut::Player(Command::Stop)).unwrap();
                            }
                            // play next song
                            i => self.tracks.get(i + 1).unwrap().borrow_mut().activate(),
                        }
                    }
                }
            }
            QueueIn::PlayPrevious => {
                if self.tracks.is_empty() {
                    return;
                }

                let settings = Settings::get().lock().unwrap();
                let repeat = settings.repeat.clone();
                let shuffle = settings.shuffle.clone();
                drop(settings);

                if let Some(index) = self.playing_index() {
                    match index {
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
                        0 => self.tracks.get(0).unwrap().borrow_mut().activate(),
                        i => self.tracks.get(i - 1).unwrap().borrow_mut().activate(),
                    }
                }
            }
            QueueIn::DisplayToast(title) => sender.output(QueueOut::DisplayToast(title)).unwrap(),
            QueueIn::Favorite(id, state) => {
                (0..self.tracks.len())
                    .filter_map(|i| self.tracks.get(i))
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
            QueueIn::JumpToCurrent => {
                // where the current song in the window will up end from start
                const CURRENT_POSITION: f64 = 0.4;
                if let Some(current) = self.playing_index() {
                    let height = self.tracks.view.height();
                    let adj = self.scrolled.vadjustment();
                    let scroll_y = f64::from(height) / self.tracks.len() as f64 * current as f64
                        - f64::from(self.scrolled.height()) * CURRENT_POSITION;
                    adj.set_value(scroll_y);
                    self.scrolled.set_vadjustment(Some(&adj));
                }
            }
            QueueIn::Rerandomize => {
                self.randomized_indices = (0..self.tracks.len() as usize).collect();
                let mut rng = rand::rng();
                self.randomized_indices.shuffle(&mut rng);
            }
            QueueIn::DragOverSpace => {
                // show indicator on last item
                if let Some(last) = self.tracks.get(self.tracks.len()) {
                    last.borrow().add_drag_indicator_bottom();
                }
            }
            QueueIn::Cover(msg) => match msg {
                CoverOut::DisplayToast(msg) => sender.output(QueueOut::DisplayToast(msg)).unwrap(),
            },
            QueueIn::MoveSong { src, dest, half } => {
                //remove drag indicators
                (0..self.tracks.len())
                    .filter_map(|i| self.tracks.get(i))
                    .for_each(|entry| entry.borrow().reset_drag_indicators());

                // do nothing when src is dest
                if src == dest {
                    return;
                }

                //find src and dest row
                let Some((src_index, src_entry)) = (0..self.tracks.len())
                    .filter_map(|i| self.tracks.get(i).map(|entry| (i, entry)))
                    .find(|(_i, entry)| entry.borrow().uid() == &src)
                else {
                    sender
                        .output(QueueOut::DisplayToast(String::from(
                            "source not found in move_song",
                        )))
                        .unwrap();
                    return;
                };
                let Some((dest_index, _dest_entry)) = (0..self.tracks.len())
                    .filter_map(|i| self.tracks.get(i).map(|entry| (i, entry)))
                    .find(|(_i, entry)| entry.borrow().uid() == &dest)
                else {
                    sender
                        .output(QueueOut::DisplayToast(String::from(
                            "dest not found in move_song",
                        )))
                        .unwrap();
                    return;
                };

                //remove src
                let src_row =
                    QueueSongRow::new(&self.subsonic, &src_entry.borrow().item(), &sender);
                self.tracks.remove(src_index);

                // insert based on cursor position and order of src and dest
                //TODO try to insert first and delete then, to avoid scrolling ScrolledWindow
                match (&half, src_index <= dest_index) {
                    (DropHalf::Above, true) => self.tracks.insert(dest_index - 1, src_row),
                    (DropHalf::Above, false) | (DropHalf::Below, true) => {
                        self.tracks.insert(dest_index, src_row)
                    }
                    (DropHalf::Below, false) => self.tracks.insert(dest_index + 1, src_row),
                }
            }
            QueueIn::InsertSongs { dest, drop, half } => {
                //remove drag indicators
                (0..self.tracks.len())
                    .filter_map(|i| self.tracks.get(i))
                    .for_each(|entry| entry.borrow().reset_drag_indicators());

                // find index of uid
                let Some((dest, _dest_entry)) = (0..self.tracks.len())
                    .filter_map(|i| self.tracks.get(i).map(|entry| (i, entry)))
                    .find(|(_i, entry)| entry.borrow().uid() == &dest)
                else {
                    sender
                        .output(QueueOut::DisplayToast(String::from(
                            "dest not found in insert_songs",
                        )))
                        .unwrap();
                    return;
                };

                let songs = drop.get_songs(&self.subsonic);
                for song in songs.iter().rev() {
                    let row = QueueSongRow::new(&self.subsonic, song, &sender);
                    match half {
                        DropHalf::Above => self.tracks.insert(dest, row),
                        DropHalf::Below => self.tracks.insert(dest + 1, row),
                    }
                }
                widgets.queue_stack.set_visible_child_enum(&QueueStack::Queue);
            }
            QueueIn::DraggedOverRow { dest, y } => {
                //disable reordering item when searching
                let settings = Settings::get().lock().unwrap();
                if settings.search_active && !settings.search_text.is_empty() {
                    return;
                }
                drop(settings);

                let Some(src_entry) = (0..self.tracks.len())
                    .filter_map(|i| self.tracks.get(i))
                    .find(|entry| entry.borrow().uid() == &dest)
                else {
                    tracing::warn!("source index {dest} while dragging over not found");
                    return;
                };

                let fav_btn = src_entry.borrow().fav_btn().clone();
                if let Some(fav_btn) = fav_btn {
                    let widget_height = fav_btn.height();
                    if y < f64::from(widget_height) * 0.5f64 {
                        src_entry.borrow().add_drag_indicator_top();
                    } else {
                        src_entry.borrow().add_drag_indicator_bottom();
                    }
                }
            }
            QueueIn::DragLeaveRow => {
                (0..self.tracks.len())
                    .filter_map(|i| self.tracks.get(i))
                    .for_each(|track| track.borrow().reset_drag_indicators());
            }
            QueueIn::Activate(index) => {
                (0..self.tracks.len())
                    .filter_map(|i| self.tracks.get(i))
                    .for_each(|track| {
                        track.borrow_mut().set_play_state(&PlayState::Stop);
                    });

                if let Some(track) = self.tracks.get(index) {
                    // self.playing_index = Some(index); //TODO update playing index
                    track.borrow_mut().set_play_state(&PlayState::Play);
                    sender
                        .output(QueueOut::Play(Box::new(track.borrow().item().clone())))
                        .unwrap();
                }
            }
            QueueIn::SelectionChanged => {
                let is_some = (0..self.tracks.len()).any(|i| {
                    self.tracks.view.model().unwrap().is_selected(i)
                });

                self.remove_items.set_sensitive(is_some);
                //TODO remove SomeIsSelected
            }
        }
    }
}
