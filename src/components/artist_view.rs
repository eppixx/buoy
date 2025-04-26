use std::{cell::RefCell, rc::Rc};

use fuzzy_matcher::FuzzyMatcher;
use gettextrs::gettext;
use itertools::Itertools;
use rand::seq::IteratorRandom;
use relm4::{
    gtk::{
        self, pango,
        prelude::{BoxExt, ButtonExt, OrientableExt, ToValue, WidgetExt},
    },
    ComponentController, RelmWidgetExt,
};

use crate::{
    client::Client,
    components::{
        album_element::{get_info_of_flowboxchild, AlbumElement, AlbumElementIn, AlbumElementOut},
        cover::{Cover, CoverIn, CoverOut},
    },
    factory::artist_song_row::ArtistSongRow,
    gtk_helper::{loading_widget::LoadingWidgetState, stack::StackExt},
    subsonic::Subsonic,
    types::{Droppable, Id},
};

#[derive(Debug)]
pub struct ArtistView {
    subsonic: Rc<RefCell<Subsonic>>,
    id: Id,
    cover: relm4::Controller<Cover>,
    favorite: gtk::Button,
    title: String,
    bio: String,
    albums: relm4::factory::FactoryVecDeque<AlbumElement>,
    most_played: relm4::typed_view::list::TypedListView<ArtistSongRow, gtk::SingleSelection>,
    random_songs: relm4::typed_view::list::TypedListView<ArtistSongRow, gtk::SingleSelection>,
    similar_songs: relm4::typed_view::list::TypedListView<ArtistSongRow, gtk::SingleSelection>,
}

#[derive(Debug)]
pub enum ArtistViewIn {
    AlbumElement(AlbumElementOut),
    Cover(CoverOut),
    FilterChanged(String),
    UpdateFavoriteArtist(String, bool),
    UpdateFavoriteAlbum(String, bool),
    HoverCover(bool),
    ClickedRandomize,
}

#[derive(Debug)]
pub enum ArtistViewOut {
    AlbumClicked(Id),
    AppendArtist(Droppable),
    InsertAfterCurrentPlayed(Droppable),
    ReplaceQueue(Droppable),
    DisplayToast(String),
    FavoriteAlbumClicked(String, bool),
    FavoriteArtistClicked(String, bool),
    Download(Droppable),
}

#[derive(Debug)]
pub enum ArtistViewCmd {
    LoadedArtistInfo(Result<submarine::data::ArtistInfo, submarine::SubsonicError>),
    LoadedSimilarSongs(Result<Vec<submarine::data::Child>, submarine::SubsonicError>),
}

#[relm4::component(pub)]
impl relm4::Component for ArtistView {
    type Init = (Rc<RefCell<Subsonic>>, Id);
    type Input = ArtistViewIn;
    type Output = ArtistViewOut;
    type Widgets = ArtistViewWidgets;
    type CommandOutput = ArtistViewCmd;

    fn init(
        (subsonic, id): Self::Init,
        root: Self::Root,
        sender: relm4::ComponentSender<Self>,
    ) -> relm4::ComponentParts<Self> {
        //check id
        let Id::Artist(_) = &id else {
            panic!("given id: '{id}' is not an artist");
        };
        let artist = subsonic.borrow().find_artist(id.as_ref()).unwrap();

        let mut model = Self {
            subsonic: subsonic.clone(),
            id,
            cover: Cover::builder()
                .launch((subsonic.clone(), artist.clone().cover_art))
                .forward(sender.input_sender(), ArtistViewIn::Cover),
            favorite: gtk::Button::default(),
            title: artist.name.clone(),
            bio: String::new(),
            albums: relm4::factory::FactoryVecDeque::builder()
                .launch(gtk::FlowBox::default())
                .forward(sender.input_sender(), ArtistViewIn::AlbumElement),
            most_played: relm4::typed_view::list::TypedListView::<
                ArtistSongRow,
                gtk::SingleSelection,
            >::new(),
            random_songs: relm4::typed_view::list::TypedListView::<
                ArtistSongRow,
                gtk::SingleSelection,
            >::new(),
            similar_songs: relm4::typed_view::list::TypedListView::<
                ArtistSongRow,
                gtk::SingleSelection,
            >::new(),
        };
        let widgets = view_output!();

        //setup DropSource
        let droppable = Droppable::Artist(Box::new(artist.clone()));
        let content = gtk::gdk::ContentProvider::for_value(&droppable.to_value());
        let drag_src = gtk::DragSource::new();
        drag_src.set_actions(gtk::gdk::DragAction::COPY);
        drag_src.set_content(Some(&content));
        let cover = artist.cover_art.clone();
        drag_src.connect_drag_begin(move |src, _drag| {
            if let Some(cover_id) = &cover {
                let cover = subsonic.borrow().cover_icon(cover_id);
                if let Some(tex) = cover {
                    src.set_icon(Some(&tex), 0, 0);
                }
            }
        });
        model.cover.widget().add_controller(drag_src);

        // load cover
        model
            .cover
            .emit(CoverIn::LoadArtist(Box::new(artist.clone())));
        model.cover.model().add_css_class_image("size150");

        // set favorite icon
        if artist.starred.is_some() {
            model.favorite.set_icon_name("starred-symbolic");
        }

        // load albums
        let mut guard = model.albums.guard();
        for album in model.subsonic.borrow().albums_from_artist(&artist) {
            guard.push_back((model.subsonic.clone(), Id::album(&album.id)));
        }
        drop(guard);

        // load metainfo on artist
        let id = artist.id.clone();
        sender.oneshot_command(async move {
            let client = Client::get().unwrap();
            let info = client.get_artist_info2(id, Some(5), Some(false)).await;
            ArtistViewCmd::LoadedArtistInfo(info)
        });

        let songs = model.subsonic.borrow().songs_of_artist(&artist.id);
        if !songs.is_empty() {
            // calc most played songs
            let most_played: Vec<_> = songs
                .iter()
                .filter(|s| s.play_count.is_some())
                .sorted_by(|a, b| a.play_count.cmp(&b.play_count))
                .take(5)
                .cloned()
                .collect();
            if most_played.is_empty() {
                widgets
                    .most_played_stack
                    .set_visible_child_enum(&LoadingWidgetState::Empty);
            } else {
                for song in most_played {
                    let new_row = ArtistSongRow::new(&model.subsonic, &song, &sender);
                    model.most_played.append(new_row);
                }
                widgets
                    .most_played_stack
                    .set_visible_child_enum(&LoadingWidgetState::NotEmpty);
            }
        }

        // calc random songs
        sender.input(ArtistViewIn::ClickedRandomize);

        // load similar songs
        let id = artist.id.clone();
        sender.oneshot_command(async move {
            let client = Client::get().unwrap();
            ArtistViewCmd::LoadedSimilarSongs(client.get_similar_songs2(id, Some(5)).await)
        });

        // add widgets to SizeGroup
        let group = gtk::SizeGroup::new(gtk::SizeGroupMode::Both);
        group.add_widget(&widgets.most_played_box);
        group.add_widget(&widgets.random_songs_box);
        group.add_widget(&widgets.similar_songs_box);

        relm4::ComponentParts { model, widgets }
    }

    view! {
        gtk::Box {
            set_widget_name: "artist-view",
            set_orientation: gtk::Orientation::Vertical,

            gtk::Box {
                set_spacing: 15,

                gtk::Overlay {
                    add_overlay = &model.favorite.clone() {
                        set_halign: gtk::Align::End,
                        set_valign: gtk::Align::End,
                        set_width_request: 24,
                        set_height_request: 24,
                        set_icon_name: "non-starred-symbolic",

                        connect_clicked[sender, artist] => move |btn| {
                            let state = match btn.icon_name().as_deref() {
                                Some("starred-symbolic") => false,
                                Some("non-starred-symbolic") => true,
                                name => unreachable!("unknown icon name: {name:?}"),
                            };
                            sender.output(ArtistViewOut::FavoriteArtistClicked(artist.id.clone(), state)).unwrap();
                        }
                    },

                    #[wrap(Some)]
                    set_child = &gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        model.cover.widget().clone() -> gtk::Box {},
                    },

                    add_controller = gtk::EventControllerMotion {
                        connect_enter[sender] => move |_event, _x, _y| {
                            sender.input(ArtistViewIn::HoverCover(true));
                        },
                        connect_leave => ArtistViewIn::HoverCover(false),
                    },
                },

                gtk::WindowHandle {
                    set_hexpand: true,

                    gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_spacing: 5,

                        gtk::Label {
                            add_css_class: granite::STYLE_CLASS_H2_LABEL,
                            #[watch]
                            set_label: &model.title,
                            set_halign: gtk::Align::Start,
                        },
                        gtk::Label {
                            #[watch]
                            set_markup: &gtk::glib::markup_escape_text(&model.bio),
                            set_halign: gtk::Align::Start,
                            set_single_line_mode: false,
                            set_lines: -1,
                            set_wrap: true,
                        },
                        gtk::Box {
                            set_spacing: 15,

                            gtk::Button {
                                gtk::Box {
                                    gtk::Image {
                                        set_icon_name: Some("list-add-symbolic"),
                                    },
                                    gtk::Label {
                                        set_label: &gettext("Append"),
                                    },
                                },
                                set_tooltip: &gettext("Append Artist to end of queue"),
                                connect_clicked[sender, artist] => move |_btn| {
                                    sender.output(ArtistViewOut::AppendArtist(Droppable::Artist(Box::new(artist.clone())))).unwrap();
                                }
                            },
                            gtk::Button {
                                gtk::Box {
                                    gtk::Image {
                                        set_icon_name: Some("list-add-symbolic"),
                                    },
                                    gtk::Label {
                                        set_label: &gettext("Play next"),
                                    }
                                },
                                set_tooltip: &gettext("Insert Artist after currently played or paused item"),
                                connect_clicked[sender, artist] => move |_btn| {
                                    sender.output(ArtistViewOut::InsertAfterCurrentPlayed(Droppable::Artist(Box::new(artist.clone())))).unwrap();
                                }
                            },
                            gtk::Button {
                                gtk::Box {
                                    gtk::Image {
                                        set_icon_name: Some("emblem-symbolic-link-symbolic"),
                                    },
                                    gtk::Label {
                                        set_label: &gettext("Replace queue"),
                                    }
                                },
                                set_tooltip: &gettext("Replaces current queue with this artist"),
                                connect_clicked[sender, artist] => move |_btn| {
                                    sender.output(ArtistViewOut::ReplaceQueue(Droppable::Artist(Box::new(artist.clone())))).unwrap();
                                }
                            },
                            gtk::Button {
                                gtk::Box {
                                    gtk::Image {
                                        set_icon_name: Some("browser-download-symbolic"),
                                    },
                                    gtk::Label {
                                        set_label: &gettext("Download Artist"),
                                    }
                                },
                                set_tooltip: &gettext("Click to select a folder to download this artist to"),
                                connect_clicked[sender, artist] => move |_btn| {
                                    sender.output(ArtistViewOut::Download(Droppable::Artist(Box::new(artist.clone())))).unwrap();
                                }
                            }
                        }
                    }
                }
            },

            gtk::ScrolledWindow {
                set_vexpand: true,

                gtk::Box {
                    set_spacing: 10,
                    set_orientation: gtk::Orientation::Vertical,

                    // box for most played, random and similar songs
                    gtk::Box {
                        set_spacing: 10,

                        append: most_played_box = &gtk::Box {
                            set_orientation: gtk::Orientation::Vertical,

                            gtk::Label {
                                add_css_class: granite::STYLE_CLASS_H2_LABEL,
                                set_halign: gtk::Align::Start,
                                set_label: &gettext("Most played"),
                            },

                            append: most_played_stack = &gtk::Stack {
                                set_transition_type: gtk::StackTransitionType::Crossfade,
                                set_transition_duration: 200,

                                add_enumed[LoadingWidgetState::Loading] = &gtk::Box {
                                    set_orientation: gtk::Orientation::Vertical,
                                    set_valign: gtk::Align::Center,

                                    gtk::Spinner {
                                        add_css_class: "size32",
                                        set_spinning: true,
                                        start: (),
                                    }
                                },
                                add_enumed[LoadingWidgetState::NotEmpty] = &gtk::Box {
                                    set_widget_name: "artist-song-box",

                                    model.most_played.view.clone() {
                                        set_hexpand: true,
                                    }
                                },
                                add_enumed[LoadingWidgetState::Empty] = &gtk::Box {
                                    set_orientation: gtk::Orientation::Vertical,
                                    set_valign: gtk::Align::Center,

                                    gtk::Label {
                                        add_css_class: granite::STYLE_CLASS_H3_LABEL,
                                        set_label: &gettext("No top played songs yet"),
                                    },
                                },

                                set_visible_child_enum: &LoadingWidgetState::Loading,
                            }
                        },

                        append: random_songs_box = &gtk::Box {
                            set_orientation: gtk::Orientation::Vertical,

                            gtk::Box {
                                set_spacing: 10,

                                gtk::Label {
                                    add_css_class: granite::STYLE_CLASS_H2_LABEL,
                                    set_halign: gtk::Align::Start,
                                    set_label: &gettext("Random songs")
                                },
                                gtk::Button {
                                    set_icon_name: "media-playlist-shuffle-symbolic",
                                    set_tooltip: &gettext("Rerandomize songs"),
                                    connect_clicked => ArtistViewIn::ClickedRandomize,
                                }
                            },

                            append: random_songs_stack = &gtk::Stack {
                                set_transition_type: gtk::StackTransitionType::Crossfade,
                                set_transition_duration: 200,

                                add_enumed[LoadingWidgetState::NotEmpty] = &gtk::Box {
                                    set_widget_name: "artist-song-box",

                                    model.random_songs.view.clone() {
                                        set_hexpand: true,
                                    }
                                },
                                add_enumed[LoadingWidgetState::Empty] = &gtk::Box {
                                    set_orientation: gtk::Orientation::Vertical,
                                    set_valign: gtk::Align::Center,

                                    gtk::Label {
                                        add_css_class: granite::STYLE_CLASS_H3_LABEL,
                                        set_label: &gettext("No random songs available"),
                                    },
                                },

                                set_visible_child_enum: &LoadingWidgetState::Empty,
                            }
                        },

                        append: similar_songs_box = &gtk::Box {
                            set_orientation: gtk::Orientation::Vertical,

                            gtk::Label {
                                add_css_class: granite::STYLE_CLASS_H2_LABEL,
                                set_halign: gtk::Align::Start,
                                set_label: &gettext("Similar songs")
                            },

                            append: similar_songs_stack = &gtk::Stack {
                                set_transition_type: gtk::StackTransitionType::Crossfade,
                                set_transition_duration: 200,

                                add_enumed[LoadingWidgetState::Loading] = &gtk::Box {
                                    set_orientation: gtk::Orientation::Vertical,
                                    set_valign: gtk::Align::Center,

                                    gtk::Spinner {
                                        add_css_class: "size32",
                                        set_spinning: true,
                                        start: (),
                                    }
                                },
                                add_enumed[LoadingWidgetState::NotEmpty] = &gtk::Box {
                                    set_widget_name: "artist-song-box",

                                    model.similar_songs.view.clone() {
                                        set_hexpand: true,
                                    }
                                },
                                add_enumed[LoadingWidgetState::Empty] = &gtk::Box {
                                    set_orientation: gtk::Orientation::Vertical,
                                    set_valign: gtk::Align::Center,

                                    gtk::Label {
                                        add_css_class: granite::STYLE_CLASS_H3_LABEL,
                                        set_label: &gettext("Server returned no similar songs"),
                                        set_ellipsize: pango::EllipsizeMode::End,
                                    },
                                    gtk::Label {
                                        set_label: &gettext("There my be additioinal setup steps for your server required"),
                                        set_ellipsize: pango::EllipsizeMode::End,

                                    }
                                },

                                set_visible_child_enum: &LoadingWidgetState::Loading,
                            }
                        }
                    },

                    gtk::Label {
                        add_css_class: granite::STYLE_CLASS_H2_LABEL,
                        set_halign: gtk::Align::Start,
                        set_label: &gettext("All albums"),
                    },

                    model.albums.widget().clone() {
                        set_valign: gtk::Align::Start,
                    }
                },
            }
        }
    }

    fn update_with_view(
        &mut self,
        widgets: &mut Self::Widgets,
        msg: Self::Input,
        sender: relm4::ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match msg {
            ArtistViewIn::AlbumElement(msg) => match msg {
                AlbumElementOut::Clicked(id) => {
                    sender.output(ArtistViewOut::AlbumClicked(id)).unwrap();
                }
                AlbumElementOut::DisplayToast(title) => {
                    sender.output(ArtistViewOut::DisplayToast(title)).unwrap();
                }
                AlbumElementOut::FavoriteClicked(id, state) => sender
                    .output(ArtistViewOut::FavoriteAlbumClicked(id, state))
                    .unwrap(),
            },
            ArtistViewIn::Cover(msg) => match msg {
                CoverOut::DisplayToast(title) => {
                    sender.output(ArtistViewOut::DisplayToast(title)).unwrap();
                }
            },
            ArtistViewIn::FilterChanged(search) => {
                self.albums.widget().set_filter_func(move |element| {
                    let (title, _artist) = get_info_of_flowboxchild(element).unwrap();

                    //actual matching
                    let matcher = fuzzy_matcher::skim::SkimMatcherV2::default();
                    let score = matcher.fuzzy_match(&title.text(), &search);
                    score.is_some()
                });
            }
            ArtistViewIn::UpdateFavoriteArtist(id, state) => {
                if self.id.as_ref() == id {
                    match state {
                        true => self.favorite.set_icon_name("starred-symbolic"),
                        false => self.favorite.set_icon_name("non-starred-symbolic"),
                    }
                }
            }
            ArtistViewIn::UpdateFavoriteAlbum(id, state) => {
                self.albums.broadcast(AlbumElementIn::Favorited(id, state));
            }
            ArtistViewIn::HoverCover(false) => {
                self.favorite.remove_css_class("neutral-color");
                if self.favorite.icon_name().as_deref() != Some("starred-symbolic") {
                    self.favorite.set_visible(false);
                }
            }
            ArtistViewIn::HoverCover(true) => {
                self.favorite.add_css_class("neutral-color");
                self.favorite.set_visible(true);
            }
            ArtistViewIn::ClickedRandomize => {
                let songs = self.subsonic.borrow().songs_of_artist(self.id.inner());
                let mut rng = rand::rng();
                let random_songs = songs.iter().choose_multiple(&mut rng, 5);
                if random_songs.is_empty() {
                    widgets
                        .random_songs_stack
                        .set_visible_child_enum(&LoadingWidgetState::Empty);
                } else {
                    self.random_songs.clear();
                    for song in random_songs {
                        let new_row = ArtistSongRow::new(&self.subsonic, song, &sender);
                        self.random_songs.append(new_row);
                    }
                    widgets
                        .random_songs_stack
                        .set_visible_child_enum(&LoadingWidgetState::NotEmpty);
                }
            }
        }
    }

    fn update_cmd_with_view(
        &mut self,
        widgets: &mut Self::Widgets,
        msg: Self::CommandOutput,
        sender: relm4::ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match msg {
            ArtistViewCmd::LoadedArtistInfo(Err(e)) => sender
                .output(ArtistViewOut::DisplayToast(format!(
                    "error loading artist: {e}"
                )))
                .unwrap(),
            ArtistViewCmd::LoadedArtistInfo(Ok(artist)) => {
                self.bio = artist
                    .base
                    .biography
                    .unwrap_or(gettext("No biography found"));
            }
            ArtistViewCmd::LoadedSimilarSongs(Err(e)) => sender
                .output(ArtistViewOut::DisplayToast(format!(
                    "error loading similar songs: {e}"
                )))
                .unwrap(),
            ArtistViewCmd::LoadedSimilarSongs(Ok(songs)) => {
                if songs.is_empty() {
                    widgets
                        .similar_songs_stack
                        .set_visible_child_enum(&LoadingWidgetState::Empty);
                    return;
                }
                for song in songs {
                    let new_row = ArtistSongRow::new(&self.subsonic, &song, &sender);
                    self.similar_songs.append(new_row);
                }
                widgets
                    .similar_songs_stack
                    .set_visible_child_enum(&LoadingWidgetState::NotEmpty);
            }
        }
    }
}
