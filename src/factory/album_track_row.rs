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
    common::{
        convert_for_label,
        types::{Droppable, Id},
    },
    components::album_view::{AlbumView, AlbumViewOut},
    factory::SetupFinished,
    subsonic::Subsonic,
};

#[derive(Debug)]
pub struct AlbumTrackRow {
    subsonic: Rc<RefCell<Subsonic>>,
    item: submarine::data::Child,
    play_count: Option<gtk::Label>,
    fav_btn: Option<gtk::Button>,
    title_box: gtk::Viewport,
    sender: relm4::ComponentSender<AlbumView>,
    multiple_drag_src: Option<gtk::DragSource>,
}

impl PartialEq for AlbumTrackRow {
    fn eq(&self, other: &Self) -> bool {
        self.item == other.item
    }
}

impl AlbumTrackRow {
    pub fn new(
        subsonic: &Rc<RefCell<Subsonic>>,
        item: submarine::data::Child,
        sender: relm4::ComponentSender<AlbumView>,
    ) -> Self {
        let result = Self {
            subsonic: subsonic.clone(),
            item,
            play_count: None,
            fav_btn: None,
            title_box: gtk::Viewport::default(),
            sender: sender.clone(),
            multiple_drag_src: None,
        };

        //setup title label
        let title_label = gtk::Label::builder()
            .halign(gtk::Align::Start)
            .ellipsize(gtk::pango::EllipsizeMode::End)
            .label(&result.item.title)
            .build();
        result.title_box.set_child(Some(&title_label));

        result
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
}

pub struct Model {
    subsonic: Rc<RefCell<Option<Rc<RefCell<Subsonic>>>>>,
    child: Rc<RefCell<Option<submarine::data::Child>>>,
    drag_src: gtk::DragSource,
}

impl Model {
    fn new() -> (gtk::Viewport, Self) {
        let model = Model {
            subsonic: Rc::new(RefCell::new(None)),
            child: Rc::new(RefCell::new(None)),
            drag_src: gtk::DragSource::default(),
        };

        let root = gtk::Viewport::default();

        // create DragSource
        model.drag_src.set_actions(gtk::gdk::DragAction::COPY);

        // set drag icon
        let subsonic = model.subsonic.clone();
        let child = model.child.clone();
        model.drag_src.connect_drag_begin(move |src, _drag| {
            let Some(ref subsonic) = *subsonic.borrow() else {
                return;
            };
            let Some(ref child) = *child.borrow() else {
                return;
            };
            let Some(album) = subsonic.borrow().album_of_song(child) else {
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

    fn set_from_row(&self, row: &AlbumTrackRow) {
        self.subsonic.replace(Some(row.subsonic.clone()));
        self.child.replace(Some(row.item.clone()));

        let drop = Droppable::Child(Box::new(row.item.clone()));
        let content = gtk::gdk::ContentProvider::for_value(&drop.to_value());
        self.drag_src.set_content(Some(&content));
    }
}

pub struct PositionColumn;

impl relm4::typed_view::column::RelmColumn for PositionColumn {
    type Root = gtk::Viewport;
    type Item = AlbumTrackRow;
    type Widgets = (Model, gtk::Label);

    const COLUMN_NAME: &'static str = "#";
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
    type Item = AlbumTrackRow;
    type Widgets = Model;

    const COLUMN_NAME: &'static str = "Title";
    const ENABLE_RESIZE: bool = true;
    const ENABLE_EXPAND: bool = true;

    fn setup(_item: &gtk::ListItem) -> (Self::Root, Self::Widgets) {
        Model::new()
    }

    fn bind(item: &mut Self::Item, model: &mut Self::Widgets, root: &mut Self::Root) {
        model.set_from_row(item);
        root.set_child(Some(&item.title_box));
    }

    fn sort_fn() -> relm4::typed_view::OrdFn<Self::Item> {
        Some(Box::new(|a, b| a.item.title.cmp(&b.item.title)))
    }
}

pub struct ArtistColumn;

impl relm4::typed_view::column::RelmColumn for ArtistColumn {
    type Root = gtk::Viewport;
    type Item = AlbumTrackRow;
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
                sender.output(AlbumViewOut::ArtistClicked(id)).unwrap();
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
    type Item = AlbumTrackRow;
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
        let stock = gettext("Unknown genre");
        label.set_label(item.item.genre.as_deref().unwrap_or(&stock));
    }

    fn sort_fn() -> relm4::typed_view::OrdFn<Self::Item> {
        Some(Box::new(|a, b| a.item.genre.cmp(&b.item.genre)))
    }
}

pub struct LengthColumn;

impl relm4::typed_view::column::RelmColumn for LengthColumn {
    type Root = gtk::Viewport;
    type Item = AlbumTrackRow;
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
    type Item = AlbumTrackRow;
    type Widgets = (Model, gtk::Label);

    const COLUMN_NAME: &'static str = "Play count";
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
        Some(Box::new(|a, b| a.item.bit_rate.cmp(&b.item.bit_rate)))
    }
}

pub struct BitRateColumn;

impl relm4::typed_view::column::RelmColumn for BitRateColumn {
    type Root = gtk::Viewport;
    type Item = AlbumTrackRow;
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
    type Item = AlbumTrackRow;
    type Widgets = (Rc<RefCell<String>>, gtk::Button, Model, SetupFinished);

    const COLUMN_NAME: &'static str = "Favorite";
    const ENABLE_RESIZE: bool = false;

    fn setup(_item: &gtk::ListItem) -> (Self::Root, Self::Widgets) {
        let (view, model) = Model::new();
        let fav_btn = super::create_fav_btn();
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
                        .output(AlbumViewOut::FavoriteSongClicked(
                            cell.borrow().clone(),
                            false,
                        ))
                        .unwrap();
                }
                Some("non-starred-symbolic") => {
                    btn.set_icon_name("starred-symbolic");
                    sender
                        .output(AlbumViewOut::FavoriteSongClicked(
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

    fn unbind(item: &mut Self::Item, _widgets: &mut Self::Widgets, _root: &mut Self::Root) {
        item.fav_btn = None;
    }

    fn sort_fn() -> relm4::typed_view::OrdFn<Self::Item> {
        Some(Box::new(|a, b| b.item.starred.cmp(&a.item.starred)))
    }
}
