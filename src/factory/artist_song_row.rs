use std::{cell::RefCell, rc::Rc};

use gettextrs::gettext;
use relm4::{
    gtk::{
        self, pango,
        prelude::{BoxExt, OrientableExt, ToValue, WidgetExt},
    },
    Component, ComponentController,
};

use crate::{
    common::{self, types::Droppable},
    components::{
        artist_view::{ArtistView, ArtistViewIn},
        cover::{Cover, CoverIn},
    },
    subsonic::Subsonic,
};

#[derive(Debug)]
pub struct ArtistSongRow {
    subsonic: Rc<RefCell<Subsonic>>,
    item: submarine::data::Child,
    sender: relm4::ComponentSender<ArtistView>,
    drag_src: Option<gtk::DragSource>,
}

impl ArtistSongRow {
    pub fn new(
        subsonic: &Rc<RefCell<Subsonic>>,
        child: &submarine::data::Child,
        sender: &relm4::ComponentSender<ArtistView>,
    ) -> Self {
        Self {
            subsonic: subsonic.clone(),
            item: child.clone(),
            sender: sender.clone(),
            drag_src: None,
        }
    }

    pub fn item(&self) -> &submarine::data::Child {
        &self.item
    }
}

pub struct Model {
    subsonic: Rc<RefCell<Option<Rc<RefCell<Subsonic>>>>>,
    sender: Rc<RefCell<Option<relm4::ComponentSender<ArtistView>>>>,
    child: Rc<RefCell<Option<submarine::data::Child>>>,
    drag_src: gtk::DragSource,

    cover: Option<relm4::Controller<Cover>>,
    title: gtk::Label,
    artist: gtk::Label,
    length: gtk::Label,
}

impl Model {
    fn set_from_row(&self, row: &mut ArtistSongRow) {
        self.subsonic.replace(Some(row.subsonic.clone()));
        self.child.replace(Some(row.item.clone()));
        self.sender.replace(Some(row.sender.clone()));
        row.drag_src = Some(self.drag_src.clone());

        let drop = Droppable::Child(Box::new(row.item().clone()));
        let content = gtk::gdk::ContentProvider::for_value(&drop.to_value());
        self.drag_src.set_content(Some(&content));
    }
}

impl relm4::typed_view::list::RelmListItem for ArtistSongRow {
    type Root = gtk::Viewport;
    type Widgets = (Model, gtk::Viewport);

    fn setup(_item: &gtk::ListItem) -> (Self::Root, Self::Widgets) {
        relm4::view! {
            my_box = gtk::Box {
                set_spacing: 10,

                append: cover_box = &gtk::Viewport,

                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,

                    append: title = &gtk::Label {
                        set_text: "no title given",
                        set_hexpand: true,
                        set_width_chars: 3,
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
            }
        }

        let model = Model {
            subsonic: Rc::new(RefCell::new(None)),
            sender: Rc::new(RefCell::new(None)),
            child: Rc::new(RefCell::new(None)),
            drag_src: gtk::DragSource::default(),
            cover: None,
            title,
            artist,
            length,
        };

        // create DragSource
        model.drag_src.set_actions(gtk::gdk::DragAction::COPY);

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

            // set drag icon
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
                    .forward(self.sender.input_sender(), ArtistViewIn::Cover);
                cover.model().add_css_class_image("size32");
                cover_box.set_child(Some(cover.widget()));
                cover.emit(CoverIn::LoadSong(Box::new(self.item.clone())));
                widgets.cover = Some(cover);
            }
            Some(cover) => cover.emit(CoverIn::LoadSong(Box::new(self.item.clone()))),
        }
        widgets.set_from_row(self);

        // set labels
        widgets.title.set_label(&self.item.title);
        widgets.artist.set_label(
            self.item
                .artist
                .as_deref()
                .unwrap_or(&gettext("Unkonwn Artist")),
        );
        let length = common::convert_for_label(i64::from(self.item.duration.unwrap_or(0)) * 1000);
        widgets.length.set_label(&length);
    }
}
