use std::{cell::RefCell, rc::Rc};

use gettextrs::gettext;
use granite::prelude::ToValue;
use relm4::{
    gtk::{
        self,
        prelude::{ButtonExt, WidgetExt},
    },
    Component, ComponentController, RelmWidgetExt,
};

use crate::{
    common::convert_for_label,
    components::{
        albums_view::{AlbumsView, AlbumsViewIn, AlbumsViewOut},
        cover::Cover,
    },
    factory::SetupFinished,
    subsonic::Subsonic,
    types::{Droppable, Id},
};

#[derive(Debug)]
pub struct AlbumRow {
    subsonic: Rc<RefCell<Subsonic>>,
    item: submarine::data::Child,
    cover: relm4::Controller<Cover>,
    fav_btn: Option<gtk::Button>,
    sender: relm4::ComponentSender<AlbumsView>,
}

impl PartialEq for AlbumRow {
    fn eq(&self, other: &Self) -> bool {
        self.item == other.item
    }
}

impl AlbumRow {
    pub fn new(
        subsonic: &Rc<RefCell<Subsonic>>,
        item: submarine::data::Child,
        sender: relm4::ComponentSender<AlbumsView>,
    ) -> Self {
        let cover = Cover::builder()
            .launch((subsonic.clone(), item.cover_art.clone()))
            .forward(sender.input_sender(), AlbumsViewIn::Cover);
        cover.model().change_size(75);

        Self {
            subsonic: subsonic.clone(),
            item,
            cover,
            fav_btn: None,
            sender,
        }
    }

    pub fn item(&self) -> &submarine::data::Child {
        &self.item
    }

    pub fn item_mut(&mut self) -> &mut submarine::data::Child {
        &mut self.item
    }

    pub fn fav_btn(&self) -> &Option<gtk::Button> {
        &self.fav_btn
    }
}

pub struct Model {
    subsonic: Rc<RefCell<Option<Rc<RefCell<Subsonic>>>>>,
    album: Rc<RefCell<Option<submarine::data::Child>>>,
    drag_src: gtk::DragSource,
}

impl Model {
    fn new() -> (gtk::Viewport, Self) {
        let model = Model {
            subsonic: Rc::new(RefCell::new(None)),
            album: Rc::new(RefCell::new(None)),
            drag_src: gtk::DragSource::default(),
        };

        let root = gtk::Viewport::default();

        // create DragSource
        model.drag_src.set_actions(gtk::gdk::DragAction::COPY);

        // set drag icon
        let subsonic = model.subsonic.clone();
        let album = model.album.clone();
        model.drag_src.connect_drag_begin(move |src, _drag| {
            let Some(ref subsonic) = *subsonic.borrow() else {
                return;
            };
            let Some(ref album) = *album.borrow() else {
                return;
            };

            if let Some(cover_id) = &album.cover_art {
                let cover = subsonic.borrow().cover_icon(cover_id);
                if let Some(tex) = cover {
                    src.set_icon(Some(&tex), 0, 0);
                }
            }
        });

        root.add_controller(model.drag_src.clone());

        (root, model)
    }

    fn set_from_row(&self, row: &AlbumRow) {
        self.subsonic.replace(Some(row.subsonic.clone()));
        self.album.replace(Some(row.item.clone()));

        let drop = Droppable::AlbumChild(Box::new(row.item.clone()));
        let content = gtk::gdk::ContentProvider::for_value(&drop.to_value());
        self.drag_src.set_content(Some(&content));
    }
}

pub struct CoverColumn;

impl relm4::typed_view::column::RelmColumn for CoverColumn {
    type Root = gtk::Viewport;
    type Item = AlbumRow;
    type Widgets = Model;

    const COLUMN_NAME: &'static str = "Cover";
    const ENABLE_RESIZE: bool = false;
    const ENABLE_EXPAND: bool = false;

    fn setup(_item: &gtk::ListItem) -> (Self::Root, Self::Widgets) {
        let (view, model) = Model::new();
        view.set_margin_end(12);
        (view, model)
    }

    fn bind(item: &mut Self::Item, model: &mut Self::Widgets, root: &mut Self::Root) {
        model.set_from_row(item);
        root.set_child(Some(item.cover.widget()));
    }
}

pub struct TitleColumn;

impl relm4::typed_view::column::RelmColumn for TitleColumn {
    type Root = gtk::Viewport;
    type Item = AlbumRow;
    type Widgets = (Model, gtk::Label);

    const COLUMN_NAME: &'static str = "Album";
    const ENABLE_RESIZE: bool = true;
    const ENABLE_EXPAND: bool = true;

    fn setup(_item: &gtk::ListItem) -> (Self::Root, Self::Widgets) {
        let (view, model) = Model::new();
        let label = gtk::Label::builder()
            .halign(gtk::Align::Start)
            .ellipsize(gtk::pango::EllipsizeMode::End)
            .build();
        view.set_child(Some(&label));
        (view, (model, label))
    }

    fn bind(item: &mut Self::Item, (model, label): &mut Self::Widgets, _root: &mut Self::Root) {
        model.set_from_row(item);
        label.set_label(&item.item.title);
    }

    fn sort_fn() -> relm4::typed_view::OrdFn<Self::Item> {
        Some(Box::new(|a, b| a.item.title.cmp(&b.item.title)))
    }
}

pub struct ArtistColumn;

impl relm4::typed_view::column::RelmColumn for ArtistColumn {
    type Root = gtk::Viewport;
    type Item = AlbumRow;
    type Widgets = (Model, gtk::Label);

    const COLUMN_NAME: &'static str = "Artist";
    const ENABLE_RESIZE: bool = true;
    const ENABLE_EXPAND: bool = true;

    fn setup(_item: &gtk::ListItem) -> (Self::Root, Self::Widgets) {
        let (view, model) = Model::new();
        let label = gtk::Label::builder()
            .halign(gtk::Align::Start)
            .ellipsize(gtk::pango::EllipsizeMode::End)
            .build();
        view.set_child(Some(&label));
        (view, (model, label))
    }

    fn bind(item: &mut Self::Item, (model, label): &mut Self::Widgets, _root: &mut Self::Root) {
        model.set_from_row(item);

        let stock = gettext("Unknown Artist");
        let artist = item.item.artist.as_deref().unwrap_or(&stock);
        if let Some(artist_id) = &item.item.artist_id {
            // set text with link
            let artist = gtk::glib::markup_escape_text(artist);
            label.set_markup(&format!("<a href=\"\">{artist}</a>"));
            let artist_id = artist_id.clone();
            let sender = item.sender.clone();
            label.connect_activate_link(move |_label, _id| {
                let id = Id::artist(&artist_id);
                sender.output(AlbumsViewOut::ClickedArtist(id)).unwrap();
                gtk::glib::signal::Propagation::Stop
            });
        } else {
            // set plain text
            label.set_text(artist);
        }
    }

    fn sort_fn() -> relm4::typed_view::OrdFn<Self::Item> {
        Some(Box::new(|a, b| a.item.artist.cmp(&b.item.artist)))
    }
}

pub struct GenreColumn;

impl relm4::typed_view::column::RelmColumn for GenreColumn {
    type Root = gtk::Viewport;
    type Item = AlbumRow;
    type Widgets = (Model, gtk::Label);

    const COLUMN_NAME: &'static str = "Genre";
    const ENABLE_RESIZE: bool = true;
    const ENABLE_EXPAND: bool = true;

    fn setup(_item: &gtk::ListItem) -> (Self::Root, Self::Widgets) {
        let (view, model) = Model::new();
        let label = gtk::Label::builder()
            .halign(gtk::Align::Start)
            .ellipsize(gtk::pango::EllipsizeMode::End)
            .build();
        view.set_child(Some(&label));
        (view, (model, label))
    }

    fn bind(item: &mut Self::Item, (model, label): &mut Self::Widgets, _root: &mut Self::Root) {
        model.set_from_row(item);
        label.set_label(
            item.item
                .genre
                .as_deref()
                .unwrap_or(&gettext("Unknown genre")),
        );
    }

    fn sort_fn() -> relm4::typed_view::OrdFn<Self::Item> {
        Some(Box::new(|a, b| a.item.genre.cmp(&b.item.genre)))
    }
}

pub struct YearColumn;

impl relm4::typed_view::column::RelmColumn for YearColumn {
    type Root = gtk::Viewport;
    type Item = AlbumRow;
    type Widgets = (Model, gtk::Label);

    const COLUMN_NAME: &'static str = "Year";
    const ENABLE_RESIZE: bool = false;
    const ENABLE_EXPAND: bool = false;

    fn setup(_item: &gtk::ListItem) -> (Self::Root, Self::Widgets) {
        let (view, model) = Model::new();
        let label = gtk::Label::builder()
            .halign(gtk::Align::Start)
            .ellipsize(gtk::pango::EllipsizeMode::End)
            .label(gettext("No year"))
            .build();
        view.set_child(Some(&label));
        (view, (model, label))
    }

    fn bind(item: &mut Self::Item, (model, label): &mut Self::Widgets, _root: &mut Self::Root) {
        model.set_from_row(item);
        if let Some(year) = &item.item.year {
            label.set_label(&year.to_string());
        }
    }

    fn sort_fn() -> relm4::typed_view::OrdFn<Self::Item> {
        Some(Box::new(|a, b| a.item.year.cmp(&b.item.year)))
    }
}

pub struct CdColumn;

impl relm4::typed_view::column::RelmColumn for CdColumn {
    type Root = gtk::Viewport;
    type Item = AlbumRow;
    type Widgets = (Model, gtk::Label);

    const COLUMN_NAME: &'static str = "CDs";
    const ENABLE_RESIZE: bool = false;
    const ENABLE_EXPAND: bool = false;

    fn setup(_item: &gtk::ListItem) -> (Self::Root, Self::Widgets) {
        let (view, model) = Model::new();
        let label = gtk::Label::builder()
            .halign(gtk::Align::Start)
            .ellipsize(gtk::pango::EllipsizeMode::End)
            .label(gettext("No Cd"))
            .build();
        view.set_child(Some(&label));
        (view, (model, label))
    }

    fn bind(item: &mut Self::Item, (model, label): &mut Self::Widgets, _root: &mut Self::Root) {
        model.set_from_row(item);
        if let Some(number) = &item.item.disc_number {
            label.set_label(&number.to_string());
        }
    }

    fn sort_fn() -> relm4::typed_view::OrdFn<Self::Item> {
        Some(Box::new(|a, b| a.item.disc_number.cmp(&b.item.disc_number)))
    }
}

pub struct LengthColumn;

impl relm4::typed_view::column::RelmColumn for LengthColumn {
    type Root = gtk::Viewport;
    type Item = AlbumRow;
    type Widgets = (Model, gtk::Label);

    const COLUMN_NAME: &'static str = "Length";
    const ENABLE_RESIZE: bool = false;
    const ENABLE_EXPAND: bool = false;

    fn setup(_item: &gtk::ListItem) -> (Self::Root, Self::Widgets) {
        let (view, model) = Model::new();
        let label = gtk::Label::default();
        view.set_child(Some(&label));
        (view, (model, label))
    }

    fn bind(item: &mut Self::Item, (model, label): &mut Self::Widgets, _root: &mut Self::Root) {
        model.set_from_row(item);
        let length = convert_for_label(i64::from(item.item.duration.unwrap_or(0)) * 1000);
        label.set_label(&length);
    }

    fn sort_fn() -> relm4::typed_view::OrdFn<Self::Item> {
        Some(Box::new(|a, b| a.item.duration.cmp(&b.item.duration)))
    }
}

pub struct PlayCountColumn;

impl relm4::typed_view::column::RelmColumn for PlayCountColumn {
    type Root = gtk::Viewport;
    type Item = AlbumRow;
    type Widgets = (Model, gtk::Label);

    const COLUMN_NAME: &'static str = "Plays";
    const ENABLE_RESIZE: bool = false;
    const ENABLE_EXPAND: bool = false;

    fn setup(_item: &gtk::ListItem) -> (Self::Root, Self::Widgets) {
        let (view, model) = Model::new();
        let label = gtk::Label::default();
        view.set_child(Some(&label));
        (view, (model, label))
    }

    fn bind(item: &mut Self::Item, (model, label): &mut Self::Widgets, _root: &mut Self::Root) {
        model.set_from_row(item);
        let play_count = item.item.play_count;
        let play_count = play_count.map(|n| n.to_string());
        label.set_label(&play_count.unwrap_or(String::from("-")));
    }

    fn sort_fn() -> relm4::typed_view::OrdFn<Self::Item> {
        Some(Box::new(|a, b| a.item.play_count.cmp(&b.item.play_count)))
    }
}

pub struct FavColumn;

impl relm4::typed_view::column::RelmColumn for FavColumn {
    type Root = gtk::Viewport;
    type Item = AlbumRow;
    type Widgets = (Rc<RefCell<String>>, gtk::Button, Model, SetupFinished);

    const COLUMN_NAME: &'static str = "Favorite";
    const ENABLE_RESIZE: bool = false;
    const ENABLE_EXPAND: bool = false;

    fn setup(_item: &gtk::ListItem) -> (Self::Root, Self::Widgets) {
        let (view, model) = Model::new();
        let fav_btn = gtk::Button::new();
        fav_btn.set_tooltip(&gettext("Click to (un)favorite song"));
        fav_btn.set_focus_on_click(false);

        let cell = Rc::new(RefCell::new(String::new()));
        view.set_child(Some(&fav_btn));
        (view, (cell, fav_btn, model, SetupFinished(false)))
    }

    fn bind(
        item: &mut Self::Item,
        (cell, fav_btn, model, finished): &mut Self::Widgets,
        _root: &mut Self::Root,
    ) {
        model.set_from_row(item);

        match item.item.starred.is_some() {
            true => fav_btn.set_icon_name("starred-symbolic"),
            false => fav_btn.set_icon_name("non-starred-symbolic"),
        }

        cell.replace(item.item.id.clone());

        if !finished.0 {
            finished.0 = true;
            let sender = item.sender.clone();
            let cell = cell.clone();
            fav_btn.connect_clicked(move |btn| match btn.icon_name().as_deref() {
                Some("starred-symbolic") => {
                    btn.set_icon_name("non-starred-symbolic");
                    sender
                        .output(AlbumsViewOut::FavoriteClicked(cell.borrow().clone(), false))
                        .unwrap();
                }
                Some("non-starred-symbolic") => {
                    btn.set_icon_name("starred-symbolic");
                    sender
                        .output(AlbumsViewOut::FavoriteClicked(cell.borrow().clone(), true))
                        .unwrap();
                }
                _ => unreachable!("unkown icon name"),
            });
        }

        item.fav_btn = Some(fav_btn.clone());
    }

    fn unbind(item: &mut Self::Item, _widgets: &mut Self::Widgets, _root: &mut Self::Root) {
        item.fav_btn = None;
    }

    fn sort_fn() -> relm4::typed_view::OrdFn<Self::Item> {
        Some(Box::new(|a, b| b.item.starred.cmp(&a.item.starred)))
    }
}
