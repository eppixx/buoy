use std::{cell::RefCell, rc::Rc};

use gettextrs::gettext;
use granite::prelude::ToValue;
use relm4::{
    gtk::{
        self,
        prelude::{BoxExt, ButtonExt, WidgetExt},
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

    fn create_drag_src(&self) -> gtk::DragSource {
        // create DragSource with content
        let src = gtk::DragSource::default();
        let drop = Droppable::Artist(Box::new(self.item.clone()));
        let content = gtk::gdk::ContentProvider::for_value(&drop.to_value());
        src.set_content(Some(&content));
        src.set_actions(gtk::gdk::DragAction::COPY);

        // set drag icon
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
        view.set_child(Some(item.cover.widget()));
    }
}

pub struct TitleColumn;

impl relm4::typed_view::column::RelmColumn for TitleColumn {
    type Root = gtk::Box;
    type Item = ArtistRow;
    type Widgets = (gtk::Label, SetupFinished);

    const COLUMN_NAME: &'static str = "Name";
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
        (b, (label, SetupFinished(false)))
    }

    fn bind(item: &mut Self::Item, (label, finished): &mut Self::Widgets, _b: &mut Self::Root) {
        label.set_label(&item.item.name);

        if !finished.0 {
            finished.0 = true;
            let list_item = super::get_list_item_widget(label).unwrap();
            let drag_src = item.create_drag_src();
            list_item.add_controller(drag_src);
        }
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

    fn bind(item: &mut Self::Item, label: &mut Self::Widgets, _b: &mut Self::Root) {
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
    type Widgets = (Rc<RefCell<String>>, gtk::Button, SetupFinished);

    const COLUMN_NAME: &'static str = "Favorite";
    const ENABLE_RESIZE: bool = false;

    fn setup(_item: &gtk::ListItem) -> (Self::Root, Self::Widgets) {
        let fav_btn = gtk::Button::new();
        fav_btn.set_tooltip(&gettext("Click to (un)favorite song"));
        fav_btn.set_focus_on_click(false);

        let cell = Rc::new(RefCell::new(String::new()));
        (
            gtk::Viewport::default(),
            (cell, fav_btn, SetupFinished(false)),
        )
    }

    fn bind(
        item: &mut Self::Item,
        (cell, fav_btn, finished): &mut Self::Widgets,
        view: &mut Self::Root,
    ) {
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
        view.set_child(Some(fav_btn));
    }

    fn unbind(item: &mut Self::Item, _widgets: &mut Self::Widgets, _root: &mut Self::Root) {
        item.fav_btn = None;
    }

    fn sort_fn() -> relm4::typed_view::OrdFn<Self::Item> {
        Some(Box::new(|a, b| b.item.starred.cmp(&a.item.starred)))
    }
}
