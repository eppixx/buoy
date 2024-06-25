
use fuzzy_matcher::FuzzyMatcher;
use relm4::{
    gtk::{
        self, glib,
        prelude::{BoxExt, ButtonExt, OrientableExt, ToValue, WidgetExt},
    },
    ComponentController,
};

use std::{cell::RefCell, rc::Rc};

use super::cover::{Cover, CoverOut};
use crate::{
    client::Client,
    common::convert_for_label,
    components::cover::CoverIn,
    subsonic::Subsonic,
    types::Droppable,
};
use crate::factory::playlist_tracks_row::{
    AlbumColumn, ArtistColumn, FavColumn, LengthColumn, PlaylistTracksRow, PositionColumn, TitleColumn,
};

#[derive(Debug)]
pub struct AlbumView {
    cover: relm4::Controller<Cover>,
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
    DisplayToast(String),
}

#[derive(Debug)]
pub enum AlbumViewIn {
    AlbumTracks,
    Cover(CoverOut),
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
            cover: Cover::builder()
                .launch((subsonic.clone(), cover))
                .forward(sender.input_sender(), AlbumViewIn::Cover),
            title: String::from("Unkonwn Title"),
            artist: None,
            info: String::new(),
            tracks,
        };

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

                model.cover.widget().clone() -> gtk::Box {},

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

            model.tracks.view.clone() {
                set_vexpand: true,
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
                    self.tracks.append(PlaylistTracksRow::new(track.clone()));
                }

                // update dragSource
                let drop = Droppable::AlbumWithSongs(Box::new(album.clone()));
                let content = gtk::gdk::ContentProvider::for_value(&drop.to_value());
                let drag_src = gtk::DragSource::new();
                drag_src.set_actions(gtk::gdk::DragAction::MOVE);
                drag_src.set_content(Some(&content));
                self.cover.widget().add_controller(drag_src);

                //update self
                self.info = build_info_string(&album);
                self.cover
                    .emit(CoverIn::LoadAlbumId3(Box::new(album.clone())));
                self.title = album.base.name;
                self.artist = album.base.artist;
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
