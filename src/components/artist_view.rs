use std::{cell::RefCell, rc::Rc};

use fuzzy_matcher::FuzzyMatcher;
use gettextrs::gettext;
use relm4::{
    gtk::{
        self,
        prelude::{BoxExt, ButtonExt, OrientableExt, ToValue, WidgetExt},
    },
    ComponentController, RelmWidgetExt,
};

use crate::components::{
    album_element::{
        get_info_of_flowboxchild, AlbumElement, AlbumElementIn, AlbumElementInit, AlbumElementOut,
    },
    cover::{Cover, CoverIn, CoverOut},
};
use crate::{client::Client, subsonic::Subsonic, types::Droppable};

#[derive(Debug)]
pub struct ArtistView {
    subsonic: Rc<RefCell<Subsonic>>,
    init: submarine::data::ArtistId3,
    cover: relm4::Controller<Cover>,
    favorite: gtk::Button,
    title: String,
    bio: String,
    albums: relm4::factory::FactoryVecDeque<AlbumElement>,
}

#[derive(Debug)]
pub enum ArtistViewIn {
    AlbumElement(AlbumElementOut),
    Cover(CoverOut),
    SearchChanged(String),
    FavoritedArtist(String, bool),
    FavoritedAlbum(String, bool),
    HoverCover(bool),
}

#[derive(Debug)]
pub enum ArtistViewOut {
    AlbumClicked(AlbumElementInit),
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
    LoadedAlbums(Result<submarine::data::ArtistWithAlbumsId3, submarine::SubsonicError>),
    LoadedArtist(Result<submarine::data::ArtistInfo, submarine::SubsonicError>),
}

#[relm4::component(pub)]
impl relm4::Component for ArtistView {
    type Init = (Rc<RefCell<Subsonic>>, submarine::data::ArtistId3);
    type Input = ArtistViewIn;
    type Output = ArtistViewOut;
    type Widgets = ArtistViewWidgets;
    type CommandOutput = ArtistViewCmd;

    fn init(
        (subsonic, init): Self::Init,
        root: Self::Root,
        sender: relm4::ComponentSender<Self>,
    ) -> relm4::ComponentParts<Self> {
        let model = Self {
            subsonic: subsonic.clone(),
            init: init.clone(),
            cover: Cover::builder()
                .launch((subsonic.clone(), init.clone().cover_art))
                .forward(sender.input_sender(), ArtistViewIn::Cover),
            favorite: gtk::Button::default(),
            title: init.name.clone(),
            bio: String::new(),
            albums: relm4::factory::FactoryVecDeque::builder()
                .launch(gtk::FlowBox::default())
                .forward(sender.input_sender(), ArtistViewIn::AlbumElement),
        };
        let widgets = view_output!();

        //setup DropSource
        let drop = Droppable::Artist(Box::new(init.clone()));
        let content = gtk::gdk::ContentProvider::for_value(&drop.to_value());
        let drag_src = gtk::DragSource::new();
        drag_src.set_actions(gtk::gdk::DragAction::COPY);
        drag_src.set_content(Some(&content));
        let artist = init.clone();
        drag_src.connect_drag_begin(move |src, _drag| {
            if let Some(cover_id) = &artist.cover_art {
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
            .emit(CoverIn::LoadArtist(Box::new(init.clone())));
        model.cover.model().add_css_class_image("size150");

        // set favorite icon
        if init.starred.is_some() {
            model.favorite.set_icon_name("starred-symbolic");
        }

        // load albums
        let id = init.id.clone();
        sender.oneshot_command(async move {
            let client = Client::get().unwrap();
            ArtistViewCmd::LoadedAlbums(client.get_artist(id).await)
        });

        // load metainfo on artist
        let id = init.id.clone();
        sender.oneshot_command(async move {
            let client = Client::get().unwrap();
            let info = client.get_artist_info2(id, Some(5), Some(false)).await;
            ArtistViewCmd::LoadedArtist(info)
        });

        relm4::ComponentParts { model, widgets }
    }

    view! {
        gtk::Box {
            add_css_class: "artist-view",
            set_orientation: gtk::Orientation::Vertical,

            gtk::Box {
                set_spacing: 15,
                add_css_class: "artist-view-info",

                gtk::Overlay {
                    add_overlay = &model.favorite.clone() {
                        set_halign: gtk::Align::End,
                        set_valign: gtk::Align::End,
                        set_width_request: 24,
                        set_height_request: 24,
                        set_icon_name: "non-starred-symbolic",

                        connect_clicked[sender, init] => move |btn| {
                            let state = match btn.icon_name().as_deref() {
                                Some("starred-symbolic") => false,
                                Some("non-starred-symbolic") => true,
                                name => unreachable!("unknown icon name: {name:?}"),
                            };
                            sender.output(ArtistViewOut::FavoriteArtistClicked(init.id.clone(), state)).unwrap();
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
                            add_css_class: "h2",
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

                            gtk::Box {
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
                                    connect_clicked[sender, init] => move |_btn| {
                                        sender.output(ArtistViewOut::AppendArtist(Droppable::Artist(Box::new(init.clone())))).unwrap();
                                    }
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
                                connect_clicked[sender, init] => move |_btn| {
                                    sender.output(ArtistViewOut::InsertAfterCurrentPlayed(Droppable::Artist(Box::new(init.clone())))).unwrap();
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
                                connect_clicked[sender, init] => move |_btn| {
                                    sender.output(ArtistViewOut::ReplaceQueue(Droppable::Artist(Box::new(init.clone())))).unwrap();
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
                                connect_clicked[sender, init] => move |_btn| {
                                    sender.output(ArtistViewOut::Download(Droppable::Artist(Box::new(init.clone())))).unwrap();
                                }
                            }
                        }
                    }
                }
            },

            gtk::ScrolledWindow {
                set_vexpand: true,

                model.albums.widget().clone() {
                    set_valign: gtk::Align::Start,
                },
            }
        }
    }

    fn update(
        &mut self,
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
            ArtistViewIn::SearchChanged(search) => {
                self.albums.widget().set_filter_func(move |element| {
                    let (title, _artist) = get_info_of_flowboxchild(element);

                    //actual matching
                    let matcher = fuzzy_matcher::skim::SkimMatcherV2::default();
                    let score = matcher.fuzzy_match(&title.text(), &search);
                    score.is_some()
                });
            }
            ArtistViewIn::FavoritedArtist(id, state) => {
                if self.init.id == id {
                    match state {
                        true => self.favorite.set_icon_name("starred-symbolic"),
                        false => self.favorite.set_icon_name("non-starred-symbolic"),
                    }
                }
            }
            ArtistViewIn::FavoritedAlbum(id, state) => {
                self.albums.broadcast(AlbumElementIn::Favorited(id, state));
            }
            ArtistViewIn::HoverCover(false) => {
                self.favorite.remove_css_class("cover-favorite");
                if self.favorite.icon_name().as_deref() != Some("starred-symbolic") {
                    self.favorite.set_visible(false);
                }
            }
            ArtistViewIn::HoverCover(true) => {
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
            ArtistViewCmd::LoadedArtist(Err(e)) | ArtistViewCmd::LoadedAlbums(Err(e)) => sender
                .output(ArtistViewOut::DisplayToast(format!(
                    "error loading artist: {e}"
                )))
                .unwrap(),
            ArtistViewCmd::LoadedArtist(Ok(artist)) => {
                if let Some(bio) = artist.base.biography {
                    self.bio = bio;
                } else {
                    self.bio = gettext("No biography found");
                }

                // TODO do smth with similar artists
            }
            ArtistViewCmd::LoadedAlbums(Ok(artist)) => {
                let mut guard = self.albums.guard();
                for album in artist.album {
                    guard.push_back((
                        self.subsonic.clone(),
                        AlbumElementInit::AlbumId3(Box::new(album)),
                    ));
                }
            }
        }
    }
}
