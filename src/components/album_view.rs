use fuzzy_matcher::FuzzyMatcher;
use relm4::{
    gtk::{
        self, glib,
        prelude::{BoxExt, ButtonExt, OrientableExt, ToValue, WidgetExt, ListModelExtManual, ListModelExt}
    },
    ComponentController,
};

use std::{cell::RefCell, rc::Rc};

use super::cover::{Cover, CoverOut};
use crate::factory::playlist_tracks_row::{
    AlbumColumn, ArtistColumn, FavColumn, LengthColumn, PlaylistTracksRow, PositionColumn,
    TitleColumn,
};
use crate::{
    client::Client, common::convert_for_label, components::cover::CoverIn, subsonic::Subsonic,
    types::Droppable,
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
    tracks: relm4::typed_view::column::TypedColumnView<PlaylistTracksRow, gtk::SingleSelection>,
}

#[derive(Debug, Clone)]
pub enum AlbumViewInit {
    Child(Box<submarine::data::Child>),
    AlbumId3(Box<submarine::data::AlbumId3>),
}

#[derive(Debug)]
pub enum AlbumViewOut {
    AppendAlbum(Droppable),
    InsertAfterCurrentPLayed(Droppable),
    FavoriteClicked(String, bool),
    DisplayToast(String),
}

#[derive(Debug)]
pub enum AlbumViewIn {
    AlbumTracks,
    Cover(CoverOut),
    FavoritedAlbum(String, bool),
    FavoritedSong(String, bool),
    SearchChanged(String),
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
            PlaylistTracksRow,
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
                            sender.output(AlbumViewOut::FavoriteClicked(id, state)).expect("sending failed");
                        }
                    }
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
                                            sender.output(AlbumViewOut::InsertAfterCurrentPLayed(Droppable::AlbumChild(child.clone()))).unwrap();
                                        }
                                        AlbumViewInit::AlbumId3(album) => {
                                            sender.output(AlbumViewOut::InsertAfterCurrentPLayed(Droppable::Album(album.clone()))).unwrap();
                                        }
                                    }
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
                CoverOut::DisplayToast(title) => sender
                    .output(AlbumViewOut::DisplayToast(title))
                    .expect("sending failed"),
            },
            AlbumViewIn::SearchChanged(search) => {
                self.tracks.clear_filters();
                self.tracks.add_filter(move |row| {
                    let matcher = fuzzy_matcher::skim::SkimMatcherV2::default();
                    let test = format!(
                        "{} {} {}",
                        row.item.title,
                        row.item.artist.as_deref().unwrap_or_default(),
                        row.item.album.as_deref().unwrap_or_default()
                    );
                    let score = matcher.fuzzy_match(&test, &search);
                    score.is_some()
                });
            }
            AlbumViewIn::FavoritedAlbum(id, state) => match &self.init {
                AlbumViewInit::Child(child) => {
                    if child.id == id {
                        match state {
                            true => self.favorite.set_icon_name("starred-symbolic"),
                            false => self.favorite.set_icon_name("non-starred-symbolic"),
                        }
                    }
                }
                AlbumViewInit::AlbumId3(album) => {
                    if album.id == id {
                        match state {
                            true => self.favorite.set_icon_name("starred-symbolic"),
                            false => self.favorite.set_icon_name("non-starred-symbolic"),
                        }
                    }
                }
            },
            AlbumViewIn::FavoritedSong(id, state) => {
                use relm4::typed_view::TypedListItem;

                let len = self.tracks.view.columns().n_items();
                let tracks: Vec<TypedListItem<PlaylistTracksRow>> = (0..len).into_iter().filter_map(|i| self.tracks.get(i)).collect();
                for track in tracks {
                    let track_id = track.borrow().item.id.clone();
                    if track_id == id {
                        match state {
                            true => {
                                track.borrow_mut().fav.set_value(String::from("starred-symbolic"));
                                track.borrow_mut().item.starred = Some(chrono::offset::Local::now().into());
                            }
                            false => {
                                track.borrow_mut().fav.set_value(String::from("non-starred-symbolic"));
                                track.borrow_mut().item.starred = None;
                            }
                        }
                    }
                }
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
                    .expect("sending failed");
            }
            AlbumViewCmd::LoadedAlbum(Ok(album)) => {
                //load tracks
                for track in &album.song {
                    self.tracks.append(PlaylistTracksRow::new(&self.subsonic, track.clone()));
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
                if album.base.starred.is_some() {
                    self.favorite.set_icon_name("starred-symbolic");
                }
            }
        }
    }
}

fn build_info_string(child: &submarine::data::AlbumWithSongsId3) -> String {
    let songs = format!("Songs: {}", child.song.len());
    let length = format!(
        " • Length: {}",
        convert_for_label(i64::from(child.base.duration) * 1000)
    );
    let year = match child.base.year {
        None => String::new(),
        Some(year) => format!(" • Release: {year}"),
    };
    let played = match child.base.play_count {
        None => String::new(),
        Some(count) => format!(" • played {count} times"),
    };
    let genre = match &child.base.genre {
        None => String::new(),
        Some(genre) => format!(" • Genre: {genre}"),
    };
    format!("{songs}{length}{year}{played}{genre}")
}
