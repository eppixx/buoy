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
    common::convert_for_label,
    components::cover::{Cover, CoverIn, CoverOut},
    css::DragState,
    gtk_helper::stack::StackExt,
    play_state::PlayState,
    subsonic::Subsonic,
    types::{Droppable, Id},
};

use super::DropHalf;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QueueIndex(pub relm4::factory::DynamicIndex, pub submarine::data::Child);

#[derive(Debug, Clone)]
pub enum QueueSongIn {
    Activated,
    DraggedOver(f64),
    DragLeave,
    NewState(PlayState),
    DroppedSong { drop: Droppable, y: f64 },
    MoveSong { index: Box<QueueIndex>, y: f64 },
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
    MoveSong {
        src: relm4::factory::DynamicIndex,
        dest: relm4::factory::DynamicIndex,
        half: DropHalf,
    },
    DropSongs {
        src: Vec<submarine::data::Child>,
        dest: relm4::factory::DynamicIndex,
        half: DropHalf,
    },
    DisplayToast(String),
    FavoriteClicked(String, bool),
}

#[derive(Debug)]
pub struct QueueSong {
    subsonic: Rc<RefCell<Subsonic>>,
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

#[relm4::factory(pub)]
impl relm4::factory::FactoryComponent for QueueSong {
    type Init = (Rc<RefCell<Subsonic>>, submarine::data::Child);
    type Input = QueueSongIn;
    type Output = QueueSongOut;
    type ParentWidget = gtk::ListBox;
    type Widgets = QueueSongWidgets;
    type CommandOutput = ();

    fn init_model(
        (subsonic, child): Self::Init,
        index: &relm4::factory::DynamicIndex,
        sender: relm4::factory::FactorySender<Self>,
    ) -> Self {
        let cover = Cover::builder()
            .launch((subsonic.clone(), child.cover_art.clone()))
            .forward(sender.input_sender(), QueueSongIn::Cover);
        cover.model().add_css_class_image("size32");
        cover.emit(CoverIn::LoadSong(Box::new(child.clone())));

        let icon_name = match child.starred {
            Some(_) => "starred-symbolic",
            None => "non-starred-symbolic",
        };

        let model = Self {
            subsonic: subsonic.clone(),
            root_widget: gtk::ListBoxRow::new(),
            info: child,
            cover,
            favorited: gtk::Button::from_icon_name(icon_name),
            index: index.clone(),
            sender: sender.clone(),
            drag_src: gtk::DragSource::new(),
        };

        DragState::reset(&model.root_widget);

        // setup DragSource
        let index = Droppable::QueueSongs(vec![QueueIndex(index.clone(), model.info.clone())]);
        let content = gdk::ContentProvider::for_value(&index.to_value());
        model.drag_src.set_content(Some(&content));
        model.drag_src.set_actions(gdk::DragAction::MOVE);
        let album = subsonic.borrow().album_of_song(&model.info.clone());
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
                            add_css_class: "bordered",
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
                set_types: &[<Droppable as gtk::prelude::StaticType>::static_type()],

                connect_drop[sender] => move |_target, value, _x, y| {
                    if let Ok(drop) = value.get::<Droppable>() {
                        match &drop {
                            Droppable::QueueSongs(songs) => {
                                sender.input(QueueSongIn::MoveSong { index: Box::new(songs[0].clone()), y});
                            }
                            _ => sender.input(QueueSongIn::DroppedSong { drop, y }),
                        }
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
                    DragState::drop_shadow_top(&self.root_widget);
                } else {
                    DragState::drop_shadow_bottom(&self.root_widget);
                }
            }
            QueueSongIn::DragLeave => DragState::reset(&self.root_widget),
            QueueSongIn::NewState(state) => {
                widgets.icon_stack.set_visible_child_enum(&state);
            }
            QueueSongIn::DroppedSong { drop, y } => {
                sender.input(QueueSongIn::DragLeave);

                let songs = match drop {
                    Droppable::Queue(ids) => ids,
                    Droppable::QueueSongs(_songs) => {
                        unreachable!("should be moved instead of dropped")
                    }
                    Droppable::Child(c) => vec![*c],
                    Droppable::AlbumWithSongs(album) => album.song,
                    Droppable::Playlist(playlist) => playlist.entry,
                    Droppable::Album(album) => self.subsonic.borrow().songs_of_album(album.id),
                    Droppable::Artist(artist) => self.subsonic.borrow().songs_of_artist(artist.id),
                    Droppable::ArtistWithAlbums(artist) => artist
                        .album
                        .iter()
                        .flat_map(|album| self.subsonic.borrow().songs_of_album(&album.id))
                        .collect(),
                    Droppable::AlbumChild(album) => self.subsonic.borrow().songs_of_album(album.id),
                    Droppable::PlaylistItems(songs) => {
                        songs.into_iter().map(|song| song.child).collect()
                    }
                };
                let half = DropHalf::calc(self.root_widget.height(), y);
                sender
                    .output(QueueSongOut::DropSongs {
                        src: songs,
                        dest: self.index.clone(),
                        half,
                    })
                    .unwrap();
            }
            QueueSongIn::MoveSong { index, y } => {
                sender.input(QueueSongIn::DragLeave);

                let half = DropHalf::calc(self.root_widget.height(), y);
                sender
                    .output(QueueSongOut::MoveSong {
                        src: index.0.clone(),
                        dest: self.index.clone(),
                        half,
                    })
                    .unwrap();
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
            QueueSongIn::FavoriteSong(_, _) => {} // this song is not changed
        }
    }
}
