use std::{cell::RefCell, rc::Rc};

use gettextrs::gettext;
use relm4::{
    gtk::{
        self,
        prelude::{ButtonExt, ToValue, WidgetExt},
    },
    RelmWidgetExt,
};

use crate::{
    common::convert_for_label,
    components::playlists_view::{PlaylistsView, PlaylistsViewOut},
    css::DragState,
    subsonic::Subsonic,
    types::{Droppable, Id},
};

use super::SetupFinished;

static UID: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlaylistUid {
    pub uid: usize,
    pub child: submarine::data::Child,
}

#[derive(Debug, Clone)]
pub struct PlaylistRow {
    uid: usize,
    subsonic: Rc<RefCell<Subsonic>>,
    item: submarine::data::Child,
    play_count: Option<gtk::Label>,
    fav_btn: Option<gtk::Button>,
    sender: relm4::AsyncComponentSender<PlaylistsView>,
}

impl PartialEq for PlaylistRow {
    fn eq(&self, other: &Self) -> bool {
        self.item == other.item
    }
}

impl PlaylistRow {
    pub fn new(
        subsonic: &Rc<RefCell<Subsonic>>,
        item: submarine::data::Child,
        sender: relm4::AsyncComponentSender<PlaylistsView>,
    ) -> Self {
        let uid = UID.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Self {
            uid,
            subsonic: subsonic.clone(),
            item,
            play_count: None,
            fav_btn: None,
            sender: sender.clone(),
        }
    }

    pub fn uid(&self) -> &usize {
        &self.uid
    }

    pub fn item(&self) -> &submarine::data::Child {
        &self.item
    }

    pub fn item_mut(&mut self) -> &mut submarine::data::Child {
        &mut self.item
    }

    pub fn set_play_count(&mut self, play_count: Option<i64>) {
        self.item.play_count = play_count;

        // update label
        if let Some(ref count) = self.play_count {
            let play_count = play_count.map(|n| n.to_string());
            count.set_label(&play_count.unwrap_or(String::from("-")));
        }
    }

    pub fn fav_btn(&self) -> &Option<gtk::Button> {
        &self.fav_btn
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
    album: Rc<RefCell<Option<submarine::data::Child>>>,
    drag_src: gtk::DragSource,
    sender: Rc<RefCell<Option<relm4::AsyncComponentSender<PlaylistsView>>>>,
    uid: Rc<RefCell<Option<usize>>>,
}

impl Model {
    fn new() -> (gtk::Viewport, Self) {
        let model = Model {
            subsonic: Rc::new(RefCell::new(None)),
            album: Rc::new(RefCell::new(None)),
            drag_src: gtk::DragSource::default(),
            sender: Rc::new(RefCell::new(None)),
            uid: Rc::new(RefCell::new(None)),
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

            let Some(album) = subsonic.borrow().album_of_song(album) else {
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

    fn set_from_row(&self, row: &PlaylistRow) {
        self.subsonic.replace(Some(row.subsonic.clone()));
        self.album.replace(Some(row.item.clone()));
        self.sender.replace(Some(row.sender.clone()));
        self.uid.replace(Some(row.uid));

        let drop = PlaylistUid {
            uid: row.uid,
            child: row.item.clone(),
        };
        let drop = Droppable::PlaylistItems(vec![drop]);
        let content = gtk::gdk::ContentProvider::for_value(&drop.to_value());
        self.drag_src.set_content(Some(&content));
    }
}

pub struct TitleColumn;

impl relm4::typed_view::column::RelmColumn for TitleColumn {
    type Root = gtk::Viewport;
    type Item = PlaylistRow;
    type Widgets = (Model, gtk::Label);

    const COLUMN_NAME: &'static str = "Title";
    const ENABLE_RESIZE: bool = true;
    const ENABLE_EXPAND: bool = true;

    fn setup(_item: &gtk::ListItem) -> (Self::Root, Self::Widgets) {
        let (view, model) = Model::new();
        let label = gtk::Label::builder()
            .halign(gtk::Align::Start)
            .ellipsize(gtk::pango::EllipsizeMode::End)
            .label("No title given")
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
    type Item = PlaylistRow;
    type Widgets = (Model, gtk::Label);

    const COLUMN_NAME: &'static str = "Artist";
    const ENABLE_RESIZE: bool = true;
    const ENABLE_EXPAND: bool = true;

    fn setup(_item: &gtk::ListItem) -> (Self::Root, Self::Widgets) {
        let label = gtk::Label::builder()
            .halign(gtk::Align::Start)
            .ellipsize(gtk::pango::EllipsizeMode::End)
            .build();
        let (view, model) = Model::new();
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
                sender.output(PlaylistsViewOut::ClickedArtist(id)).unwrap();
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

pub struct AlbumColumn;

impl relm4::typed_view::column::RelmColumn for AlbumColumn {
    type Root = gtk::Viewport;
    type Item = PlaylistRow;
    type Widgets = (Model, gtk::Label);

    const COLUMN_NAME: &'static str = "Album";
    const ENABLE_RESIZE: bool = true;
    const ENABLE_EXPAND: bool = true;

    fn setup(_item: &gtk::ListItem) -> (Self::Root, Self::Widgets) {
        let label = gtk::Label::builder()
            .halign(gtk::Align::Start)
            .ellipsize(gtk::pango::EllipsizeMode::End)
            .build();
        let (view, model) = Model::new();
        view.set_child(Some(&label));
        (view, (model, label))
    }

    fn bind(item: &mut Self::Item, (model, label): &mut Self::Widgets, _root: &mut Self::Root) {
        model.set_from_row(item);

        let stock = gettext("Unknown Album");
        let album = item.item.album.as_deref().unwrap_or(&stock);
        if let Some(album_id) = &item.item.album_id {
            // set text with link
            let album = gtk::glib::markup_escape_text(album);
            label.set_markup(&format!("<a href=\"\">{album}</a>"));
            let album_id = album_id.clone();
            let sender = item.sender.clone();
            label.connect_activate_link(move |_label, _id| {
                let id = Id::album(&album_id);
                sender.output(PlaylistsViewOut::ClickedAlbum(id)).unwrap();
                gtk::glib::signal::Propagation::Stop
            });
        } else {
            // set plain text
            label.set_text(album);
        }
    }

    fn sort_fn() -> relm4::typed_view::OrdFn<Self::Item> {
        Some(Box::new(|a, b| a.item.album.cmp(&b.item.album)))
    }
}

pub struct GenreColumn;

impl relm4::typed_view::column::RelmColumn for GenreColumn {
    type Root = gtk::Viewport;
    type Item = PlaylistRow;
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

pub struct LengthColumn;

impl relm4::typed_view::column::RelmColumn for LengthColumn {
    type Root = gtk::Viewport;
    type Item = PlaylistRow;
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
    type Item = PlaylistRow;
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
        item.play_count = Some(label.clone());
    }

    fn unbind(item: &mut Self::Item, (_model, _label): &mut Self::Widgets, _root: &mut Self::Root) {
        item.play_count = None;
    }

    fn sort_fn() -> relm4::typed_view::OrdFn<Self::Item> {
        Some(Box::new(|a, b| a.item.play_count.cmp(&b.item.play_count)))
    }
}

pub struct BitRateColumn;

impl relm4::typed_view::column::RelmColumn for BitRateColumn {
    type Root = gtk::Viewport;
    type Item = PlaylistRow;
    type Widgets = (Model, gtk::Label);

    const COLUMN_NAME: &'static str = "Bitrate";
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

        let bitrate = item.item.bit_rate;
        let bitrate = bitrate.map(|n| n.to_string());
        label.set_label(&bitrate.unwrap_or(String::from("-")));
    }

    fn sort_fn() -> relm4::typed_view::OrdFn<Self::Item> {
        Some(Box::new(|a, b| a.item.bit_rate.cmp(&b.item.bit_rate)))
    }
}

pub struct FavColumn;

impl relm4::typed_view::column::RelmColumn for FavColumn {
    type Root = gtk::Viewport;
    type Item = PlaylistRow;
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
                        .output(PlaylistsViewOut::FavoriteClicked(
                            cell.borrow().clone(),
                            false,
                        ))
                        .unwrap();
                }
                Some("non-starred-symbolic") => {
                    btn.set_icon_name("starred-symbolic");
                    sender
                        .output(PlaylistsViewOut::FavoriteClicked(
                            cell.borrow().clone(),
                            true,
                        ))
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
