use relm4::{
    gtk::{
        self, gdk, glib, pango,
        prelude::ToValue,
        traits::{
            BoxExt, ButtonExt, EventControllerExt, GestureSingleExt, ListBoxRowExt, OrientableExt,
            WidgetExt,
        },
    },
    prelude::{DynamicIndex, FactoryComponent},
    Component, ComponentController, FactorySender, RelmWidgetExt,
};

use crate::{
    client::Client,
    components::{
        cover::{Cover, CoverIn},
        queue::QueueIn,
        seekbar,
    },
    css::DragState,
    play_state::PlayState,
    types::Id,
};

#[derive(Clone, Debug, PartialEq, Eq, glib::Boxed)]
#[boxed_type(name = "QueueSongIndex")]
struct Index(DynamicIndex);

#[derive(Debug)]
pub enum QueueSongIn {
    Activated,
    DraggedOver(f64),
    DragDropped {
        src: DynamicIndex,
        dest: DynamicIndex,
        y: f64,
    },
    DragLeave,
    NewState(PlayState),
    StarredClicked,
    DroppedItem {
        value: glib::Value,
        y: f64,
    },
    LoadSongInfo,
}

#[derive(Debug)]
pub enum QueueSongOut {
    Activated(DynamicIndex, Box<submarine::data::Child>),
    Clicked(DynamicIndex),
    ShiftClicked(DynamicIndex),
    Remove,
    DropAbove {
        src: DynamicIndex,
        dest: DynamicIndex,
    },
    DropBelow {
        src: DynamicIndex,
        dest: DynamicIndex,
    },
}

#[derive(Debug)]
pub struct QueueSong {
    root_widget: gtk::ListBoxRow,
    info: Option<submarine::data::Child>,
    id: Id,
    cover: relm4::Controller<Cover>,
    playing: PlayState,
    title: String,
    artist: String,
    length: i64, //length of song in ms
    favorited: bool,
    index: DynamicIndex,
    sender: FactorySender<Self>,
    drag_src: gtk::DragSource,
}

impl QueueSong {
    pub fn new_play_state(&self, state: PlayState) -> (Option<DynamicIndex>, Option<Id>) {
        self.sender.input(QueueSongIn::NewState(state.clone()));
        match state {
            PlayState::Play => (Some(self.index.clone()), Some(self.id.clone())),
            PlayState::Pause => (Some(self.index.clone()), None),
            PlayState::Stop => (None, None),
        }
    }

    pub fn root_widget(&self) -> &gtk::ListBoxRow {
        &self.root_widget
    }
}

#[derive(Debug)]
pub enum QueueItemCmd {
    LoadedTrack(Box<Option<submarine::data::Child>>),
    Favorited(Option<bool>), //None when request not successful
}

#[relm4::factory(pub)]
impl FactoryComponent for QueueSong {
    type ParentWidget = gtk::ListBox;
    type ParentInput = QueueIn;
    type CommandOutput = QueueItemCmd;
    type Input = QueueSongIn;
    type Output = QueueSongOut;
    type Init = Id;
    type Widgets = QueueSongWidgets;

    fn init_model(id: Self::Init, index: &DynamicIndex, sender: FactorySender<Self>) -> Self {
        let mut model = Self {
            root_widget: gtk::ListBoxRow::new(),
            info: None,
            id,
            cover: Cover::builder().launch(()).detach(),
            playing: PlayState::Stop,
            title: String::from("song"),
            artist: String::from("Unknown Artist"),
            length: 0,
            favorited: false,
            index: index.clone(),
            sender: sender.clone(),
            drag_src: gtk::DragSource::new(),
        };

        sender.input(QueueSongIn::LoadSongInfo);

        DragState::reset(&mut model.root_widget);

        // setup DragSource
        let index = Index(index.clone());
        let content = gdk::ContentProvider::for_value(&index.to_value());
        model.drag_src.set_content(Some(&content));
        model.drag_src.set_actions(gdk::DragAction::MOVE);

        model
    }

    fn forward_to_parent(output: Self::Output) -> Option<QueueIn> {
        match output {
            QueueSongOut::Activated(index, info) => Some(QueueIn::Activated(index, info)),
            QueueSongOut::Clicked(index) => Some(QueueIn::Clicked(index)),
            QueueSongOut::ShiftClicked(index) => Some(QueueIn::ShiftClicked(index)),
            QueueSongOut::Remove => Some(QueueIn::Remove),
            QueueSongOut::DropAbove { src, dest } => Some(QueueIn::DropAbove { src, dest }),
            QueueSongOut::DropBelow { src, dest } => Some(QueueIn::DropBelow { src, dest }),
        }
    }

    fn update(&mut self, message: Self::Input, sender: FactorySender<Self>) {
        match message {
            QueueSongIn::Activated => {
                if let Some(info) = &self.info {
                    self.new_play_state(PlayState::Play);
                    sender.output(QueueSongOut::Activated(
                        self.index.clone(),
                        Box::new(info.clone()),
                    ));
                }
            }
            QueueSongIn::DraggedOver(y) => {
                let widget_height = self.root_widget.height();
                if y < widget_height as f64 * 0.5f64 {
                    DragState::drop_shadow_top(&mut self.root_widget);
                } else {
                    DragState::drop_shadow_bottom(&mut self.root_widget);
                }
            }
            QueueSongIn::DragDropped { src, dest, y } => {
                let widget_height = self.root_widget.height();
                if y < widget_height as f64 * 0.5f64 {
                    sender.output(QueueSongOut::DropAbove { src, dest });
                } else {
                    sender.output(QueueSongOut::DropBelow { src, dest });
                }
            }
            QueueSongIn::DragLeave => DragState::reset(&mut self.root_widget),
            QueueSongIn::NewState(state) => self.playing = state,
            QueueSongIn::StarredClicked => {
                let id = self.id.clone();
                let favorite = self.favorited;
                sender.oneshot_command(async move {
                    let client = Client::get().lock().unwrap().inner.clone().unwrap();
                    let empty: Vec<&str> = vec![];
                    let result = match favorite {
                        true => client.unstar(vec![id.inner()], empty.clone(), empty).await,
                        false => client.star(vec![id.inner()], empty.clone(), empty).await,
                    };
                    match result {
                        Ok(_) => QueueItemCmd::Favorited(Some(!favorite)),
                        Err(_) => QueueItemCmd::Favorited(None),
                    }
                });
            }
            QueueSongIn::DroppedItem { value, y } => {
                sender.input(QueueSongIn::DragLeave);

                // drop is a index
                if let Ok(src_index) = value.get::<Index>() {
                    sender.input(QueueSongIn::DragDropped {
                        src: src_index.0.clone(),
                        dest: self.index.clone(),
                        y,
                    });
                }

                // drop is a id
                if let Ok(id) = value.get::<Id>() {
                    todo!();
                    // return true;
                }
            }
            QueueSongIn::LoadSongInfo => {
                let id = self.id.clone();
                sender.oneshot_command(async move {
                    let client = Client::get().lock().unwrap().inner.clone().unwrap();
                    match client.get_song(id.inner()).await {
                        Ok(child) => QueueItemCmd::LoadedTrack(Box::new(Some(child))),
                        Err(_) => QueueItemCmd::LoadedTrack(Box::new(None)),
                    }
                });
            }
        }
    }

    view! {
        self.root_widget.clone() -> gtk::ListBoxRow {
            add_css_class: "queue-song",

            gtk::Box {
                set_spacing: 10,

                #[transition = "Crossfade"]
                append = match self.playing {
                    PlayState::Play => {
                        gtk::Image {
                            add_css_class: "size50",
                            set_icon_name: Some("audio-volume-high-symbolic"),
                        }
                    }
                    PlayState::Pause => {
                        gtk::Image {
                            add_css_class: "size50",
                            set_icon_name: Some("media-playback-pause-symbolic"),
                        }
                    }
                    PlayState::Stop => {
                        &self.cover.widget().clone() {
                            add_css_class: "size50",
                            add_css_class: "cover",
                        }
                    }
                },

                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_valign: gtk::Align::Center,

                    gtk::Label {
                        #[watch]
                        set_label: &self.title,
                        set_width_chars: 3,
                        set_hexpand: true,
                        set_halign: gtk::Align::Start,
                        set_ellipsize: pango::EllipsizeMode::End,
                    },
                    gtk:: Label {
                        #[watch]
                        set_markup: &format!("<span style=\"italic\">{}</span>", self.artist),
                        set_width_chars: 3,
                        set_hexpand: true,
                        set_halign: gtk::Align::Start,
                        set_ellipsize: pango::EllipsizeMode::End,
                    }
                },

                gtk::Label {
                    #[watch]
                    set_label: &seekbar::convert_for_label(self.length),
                },

                #[transition = "Crossfade"]
                if self.favorited {
                    gtk::Button {
                        set_icon_name: "starred",
                        set_tooltip: "Click to unfavorite song",
                        set_focus_on_click: false,
                        connect_clicked => QueueSongIn::StarredClicked,
                    }
                } else {
                    gtk::Button {
                        set_icon_name: "non-starred",
                        set_tooltip: "Click to favorite song",
                        set_focus_on_click: false,
                        connect_clicked => QueueSongIn::StarredClicked,
                    }
                },
            },

            // make item draggable
            add_controller: self.drag_src.clone(),

            // activate is when pressed enter on item
            connect_activate => QueueSongIn::Activated,

            // accept drop from queue items and id's and render drop indicators
            add_controller = gtk::DropTarget {
                set_actions: gdk::DragAction::MOVE,
                set_types: &[<Index as gtk::prelude::StaticType>::static_type(),
                             <Id as gtk::prelude::StaticType>::static_type(),
                ],

                connect_drop[sender] => move |_target, value, _x, y| {
                    sender.input(QueueSongIn::DroppedItem {
                        value: value.clone(),
                        y,
                    });
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
                connect_pressed[sender, index] => move |_widget, n, _x, _y|{
                    if n == 1 {
                        let state = _widget.current_event_state();
                        if !(state.contains(gdk::ModifierType::SHIFT_MASK)
                             || state.contains(gdk::ModifierType::CONTROL_MASK) ) {
                            // normal click
                            sender.output(QueueSongOut::Clicked(index.clone()));
                        } else if state.contains(gdk::ModifierType::SHIFT_MASK) {
                            // shift click
                            sender.output(QueueSongOut::ShiftClicked(index.clone()));
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
                        sender.output(QueueSongOut::Remove);
                    }
                    gtk::Inhibit(false)
                }
            },
        }
    }

    fn update_cmd(&mut self, message: Self::CommandOutput, _sender: relm4::FactorySender<Self>) {
        match message {
            QueueItemCmd::LoadedTrack(child) => {
                let child = if let Some(child) = *child {
                    child
                } else {
                    return;
                };

                // settings song data
                self.title = child.title.clone();
                if let Some(artist) = &child.artist {
                    self.artist = artist.clone();
                }
                if let Some(length) = &child.duration {
                    self.length = *length as i64 * 1000;
                }
                if child.starred.is_some() {
                    self.favorited = true;
                }
                self.cover.emit(CoverIn::LoadImage(child.cover_art.clone()));
                self.info = Some(child);
            }
            QueueItemCmd::Favorited(state) => {
                match state {
                    Some(state) => self.favorited = state,
                    None => {} //TODO some error handling
                }
            }
        }
    }
}
