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

use crate::{client::Client, subsonic::Subsonic, types::Droppable};
use crate::{
    components::{
        album_element::{get_info_of_flowboxchild, AlbumElement, AlbumElementIn, AlbumElementOut},
        cover::{Cover, CoverIn, CoverOut},
    },
    types::Id,
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
}

#[derive(Debug)]
pub enum ArtistViewIn {
    AlbumElement(AlbumElementOut),
    Cover(CoverOut),
    FilterChanged(String),
    FavoritedArtist(String, bool),
    FavoritedAlbum(String, bool),
    HoverCover(bool),
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
                                    connect_clicked[sender, artist] => move |_btn| {
                                        sender.output(ArtistViewOut::AppendArtist(Droppable::Artist(Box::new(artist.clone())))).unwrap();
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
            ArtistViewIn::FilterChanged(search) => {
                self.albums.widget().set_filter_func(move |element| {
                    let (title, _artist) = get_info_of_flowboxchild(element);

                    //actual matching
                    let matcher = fuzzy_matcher::skim::SkimMatcherV2::default();
                    let score = matcher.fuzzy_match(&title.text(), &search);
                    score.is_some()
                });
            }
            ArtistViewIn::FavoritedArtist(id, state) => {
                if self.id.as_ref() == id {
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
                self.favorite.remove_css_class("neutral-color");
                if self.favorite.icon_name().as_deref() != Some("starred-symbolic") {
                    self.favorite.set_visible(false);
                }
            }
            ArtistViewIn::HoverCover(true) => {
                self.favorite.add_css_class("neutral-color");
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

                // TODO do smth with similar artists
            }
        }
    }
}
