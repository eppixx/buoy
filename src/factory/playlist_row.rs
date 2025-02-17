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
    pub item: submarine::data::Child,
    pub title_box: gtk::Box,
    sender: relm4::ComponentSender<PlaylistsView>,
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
        sender: relm4::ComponentSender<PlaylistsView>,
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

    fn create_drag_src(&self, uid: &Rc<RefCell<usize>>) -> gtk::DragSource {
        // prepare content
        let drop = PlaylistIndex {
            uid: *uid.borrow(),
            child: self.item.clone(),
        };
        let drop = Droppable::PlaylistItems(vec![drop]);
        let content = gtk::gdk::ContentProvider::for_value(&drop.to_value());

        // create DragSource
        let src = gtk::DragSource::default();
        src.set_content(Some(&content));
        src.set_actions(gtk::gdk::DragAction::COPY);

        // set drag icon
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

    fn create_drop_target(&self, uid: &Rc<RefCell<usize>>) -> gtk::DropTarget {
        let actions = gdk::DragAction::MOVE | gdk::DragAction::COPY;
        let target = gtk::DropTarget::new(gtk::glib::types::Type::INVALID, actions);
        target.set_types(&[<Droppable as gtk::prelude::StaticType>::static_type()]);

        let sender = self.sender.clone();
        let cell = uid.clone();
        target.connect_drop(move |_target, value, _x, y| {
            if let Ok(drop) = value.get::<Droppable>() {
                match drop {
                    Droppable::PlaylistItems(items) => {
                        for item in items.iter().rev() {
                            let src_uid = item.uid;
                            sender.input(PlaylistsViewIn::MoveSong {
                                src: src_uid,
                                dest: *cell.borrow(),
                                y,
                            });
                        }
                    }
                    Droppable::QueueSongs(children) => {
                        let songs = children.into_iter().map(|song| song.1).collect();
                        sender.input(PlaylistsViewIn::InsertSongs(songs, *cell.borrow(), y));
                    }
                    _ => todo!(), //TODO handle other soures
                }
                return true;
            }
            false
        });

        //sending motion for added indicators
        let sender = self.sender.clone();
        let cell = uid.clone();
        target.connect_motion(move |_drop, _x, y| {
            sender.input(PlaylistsViewIn::DraggedOver {
                uid: *cell.borrow(),
                y,
            });
            gdk::DragAction::MOVE
        });

        //remove indicator on leave
        let sender = self.sender.clone();
        target.connect_leave(move |_drop| {
            sender.input(PlaylistsViewIn::DragLeave);
        });

        target
    }

    pub fn add_drag_indicator_top(&self) {
        if let Some(list_item) = super::get_list_item_widget(&self.title_box) {
            list_item.remove_css_class("drag-indicator-bottom");
            list_item.add_css_class("drag-indicator-top");
        }
    }

    pub fn add_drag_indicator_bottom(&self) {
        if let Some(list_item) = super::get_list_item_widget(&self.title_box) {
            list_item.add_css_class("drag-indicator-bottom");
            list_item.remove_css_class("drag-indicator-top");
        }
    }

    pub fn reset_drag_indicators(&self) {
        if let Some(list_item) = super::get_list_item_widget(&self.title_box) {
            list_item.remove_css_class("drag-indicator-bottom");
            list_item.remove_css_class("drag-indicator-top");
        }
    }
}

pub struct TitleColumn;

impl relm4::typed_view::column::RelmColumn for TitleColumn {
    type Root = gtk::Viewport;
    type Item = PlaylistRow;
    type Widgets = (Rc<RefCell<usize>>, SetupFinished);

    const COLUMN_NAME: &'static str = "Title";
    const ENABLE_RESIZE: bool = true;
    const ENABLE_EXPAND: bool = true;

    fn setup(_item: &gtk::ListItem) -> (Self::Root, Self::Widgets) {
        (
            gtk::Viewport::default(),
            (Rc::new(RefCell::new(0)), SetupFinished(false)),
        )
    }

    fn bind(item: &mut Self::Item, (cell, finished): &mut Self::Widgets, view: &mut Self::Root) {
        view.set_child(Some(&item.title_box));
        cell.replace(item.uid);

        // we need only 1 DragSource for the ListItem as it is updated by updating cell
        if !finished.0 {
            finished.0 = true;
            let list_item = super::get_list_item_widget(&item.title_box).unwrap();
            let drop_target = item.create_drop_target(cell);
            list_item.add_controller(drop_target);
            let drag_src = item.create_drag_src(cell);
            list_item.add_controller(drag_src);
        }
    }

    fn sort_fn() -> relm4::typed_view::OrdFn<Self::Item> {
        Some(Box::new(|a, b| a.item.title.cmp(&b.item.title)))
    }
}

pub struct ArtistColumn;

impl relm4::typed_view::column::RelmColumn for ArtistColumn {
    type Root = gtk::Viewport;
    type Item = PlaylistRow;
    type Widgets = gtk::Label;

    const COLUMN_NAME: &'static str = "Artist";
    const ENABLE_RESIZE: bool = true;
    const ENABLE_EXPAND: bool = true;

    fn setup(_item: &gtk::ListItem) -> (Self::Root, Self::Widgets) {
        let artist_label = gtk::Label::builder()
            .halign(gtk::Align::Start)
            .ellipsize(gtk::pango::EllipsizeMode::End)
            .build();
        (gtk::Viewport::default(), artist_label)
    }

    fn bind(item: &mut Self::Item, artist_label: &mut Self::Widgets, view: &mut Self::Root) {
        let stock = gettext("Unknown Artist");
        let artist = item.item.artist.as_deref().unwrap_or(&stock);
        if let Some(artist_id) = &item.item.artist_id {
            // set text with link
            let artist = gtk::glib::markup_escape_text(artist);
            artist_label.set_markup(&format!("<a href=\"\">{artist}</a>"));
            let artist_id = artist_id.clone();
            let sender = item.sender.clone();
            artist_label.connect_activate_link(move |_label, _id| {
                let id = Id::artist(&artist_id);
                sender.output(PlaylistsViewOut::ClickedArtist(id)).unwrap();
                gtk::glib::signal::Propagation::Stop
            });
        } else {
            // set plain text
            artist_label.set_text(artist);
        }
        view.set_child(Some(artist_label));
    }

    fn unbind(_item: &mut Self::Item, artist_label: &mut Self::Widgets, view: &mut Self::Root) {
        artist_label.set_text("");
        view.set_child(None::<&gtk::Widget>);
    }

    fn sort_fn() -> relm4::typed_view::OrdFn<Self::Item> {
        Some(Box::new(|a, b| a.item.artist.cmp(&b.item.artist)))
    }
}

pub struct AlbumColumn;

impl relm4::typed_view::column::RelmColumn for AlbumColumn {
    type Root = gtk::Viewport;
    type Item = PlaylistRow;
    type Widgets = gtk::Label;

    const COLUMN_NAME: &'static str = "Album";
    const ENABLE_RESIZE: bool = true;
    const ENABLE_EXPAND: bool = true;

    fn setup(_item: &gtk::ListItem) -> (Self::Root, Self::Widgets) {
        let album_label = gtk::Label::builder()
            .halign(gtk::Align::Start)
            .ellipsize(gtk::pango::EllipsizeMode::End)
            .build();
        (gtk::Viewport::default(), album_label)
    }

    fn bind(item: &mut Self::Item, album_label: &mut Self::Widgets, view: &mut Self::Root) {
        let stock = gettext("Unknown Album");
        let album = item.item.album.as_deref().unwrap_or(&stock);
        if let Some(album_id) = &item.item.album_id {
            // set text with link
            let album = gtk::glib::markup_escape_text(album);
            album_label.set_markup(&format!("<a href=\"\">{album}</a>"));
            let album_id = album_id.clone();
            let sender = item.sender.clone();
            album_label.connect_activate_link(move |_label, _id| {
                let id = Id::album(&album_id);
                sender.output(PlaylistsViewOut::ClickedAlbum(id)).unwrap();
                gtk::glib::signal::Propagation::Stop
            });
        } else {
            // set plain text
            album_label.set_text(album);
        }

        view.set_child(Some(album_label));
    }

    fn unbind(_item: &mut Self::Item, album_label: &mut Self::Widgets, view: &mut Self::Root) {
        album_label.set_text("");
        view.set_child(None::<&gtk::Widget>);
    }

    fn sort_fn() -> relm4::typed_view::OrdFn<Self::Item> {
        Some(Box::new(|a, b| a.item.album.cmp(&b.item.album)))
    }
}

pub struct GenreColumn;

impl relm4::typed_view::column::RelmColumn for GenreColumn {
    type Root = gtk::Box;
    type Item = PlaylistRow;
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
        b.set_hexpand(true);
        b.append(&label);
        (b, (label))
    }

    fn bind(item: &mut Self::Item, label: &mut Self::Widgets, _b: &mut Self::Root) {
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
    type Root = gtk::Box;
    type Item = PlaylistRow;
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

    fn bind(item: &mut Self::Item, label: &mut Self::Widgets, _b: &mut Self::Root) {
        let length = convert_for_label(i64::from(item.item.duration.unwrap_or(0)) * 1000);
        label.set_label(&length);
    }

    fn sort_fn() -> relm4::typed_view::OrdFn<Self::Item> {
        Some(Box::new(|a, b| a.item.duration.cmp(&b.item.duration)))
    }
}

pub struct BitRateColumn;

impl relm4::typed_view::column::RelmColumn for BitRateColumn {
    type Root = gtk::Box;
    type Item = PlaylistRow;
    type Widgets = gtk::Label;

    const COLUMN_NAME: &'static str = "Bitrate";
    const ENABLE_RESIZE: bool = false;
    const ENABLE_EXPAND: bool = false;

    fn setup(_item: &gtk::ListItem) -> (Self::Root, Self::Widgets) {
        let b = gtk::Box::default();
        let label = gtk::Label::default();
        b.set_hexpand(true);
        b.append(&label);
        (b, (label))
    }

    fn bind(item: &mut Self::Item, label: &mut Self::Widgets, _b: &mut Self::Root) {
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

        view.set_child(Some(fav_btn));
    }

    fn sort_fn() -> relm4::typed_view::OrdFn<Self::Item> {
        Some(Box::new(|a, b| b.item.starred.cmp(&a.item.starred)))
    }
}
