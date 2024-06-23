use relm4::gtk::{
    self,
    prelude::{BoxExt, ToValue, WidgetExt},
};

use crate::{common::convert_for_label, types::Droppable};

#[derive(Debug, PartialEq, Eq)]
pub struct PlaylistTracksRow {
    pub item: submarine::data::Child,
}
impl PlaylistTracksRow {
    pub fn new(item: submarine::data::Child) -> Self {
        Self { item }
    }

    fn get_drag_src(&self) -> gtk::DragSource {
        let src = gtk::DragSource::default();
        let drop = Droppable::Child(Box::new(self.item.clone()));
        let content = gtk::gdk::ContentProvider::for_value(&drop.to_value());
        src.set_content(Some(&content));
        src.set_actions(gtk::gdk::DragAction::MOVE);
        src
    }
}

pub struct TitleColumn;

impl relm4::typed_view::column::RelmColumn for TitleColumn {
    type Root = gtk::Box;
    type Item = PlaylistTracksRow;
    type Widgets = ();

    const COLUMN_NAME: &'static str = "Title";
    const ENABLE_RESIZE: bool = true;
    const ENABLE_EXPAND: bool = true;

    fn setup(_item: &gtk::ListItem) -> (Self::Root, Self::Widgets) {
        (gtk::Box::default(), ())
    }

    fn bind(item: &mut Self::Item, _: &mut Self::Widgets, b: &mut Self::Root) {
        let label = gtk::Label::builder()
            .halign(gtk::Align::Start)
            .ellipsize(gtk::pango::EllipsizeMode::End)
            .build();
        label.set_label(&item.item.title);
        b.add_controller(item.get_drag_src());
        b.set_hexpand(true);
        b.append(&label);
    }

    fn sort_fn() -> relm4::typed_view::OrdFn<Self::Item> {
        Some(Box::new(|a, b| a.item.title.cmp(&b.item.title)))
    }
}

pub struct ArtistColumn;

impl relm4::typed_view::column::RelmColumn for ArtistColumn {
    type Root = gtk::Box;
    type Item = PlaylistTracksRow;
    type Widgets = ();

    const COLUMN_NAME: &'static str = "Artist";
    const ENABLE_RESIZE: bool = true;
    const ENABLE_EXPAND: bool = true;

    fn setup(_item: &gtk::ListItem) -> (Self::Root, Self::Widgets) {
        (gtk::Box::default(), ())
    }

    fn bind(item: &mut Self::Item, _: &mut Self::Widgets, b: &mut Self::Root) {
        let label = gtk::Label::builder()
            .halign(gtk::Align::Start)
            .ellipsize(gtk::pango::EllipsizeMode::End)
            .build();
        label.set_label(item.item.artist.as_deref().unwrap_or("Unknown Artist"));
        b.add_controller(item.get_drag_src());
        b.set_hexpand(true);
        b.append(&label);
    }

    fn sort_fn() -> relm4::typed_view::OrdFn<Self::Item> {
        Some(Box::new(|a, b| a.item.artist.cmp(&b.item.artist)))
    }
}

pub struct AlbumColumn;

impl relm4::typed_view::column::RelmColumn for AlbumColumn {
    type Root = gtk::Box;
    type Item = PlaylistTracksRow;
    type Widgets = ();

    const COLUMN_NAME: &'static str = "Album";
    const ENABLE_RESIZE: bool = true;
    const ENABLE_EXPAND: bool = true;

    fn setup(_item: &gtk::ListItem) -> (Self::Root, Self::Widgets) {
        (gtk::Box::default(), ())
    }

    fn bind(item: &mut Self::Item, _: &mut Self::Widgets, b: &mut Self::Root) {
        let label = gtk::Label::builder()
            .halign(gtk::Align::Start)
            .ellipsize(gtk::pango::EllipsizeMode::End)
            .build();
        label.set_label(item.item.album.as_deref().unwrap_or("Unknown Album"));
        b.add_controller(item.get_drag_src());
        b.set_hexpand(true);
        b.append(&label);
    }

    fn sort_fn() -> relm4::typed_view::OrdFn<Self::Item> {
        Some(Box::new(|a, b| a.item.album.cmp(&b.item.album)))
    }
}

pub struct LengthColumn;

impl relm4::typed_view::column::RelmColumn for LengthColumn {
    type Root = gtk::Box;
    type Item = PlaylistTracksRow;
    type Widgets = ();

    const COLUMN_NAME: &'static str = "Length";
    const ENABLE_RESIZE: bool = false;

    fn setup(_item: &gtk::ListItem) -> (Self::Root, Self::Widgets) {
        (gtk::Box::default(), ())
    }

    fn bind(item: &mut Self::Item, _: &mut Self::Widgets, b: &mut Self::Root) {
        let label = gtk::Label::default();
        let length = convert_for_label(i64::from(item.item.duration.unwrap_or(0)) * 1000);
        label.set_label(&length);
        b.add_controller(item.get_drag_src());
        b.set_hexpand(true);
        b.append(&label);
    }

    fn sort_fn() -> relm4::typed_view::OrdFn<Self::Item> {
        Some(Box::new(|a, b| a.item.duration.cmp(&b.item.duration)))
    }
}

pub struct FavColumn;

impl relm4::typed_view::column::RelmColumn for FavColumn {
    type Root = gtk::Image;
    type Item = PlaylistTracksRow;
    type Widgets = ();

    const COLUMN_NAME: &'static str = "Favorite";
    const ENABLE_RESIZE: bool = false;

    fn setup(_item: &gtk::ListItem) -> (Self::Root, Self::Widgets) {
        (gtk::Image::from_icon_name("non-starred"), ())
    }

    fn bind(item: &mut Self::Item, _: &mut Self::Widgets, image: &mut Self::Root) {
        if item.item.starred.is_some() {
            image.set_from_icon_name(Some("starred"));
        }
    }

    fn sort_fn() -> relm4::typed_view::OrdFn<Self::Item> {
        Some(Box::new(|a, b| a.item.album.cmp(&b.item.album)))
    }
}
