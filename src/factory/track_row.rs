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
    components::{
        album_view::{AlbumView, AlbumViewOut},
        tracks_view::{TracksView, TracksViewOut},
    },
    subsonic::Subsonic,
    types::{Droppable, Id},
};

#[derive(Debug)]
pub struct TrackRow {
    subsonic: Rc<RefCell<Subsonic>>,
    pub item: submarine::data::Child,
    fav_btn: gtk::Button,
    position_box: gtk::Box,
    position_box_drag: gtk::DragSource,
    title_box: gtk::Box,
    title_box_drag: gtk::DragSource,
    artist_box: gtk::Box,
    artist_box_drag: gtk::DragSource,
    album_box: gtk::Box,
    album_box_drag: gtk::DragSource,
    length_box: gtk::Box,
    length_box_drag: gtk::DragSource,
    bitrate_box: gtk::Box,
    bitrate_box_drag: gtk::DragSource,
    content_set: bool,
}

impl PartialEq for TrackRow {
    fn eq(&self, other: &Self) -> bool {
        self.item == other.item
    }
}

impl TrackRow {
    pub fn new(subsonic: &Rc<RefCell<Subsonic>>, item: submarine::data::Child) -> Self {
        let fav = match item.starred.is_some() {
            true => String::from("starred-symbolic"),
            false => String::from("non-starred-symbolic"),
        };

        let fav_btn = gtk::Button::from_icon_name(&fav);
        fav_btn.set_tooltip(&gettext("Click to (un)favorite song"));
        fav_btn.set_focus_on_click(false);

        let artist_label = gtk::Label::builder()
            .halign(gtk::Align::Start)
            .ellipsize(gtk::pango::EllipsizeMode::End)
            .build();
        artist_label.inline_css("color: inherit");

        let album_label = gtk::Label::builder()
            .halign(gtk::Align::Start)
            .ellipsize(gtk::pango::EllipsizeMode::End)
            .build();
        album_label.inline_css("color: inherit");

        let result = Self {
            subsonic: subsonic.clone(),
            item,
            fav_btn,
            position_box: gtk::Box::default(),
            position_box_drag: gtk::DragSource::default(),
            title_box: gtk::Box::default(),
            title_box_drag: gtk::DragSource::default(),
            album_box: gtk::Box::default(),
            album_box_drag: gtk::DragSource::default(),
            artist_box: gtk::Box::default(),
            artist_box_drag: gtk::DragSource::default(),
            length_box: gtk::Box::default(),
            length_box_drag: gtk::DragSource::default(),
            bitrate_box: gtk::Box::default(),
            bitrate_box_drag: gtk::DragSource::default(),
            content_set: false,
        };

        //setup position label
        let position_label = gtk::Label::builder().build();
        let mut text = String::new();
        if let Some(cd) = result.item.disc_number {
            text.push_str(&cd.to_string());
            text.push('.');
        }
        if let Some(track) = result.item.track {
            text = format!("{text}{track:02}");
        }
        position_label.set_label(&text);
        result.position_box.append(&position_label);

        // setup title label
        let title_label = gtk::Label::builder()
            .halign(gtk::Align::Start)
            .ellipsize(gtk::pango::EllipsizeMode::End)
            .label(&result.item.title)
            .build();
        result.title_box.append(&title_label);
        result.title_box.add_controller(result.get_drag_src());
        result
            .title_box_drag
            .set_actions(gtk::gdk::DragAction::COPY);

        //setup length_box
        let length = convert_for_label(i64::from(result.item.duration.unwrap_or(0)) * 1000);
        let length_label = gtk::Label::new(Some(&length));
        result.length_box.append(&length_label);
        result.length_box.add_controller(result.get_drag_src());

        //setup bitrate_box
        let bitrate = result.item.bit_rate;
        let bitrate = bitrate.map(|n| n.to_string());
        let bitrate_label = gtk::Label::new(Some(&bitrate.unwrap_or(String::from("-"))));
        result.bitrate_box.append(&bitrate_label);
        result.bitrate_box.add_controller(result.get_drag_src());

        result
    }

    pub fn new_track(
        subsonic: &Rc<RefCell<Subsonic>>,
        item: submarine::data::Child,
        sender: &relm4::ComponentSender<TracksView>,
    ) -> Self {
        let result = Self::new(subsonic, item);

        let id = result.item.id.clone();
        let send = sender.clone();
        result
            .fav_btn
            .connect_clicked(move |btn| match btn.icon_name().as_deref() {
                Some("starred-symbolic") => {
                    btn.set_icon_name("non-starred-symbolic");
                    send.output(TracksViewOut::FavoriteClicked(id.clone(), false))
                        .unwrap();
                }
                Some("non-starred-symbolic") => {
                    btn.set_icon_name("starred-symbolic");
                    send.output(TracksViewOut::FavoriteClicked(id.clone(), true))
                        .unwrap();
                }
                _ => unreachable!("unkown icon name"),
            });

        // setup album label
        let album_label = gtk::Label::builder()
            .halign(gtk::Align::Start)
            .ellipsize(gtk::pango::EllipsizeMode::End)
            .build();
        album_label.inline_css("color: inherit");
        let stock = gettext("Unknown Album");
        let album = result.item.album.as_deref().unwrap_or(&stock);
        let send = sender.clone();
        if let Some(album_id) = &result.item.album_id {
            let album = gtk::glib::markup_escape_text(album);
            album_label.set_markup(&format!("<a href=\"\">{album}</a>"));
            let album_id = album_id.clone();
            album_label.connect_activate_link(move |_label, _id| {
                let id = Id::album(&album_id);
                send.output(TracksViewOut::ClickedAlbum(id)).unwrap();
                gtk::glib::signal::Propagation::Stop
            });
        } else {
            album_label.set_text(album);
        }
        result.album_box.append(&album_label);
        result.album_box.add_controller(result.get_drag_src());
        result
            .album_box_drag
            .set_actions(gtk::gdk::DragAction::COPY);

        // setup artist label
        let artist_label = gtk::Label::builder()
            .halign(gtk::Align::Start)
            .ellipsize(gtk::pango::EllipsizeMode::End)
            .build();
        let sender = sender.clone();
        artist_label.inline_css("color: inherit");
        let stock = gettext("Unknown Artist");
        let artist = result.item.artist.as_deref().unwrap_or(&stock);
        if let Some(artist_id) = &result.item.artist_id {
            let artist = gtk::glib::markup_escape_text(artist);
            artist_label.set_markup(&format!("<a href=\"\">{artist}</a>"));
            let artist_id = artist_id.clone();
            artist_label.connect_activate_link(move |_label, _id| {
                let id = Id::artist(&artist_id);
                sender.output(TracksViewOut::ClickedArtist(id)).unwrap();
                gtk::glib::signal::Propagation::Stop
            });
        } else {
            artist_label.set_text(artist);
        }
        result.artist_box.append(&artist_label);
        result.artist_box.add_controller(result.get_drag_src());
        result
            .artist_box_drag
            .set_actions(gtk::gdk::DragAction::COPY);

        result
    }

    pub fn new_album_track(
        subsonic: &Rc<RefCell<Subsonic>>,
        item: submarine::data::Child,
        sender: relm4::ComponentSender<AlbumView>,
    ) -> Self {
        let result = Self::new(subsonic, item);

        let id = result.item.id.clone();
        let send = sender.clone();
        result
            .fav_btn
            .connect_clicked(move |btn| match btn.icon_name().as_deref() {
                Some("starred-symbolic") => {
                    btn.set_icon_name("non-starred-symbolic");
                    send.output(AlbumViewOut::FavoriteClicked(id.clone(), false))
                        .unwrap();
                }
                Some("non-starred-symbolic") => {
                    btn.set_icon_name("starred-symbolic");
                    send.output(AlbumViewOut::FavoriteClicked(id.clone(), true))
                        .unwrap();
                }
                _ => unreachable!("unkown icon name"),
            });

        // setup album label
        let album_label = gtk::Label::builder()
            .halign(gtk::Align::Start)
            .ellipsize(gtk::pango::EllipsizeMode::End)
            .build();
        let stock = gettext("Unknown Album");
        let album = result.item.album.as_deref().unwrap_or(&stock);
        album_label.set_label(album);
        result.album_box.append(&album_label);

        // setup artist label
        let artist_label = gtk::Label::builder()
            .halign(gtk::Align::Start)
            .ellipsize(gtk::pango::EllipsizeMode::End)
            .build();
        artist_label.inline_css("color: inherit");
        let stock = gettext("Unknown Album");
        let artist = result.item.artist.as_deref().unwrap_or(&stock);
        if let Some(artist_id) = &result.item.artist_id {
            let artist = gtk::glib::markup_escape_text(artist);
            artist_label.set_markup(&format!("<a href=\"\">{artist}</a>"));
            let artist_id = artist_id.clone();
            artist_label.connect_activate_link(move |_label, _id| {
                let id = Id::artist(&artist_id);
                sender.output(AlbumViewOut::ArtistClicked(id)).unwrap();
                gtk::glib::signal::Propagation::Stop
            });
        } else {
            artist_label.set_text(artist);
        }
        result.artist_box.append(&artist_label);
        result.artist_box.add_controller(result.get_drag_src());
        result
            .artist_box_drag
            .set_actions(gtk::gdk::DragAction::COPY);

        result
    }

    fn get_widgets(&self) -> [(&gtk::DragSource, &gtk::Box); 6] {
        //add new widgets here
        [
            (&self.position_box_drag, &self.position_box),
            (&self.title_box_drag, &self.title_box),
            (&self.artist_box_drag, &self.artist_box),
            (&self.album_box_drag, &self.album_box),
            (&self.length_box_drag, &self.length_box),
            (&self.bitrate_box_drag, &self.bitrate_box),
        ]
    }

    pub fn set_drag_src(&mut self, drop: Droppable) {
        for (src, widget) in self.get_widgets() {
            //remove DragSource before add new one
            if self.content_set {
                widget.remove_controller(src);
            }

            let content = gtk::gdk::ContentProvider::for_value(&drop.to_value());
            src.set_content(Some(&content));
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
            widget.add_controller(src.clone());
        }
        self.content_set = true;
    }

    pub fn remove_drag_src(&mut self) {
        if !self.content_set {
            return;
        }
        for (src, widget) in self.get_widgets() {
            widget.remove_controller(src);
        }
        self.content_set = false;
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
    type Root = gtk::Viewport;
    type Item = TrackRow;
    type Widgets = ();

    const COLUMN_NAME: &'static str = "#";
    const ENABLE_RESIZE: bool = false;
    const ENABLE_EXPAND: bool = false;

    fn setup(_item: &gtk::ListItem) -> (Self::Root, Self::Widgets) {
        (gtk::Viewport::default(), ())
    }

    fn bind(item: &mut Self::Item, _: &mut Self::Widgets, view: &mut Self::Root) {
        view.set_child(Some(&item.position_box));
    }

    fn sort_fn() -> relm4::typed_view::OrdFn<Self::Item> {
        Some(Box::new(|a, b| a.item.track.cmp(&b.item.track)))
    }
}

pub struct TitleColumn;

impl relm4::typed_view::column::RelmColumn for TitleColumn {
    type Root = gtk::Viewport;
    type Item = TrackRow;
    type Widgets = ();

    const COLUMN_NAME: &'static str = "Title";
    const ENABLE_RESIZE: bool = true;
    const ENABLE_EXPAND: bool = true;

    fn setup(_item: &gtk::ListItem) -> (Self::Root, Self::Widgets) {
        (gtk::Viewport::default(), ())
    }

    fn bind(item: &mut Self::Item, _: &mut Self::Widgets, view: &mut Self::Root) {
        view.set_child(Some(&item.title_box));
    }

    fn sort_fn() -> relm4::typed_view::OrdFn<Self::Item> {
        Some(Box::new(|a, b| a.item.title.cmp(&b.item.title)))
    }
}

pub struct ArtistColumn;

impl relm4::typed_view::column::RelmColumn for ArtistColumn {
    type Root = gtk::Viewport;
    type Item = TrackRow;
    type Widgets = ();

    const COLUMN_NAME: &'static str = "Artist";
    const ENABLE_RESIZE: bool = true;
    const ENABLE_EXPAND: bool = true;

    fn setup(_item: &gtk::ListItem) -> (Self::Root, Self::Widgets) {
        (gtk::Viewport::default(), ())
    }

    fn bind(item: &mut Self::Item, _: &mut Self::Widgets, view: &mut Self::Root) {
        view.set_child(Some(&item.artist_box));
    }

    fn sort_fn() -> relm4::typed_view::OrdFn<Self::Item> {
        Some(Box::new(|a, b| a.item.artist.cmp(&b.item.artist)))
    }
}

pub struct AlbumColumn;

impl relm4::typed_view::column::RelmColumn for AlbumColumn {
    type Root = gtk::Viewport;
    type Item = TrackRow;
    type Widgets = ();

    const COLUMN_NAME: &'static str = "Album";
    const ENABLE_RESIZE: bool = true;
    const ENABLE_EXPAND: bool = true;

    fn setup(_item: &gtk::ListItem) -> (Self::Root, Self::Widgets) {
        (gtk::Viewport::default(), ())
    }

    fn bind(item: &mut Self::Item, _: &mut Self::Widgets, view: &mut Self::Root) {
        view.set_child(Some(&item.album_box));
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

    fn bind(item: &mut Self::Item, label: &mut Self::Widgets, b: &mut Self::Root) {
        label.set_label(
            item.item
                .genre
                .as_deref()
                .unwrap_or(&gettext("Unknown genre")),
        );
        b.add_controller(item.get_drag_src());
    }

    fn sort_fn() -> relm4::typed_view::OrdFn<Self::Item> {
        Some(Box::new(|a, b| a.item.genre.cmp(&b.item.genre)))
    }
}

pub struct LengthColumn;

impl relm4::typed_view::column::RelmColumn for LengthColumn {
    type Root = gtk::Viewport;
    type Item = TrackRow;
    type Widgets = ();

    const COLUMN_NAME: &'static str = "Length";
    const ENABLE_RESIZE: bool = false;
    const ENABLE_EXPAND: bool = false;

    fn setup(_item: &gtk::ListItem) -> (Self::Root, Self::Widgets) {
        (gtk::Viewport::default(), ())
    }

    fn bind(item: &mut Self::Item, _: &mut Self::Widgets, view: &mut Self::Root) {
        view.set_child(Some(&item.length_box));
    }

    fn sort_fn() -> relm4::typed_view::OrdFn<Self::Item> {
        Some(Box::new(|a, b| a.item.duration.cmp(&b.item.duration)))
    }
}

pub struct BitRateColumn;

impl relm4::typed_view::column::RelmColumn for BitRateColumn {
    type Root = gtk::Viewport;
    type Item = TrackRow;
    type Widgets = ();

    const COLUMN_NAME: &'static str = "Bitrate";
    const ENABLE_RESIZE: bool = false;
    const ENABLE_EXPAND: bool = false;

    fn setup(_item: &gtk::ListItem) -> (Self::Root, Self::Widgets) {
        (gtk::Viewport::default(), ())
    }

    fn bind(item: &mut Self::Item, _: &mut Self::Widgets, view: &mut Self::Root) {
        view.set_child(Some(&item.bitrate_box));
    }

    fn sort_fn() -> relm4::typed_view::OrdFn<Self::Item> {
        Some(Box::new(|a, b| a.item.bit_rate.cmp(&b.item.bit_rate)))
    }
}

pub struct FavColumn;

impl relm4::typed_view::column::RelmColumn for FavColumn {
    type Root = gtk::Viewport;
    type Item = TrackRow;
    type Widgets = ();

    const COLUMN_NAME: &'static str = "Favorite";
    const ENABLE_RESIZE: bool = false;

    fn setup(_item: &gtk::ListItem) -> (Self::Root, Self::Widgets) {
        (gtk::Viewport::default(), ())
    }

    fn bind(item: &mut Self::Item, _: &mut Self::Widgets, view: &mut Self::Root) {
        view.set_child(Some(&item.fav_btn));
    }

    fn sort_fn() -> relm4::typed_view::OrdFn<Self::Item> {
        Some(Box::new(|a, b| b.item.starred.cmp(&a.item.starred)))
    }
}
