use std::{cell::RefCell, rc::Rc};

use fuzzy_matcher::FuzzyMatcher;
use relm4::{
    gtk::{
        self, glib,
        prelude::{BoxExt, ButtonExt, ListModelExt, OrientableExt, ToValue, WidgetExt},
    },
    ComponentController,
};

use crate::{
    client::Client,
    common::convert_for_label,
    components::cover::{Cover, CoverIn, CoverOut},
    subsonic::Subsonic,
    types::Droppable,
};
use crate::{
    factory::track_row::{
        AlbumColumn, ArtistColumn, FavColumn, LengthColumn, TrackRow, PositionColumn,
        TitleColumn,
    },
    settings::Settings,
};

#[derive(Debug)]
pub struct AlbumView {
    subsonic: Rc<RefCell<Subsonic>>,
    init: AlbumViewInit,
    cover: relm4::Controller<Cover>,
    favorite: gtk::Button,
    title: String,
    artist: Option<String>,
    info: String,
    tracks: relm4::typed_view::column::TypedColumnView<TrackRow, gtk::SingleSelection>,
}

#[derive(Debug, Clone)]
pub enum AlbumViewInit {
    Child(Box<submarine::data::Child>),
    AlbumId3(Box<submarine::data::AlbumId3>),
}

#[derive(Debug)]
pub enum AlbumViewOut {
    AppendAlbum(Droppable),
    InsertAfterCurrentPlayed(Droppable),
    ReplaceQueue(Droppable),
    FavoriteClicked(String, bool),
    DisplayToast(String),
    Download(Droppable),
}

#[derive(Debug)]
pub enum AlbumViewIn {
    AlbumTracks,
    Cover(CoverOut),
    FavoritedAlbum(String, bool),
    FavoritedSong(String, bool),
    SearchChanged(String),
    HoverCover(bool),
}

#[derive(Debug)]
pub enum AlbumViewCmd {
    LoadedAlbum(Result<submarine::data::AlbumWithSongsId3, submarine::SubsonicError>),
}

#[relm4::component(pub)]
impl relm4::Component for AlbumView {
    type Init = (Rc<RefCell<Subsonic>>, AlbumViewInit);
    type Input = AlbumViewIn;
    type Output = AlbumViewOut;
    type Widgets = AlbumViewWidgets;
    type CommandOutput = AlbumViewCmd;

    fn init(
        (subsonic, init): Self::Init,
        root: Self::Root,
        sender: relm4::ComponentSender<Self>,
    ) -> relm4::ComponentParts<Self> {
        let cover = match init.clone() {
            AlbumViewInit::Child(child) => child.cover_art,
            AlbumViewInit::AlbumId3(album) => album.cover_art,
        };

        let mut tracks = relm4::typed_view::column::TypedColumnView::<
            TrackRow,
            gtk::SingleSelection,
        >::new();
        tracks.append_column::<PositionColumn>();
        tracks.append_column::<TitleColumn>();
        tracks.append_column::<ArtistColumn>();
        tracks.append_column::<AlbumColumn>();
        tracks.append_column::<LengthColumn>();
        tracks.append_column::<FavColumn>();

        let model = Self {
            subsonic: subsonic.clone(),
            init: init.clone(),
            cover: Cover::builder()
                .launch((subsonic.clone(), cover))
                .forward(sender.input_sender(), AlbumViewIn::Cover),
            favorite: gtk::Button::default(),
            title: String::from("Unkonwn Title"),
            artist: None,
            info: String::new(),
            tracks,
        };

        let init2 = init.clone();
        let widgets = view_output!();
        model.cover.model().add_css_class_image("size100");

        //load album
        sender.oneshot_command(async move {
            let id = match &init {
                AlbumViewInit::Child(child) => &child.id,
                AlbumViewInit::AlbumId3(album) => &album.id,
            };

            let client = Client::get().unwrap();
            AlbumViewCmd::LoadedAlbum(client.get_album(id).await)
        });

        relm4::ComponentParts { model, widgets }
    }

    view! {
        gtk::Box {
            add_css_class: "album-view",
            set_orientation: gtk::Orientation::Vertical,

            gtk::Box {
                set_spacing: 15,
                add_css_class: "album-view-info",

                gtk::Overlay {
                    #[wrap(Some)]
                    set_child = &model.cover.widget().clone() -> gtk::Box {},
                    add_overlay = &model.favorite.clone() {
                        set_halign: gtk::Align::End,
                        set_valign: gtk::Align::End,
                        set_width_request: 24,
                        set_height_request: 24,
                        set_icon_name: "non-starred-symbolic",

                        connect_clicked[sender] => move |btn| {
                            let id: String = match &init2 {
                                AlbumViewInit::Child(child) => child.id.clone(),
                                AlbumViewInit::AlbumId3(album) => album.id.clone(),
                            };
                            let state = match btn.icon_name().as_deref() {
                                Some("non-starred-symbolic") => true,
                                Some("starred-symbolic") => false,
                                _ => true,
                            };
                            sender.output(AlbumViewOut::FavoriteClicked(id, state)).unwrap();
                        }
                    },

                    add_controller = gtk::EventControllerMotion {
                        connect_enter[sender] => move |_event, _x, _y| {
                            sender.input(AlbumViewIn::HoverCover(true));
                        },
                        connect_leave => AlbumViewIn::HoverCover(false),
                    },
                },

                gtk::WindowHandle {
                    set_hexpand: true,

                    gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_spacing: 8,

                        gtk::Label {
                            add_css_class: "h3",
                            #[watch]
                            set_label: &model.title,
                            set_halign: gtk::Align::Start,
                        },
                        gtk::Label {
                            #[watch]
                            set_markup: &format!("by <span style=\"italic\">{}</span>",
                                                 glib::markup_escape_text(model.artist.as_deref().unwrap_or("Unkown Artist"))),
                            set_halign: gtk::Align::Start,
                        },
                        gtk::Label {
                            #[watch]
                            set_label: &model.info,
                            set_halign: gtk::Align::Start,
                        },
                        gtk::Box {
                            set_spacing: 15,
                            gtk::Button {
                                gtk::Box {
                                    gtk::Image {
                                        set_icon_name: Some("list-add-symbolic"),
                                    },
                                    gtk::Label {
                                        set_label: "Append",
                                    }
                                },
                                set_tooltip_text: Some("Append Album to end of queue"),
                                connect_clicked[sender, init] => move |_btn| {
                                    match &init {
                                        AlbumViewInit::Child(child) => {
                                            sender.output(AlbumViewOut::AppendAlbum(Droppable::AlbumChild(child.clone()))).unwrap();
                                        }
                                        AlbumViewInit::AlbumId3(album) => {
                                            sender.output(AlbumViewOut::AppendAlbum(Droppable::Album(album.clone()))).unwrap();
                                        }
                                    }
                                }
                            },
                            gtk::Button {
                                gtk::Box {
                                    gtk::Image {
                                        set_icon_name: Some("list-add-symbolic"),
                                    },
                                    gtk::Label {
                                        set_label: "Play next"
                                    }
                                },
                                set_tooltip_text: Some("Insert Album after currently played or paused item"),
                                connect_clicked[sender, init] => move |_btn| {
                                    match &init {
                                        AlbumViewInit::Child(child) => {
                                            sender.output(AlbumViewOut::InsertAfterCurrentPlayed(Droppable::AlbumChild(child.clone()))).unwrap();
                                        }
                                        AlbumViewInit::AlbumId3(album) => {
                                            sender.output(AlbumViewOut::InsertAfterCurrentPlayed(Droppable::Album(album.clone()))).unwrap();
                                        }
                                    }
                                }
                            },
                            gtk::Button {
                                gtk::Box {
                                    gtk::Image {
                                        set_icon_name: Some("emblem-symbolic-link-symbolic"),
                                    },
                                    gtk::Label {
                                        set_label: "Replace queue",
                                    }
                                },
                                set_tooltip_text: Some("Replaces current queue with this album"),
                                connect_clicked[sender, init] => move |_btn| {
                                    match &init {
                                        AlbumViewInit::Child(child) => {
                                            sender.output(AlbumViewOut::ReplaceQueue(Droppable::AlbumChild(child.clone()))).unwrap();
                                        }
                                        AlbumViewInit::AlbumId3(album) => {
                                            sender.output(AlbumViewOut::ReplaceQueue(Droppable::Album(album.clone()))).unwrap();
                                        }
                                    }
                                }
                            },
                            gtk::Button {
                                gtk::Box {
                                    gtk::Image {
                                        set_icon_name: Some("browser-download-symbolic"),
                                    },
                                    gtk::Label {
                                        set_label: "Download Album",
                                    }
                                },
                                set_tooltip_text: Some("Click to select a folder to download this album to"),
                                connect_clicked[sender, init] => move |_btn| {
                                    let drop = match &init {
                                        AlbumViewInit::Child(child) => Droppable::Child(child.clone()),
                                        AlbumViewInit::AlbumId3(id3) => Droppable::Album(id3.clone()),
                                    };
                                    sender.output(AlbumViewOut::Download(drop)).unwrap();
                                }
                            }
                        }
                    }
                },
            },

            gtk::ScrolledWindow {
                set_hexpand: true,
                set_vexpand: true,

                model.tracks.view.clone() {
                    add_css_class: "album-view-tracks-row",
                    set_vexpand: true,
                }
            },
        }
    }

    fn update(
        &mut self,
        msg: Self::Input,
        sender: relm4::ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match msg {
            AlbumViewIn::AlbumTracks => {} //do nothing
            AlbumViewIn::Cover(msg) => match msg {
                CoverOut::DisplayToast(title) => {
                    sender.output(AlbumViewOut::DisplayToast(title)).unwrap()
                }
            },
            AlbumViewIn::SearchChanged(search) => {
                self.tracks.clear_filters();
                self.tracks.add_filter(move |row| {
                    let mut search = search.clone();
                    let mut test = format!(
                        "{} {} {}",
                        row.item.title,
                        row.item.artist.as_deref().unwrap_or_default(),
                        row.item.album.as_deref().unwrap_or_default()
                    );

                    //check for case sensitivity
                    if !Settings::get().lock().unwrap().case_sensitive {
                        test = test.to_lowercase();
                        search = search.to_lowercase();
                    }

                    //actual matching
                    let fuzzy_search = Settings::get().lock().unwrap().fuzzy_search;
                    if fuzzy_search {
                        let matcher = fuzzy_matcher::skim::SkimMatcherV2::default();
                        let score = matcher.fuzzy_match(&test, &search);
                        score.is_some()
                    } else {
                        test.contains(&search)
                    }
                });
            }
            AlbumViewIn::FavoritedAlbum(id, state) => {
                let matched = match &self.init {
                    AlbumViewInit::AlbumId3(album) => album.id == id,
                    AlbumViewInit::Child(child) => child.id == id,
                };
                match state {
                    true if matched => self.favorite.set_icon_name("starred-symbolic"),
                    false if matched => self.favorite.set_icon_name("non-starred-symbolic"),
                    _ => {}
                }
            }
            AlbumViewIn::FavoritedSong(id, state) => {
                use relm4::typed_view::TypedListItem;

                let len = self.tracks.view.columns().n_items();
                let tracks: Vec<TypedListItem<TrackRow>> =
                    (0..len).filter_map(|i| self.tracks.get(i)).collect();
                for track in tracks {
                    let track_id = track.borrow().item.id.clone();
                    if track_id == id {
                        match state {
                            true => {
                                track
                                    .borrow_mut()
                                    .fav
                                    .set_value(String::from("starred-symbolic"));
                                track.borrow_mut().item.starred =
                                    Some(chrono::offset::Local::now().into());
                            }
                            false => {
                                track
                                    .borrow_mut()
                                    .fav
                                    .set_value(String::from("non-starred-symbolic"));
                                track.borrow_mut().item.starred = None;
                            }
                        }
                    }
                }
            }
            AlbumViewIn::HoverCover(false) => {
                self.favorite.remove_css_class("cover-favorite");
                if self.favorite.icon_name().as_deref() != Some("starred-symbolic") {
                    self.favorite.set_visible(false);
                }
            }
            AlbumViewIn::HoverCover(true) => {
                self.favorite.add_css_class("cover-favorite");
                self.favorite.set_visible(true);
            }
        }
    }

    fn update_cmd(
        &mut self,
        msg: Self::CommandOutput,
        sender: relm4::ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match msg {
            AlbumViewCmd::LoadedAlbum(Err(e)) => {
                sender
                    .output(AlbumViewOut::DisplayToast(format!("could not load: {e:?}")))
                    .unwrap();
            }
            AlbumViewCmd::LoadedAlbum(Ok(album)) => {
                //load tracks
                for track in &album.song {
                    self.tracks
                        .append(TrackRow::new(&self.subsonic, track.clone()));
                }

                // update dragSource
                let drop = Droppable::AlbumWithSongs(Box::new(album.clone()));
                let content = gtk::gdk::ContentProvider::for_value(&drop.to_value());
                let drag_src = gtk::DragSource::new();
                drag_src.set_actions(gtk::gdk::DragAction::COPY);
                drag_src.set_content(Some(&content));
                let subsonic = self.subsonic.clone();
                let alb = album.clone();
                drag_src.connect_drag_begin(move |src, _drag| {
                    if let Some(cover_id) = &alb.base.cover_art {
                        let cover = subsonic.borrow().cover_icon(cover_id);
                        if let Some(tex) = cover {
                            src.set_icon(Some(&tex), 0, 0);
                        }
                    }
                });
                self.cover.widget().add_controller(drag_src);

                //update self
                self.info = build_info_string(&album);
                self.cover
                    .emit(CoverIn::LoadAlbumId3(Box::new(album.clone())));
                self.title = album.base.name;
                self.artist = album.base.artist;
                self.favorite.set_visible(false);
                if album.base.starred.is_some() {
                    self.favorite.set_icon_name("starred-symbolic");
                    self.favorite.set_visible(true);
                }
            }
        }
    }
}

fn build_info_string(child: &submarine::data::AlbumWithSongsId3) -> String {
    let mut result = String::from("Songs: ");
    result.push_str(&child.song.len().to_string());
    result.push_str(" • Length: ");
    result.push_str(&convert_for_label(i64::from(child.base.duration) * 1000));
    if let Some(year) = child.base.year {
        result.push_str(" • Release: ");
        result.push_str(&year.to_string());
    }
    if let Some(played) = child.base.play_count {
        result.push_str(" • played ");
        result.push_str(&played.to_string());
        result.push_str(" times");
    }
    if let Some(genre) = &child.base.genre {
        result.push_str(" • Genre: ");
        result.push_str(&genre.to_string());
    }
    result
}
