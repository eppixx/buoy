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
    components::{
        artists_view::{ArtistsView, ArtistsViewIn, ArtistsViewOut},
        cover::Cover,
    },
    subsonic::Subsonic,
    types::Droppable,
};

use super::SetupFinished;

#[derive(Debug)]
pub struct ArtistRow {
    subsonic: Rc<RefCell<Subsonic>>,
    item: submarine::data::ArtistId3,
    cover: relm4::Controller<Cover>,
    fav_btn: Option<gtk::Button>,
    sender: relm4::ComponentSender<ArtistsView>,
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
        let send = sender.clone();
        let cover = Cover::builder()
            .launch((subsonic.clone(), item.cover_art.clone()))
            .forward(send.input_sender(), ArtistsViewIn::Cover);
        cover.model().change_size(75);

        Self {
            subsonic: subsonic.clone(),
            item,
            cover,
            fav_btn: None,
            sender: sender.clone(),
        }
    }

    pub fn item(&self) -> &submarine::data::ArtistId3 {
        &self.item
    }

    pub fn item_mut(&mut self) -> &mut submarine::data::ArtistId3 {
        &mut self.item
    }

    pub fn fav_btn(&self) -> &Option<gtk::Button> {
        &self.fav_btn
    }
}

pub struct Model {
    subsonic: Rc<RefCell<Option<Rc<RefCell<Subsonic>>>>>,
    artist: Rc<RefCell<Option<submarine::data::ArtistId3>>>,
    drag_src: gtk::DragSource,
}

impl Model {
    fn new() -> (gtk::Viewport, Self) {
        let model = Model {
            subsonic: Rc::new(RefCell::new(None)),
            artist: Rc::new(RefCell::new(None)),
            drag_src: gtk::DragSource::default(),
        };

        let root = gtk::Viewport::default();

        // create DragSource
        model.drag_src.set_actions(gtk::gdk::DragAction::COPY);

        // set drag icon
        let subsonic = model.subsonic.clone();
        let artist = model.artist.clone();
        model.drag_src.connect_drag_begin(move |src, _drag| {
            let Some(ref subsonic) = *subsonic.borrow() else {
                return;
            };
            let Some(ref artist) = *artist.borrow() else {
                return;
            };

            if let Some(cover_id) = &artist.cover_art {
                let cover = subsonic.borrow().cover_icon(cover_id);
                if let Some(tex) = cover {
                    src.set_icon(Some(&tex), 0, 0);
                }
            }
        });

        root.add_controller(model.drag_src.clone());

        (root, model)
    }

    fn set_from_row(&self, row: &ArtistRow) {
        self.subsonic.replace(Some(row.subsonic.clone()));
        self.artist.replace(Some(row.item.clone()));

        let drop = Droppable::Artist(Box::new(row.item.clone()));
        let content = gtk::gdk::ContentProvider::for_value(&drop.to_value());
        self.drag_src.set_content(Some(&content));
    }
}

pub struct CoverColumn;

impl relm4::typed_view::column::RelmColumn for CoverColumn {
    type Root = gtk::Viewport;
    type Item = ArtistRow;
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
    type Item = ArtistRow;
    type Widgets = (Model, gtk::Label);

    const COLUMN_NAME: &'static str = "Name";
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
        label.set_label(&item.item.name);
    }

    fn sort_fn() -> relm4::typed_view::OrdFn<Self::Item> {
        Some(Box::new(|a, b| a.item.name.cmp(&b.item.name)))
    }
}

pub struct AlbumCountColumn;

impl relm4::typed_view::column::RelmColumn for AlbumCountColumn {
    type Root = gtk::Viewport;
    type Item = ArtistRow;
    type Widgets = (Model, gtk::Label);

    const COLUMN_NAME: &'static str = "Albums";
    const ENABLE_RESIZE: bool = false;
    const ENABLE_EXPAND: bool = false;

    fn setup(_item: &gtk::ListItem) -> (Self::Root, Self::Widgets) {
        let (view, model) = Model::new();
        let label = gtk::Label::builder().build();
        view.set_child(Some(&label));
        (view, (model, label))
    }

    fn bind(item: &mut Self::Item, (model, label): &mut Self::Widgets, _root: &mut Self::Root) {
        model.set_from_row(item);
        label.set_label(&item.item.album_count.to_string());
    }

    fn sort_fn() -> relm4::typed_view::OrdFn<Self::Item> {
        Some(Box::new(|a, b| b.item.album_count.cmp(&a.item.album_count)))
    }
}

pub struct FavColumn;

impl relm4::typed_view::column::RelmColumn for FavColumn {
    type Root = gtk::Viewport;
    type Item = ArtistRow;
    type Widgets = (Rc<RefCell<String>>, gtk::Button, Model, SetupFinished);

    const COLUMN_NAME: &'static str = "Favorite";
    const ENABLE_RESIZE: bool = false;

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
        _view: &mut Self::Root,
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
                        .output(ArtistsViewOut::FavoriteClicked(
                            cell.borrow().clone(),
                            false,
                        ))
                        .unwrap();
                }
                Some("non-starred-symbolic") => {
                    btn.set_icon_name("starred-symbolic");
                    sender
                        .output(ArtistsViewOut::FavoriteClicked(cell.borrow().clone(), true))
                        .unwrap();
                }
                _ => unreachable!("unkown icon name"),
            });
        }

        item.fav_btn = Some(fav_btn.clone());
    }

    fn sort_fn() -> relm4::typed_view::OrdFn<Self::Item> {
        Some(Box::new(|a, b| b.item.starred.cmp(&a.item.starred)))
    }
}
