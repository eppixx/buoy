use std::{cell::RefCell, rc::Rc};

use relm4::{
    gtk::{
        self,
        prelude::{BoxExt, ToValue, WidgetExt},
    },
    RelmObjectExt,
};

use crate::{common::convert_for_label, subsonic::Subsonic, types::Droppable};

#[derive(Debug)]
pub struct PlaylistTracksRow {
    subsonic: Rc<RefCell<Subsonic>>,
    pub item: submarine::data::Child,
    pub fav: relm4::binding::StringBinding,
}

impl PartialEq for PlaylistTracksRow {
    fn eq(&self, other: &Self) -> bool {
        self.item == other.item
    }
}

impl PlaylistTracksRow {
    pub fn new(subsonic: &Rc<RefCell<Subsonic>>, item: submarine::data::Child) -> Self {
        let fav = match item.starred.is_some() {
            true => String::from("starred-symbolic"),
            false => String::from("non-starred-symbolic"),
        };
        Self {
            subsonic: subsonic.clone(),
            item,
            fav: relm4::binding::StringBinding::new(fav),
        }
    }

    fn get_drag_src(&self) -> gtk::DragSource {
        let src = gtk::DragSource::default();
        let drop = Droppable::Child(Box::new(self.item.clone()));
        let content = gtk::gdk::ContentProvider::for_value(&drop.to_value());
        src.set_content(Some(&content));
        src.set_actions(gtk::gdk::DragAction::MOVE);

        let album = self.subsonic.borrow().album_of_song(&self.item);
        let subsonic = self.subsonic.clone();
        src.connect_drag_begin(move |src, _drag| {
            if let Some(album) = &album {
                if let Some(cover_id) = &album.cover_art {
                    let cover = subsonic.borrow().cover_icon(cover_id);
                    if let Some(tex) = cover {
                        src.set_icon(Some(&tex), 0, 0);
                    }
                }
            }
        });
        src
    }
}

pub struct PositionColumn;

impl relm4::typed_view::column::RelmColumn for PositionColumn {
    type Root = gtk::Box;
    type Item = PlaylistTracksRow;
    type Widgets = gtk::Label;

    const COLUMN_NAME: &'static str = "#";
    const ENABLE_RESIZE: bool = false;
    const ENABLE_EXPAND: bool = false;

    fn setup(_item: &gtk::ListItem) -> (Self::Root, Self::Widgets) {
        let b = gtk::Box::default();
        let label = gtk::Label::builder().build();
        b.append(&label);
        (b, (label))
    }

    fn bind(item: &mut Self::Item, label: &mut Self::Widgets, b: &mut Self::Root) {
        let mut text = String::new();
        if let Some(cd) = item.item.disc_number {
            text.push_str(&cd.to_string());
            text.push('.');
        }
        if let Some(track) = item.item.track {
            text = format!("{text}{track:02}")
        }
        label.set_label(&text);
        b.add_controller(item.get_drag_src());
    }

    fn sort_fn() -> relm4::typed_view::OrdFn<Self::Item> {
        Some(Box::new(|a, b| a.item.track.cmp(&b.item.track)))
    }
}

pub struct TitleColumn;

impl relm4::typed_view::column::RelmColumn for TitleColumn {
    type Root = gtk::Box;
    type Item = PlaylistTracksRow;
    type Widgets = gtk::Label;

    const COLUMN_NAME: &'static str = "Title";
    const ENABLE_RESIZE: bool = true;
    const ENABLE_EXPAND: bool = true;

    fn setup(_item: &gtk::ListItem) -> (Self::Root, Self::Widgets) {
        let b = gtk::Box::default();
        let label = gtk::Label::builder()
            .halign(gtk::Align::Start)
            .ellipsize(gtk::pango::EllipsizeMode::End)
            .build();
        b.set_hexpand(true);
        b.append(&label);
        (b, (label))
    }

    fn bind(item: &mut Self::Item, label: &mut Self::Widgets, b: &mut Self::Root) {
        label.set_label(&item.item.title);
        b.add_controller(item.get_drag_src());
    }

    fn sort_fn() -> relm4::typed_view::OrdFn<Self::Item> {
        Some(Box::new(|a, b| a.item.title.cmp(&b.item.title)))
    }
}

pub struct ArtistColumn;

impl relm4::typed_view::column::RelmColumn for ArtistColumn {
    type Root = gtk::Box;
    type Item = PlaylistTracksRow;
    type Widgets = gtk::Label;

    const COLUMN_NAME: &'static str = "Artist";
    const ENABLE_RESIZE: bool = true;
    const ENABLE_EXPAND: bool = true;

    fn setup(_item: &gtk::ListItem) -> (Self::Root, Self::Widgets) {
        let b = gtk::Box::default();
        let label = gtk::Label::builder()
            .halign(gtk::Align::Start)
            .ellipsize(gtk::pango::EllipsizeMode::End)
            .build();
        b.set_hexpand(true);
        b.append(&label);
        (b, (label))
    }

    fn bind(item: &mut Self::Item, label: &mut Self::Widgets, b: &mut Self::Root) {
        label.set_label(item.item.artist.as_deref().unwrap_or("Unknown Artist"));
        b.add_controller(item.get_drag_src());
    }

    fn sort_fn() -> relm4::typed_view::OrdFn<Self::Item> {
        Some(Box::new(|a, b| a.item.artist.cmp(&b.item.artist)))
    }
}

pub struct AlbumColumn;

impl relm4::typed_view::column::RelmColumn for AlbumColumn {
    type Root = gtk::Box;
    type Item = PlaylistTracksRow;
    type Widgets = gtk::Label;

    const COLUMN_NAME: &'static str = "Album";
    const ENABLE_RESIZE: bool = true;
    const ENABLE_EXPAND: bool = true;

    fn setup(_item: &gtk::ListItem) -> (Self::Root, Self::Widgets) {
        let b = gtk::Box::default();
        let label = gtk::Label::builder()
            .halign(gtk::Align::Start)
            .ellipsize(gtk::pango::EllipsizeMode::End)
            .build();
        b.set_hexpand(true);
        b.append(&label);
        (b, (label))
    }

    fn bind(item: &mut Self::Item, label: &mut Self::Widgets, b: &mut Self::Root) {
        label.set_label(item.item.album.as_deref().unwrap_or("Unknown Album"));
        b.add_controller(item.get_drag_src());
    }

    fn sort_fn() -> relm4::typed_view::OrdFn<Self::Item> {
        Some(Box::new(|a, b| a.item.album.cmp(&b.item.album)))
    }
}

pub struct LengthColumn;

impl relm4::typed_view::column::RelmColumn for LengthColumn {
    type Root = gtk::Box;
    type Item = PlaylistTracksRow;
    type Widgets = gtk::Label;

    const COLUMN_NAME: &'static str = "Length";
    const ENABLE_RESIZE: bool = false;
    const ENABLE_EXPAND: bool = false;

    fn setup(_item: &gtk::ListItem) -> (Self::Root, Self::Widgets) {
        let b = gtk::Box::default();
        let label = gtk::Label::default();
        b.set_hexpand(true);
        b.append(&label);
        (b, (label))
    }

    fn bind(item: &mut Self::Item, label: &mut Self::Widgets, b: &mut Self::Root) {
        let length = convert_for_label(i64::from(item.item.duration.unwrap_or(0)) * 1000);
        label.set_label(&length);
        b.add_controller(item.get_drag_src());
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
        image.add_write_only_binding(&item.fav, "icon_name");
    }

    fn sort_fn() -> relm4::typed_view::OrdFn<Self::Item> {
        Some(Box::new(|a, b| a.item.album.cmp(&b.item.album)))
    }
}
