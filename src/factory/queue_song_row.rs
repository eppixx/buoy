use std::{cell::RefCell, rc::Rc};

use gettextrs::gettext;
use relm4::{
    gtk::{
        self, pango,
        prelude::{BoxExt, ButtonExt, OrientableExt, ToValue, WidgetExt},
    },
    Component, ComponentController, RelmWidgetExt,
};

use crate::{
    common,
    components::{
        cover::{Cover, CoverIn},
        queue::{Queue, QueueIn},
    },
    css::DragState,
    gtk_helper::stack::StackExt,
    play_state::PlayState,
    subsonic::Subsonic,
    types::Droppable,
};

static UID: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QueueUid {
    pub uid: usize,
    pub child: submarine::data::Child,
}

#[derive(Debug, Clone)]
pub struct QueueSongRow {
    subsonic: Rc<RefCell<Subsonic>>,
    item: submarine::data::Child,
    uid: usize,
    sender: relm4::ComponentSender<Queue>,
    play_state: PlayState,
    cover_stack: Option<gtk::Stack>,
    fav_btn: Option<gtk::Button>,
}

impl QueueSongRow {
    pub fn new(
        subsonic: &Rc<RefCell<Subsonic>>,
        child: &submarine::data::Child,
        sender: &relm4::ComponentSender<Queue>,
    ) -> Self {
        let uid = UID.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Self {
            subsonic: subsonic.clone(),
            item: child.clone(),
            uid,
            sender: sender.clone(),
            play_state: PlayState::Stop,
            cover_stack: None,
            fav_btn: None,
        }
    }

    pub fn item(&self) -> &submarine::data::Child {
        &self.item
    }

    pub fn item_mut(&mut self) -> &mut submarine::data::Child {
        &mut self.item
    }

    pub fn uid(&self) -> &usize {
        &self.uid
    }

    pub fn fav_btn(&self) -> &Option<gtk::Button> {
        &self.fav_btn
    }

    pub fn set_play_state(&mut self, state: &PlayState) {
        if let Some(stack) = &self.cover_stack {
            stack.set_visible_child_enum(state);
        }
        self.play_state = state.clone();
    }

    pub fn play_state(&self) -> &PlayState {
        &self.play_state
    }

    pub fn activate(&mut self) {
        self.set_play_state(&PlayState::Play);
        self.sender.input(QueueIn::ActivateUid(self.uid));
    }

    pub fn add_drag_indicator_top(&self) {
        if let Some(fav_btn) = &self.fav_btn {
            if let Some(list_item) = super::get_list_item_widget(fav_btn) {
                DragState::drop_shadow_top(&list_item);
            }
        }
    }

    pub fn add_drag_indicator_bottom(&self) {
        if let Some(fav_btn) = &self.fav_btn {
            if let Some(list_item) = super::get_list_item_widget(fav_btn) {
                DragState::drop_shadow_bottom(&list_item);
            }
        }
    }

    pub fn reset_drag_indicators(&self) {
        if let Some(fav_btn) = &self.fav_btn {
            if let Some(list_item) = super::get_list_item_widget(fav_btn) {
                DragState::reset(&list_item);
            }
        }
    }
}

pub struct Model {
    subsonic: Rc<RefCell<Option<Rc<RefCell<Subsonic>>>>>,
    sender: Rc<RefCell<Option<relm4::ComponentSender<Queue>>>>,
    child: Rc<RefCell<Option<submarine::data::Child>>>,
    uid: Rc<RefCell<Option<usize>>>,
    drag_src: gtk::DragSource,

    cover_stack: gtk::Stack,
    cover: Option<relm4::Controller<Cover>>,
    title: gtk::Label,
    artist: gtk::Label,
    length: gtk::Label,
    fav_btn: gtk::Button,
}

impl Model {
    fn set_from_row(&self, row: &QueueSongRow) {
        self.subsonic.replace(Some(row.subsonic.clone()));
        self.child.replace(Some(row.item.clone()));
        self.sender.replace(Some(row.sender.clone()));
        self.uid.replace(Some(row.uid));

        let drop = Droppable::QueueSongs(vec![QueueUid {
            uid: row.uid,
            child: row.item.clone(),
        }]);
        let content = gtk::gdk::ContentProvider::for_value(&drop.to_value());
        self.drag_src.set_content(Some(&content));
    }
}

impl relm4::typed_view::list::RelmListItem for QueueSongRow {
    type Root = gtk::Viewport;
    type Widgets = (Model, gtk::Viewport);

    fn setup(_item: &gtk::ListItem) -> (Self::Root, Self::Widgets) {
        relm4::view! {
            my_box = gtk::Box {
                set_spacing: 10,

                append: cover_stack = &gtk::Stack {
                    set_transition_type: gtk::StackTransitionType::Crossfade,

                    add_enumed[PlayState::Stop]: cover_box = &gtk::Viewport,
                    add_enumed[PlayState::Play] = &gtk::Image {
                        add_css_class: "bordered",
                        set_icon_name: Some("audio-volume-high-symbolic"),
                    },
                    add_enumed[PlayState::Pause] = &gtk::Image {
                        add_css_class: "bordered",
                        set_icon_name: Some("media-playback-pause-symbolic"),
                    },
                },

                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,

                    append: title = &gtk::Label {
                        set_text: "no title given",
                        set_hexpand: true,
                        set_width_chars: 3,
                        set_hexpand: true,
                        set_halign: gtk::Align::Start,
                        set_ellipsize: pango::EllipsizeMode::End,
                    },
                    append: artist = &gtk::Label {
                        set_text: "no artist given",
                        set_width_chars: 3,
                        set_hexpand: true,
                        set_halign: gtk::Align::Start,
                        set_ellipsize: pango::EllipsizeMode::End,
                    }
                },
                append: length = &gtk::Label,
                append: fav_btn = &gtk::Button {
                    set_tooltip: &gettext("Click to (un)favorite song"),
                    set_focus_on_click: false,
                },
            }
        }

        let model = Model {
            subsonic: Rc::new(RefCell::new(None)),
            sender: Rc::new(RefCell::new(None)),
            uid: Rc::new(RefCell::new(None)),
            child: Rc::new(RefCell::new(None)),
            drag_src: gtk::DragSource::default(),
            cover_stack,
            cover: None,
            title,
            artist,
            length,
            fav_btn,
        };

        // create DragSource
        model.drag_src.set_actions(gtk::gdk::DragAction::MOVE);

        // set drag icon
        let subsonic = model.subsonic.clone();
        let child = model.child.clone();
        model.drag_src.connect_drag_begin(move |src, _drag| {
            let Some(ref subsonic) = *subsonic.borrow() else {
                return;
            };
            let Some(ref child) = *child.borrow() else {
                return;
            };

            if let Some(cover_id) = &child.cover_art {
                let cover = subsonic.borrow().cover_icon(cover_id);
                if let Some(tex) = cover {
                    src.set_icon(Some(&tex), 0, 0);
                }
            }
        });

        let root = gtk::Viewport::default();
        root.add_controller(model.drag_src.clone());
        root.set_child(Some(&my_box));
        (root, (model, cover_box))
    }

    fn bind(&mut self, (widgets, cover_box): &mut Self::Widgets, _root: &mut Self::Root) {
        // set cover if not set
        match &widgets.cover {
            None => {
                let cover = Cover::builder()
                    .launch((self.subsonic.clone(), self.item.cover_art.clone()))
                    .forward(self.sender.input_sender(), QueueIn::Cover);
                cover.model().add_css_class_image("size32");
                cover_box.set_child(Some(cover.widget()));
                cover.emit(CoverIn::LoadSong(Box::new(self.item.clone())));
                widgets.cover = Some(cover);
            }
            Some(cover) => cover.emit(CoverIn::LoadSong(Box::new(self.item.clone()))),
        }
        widgets.set_from_row(self);

        // set labels and button
        widgets.title.set_label(&self.item.title);
        widgets.artist.set_label(
            self.item
                .artist
                .as_deref()
                .unwrap_or(&gettext("Unkonwn Artist")),
        );
        let length = common::convert_for_label(i64::from(self.item.duration.unwrap_or(0)) * 1000);
        widgets.length.set_label(&length);
        match self.item.starred.is_some() {
            true => widgets.fav_btn.set_icon_name("starred-symbolic"),
            false => widgets.fav_btn.set_icon_name("non-starred-symbolic"),
        }

        //TODO connect fav_btn

        self.cover_stack = Some(widgets.cover_stack.clone());
        self.fav_btn = Some(widgets.fav_btn.clone());
    }

    fn unbind(&mut self, (_widgets, _cover_box): &mut Self::Widgets, _root: &mut Self::Root) {
        self.cover_stack = None;
        self.fav_btn = None;
    }
}
