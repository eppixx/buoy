use std::{cell::RefCell, rc::Rc};

use granite::prelude::ToValue;
use relm4::{
    gtk::{
        self,
        prelude::{BoxExt, ButtonExt, WidgetExt},
    },
    Component, ComponentController, RelmObjectExt, RelmWidgetExt,
};

use crate::{
    common::convert_for_label,
    components::{
        albums_view::{AlbumsView, AlbumsViewIn, AlbumsViewOut},
        cover::Cover,
    },
    subsonic::Subsonic,
    types::Droppable,
};

#[derive(Debug)]
pub struct AlbumRow {
    pub subsonic: Rc<RefCell<Subsonic>>,
    pub item: submarine::data::Child,
    pub fav: relm4::binding::StringBinding,
    pub cover: relm4::Controller<Cover>,
    artist_label: gtk::Label,
    fav_btn: gtk::Button,
}

impl PartialEq for AlbumRow {
    fn eq(&self, other: &Self) -> bool {
        self.item == other.item
    }
}

impl AlbumRow {
    pub fn new(
        subsonic: Rc<RefCell<Subsonic>>,
        item: submarine::data::Child,
        sender: relm4::ComponentSender<AlbumsView>,
    ) -> Self {
        let fav = match item.starred.is_some() {
            true => String::from("starred-symbolic"),
            false => String::from("non-starred-symbolic"),
        };

        let cover = Cover::builder()
            .launch((subsonic.clone(), item.cover_art.clone()))
            .forward(sender.input_sender(), AlbumsViewIn::Cover);
        cover.model().change_size(75);

        let fav_btn = gtk::Button::from_icon_name(&fav);
        fav_btn.set_tooltip("Click to (un)favorite album");
        fav_btn.set_focus_on_click(false);
        let id = item.id.clone();
        let send = sender.clone();
        fav_btn.connect_clicked(move |btn| match btn.icon_name().as_deref() {
            Some("starred-symbolic") => send
                .output(AlbumsViewOut::FavoriteClicked(id.clone(), false))
                .unwrap(),
            Some("non-starred-symbolic") => send
                .output(AlbumsViewOut::FavoriteClicked(id.clone(), true))
                .unwrap(),
            _ => unreachable!("unkown icon name"),
        });

        let artist = item.artist.as_deref().unwrap_or("Unknown Artist");
        let send = sender.clone();
        let artist_label = gtk::Label::builder()
            .halign(gtk::Align::Start)
            .valign(gtk::Align::Center)
            .ellipsize(gtk::pango::EllipsizeMode::End);
        let artist_label = if let Some(artist_id) = item.artist_id.clone() {
            let artist = gtk::glib::markup_escape_text(artist);
            let artist_label = artist_label
                .label(format!("<a href=\"\">{artist}</a>"))
                .use_markup(true)
                .build();
            artist_label.connect_activate_link(move |_label, _id| {
                send.output(AlbumsViewOut::ClickedArtist(artist_id.clone()))
                    .unwrap();
                gtk::glib::signal::Propagation::Stop
            });
            artist_label
        } else {
            artist_label.label(artist).build()
        };

        Self {
            subsonic: subsonic.clone(),
            item,
            fav: relm4::binding::StringBinding::new(fav),
            cover,
            fav_btn,
            artist_label,
        }
    }

    fn get_drag_src(&self) -> gtk::DragSource {
        let src = gtk::DragSource::default();
        let drop = Droppable::Child(Box::new(self.item.clone()));
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
    type Item = AlbumRow;
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
    type Root = gtk::Box;
    type Item = AlbumRow;
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
        b.add_css_class(granite::STYLE_CLASS_FLAT);
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
    type Root = gtk::Viewport;
    type Item = AlbumRow;
    type Widgets = ();

    const COLUMN_NAME: &'static str = "Artist";
    const ENABLE_RESIZE: bool = true;
    const ENABLE_EXPAND: bool = true;

    fn setup(_item: &gtk::ListItem) -> (Self::Root, Self::Widgets) {
        (gtk::Viewport::default(), ())
    }

    fn bind(item: &mut Self::Item, _: &mut Self::Widgets, view: &mut Self::Root) {
        view.set_child(Some(&item.artist_label));
        view.add_controller(item.get_drag_src());
    }

    fn sort_fn() -> relm4::typed_view::OrdFn<Self::Item> {
        Some(Box::new(|a, b| a.item.artist.cmp(&b.item.artist)))
    }
}

pub struct GenreColumn;

impl relm4::typed_view::column::RelmColumn for GenreColumn {
    type Root = gtk::Box;
    type Item = AlbumRow;
    type Widgets = gtk::Label;

    const COLUMN_NAME: &'static str = "Genre";
    const ENABLE_RESIZE: bool = true;
    const ENABLE_EXPAND: bool = true;

    fn setup(_item: &gtk::ListItem) -> (Self::Root, Self::Widgets) {
        let b = gtk::Box::default();
        let label = gtk::Label::builder()
            .halign(gtk::Align::Start)
            .ellipsize(gtk::pango::EllipsizeMode::End)
            .build();
        b.append(&label);
        (b, (label))
    }

    fn bind(item: &mut Self::Item, label: &mut Self::Widgets, b: &mut Self::Root) {
        label.set_label(item.item.genre.as_deref().unwrap_or("Unknown genre"));
        b.add_controller(item.get_drag_src());
    }

    fn sort_fn() -> relm4::typed_view::OrdFn<Self::Item> {
        Some(Box::new(|a, b| a.item.genre.cmp(&b.item.genre)))
    }
}

pub struct YearColumn;

impl relm4::typed_view::column::RelmColumn for YearColumn {
    type Root = gtk::Box;
    type Item = AlbumRow;
    type Widgets = gtk::Label;

    const COLUMN_NAME: &'static str = "Year";
    const ENABLE_RESIZE: bool = true;
    const ENABLE_EXPAND: bool = true;

    fn setup(_item: &gtk::ListItem) -> (Self::Root, Self::Widgets) {
        let b = gtk::Box::default();
        let label = gtk::Label::builder()
            .halign(gtk::Align::Start)
            .ellipsize(gtk::pango::EllipsizeMode::End)
            .build();
        b.append(&label);
        (b, (label))
    }

    fn bind(item: &mut Self::Item, label: &mut Self::Widgets, b: &mut Self::Root) {
        if let Some(year) = &item.item.year {
            label.set_label(&year.to_string());
        }
        b.add_controller(item.get_drag_src());
    }

    fn sort_fn() -> relm4::typed_view::OrdFn<Self::Item> {
        Some(Box::new(|a, b| a.item.year.cmp(&b.item.year)))
    }
}

pub struct CdColumn;

impl relm4::typed_view::column::RelmColumn for CdColumn {
    type Root = gtk::Box;
    type Item = AlbumRow;
    type Widgets = gtk::Label;

    const COLUMN_NAME: &'static str = "CDs";
    const ENABLE_RESIZE: bool = true;
    const ENABLE_EXPAND: bool = true;

    fn setup(_item: &gtk::ListItem) -> (Self::Root, Self::Widgets) {
        let b = gtk::Box::default();
        let label = gtk::Label::builder()
            .halign(gtk::Align::Start)
            .ellipsize(gtk::pango::EllipsizeMode::End)
            .build();
        b.append(&label);
        (b, (label))
    }

    fn bind(item: &mut Self::Item, label: &mut Self::Widgets, b: &mut Self::Root) {
        if let Some(number) = &item.item.disc_number {
            label.set_label(&number.to_string());
        }
        b.add_controller(item.get_drag_src());
    }

    fn sort_fn() -> relm4::typed_view::OrdFn<Self::Item> {
        Some(Box::new(|a, b| a.item.disc_number.cmp(&b.item.disc_number)))
    }
}

pub struct LengthColumn;

impl relm4::typed_view::column::RelmColumn for LengthColumn {
    type Root = gtk::Box;
    type Item = AlbumRow;
    type Widgets = gtk::Label;

    const COLUMN_NAME: &'static str = "Length";
    const ENABLE_RESIZE: bool = false;
    const ENABLE_EXPAND: bool = false;

    fn setup(_item: &gtk::ListItem) -> (Self::Root, Self::Widgets) {
        let b = gtk::Box::default();
        let label = gtk::Label::default();
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
    type Root = gtk::Viewport;
    type Item = AlbumRow;
    type Widgets = ();

    const COLUMN_NAME: &'static str = "Favorite";
    const ENABLE_RESIZE: bool = false;

    fn setup(_item: &gtk::ListItem) -> (Self::Root, Self::Widgets) {
        let view = gtk::Viewport::default();
        view.set_valign(gtk::Align::Center);
        (view, ())
    }

    fn bind(item: &mut Self::Item, _: &mut Self::Widgets, view: &mut Self::Root) {
        item.fav_btn.add_write_only_binding(&item.fav, "icon_name");
        view.set_child(Some(&item.fav_btn));
    }

    fn sort_fn() -> relm4::typed_view::OrdFn<Self::Item> {
        Some(Box::new(|a, b| b.item.starred.cmp(&a.item.starred)))
    }
}
