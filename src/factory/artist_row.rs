use std::{cell::RefCell, rc::Rc};

use granite::prelude::ToValue;
use relm4::{
    gtk::{
        self,
        prelude::{BoxExt, WidgetExt, ButtonExt},
    },
    Component, ComponentController, RelmObjectExt,
};

use crate::{
    components::{artists_view::{ArtistsView, ArtistsViewIn}, cover::Cover},
    subsonic::Subsonic,
    types::Droppable,
};

#[derive(Debug)]
pub struct ArtistRow {
    pub subsonic: Rc<RefCell<Subsonic>>,
    pub item: submarine::data::ArtistId3,
    pub fav: relm4::binding::StringBinding,
    pub cover: relm4::Controller<Cover>,
}

impl PartialEq for ArtistRow {
    fn eq(&self, other: &Self) -> bool {
        self.item == other.item
    }
}

impl ArtistRow {
    pub fn new(
        subsonic: &Rc<RefCell<Subsonic>>,
        item: submarine::data::ArtistId3,
        sender: relm4::ComponentSender<ArtistsView>,
    ) -> Self {
        let fav = match item.starred.is_some() {
            true => String::from("starred-symbolic"),
            false => String::from("non-starred-symbolic"),
        };

        let cover = Cover::builder()
            .launch((subsonic.clone(), item.cover_art.clone()))
            .forward(sender.input_sender(), ArtistsViewIn::Cover);
        cover.model().change_size(75);

        Self {
            subsonic: subsonic.clone(),
            item,
            fav: relm4::binding::StringBinding::new(fav),
            cover,
        }
    }

    fn get_drag_src(&self) -> gtk::DragSource {
        let src = gtk::DragSource::default();
        let drop = Droppable::Artist(Box::new(self.item.clone()));
        let content = gtk::gdk::ContentProvider::for_value(&drop.to_value());
        src.set_content(Some(&content));
        src.set_actions(gtk::gdk::DragAction::MOVE);

        let artist_art = self.item.cover_art.clone();
        let subsonic = self.subsonic.clone();
        src.connect_drag_begin(move |src, _drag| {
            if let Some(cover_id) = &artist_art {
                let cover = subsonic.borrow().cover_icon(cover_id);
                if let Some(tex) = cover {
                    src.set_icon(Some(&tex), 0, 0);
                }
            }
        });
        src
    }
}

pub struct CoverColumn;

impl relm4::typed_view::column::RelmColumn for CoverColumn {
    type Root = gtk::Viewport;
    type Item = ArtistRow;
    type Widgets = ();

    const COLUMN_NAME: &'static str = "Cover";
    const ENABLE_RESIZE: bool = false;
    const ENABLE_EXPAND: bool = false;

    fn setup(_item: &gtk::ListItem) -> (Self::Root, Self::Widgets) {
        let view = gtk::Viewport::default();
        view.set_margin_end(12);
        (view, ())
    }

    fn bind(item: &mut Self::Item, _: &mut Self::Widgets, view: &mut Self::Root) {
        view.add_controller(item.get_drag_src());
        view.set_child(Some(item.cover.widget()));
    }
}

pub struct TitleColumn;

impl relm4::typed_view::column::RelmColumn for TitleColumn {
    type Root = gtk::Button;
    type Item = ArtistRow;
    type Widgets = gtk::Label;

    const COLUMN_NAME: &'static str = "Name";
    const ENABLE_RESIZE: bool = true;
    const ENABLE_EXPAND: bool = true;

    fn setup(_item: &gtk::ListItem) -> (Self::Root, Self::Widgets) {
        let btn = gtk::Button::default();
        let label = gtk::Label::builder()
            .halign(gtk::Align::Start)
            .ellipsize(gtk::pango::EllipsizeMode::End)
            .build();
        btn.set_hexpand(true);
        btn.add_css_class(granite::STYLE_CLASS_FLAT);
        btn.set_focusable(false);
        btn.set_child(Some(&label));
        (btn, (label))
    }

    fn bind(item: &mut Self::Item, label: &mut Self::Widgets, btn: &mut Self::Root) {
        label.set_label(&item.item.name);
        btn.add_controller(item.get_drag_src());
    }

    fn sort_fn() -> relm4::typed_view::OrdFn<Self::Item> {
        Some(Box::new(|a, b| a.item.name.cmp(&b.item.name)))
    }
}

pub struct AlbumCountColumn;

impl relm4::typed_view::column::RelmColumn for AlbumCountColumn {
    type Root = gtk::Box;
    type Item = ArtistRow;
    type Widgets = gtk::Label;

    const COLUMN_NAME: &'static str = "Albums";
    const ENABLE_RESIZE: bool = false;
    const ENABLE_EXPAND: bool = false;

    fn setup(_item: &gtk::ListItem) -> (Self::Root, Self::Widgets) {
        let b = gtk::Box::default();
        let label = gtk::Label::builder().build();
        b.append(&label);
        (b, (label))
    }

    fn bind(item: &mut Self::Item, label: &mut Self::Widgets, b: &mut Self::Root) {
        label.set_label(&item.item.album_count.to_string());
        b.add_controller(item.get_drag_src());
    }

    fn sort_fn() -> relm4::typed_view::OrdFn<Self::Item> {
        Some(Box::new(|a, b| b.item.album_count.cmp(&a.item.album_count)))
    }
}


pub struct FavColumn;

impl relm4::typed_view::column::RelmColumn for FavColumn {
    type Root = gtk::Image;
    type Item = ArtistRow;
    type Widgets = ();

    const COLUMN_NAME: &'static str = "Favorite";
    const ENABLE_RESIZE: bool = false;

    fn setup(_item: &gtk::ListItem) -> (Self::Root, Self::Widgets) {
        (gtk::Image::from_icon_name("non-starred"), ())
    }

    fn bind(item: &mut Self::Item, _: &mut Self::Widgets, image: &mut Self::Root) {
        image.add_write_only_binding(&item.fav, "icon_name");
    }

    fn sort_fn() -> relm4::typed_view::OrdFn<Self::Item> {
        Some(Box::new(|a, b| b.item.starred.cmp(&a.item.starred)))
    }
}
