use std::{cell::RefCell, rc::Rc};

use gettextrs::gettext;
use relm4::{
    gtk::{
        self, gdk,
        prelude::{BoxExt, ButtonExt, ToValue, WidgetExt},
    },
    RelmWidgetExt,
};

use crate::{
    common::convert_for_label,
    components::playlists_view::{PlaylistsView, PlaylistsViewIn, PlaylistsViewOut},
    css::DragState,
    factory::DropHalf,
    settings::Settings,
    subsonic::Subsonic,
    types::{Droppable, Id},
};

use super::SetupFinished;

static UID: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlaylistIndex {
    pub uid: usize,
    pub child: submarine::data::Child,
}

#[derive(Debug)]
pub struct PlaylistRow {
    uid: usize,
    subsonic: Rc<RefCell<Subsonic>>,
    item: submarine::data::Child,
    title_box: gtk::Box,
    sender: relm4::AsyncComponentSender<PlaylistsView>,
    multiple_drag_sources: Option<gtk::DragSource>,
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
        let result = Self {
            uid,
            subsonic: subsonic.clone(),
            item,
            title_box: gtk::Box::default(),
            sender: sender.clone(),
            multiple_drag_sources: None,
        };

        // setup title label
        let title_label = gtk::Label::builder()
            .halign(gtk::Align::Start)
            .ellipsize(gtk::pango::EllipsizeMode::End)
            .label(&result.item.title)
            .build();
        result.title_box.append(&title_label);

        result
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

    pub fn title_box(&self) -> &gtk::Box {
        &self.title_box
    }

    pub fn set_drag_src(&mut self, drop: Vec<PlaylistIndex>) {
        // when Vec is empty remove DragSource
        if drop.is_empty() {
            self.remove_drag_src();
            return;
        }

        // remove old DragSource if there is one
        if self.multiple_drag_sources.is_some() {
            self.remove_drag_src();
        }

        // create new DragSource
        let src = gtk::DragSource::default();
        let drop = Droppable::PlaylistItems(drop);
        let content = gtk::gdk::ContentProvider::for_value(&drop.to_value());
        src.set_content(Some(&content));
        src.set_actions(gtk::gdk::DragAction::COPY);

        //set drag item
        let subsonic = self.subsonic.clone();
        let album = self.subsonic.borrow().album_of_song(&self.item);
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

        //add this DragSource
        if let Some(list_item) = super::get_list_item_widget(&self.title_box) {
            list_item.add_controller(src.clone());
        }

        //save this DragSource
        self.multiple_drag_sources = Some(src);
    }

    pub fn remove_drag_src(&mut self) {
        if let Some(src) = &self.multiple_drag_sources {
            if let Some(list_item) = super::get_list_item_widget(&self.title_box) {
                list_item.remove_controller(src);
            }
        }
        self.multiple_drag_sources = None;
    }

    pub fn add_drag_indicator_top(&self) {
        if let Some(list_item) = super::get_list_item_widget(&self.title_box) {
            DragState::drop_shadow_top(&list_item);
        }
    }

    pub fn add_drag_indicator_bottom(&self) {
        if let Some(list_item) = super::get_list_item_widget(&self.title_box) {
            DragState::drop_shadow_bottom(&list_item);
        }
    }

    pub fn reset_drag_indicators(&self) {
        if let Some(list_item) = super::get_list_item_widget(&self.title_box) {
            DragState::reset(&list_item);
        }
    }
}

pub struct Model {
    subsonic: Rc<RefCell<Option<Rc<RefCell<Subsonic>>>>>,
    album: Rc<RefCell<Option<submarine::data::Child>>>,
    drag_src: gtk::DragSource,
    drop_target: gtk::DropTarget,
    sender: Rc<RefCell<Option<relm4::AsyncComponentSender<PlaylistsView>>>>,
    uid: Rc<RefCell<Option<usize>>>,
}

impl Model {
    fn new() -> (gtk::Viewport, Self) {
        let model = Model {
            subsonic: Rc::new(RefCell::new(None)),
            album: Rc::new(RefCell::new(None)),
            drag_src: gtk::DragSource::default(),
            drop_target: gtk::DropTarget::default(),
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

        // create drop target
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
                            .output(PlaylistsViewOut::DisplayToast(gettext(
                                "dropping songs into a playlist is not allowed while searching",
                            )))
                            .unwrap();
                        return false;
                    }
                }

                // check if drop is valid
                if let Ok(drop) = value.get::<Droppable>() {
                    match drop {
                        Droppable::PlaylistItems(items) => {
                            for item in items.iter().rev() {
                                let half = DropHalf::calc(widget.height(), y);
                                let src_uid = item.uid;
                                sender.input(PlaylistsViewIn::MoveSong {
                                    src: src_uid,
                                    dest: uid,
                                    half,
                                });
                            }
                        }
                        Droppable::QueueSongs(children) => {
                            let songs = children.into_iter().map(|song| song.1).collect();
                            let half = DropHalf::calc(widget.height(), y);
                            sender.input(PlaylistsViewIn::InsertSongs(songs, uid, half));
                        }
                        _ => todo!(), //TODO handle other soures
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

            sender.input(PlaylistsViewIn::DraggedOver { uid: cell, y });
            gdk::DragAction::MOVE
        });

        //remove indicator on leave
        let sender = model.sender.clone();
        model.drop_target.connect_leave(move |_drop| {
            let Some(ref sender) = *sender.borrow() else {
                return;
            };

            sender.input(PlaylistsViewIn::DragLeave);
        });

        root.add_controller(model.drop_target.clone());
        root.add_controller(model.drag_src.clone());

        (root, model)
    }

    fn set_from_row(&self, row: &PlaylistRow) {
        self.subsonic.replace(Some(row.subsonic.clone()));
        self.album.replace(Some(row.item.clone()));
        self.sender.replace(Some(row.sender.clone()));
        self.uid.replace(Some(row.uid));

        let drop = PlaylistIndex {
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
    type Widgets = Model;

    const COLUMN_NAME: &'static str = "Title";
    const ENABLE_RESIZE: bool = true;
    const ENABLE_EXPAND: bool = true;

    fn setup(_item: &gtk::ListItem) -> (Self::Root, Self::Widgets) {
        Model::new()
    }

    fn bind(item: &mut Self::Item, model: &mut Self::Widgets, root: &mut Self::Root) {
        root.set_child(Some(&item.title_box));
        model.set_from_row(item);
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

    fn bind(
        item: &mut Self::Item,
        (model, label): &mut Self::Widgets,
        _root: &mut Self::Root,
    ) {
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
    }

    fn sort_fn() -> relm4::typed_view::OrdFn<Self::Item> {
        Some(Box::new(|a, b| b.item.starred.cmp(&a.item.starred)))
    }
}
