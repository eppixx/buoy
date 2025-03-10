use std::{cell::RefCell, rc::Rc};

use gettextrs::gettext;
use relm4::{
    gtk::{
        self, gdk, pango,
        prelude::{BoxExt, ButtonExt, OrientableExt, ToValue, WidgetExt},
    },
    Component, ComponentController, RelmWidgetExt,
};

use crate::{
    common,
    components::{
        cover::{self, Cover, CoverIn},
        queue::{Queue, QueueIn, QueueOut},
    },
    css::DragState,
    gtk_helper::stack::StackExt,
    play_state::PlayState,
    settings::Settings,
    subsonic::Subsonic,
    types::Droppable,
};

use super::DropHalf;

static UID: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QueueUid {
    pub uid: usize,
    pub child: submarine::data::Child,
}

#[derive(Debug)]
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

    //TODO remove
    pub fn activate(&mut self) {
        self.set_play_state(&PlayState::Play);
        self.sender.input(QueueIn::Activate(self.uid as u32));
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
    drop_target: gtk::DropTarget,

    cover_stack: gtk::Stack,
    cover: Option<relm4::Controller<Cover>>,
    title: gtk::Label,
    artist: gtk::Label,
    length: gtk::Label,
    fav_btn: gtk::Button,
    size_group: gtk::SizeGroup,
}

impl Model {
    fn new(
        cover_stack: gtk::Stack,
        title: gtk::Label,
        artist: gtk::Label,
        length: gtk::Label,
        fav_btn: gtk::Button,
    ) -> (gtk::Viewport, Self) {
        let model = Model {
            subsonic: Rc::new(RefCell::new(None)),
            sender: Rc::new(RefCell::new(None)),
            uid: Rc::new(RefCell::new(None)),
            child: Rc::new(RefCell::new(None)),
            drag_src: gtk::DragSource::default(),
            drop_target: gtk::DropTarget::default(),
            cover_stack,
            cover: None,
            title,
            artist,
            length,
            fav_btn,
            size_group: gtk::SizeGroup::new(gtk::SizeGroupMode::Horizontal),
        };

        let root = gtk::Viewport::default();

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

        // set DropTarget
        model
            .drop_target
            .set_actions(gdk::DragAction::MOVE | gdk::DragAction::COPY);
        model
            .drop_target
            .set_types(&[<Droppable as gtk::prelude::StaticType>::static_type()]);
        let sender = model.sender.clone();
        let uid = model.uid.clone();
        let widget = root.clone();
        model
            .drop_target
            .connect_drop(move |_target, value, _x, y| {
                let Some(uid) = *uid.borrow() else {
                    return false;
                };
                let Some(ref sender) = *sender.borrow() else {
                    return false;
                };

                //disable dropping items while searching
                {
                    let settings = Settings::get().lock().unwrap();
                    if settings.search_active && !settings.search_text.is_empty() {
                        sender
                            .output(QueueOut::DisplayToast(gettext(
                                "dropping songs into a playlist is not allowed while searching",
                            )))
                            .unwrap();
                        return false;
                    }
                }

                // check if drop is valid
                if let Ok(drop) = value.get::<Droppable>() {
                    match drop {
                        Droppable::QueueSongs(children) => {
                            for child in children.iter().rev() {
                                let half = DropHalf::calc(widget.height(), y);
                                sender.input(QueueIn::MoveSong {
                                    src: child.uid,
                                    dest: uid,
                                    half,
                                });
                            }
                        }
                        drop => {
                            let half = DropHalf::calc(widget.height(), y);
                            sender.input(QueueIn::InsertSongs {
                                dest: uid,
                                drop,
                                half,
                            });
                        }
                    }
                    return true;
                }
                false
            });

        //sending motion for added indicators
        let sender = model.sender.clone();
        let cell = model.uid.clone();
        model.drop_target.connect_motion(move |_drop, _x, y| {
            let Some(cell) = *cell.borrow() else {
                return gdk::DragAction::empty();
            };
            let Some(ref sender) = *sender.borrow() else {
                return gdk::DragAction::empty();
            };

            sender.input(QueueIn::DraggedOverRow { dest: cell, y });
            gdk::DragAction::MOVE
        });

        //remove indicator on leave
        let sender = model.sender.clone();
        model.drop_target.connect_leave(move |_drop| {
            let Some(ref sender) = *sender.borrow() else {
                return;
            };

            sender.input(QueueIn::DragLeaveRow);
        });

        root.add_controller(model.drop_target.clone());
        root.add_controller(model.drag_src.clone());

        (root, model)
    }

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

        let (view, widgets) = Model::new(cover_stack, title, artist, length, fav_btn);
        view.set_child(Some(&my_box));
        (view, (widgets, cover_box))
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
        widgets.size_group.add_widget(&widgets.length);
        match self.item.starred.is_some() {
            true => widgets.fav_btn.set_icon_name("starred-symbolic"),
            false => widgets.fav_btn.set_icon_name("non-starred-symbolic"),
        }

        //TODO connect fav_btn

        self.cover_stack = Some(widgets.cover_stack.clone());
        self.fav_btn = Some(widgets.fav_btn.clone());
    }

    fn unbind(&mut self, (widgets, _cover_box): &mut Self::Widgets, _root: &mut Self::Root) {
        widgets.size_group.remove_widget(&widgets.length);
        self.cover_stack = None;
        self.fav_btn = None;
    }
}
