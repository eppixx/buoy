use std::{cell::RefCell, rc::Rc};

use gettextrs::gettext;
use relm4::{
    gtk::{
        self, gdk, glib, pango,
        prelude::{
            BoxExt, ButtonExt, EventControllerExt, GestureSingleExt, ListBoxRowExt, OrientableExt,
            ToValue, WidgetExt,
        },
    },
    Component, ComponentController, RelmWidgetExt,
};

use crate::{
    client::Client,
    common::convert_for_label,
    components::cover::{Cover, CoverIn, CoverOut},
    css::DragState,
    gtk_helper::stack::StackExt,
    play_state::PlayState,
    subsonic::Subsonic,
    types::{Droppable, Id},
};

#[derive(Clone, Debug, PartialEq, Eq, glib::Boxed)]
#[boxed_type(name = "QueueSongIndex")]
pub struct Index(relm4::factory::DynamicIndex);

#[derive(Debug, Clone)]
pub enum QueueSongIn {
    Activated,
    DraggedOver(f64),
    DragLeave,
    NewState(PlayState),
    DroppedSong { drop: Droppable, y: f64 },
    MoveSong { index: Index, y: f64 },
    Cover(CoverOut),
    FavoriteClicked,
    FavoriteSong(String, bool),
}

#[derive(Debug)]
pub enum QueueSongOut {
    Activated(relm4::factory::DynamicIndex, Box<submarine::data::Child>),
    Clicked(relm4::factory::DynamicIndex),
    ShiftClicked(relm4::factory::DynamicIndex),
    Remove,
    MoveAbove {
        src: relm4::factory::DynamicIndex,
        dest: relm4::factory::DynamicIndex,
    },
    MoveBelow {
        src: relm4::factory::DynamicIndex,
        dest: relm4::factory::DynamicIndex,
    },
    DropAbove {
        src: Vec<submarine::data::Child>,
        dest: relm4::factory::DynamicIndex,
    },
    DropBelow {
        src: Vec<submarine::data::Child>,
        dest: relm4::factory::DynamicIndex,
    },
    DisplayToast(String),
    FavoriteClicked(String, bool),
}

#[derive(Debug)]
pub struct QueueSong {
    root_widget: gtk::ListBoxRow,
    info: submarine::data::Child,
    cover: relm4::Controller<Cover>,
    favorited: gtk::Button,
    index: relm4::factory::DynamicIndex,
    sender: relm4::FactorySender<Self>,
    drag_src: gtk::DragSource,
}

impl QueueSong {
    pub fn new_play_state(
        &self,
        state: &PlayState,
    ) -> (Option<relm4::factory::DynamicIndex>, Option<Id>) {
        self.sender.input(QueueSongIn::NewState(state.clone()));
        match state {
            PlayState::Play => (
                Some(self.index.clone()),
                Some(Id::song(self.info.id.clone())),
            ),
            PlayState::Pause => (Some(self.index.clone()), None),
            PlayState::Stop => (None, None),
        }
    }

    pub fn root_widget(&self) -> &gtk::ListBoxRow {
        &self.root_widget
    }

    pub fn activate(&self) {
        self.sender.input(QueueSongIn::Activated);
    }

    pub fn info(&self) -> &submarine::data::Child {
        &self.info
    }

    pub fn index(&self) -> &relm4::factory::DynamicIndex {
        &self.index
    }
}

#[derive(Debug)]
pub enum QueueSongCmd {
    InsertChildrenAbove(
        Result<
            (relm4::factory::DynamicIndex, Vec<submarine::data::Child>),
            submarine::SubsonicError,
        >,
    ),
    InsertChildrenBelow(
        Result<
            (relm4::factory::DynamicIndex, Vec<submarine::data::Child>),
            submarine::SubsonicError,
        >,
    ),
}

#[relm4::factory(pub)]
impl relm4::factory::FactoryComponent for QueueSong {
    type Init = (Rc<RefCell<Subsonic>>, submarine::data::Child);
    type Input = QueueSongIn;
    type Output = QueueSongOut;
    type ParentWidget = gtk::ListBox;
    type Widgets = QueueSongWidgets;
    type CommandOutput = QueueSongCmd;

    fn init_model(
        (subsonic, init): Self::Init,
        index: &relm4::factory::DynamicIndex,
        sender: relm4::factory::FactorySender<Self>,
    ) -> Self {
        let cover = Cover::builder()
            .launch((subsonic.clone(), init.cover_art.clone()))
            .forward(sender.input_sender(), QueueSongIn::Cover);
        cover.model().add_css_class_image("size32");
        cover.emit(CoverIn::LoadSong(Box::new(init.clone())));

        let icon_name = match init.starred {
            Some(_) => "starred-symbolic",
            None => "non-starred-symbolic",
        };

        let mut model = Self {
            root_widget: gtk::ListBoxRow::new(),
            info: init.clone(),
            cover,
            favorited: gtk::Button::from_icon_name(icon_name),
            index: index.clone(),
            sender: sender.clone(),
            drag_src: gtk::DragSource::new(),
        };

        DragState::reset(&mut model.root_widget);

        // setup DragSource
        let index = Index(index.clone());
        let content = gdk::ContentProvider::for_value(&index.to_value());
        model.drag_src.set_content(Some(&content));
        model.drag_src.set_actions(gdk::DragAction::MOVE);
        let album = subsonic.borrow().album_of_song(&init);
        model.drag_src.connect_drag_begin(move |src, _drag| {
            if let Some(album) = &album {
                if let Some(cover_id) = &album.cover_art {
                    let cover = subsonic.borrow().cover_icon(cover_id);
                    if let Some(tex) = cover {
                        src.set_icon(Some(&tex), 0, 0);
                    }
                }
            }
        });

        model
    }

    view! {
        self.root_widget.clone() -> gtk::ListBoxRow {
            set_widget_name: "queue-song",

            gtk::Box {
                set_spacing: 10,

                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_valign: gtk::Align::Center,

                    // cover
                    append: icon_stack = &gtk::Stack {
                        set_transition_type: gtk::StackTransitionType::Crossfade,

                        add_enumed[PlayState::Stop] = &self.cover.widget().clone(),
                        add_enumed[PlayState::Play] = &gtk::Image {
                            add_css_class: "borderd",
                            set_icon_name: Some("audio-volume-high-symbolic"),
                        },
                        add_enumed[PlayState::Pause] = &gtk::Image {
                            add_css_class: "bordered",
                            set_icon_name: Some("media-playback-pause-symbolic"),
                        },
                    },
                },

                // title and artist
                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,

                    append: title = &gtk::Label {
                        set_label: &self.info.title,
                        set_width_chars: 3,
                        set_hexpand: true,
                        set_halign: gtk::Align::Start,
                        set_ellipsize: pango::EllipsizeMode::End,
                    },
                    append: artist = &gtk:: Label {
                        set_markup: &format!("<span style=\"italic\">{}</span>"
                            , glib::markup_escape_text(self.info.artist.as_deref().unwrap_or(&gettext("Unknown Artist")))),
                        set_width_chars: 3,
                        set_hexpand: true,
                        set_halign: gtk::Align::Start,
                        set_ellipsize: pango::EllipsizeMode::End,
                    }
                },

                // length
                gtk::Label {
                    set_label: &convert_for_label(i64::from(self.info.duration.unwrap_or(0)) * 1000),
                },

                // favorite
                self.favorited.clone() {
                    set_tooltip: &gettext("Click to (un)favorite song"),
                    set_focus_on_click: false,
                    connect_clicked => QueueSongIn::FavoriteClicked,
                },
            },

            // make item draggable
            add_controller: self.drag_src.clone(),

            // activate is when pressed enter on item
            connect_activate => QueueSongIn::Activated,

            // accept drop from queue items and id's and render drop indicators
            add_controller = gtk::DropTarget {
                set_actions: gdk::DragAction::MOVE | gdk::DragAction::COPY,
                set_types: &[<Index as gtk::prelude::StaticType>::static_type(),
                             <Droppable as gtk::prelude::StaticType>::static_type(),
                ],

                connect_drop[sender] => move |_target, value, _x, y| {
                    if let Ok(index) = value.get::<Index>() {
                        sender.input(QueueSongIn::MoveSong { index, y });
                    }
                    if let Ok(drop) = value.get::<Droppable>() {
                        sender.input(QueueSongIn::DroppedSong { drop, y });
                    }
                    true
                },

                connect_motion[sender] => move |_widget, _x, y| {
                    sender.input(QueueSongIn::DraggedOver(y));
                    //may need to return other value for drag in future
                    gdk::DragAction::MOVE
                },

                connect_leave => QueueSongIn::DragLeave,
            },

            // double left click activates item
            add_controller = gtk::GestureClick {
                set_button: 1,
                connect_pressed[sender, index] => move |widget, n, _x, _y|{
                    if n == 1 {
                        let state = widget.current_event_state();
                        if !(state.contains(gdk::ModifierType::SHIFT_MASK)
                             || state.contains(gdk::ModifierType::CONTROL_MASK) ) {
                            // normal click
                            sender.output(QueueSongOut::Clicked(index.clone())).unwrap();
                        } else if state.contains(gdk::ModifierType::SHIFT_MASK) {
                            // shift click
                            sender.output(QueueSongOut::ShiftClicked(index.clone())).unwrap();
                        }
                    }
                    else if n == 2 {
                        sender.input(QueueSongIn::Activated);
                    }
                }
            },

            // connect key presses
            add_controller = gtk::EventControllerKey {
                connect_key_pressed[sender] => move |_widget, key, _code, _state| {
                    if key == gtk::gdk::Key::Delete {
                        sender.output(QueueSongOut::Remove).unwrap();
                    }
                    gtk::glib::Propagation::Stop
                }
            },
        }
    }

    fn update_with_view(
        &mut self,
        widgets: &mut Self::Widgets,
        message: Self::Input,
        sender: relm4::FactorySender<Self>,
    ) {
        match message {
            QueueSongIn::Activated => {
                self.new_play_state(&PlayState::Play);
                sender
                    .output(QueueSongOut::Activated(
                        self.index.clone(),
                        Box::new(self.info.clone()),
                    ))
                    .unwrap();
            }
            QueueSongIn::DraggedOver(y) => {
                let widget_height = self.root_widget.height();
                if y < f64::from(widget_height) * 0.5f64 {
                    DragState::drop_shadow_top(&mut self.root_widget);
                } else {
                    DragState::drop_shadow_bottom(&mut self.root_widget);
                }
            }
            QueueSongIn::DragLeave => DragState::reset(&mut self.root_widget),
            QueueSongIn::NewState(state) => {
                widgets.icon_stack.set_visible_child_enum(&state);
            }
            QueueSongIn::DroppedSong { drop, y } => {
                sender.input(QueueSongIn::DragLeave);
                let widget_height = self.root_widget.height();
                let index = self.index.clone();
                let client = Client::get().unwrap();

                let songs = match drop {
                    Droppable::Queue(ids) => ids,
                    Droppable::Child(c) => vec![*c],
                    Droppable::AlbumWithSongs(album) => album.song,
                    Droppable::Playlist(playlist) => playlist.entry,
                    Droppable::Album(album) => {
                        sender.oneshot_command(async move {
                            match client.get_album(album.id).await {
                                Err(e) => QueueSongCmd::InsertChildrenBelow(Err(e)),
                                Ok(album) if y < f64::from(widget_height) * 0.5f64 => {
                                    QueueSongCmd::InsertChildrenAbove(Ok((index, album.song)))
                                }
                                Ok(album) => {
                                    QueueSongCmd::InsertChildrenBelow(Ok((index, album.song)))
                                }
                            }
                        });
                        vec![]
                    }
                    Droppable::Artist(artist) => {
                        sender.oneshot_command(async move {
                            match client.get_artist(artist.id).await {
                                Err(e) => QueueSongCmd::InsertChildrenBelow(Err(e)),
                                Ok(artist) => {
                                    let mut songs = vec![];
                                    for album in artist.album {
                                        match client.get_album(album.id).await {
                                            Err(e) => {
                                                return QueueSongCmd::InsertChildrenBelow(Err(e))
                                            }
                                            Ok(mut album) => songs.append(&mut album.song),
                                        }
                                    }
                                    if y < f64::from(widget_height) * 0.5f64 {
                                        QueueSongCmd::InsertChildrenAbove(Ok((index, songs)))
                                    } else {
                                        QueueSongCmd::InsertChildrenBelow(Ok((index, songs)))
                                    }
                                }
                            }
                        });
                        vec![]
                    }
                    Droppable::ArtistWithAlbums(artist) => {
                        sender.oneshot_command(async move {
                            let mut songs = vec![];
                            for album in artist.album {
                                match client.get_album(album.id).await {
                                    Err(e) => return QueueSongCmd::InsertChildrenBelow(Err(e)),
                                    Ok(mut album) => songs.append(&mut album.song),
                                }
                            }
                            if y < f64::from(widget_height) * 0.5f64 {
                                QueueSongCmd::InsertChildrenAbove(Ok((index, songs)))
                            } else {
                                QueueSongCmd::InsertChildrenBelow(Ok((index, songs)))
                            }
                        });
                        vec![]
                    }
                    Droppable::AlbumChild(album) => {
                        sender.oneshot_command(async move {
                            match client.get_album(album.id).await {
                                Err(e) => QueueSongCmd::InsertChildrenBelow(Err(e)),
                                Ok(album) if y < f64::from(widget_height) * 0.5f64 => {
                                    QueueSongCmd::InsertChildrenAbove(Ok((index, album.song)))
                                }
                                Ok(album) => {
                                    QueueSongCmd::InsertChildrenAbove(Ok((index, album.song)))
                                }
                            }
                        });
                        vec![]
                    }
                };
                if y < f64::from(widget_height) * 0.5f64 {
                    sender
                        .output(QueueSongOut::DropAbove {
                            src: songs,
                            dest: self.index.clone(),
                        })
                        .unwrap();
                } else {
                    sender
                        .output(QueueSongOut::DropBelow {
                            src: songs,
                            dest: self.index.clone(),
                        })
                        .unwrap();
                }
            }
            QueueSongIn::MoveSong { index, y } => {
                sender.input(QueueSongIn::DragLeave);

                let widget_height = self.root_widget.height();
                if y < f64::from(widget_height) * 0.5f64 {
                    sender
                        .output(QueueSongOut::MoveAbove {
                            src: index.0.clone(),
                            dest: self.index.clone(),
                        })
                        .unwrap();
                } else {
                    sender
                        .output(QueueSongOut::MoveBelow {
                            src: index.0.clone(),
                            dest: self.index.clone(),
                        })
                        .unwrap();
                }
            }
            QueueSongIn::Cover(msg) => match msg {
                CoverOut::DisplayToast(title) => {
                    sender.output(QueueSongOut::DisplayToast(title)).unwrap();
                }
            },
            QueueSongIn::FavoriteClicked => match self.favorited.icon_name().as_deref() {
                Some("starred-symbolic") => sender
                    .output(QueueSongOut::FavoriteClicked(self.info.id.clone(), false))
                    .unwrap(),
                Some("non-starred-symbolic") => sender
                    .output(QueueSongOut::FavoriteClicked(self.info.id.clone(), true))
                    .unwrap(),
                name => unimplemented!("unkonwn icon name: {name:?}"),
            },
            QueueSongIn::FavoriteSong(id, true) if id == self.info.id => {
                self.info.starred = Some(chrono::Utc::now().into());
                self.favorited.set_icon_name("starred-symbolic");
            }
            QueueSongIn::FavoriteSong(id, false) if id == self.info.id => {
                self.info.starred = None;
                self.favorited.set_icon_name("non-starred-symbolic");
            }
            QueueSongIn::FavoriteSong(_, _) => {}
        }
    }

    fn update_cmd(&mut self, message: Self::CommandOutput, sender: relm4::FactorySender<Self>) {
        match message {
            QueueSongCmd::InsertChildrenAbove(Err(e))
            | QueueSongCmd::InsertChildrenBelow(Err(e)) => sender
                .output(QueueSongOut::DisplayToast(format!(
                    "moving song failed: {e}",
                )))
                .unwrap(),
            QueueSongCmd::InsertChildrenAbove(Ok((index, songs))) => {
                sender
                    .output(QueueSongOut::DropAbove {
                        src: songs,
                        dest: index,
                    })
                    .unwrap();
            }
            QueueSongCmd::InsertChildrenBelow(Ok((index, songs))) => {
                sender
                    .output(QueueSongOut::DropBelow {
                        src: songs,
                        dest: index,
                    })
                    .unwrap();
            }
        }
    }
}
