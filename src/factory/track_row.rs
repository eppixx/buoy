use std::{cell::RefCell, rc::Rc};

use gettextrs::gettext;
use relm4::{
    gtk::{
        self,
        prelude::{BoxExt, ButtonExt, ToValue, WidgetExt},
    },
    RelmWidgetExt,
};

use crate::{
    common::convert_for_label,
    components::tracks_view::{TracksView, TracksViewIn, TracksViewOut},
    factory::SetupFinished,
    subsonic::Subsonic,
    types::{Droppable, Id},
};

static UID: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

#[derive(Debug)]
pub struct TrackRow {
    uid: usize,
    subsonic: Rc<RefCell<Subsonic>>,
    item: submarine::data::Child,
    play_count: Option<gtk::Label>,
    fav_btn: Option<gtk::Button>,
    title_box: gtk::Viewport,
    sender: relm4::ComponentSender<TracksView>,
    multiple_drag_src: Option<gtk::DragSource>,
}

impl PartialEq for TrackRow {
    fn eq(&self, other: &Self) -> bool {
        self.item == other.item
    }
}

impl TrackRow {
    pub fn new(
        subsonic: &Rc<RefCell<Subsonic>>,
        item: submarine::data::Child,
        sender: &relm4::ComponentSender<TracksView>,
    ) -> Self {
        let uid = UID.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Self {
            uid,
            subsonic: subsonic.clone(),
            item,
            play_count: None,
            fav_btn: None,
            title_box: gtk::Viewport::default(),
            sender: sender.clone(),
            multiple_drag_src: None,
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

    pub fn set_drag_src(&mut self, drop: Droppable) {
        // remove old DragSource if there is one
        if self.multiple_drag_src.is_some() {
            self.remove_drag_src();
        }

        // create new DragSource
        let src = gtk::DragSource::default();
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

        self.multiple_drag_src = Some(src);
    }

    pub fn remove_drag_src(&mut self) {
        if let Some(src) = &self.multiple_drag_src {
            if let Some(list_item) = super::get_list_item_widget(&self.title_box) {
                list_item.remove_controller(src);
            }
        }
        self.multiple_drag_src = None;
    }

    fn create_drag_src(
        &self,
        cell: &Rc<RefCell<Option<submarine::data::Child>>>,
    ) -> gtk::DragSource {
        // create DragSource with content
        let src = gtk::DragSource::default();
        let drop = Droppable::Child(Box::new(self.item.clone()));
        let content = gtk::gdk::ContentProvider::for_value(&drop.to_value());
        src.set_content(Some(&content));
        src.set_actions(gtk::gdk::DragAction::COPY);

        let subsonic = self.subsonic.clone();
        let cell = cell.clone();
        src.connect_prepare(move |src, _x, _y| {
            // upate content from cell
            if let Some(child) = cell.borrow().clone() {
                // set drag icon
                let album = subsonic.borrow().album_of_song(&child);
                if let Some(album) = &album {
                    if let Some(cover_id) = &album.cover_art {
                        let cover = subsonic.borrow().cover_icon(cover_id);
                        if let Some(tex) = cover {
                            src.set_icon(Some(&tex), 0, 0);
                        }
                    }
                }

                // set content
                let drop = Droppable::Child(Box::new(child));
                let content = gtk::gdk::ContentProvider::for_value(&drop.to_value());
                return Some(content);
            }
            None
        });
        src
    }
}

pub struct PositionColumn;

impl relm4::typed_view::column::RelmColumn for PositionColumn {
    type Root = gtk::Label;
    type Item = TrackRow;
    type Widgets = ();

    const COLUMN_NAME: &'static str = "#";
    const ENABLE_RESIZE: bool = false;
    const ENABLE_EXPAND: bool = false;

    fn setup(_item: &gtk::ListItem) -> (Self::Root, Self::Widgets) {
        (gtk::Label::default(), ())
    }

    fn bind(item: &mut Self::Item, _: &mut Self::Widgets, label: &mut Self::Root) {
        let mut text = String::new();
        if let Some(cd) = item.item.disc_number {
            text.push_str(&cd.to_string());
            text.push('.');
        }
        if let Some(track) = item.item.track {
            text = format!("{text}{track:02}");
        }
        label.set_label(&text);

        if let (Some(cd), Some(track)) = (item.item.disc_number, item.item.track) {
            label.set_tooltip(&format!(
                "{} {track:02} {} {cd}",
                gettext("Track"),
                gettext("on CD")
            ));
        };
    }

    fn sort_fn() -> relm4::typed_view::OrdFn<Self::Item> {
        Some(Box::new(|a, b| a.item.track.cmp(&b.item.track)))
    }
}

pub struct TitleColumn;

impl relm4::typed_view::column::RelmColumn for TitleColumn {
    type Root = gtk::Viewport;
    type Item = TrackRow;
    type Widgets = (
        Rc<RefCell<Option<submarine::data::Child>>>,
        Rc<RefCell<usize>>,
        gtk::Label,
        SetupFinished,
    );

    const COLUMN_NAME: &'static str = "Title";
    const ENABLE_RESIZE: bool = true;
    const ENABLE_EXPAND: bool = true;

    fn setup(_item: &gtk::ListItem) -> (Self::Root, Self::Widgets) {
        let title_label = gtk::Label::builder()
            .halign(gtk::Align::Start)
            .ellipsize(gtk::pango::EllipsizeMode::End)
            .build();

        (
            gtk::Viewport::default(),
            (
                Rc::new(RefCell::new(None)),
                Rc::new(RefCell::new(0)),
                title_label,
                SetupFinished(false),
            ),
        )
    }

    fn bind(
        item: &mut Self::Item,
        (cell, uid, label, finished): &mut Self::Widgets,
        view: &mut Self::Root,
    ) {
        view.set_child(Some(&item.title_box));
        item.title_box.set_child(Some(label));
        label.set_text(&item.item.title);
        cell.replace(Some(item.item.clone()));
        uid.replace(*item.uid());

        // we need only 1 DragSource for the ListItem as it is updated by updating cell
        if !finished.0 {
            finished.0 = true;
            let list_item = super::get_list_item_widget(&item.title_box).unwrap();
            let drag_src = item.create_drag_src(cell);
            list_item.add_controller(drag_src);

            //connect left click
            let gesture = gtk::GestureClick::default();
            let sender = item.sender.clone();
            let uid = uid.clone();
            gesture.connect_pressed(move |_controller, button, _x, _y| {
                if button == 1 {
                    sender.input(TracksViewIn::TrackClicked(*uid.borrow()));
                }
            });
            list_item.add_controller(gesture);
        }
    }

    fn unbind(item: &mut Self::Item, _: &mut Self::Widgets, view: &mut Self::Root) {
        view.set_child(None::<&gtk::Widget>);
        item.title_box.set_child(None::<&gtk::Widget>);
    }

    fn sort_fn() -> relm4::typed_view::OrdFn<Self::Item> {
        Some(Box::new(|a, b| a.item.title.cmp(&b.item.title)))
    }
}

pub struct ArtistColumn;

impl relm4::typed_view::column::RelmColumn for ArtistColumn {
    type Root = gtk::Viewport;
    type Item = TrackRow;
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
                sender.output(TracksViewOut::ClickedArtist(id)).unwrap();
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
    type Item = TrackRow;
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
                sender.output(TracksViewOut::ClickedAlbum(id)).unwrap();
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
    type Item = TrackRow;
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
    type Item = TrackRow;
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

pub struct PlayCountColumn;

impl relm4::typed_view::column::RelmColumn for PlayCountColumn {
    type Root = gtk::Viewport;
    type Item = TrackRow;
    type Widgets = gtk::Label;

    const COLUMN_NAME: &'static str = "Plays";
    const ENABLE_RESIZE: bool = false;
    const ENABLE_EXPAND: bool = false;

    fn setup(_item: &gtk::ListItem) -> (Self::Root, Self::Widgets) {
        let view = gtk::Viewport::default();
        let label = gtk::Label::default();
        view.set_child(Some(&label));
        (view, label)
    }

    fn bind(item: &mut Self::Item, label: &mut Self::Widgets, _root: &mut Self::Root) {
        let play_count = item.item.play_count;
        let play_count = play_count.map(|n| n.to_string());
        label.set_label(&play_count.unwrap_or(String::from("-")));
        item.play_count = Some(label.clone());
    }

    fn unbind(item: &mut Self::Item, _label: &mut Self::Widgets, _root: &mut Self::Root) {
        item.play_count = None;
    }

    fn sort_fn() -> relm4::typed_view::OrdFn<Self::Item> {
        Some(Box::new(|a, b| a.item.play_count.cmp(&b.item.play_count)))
    }
}

pub struct BitRateColumn;

impl relm4::typed_view::column::RelmColumn for BitRateColumn {
    type Root = gtk::Box;
    type Item = TrackRow;
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
    type Item = TrackRow;
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
                        .output(TracksViewOut::FavoriteClicked(cell.borrow().clone(), false))
                        .unwrap();
                }
                Some("non-starred-symbolic") => {
                    btn.set_icon_name("starred-symbolic");
                    sender
                        .output(TracksViewOut::FavoriteClicked(cell.borrow().clone(), true))
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
