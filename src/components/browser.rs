use std::{cell::RefCell, rc::Rc};

use relm4::{
    component::{AsyncComponent, AsyncComponentController},
    gtk::{
        self,
        traits::{BoxExt, ButtonExt, EditableExt, OrientableExt, ToggleButtonExt, WidgetExt},
    },
    Component, ComponentController, RelmWidgetExt,
};

use super::{
    album_element::AlbumElementInit,
    album_view::{AlbumViewInit, AlbumViewOut},
    albums_view::{AlbumsView, AlbumsViewOut},
    artist_view::ArtistViewOut,
    artists_view::{ArtistsView, ArtistsViewOut},
    dashboard::DashboardOutput,
};
use crate::{
    components::{album_view::AlbumView, artist_view::ArtistView, dashboard::Dashboard},
    subsonic::Subsonic,
    types::Droppable,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Views {
    Dashboard(gtk::Box),
    Artists(gtk::Box),
    Artist(gtk::Box),
    Albums(gtk::Box),
    Album(gtk::Box),
    Tracks(gtk::Box),
    Playlists(gtk::Box),
}

impl Views {
    fn widget(&self) -> &impl gtk::prelude::IsA<gtk::Widget> {
        match self {
            Self::Dashboard(w) => w,
            Self::Artists(w) => w,
            Self::Artist(w) => w,
            Self::Albums(w) => w,
            Self::Album(w) => w,
            Self::Tracks(w) => w,
            Self::Playlists(w) => w,
        }
    }
}

#[derive(Debug)]
pub struct Browser {
    subsonic: Rc<RefCell<Subsonic>>,
    history_widget: Vec<Views>,
    content: gtk::Viewport,
    back_btn: gtk::Button,
    dashboard_btn: gtk::ToggleButton,
    artists_btn: gtk::ToggleButton,
    albums_btn: gtk::ToggleButton,
    tracks_btn: gtk::ToggleButton,
    playlists_btn: gtk::ToggleButton,

    dashboards: Vec<relm4::Controller<Dashboard>>,
    artistss: Vec<relm4::component::AsyncController<ArtistsView>>,
    albumss: Vec<relm4::component::Controller<AlbumsView>>,
    album_views: Vec<relm4::Controller<AlbumView>>,
    artist_views: Vec<relm4::Controller<ArtistView>>,
}

#[derive(Debug)]
pub enum BrowserIn {
    SearchChanged(String),
    BackClicked,
    DashboardClicked,
    ArtistsClicked,
    AlbumsClicked,
    TracksClicked,
    PlaylistsClicked,
    Dashboard(DashboardOutput),
    AlbumsView(AlbumsViewOut),
    AlbumView(Box<AlbumViewOut>),
    ArtistsView(ArtistsViewOut),
    ArtistView(Box<ArtistViewOut>),
}

#[derive(Debug)]
pub enum BrowserOut {
    AppendToQueue(Droppable),
    InsertAfterCurrentInQueue(Droppable),
}

#[relm4::component(pub)]
impl relm4::SimpleComponent for Browser {
    type Init = Rc<RefCell<Subsonic>>;
    type Input = BrowserIn;
    type Output = BrowserOut;

    fn init(
        init: Self::Init,
        root: &Self::Root,
        sender: relm4::ComponentSender<Self>,
    ) -> relm4::ComponentParts<Self> {
        let model = Self {
            subsonic: init,
            history_widget: vec![],
            content: gtk::Viewport::default(),
            back_btn: gtk::Button::default(),
            dashboard_btn: gtk::ToggleButton::default(),
            artists_btn: gtk::ToggleButton::default(),
            albums_btn: gtk::ToggleButton::default(),
            tracks_btn: gtk::ToggleButton::default(),
            playlists_btn: gtk::ToggleButton::default(),

            dashboards: vec![],
            artistss: vec![],
            albumss: vec![],
            album_views: vec![],
            artist_views: vec![],
        };
        let widgets = view_output!();

        //TODO swtich default view
        sender.input(BrowserIn::DashboardClicked);
        // sender.input(BrowserIn::AlbumsClicked);

        relm4::ComponentParts { model, widgets }
    }

    view! {
        gtk::Box {
            add_css_class: "browser",
            set_orientation: gtk::Orientation::Vertical,

            gtk::Box {
                add_css_class: "browser-pathbar",

                append = &model.back_btn.clone() {
                    gtk::Box {
                        gtk::Image {
                            set_icon_name: Some("go-previous-symbolic"),
                        },
                        gtk::Label {
                            set_label: "Back",
                        },
                    },
                    connect_clicked => Self::Input::BackClicked,
                },

                gtk::Label {
                    add_css_class: "browser-pathbar-space",
                },

                gtk::Box {
                    set_spacing: 7,
                    set_hexpand: true,

                    append = &model.dashboard_btn.clone() -> gtk::ToggleButton {
                        set_icon_name: "go-home-symbolic",
                        set_tooltip: "Go to dashboard",
                        set_active: true,
                        connect_clicked => Self::Input::DashboardClicked,
                    },
                    append = &model.artists_btn.clone() -> gtk::ToggleButton {
                        set_icon_name: "avatar-default-symbolic",
                        set_tooltip: "Show Artists",
                        connect_clicked => Self::Input::ArtistsClicked,
                    },
                    append = &model.albums_btn.clone() -> gtk::ToggleButton {
                        set_icon_name: "media-optical-cd-audio-symbolic",
                        set_tooltip: "Show Albums",
                        connect_clicked => Self::Input::AlbumsClicked,
                    },
                    append = &model.tracks_btn.clone() -> gtk::ToggleButton {
                        set_icon_name: "audio-x-generic-symbolic",
                        set_tooltip: "Show Tracks",
                        connect_clicked => Self::Input::TracksClicked,
                    },
                    append = &model.playlists_btn.clone() -> gtk::ToggleButton {
                        set_icon_name: "playlist-symbolic",
                        set_tooltip: "Show playlists",
                        connect_clicked => Self::Input::PlaylistsClicked,
                    },
                },

                gtk::Label {
                    set_hexpand: true,
                    add_css_class: "browser-pathbar-space",
                },

                gtk::SearchEntry {
                    set_placeholder_text: Some("Search..."),
                    grab_focus: (),
                    connect_search_changed[sender] => move |w| {
                        sender.input(BrowserIn::SearchChanged(w.text().to_string()));
                    }
                }
            },

            append = &model.content.clone() -> gtk::Viewport {
                add_css_class: "browser-content",
            }
        }
    }

    fn update(&mut self, msg: Self::Input, sender: relm4::ComponentSender<Self>) {
        match msg {
            BrowserIn::SearchChanged(search) => {
                tracing::warn!("new search {search}");
            }
            BrowserIn::BackClicked => {
                if self.history_widget.len() > 1 {
                    // remove current view from history
                    match self.history_widget.pop() {
                        None => {}
                        Some(view) => match view {
                            Views::Dashboard(_) => _ = self.dashboards.pop(),
                            Views::Artists(_) => _ = self.artistss.pop(),
                            Views::Artist(_) => _ = self.artist_views.pop(),
                            Views::Albums(_) => _ = self.albumss.pop(),
                            Views::Album(_) => _ = self.album_views.pop(),
                            Views::Tracks(_) => todo!(),
                            Views::Playlists(_) => todo!(),
                        },
                    }

                    // untoggle all buttons
                    self.dashboard_btn.set_active(false);
                    self.artists_btn.set_active(false);
                    self.albums_btn.set_active(false);
                    self.tracks_btn.set_active(false);
                    self.playlists_btn.set_active(false);

                    //toggle the right button one if its active
                    match self.history_widget.last() {
                        Some(Views::Dashboard(_)) => self.dashboard_btn.set_active(true),
                        Some(Views::Artists(_)) => self.artists_btn.set_active(true),
                        Some(Views::Albums(_)) => self.albums_btn.set_active(true),
                        Some(Views::Tracks(_)) => self.tracks_btn.set_active(true),
                        Some(Views::Playlists(_)) => self.playlists_btn.set_active(true),
                        _ => {}
                    }

                    //change view
                    if let Some(view) = self.history_widget.last() {
                        self.content.set_child(Some(view.widget()));
                    }
                }

                //change back button sensitivity
                if self.history_widget.len() == 1 {
                    self.back_btn.set_sensitive(false);
                }
            }
            BrowserIn::DashboardClicked => {
                self.deactivate_all_buttons();
                self.dashboard_btn.set_active(true);

                let dashboard: relm4::Controller<Dashboard> = Dashboard::builder()
                    .launch(())
                    .forward(sender.input_sender(), BrowserIn::Dashboard);
                self.history_widget
                    .push(Views::Dashboard(dashboard.widget().clone()));
                self.dashboards.push(dashboard);
                self.content
                    .set_child(Some(self.history_widget.last().unwrap().widget()));
                self.back_btn.set_sensitive(true);
            }
            BrowserIn::ArtistsClicked => {
                self.deactivate_all_buttons();
                self.artists_btn.set_active(true);

                let artists: relm4::component::AsyncController<ArtistsView> =
                    ArtistsView::builder()
                        .launch(self.subsonic.clone())
                        .forward(sender.input_sender(), BrowserIn::ArtistsView);
                self.history_widget
                    .push(Views::Artists(artists.widget().clone()));
                self.artistss.push(artists);
                self.content
                    .set_child(Some(self.history_widget.last().unwrap().widget()));
                self.back_btn.set_sensitive(true);
            }
            BrowserIn::AlbumsClicked => {
                self.deactivate_all_buttons();
                self.albums_btn.set_active(true);

                let albums: relm4::component::Controller<AlbumsView> = AlbumsView::builder()
                    .launch(self.subsonic.clone())
                    .forward(sender.input_sender(), BrowserIn::AlbumsView);
                self.history_widget
                    .push(Views::Albums(albums.widget().clone()));
                self.albumss.push(albums);
                self.content
                    .set_child(Some(self.history_widget.last().unwrap().widget()));
                self.back_btn.set_sensitive(true);
            }
            BrowserIn::TracksClicked => {
                //TODO
            }
            BrowserIn::PlaylistsClicked => {
                //TODO
            }
            BrowserIn::Dashboard(output) => {
                //TODO react to output
            }
            BrowserIn::AlbumsView(msg) => match msg {
                AlbumsViewOut::Clicked(id) => {
                    self.deactivate_all_buttons();
                    let init: AlbumViewInit = match id {
                        AlbumElementInit::Child(c) => AlbumViewInit::Child(c),
                        AlbumElementInit::AlbumId3(a) => AlbumViewInit::AlbumId3(a),
                    };
                    let album: relm4::Controller<AlbumView> = AlbumView::builder()
                        .launch((self.subsonic.clone(), init))
                        .forward(sender.input_sender(), |msg| {
                            BrowserIn::AlbumView(Box::new(msg))
                        });

                    self.history_widget
                        .push(Views::Album(album.widget().clone()));
                    self.album_views.push(album);
                    self.content
                        .set_child(Some(self.history_widget.last().unwrap().widget()));
                    self.back_btn.set_sensitive(true);
                }
            },
            BrowserIn::ArtistsView(msg) => match msg {
                ArtistsViewOut::ClickedArtist(id) => {
                    self.deactivate_all_buttons();
                    let artist: relm4::Controller<ArtistView> = ArtistView::builder()
                        .launch((self.subsonic.clone(), id))
                        .forward(sender.input_sender(), |msg| {
                            BrowserIn::ArtistView(Box::new(msg))
                        });

                    self.history_widget
                        .push(Views::Artist(artist.widget().clone()));
                    self.artist_views.push(artist);
                    self.content
                        .set_child(Some(self.history_widget.last().unwrap().widget()));
                    self.back_btn.set_sensitive(true);
                }
            },
            BrowserIn::AlbumView(msg) => match *msg {
                AlbumViewOut::AppendAlbum(drop) => {
                    sender.output(BrowserOut::AppendToQueue(drop)).unwrap()
                }
                AlbumViewOut::InsertAfterCurrentPLayed(drop) => sender
                    .output(BrowserOut::InsertAfterCurrentInQueue(drop))
                    .unwrap(),
            },
            BrowserIn::ArtistView(msg) => match *msg {
                ArtistViewOut::AlbumClicked(id) => {
                    self.deactivate_all_buttons();
                    let init: AlbumViewInit = match id {
                        AlbumElementInit::Child(c) => AlbumViewInit::Child(c),
                        AlbumElementInit::AlbumId3(a) => AlbumViewInit::AlbumId3(a),
                    };
                    let album: relm4::Controller<AlbumView> = AlbumView::builder()
                        .launch((self.subsonic.clone(), init))
                        .forward(sender.input_sender(), |msg| {
                            BrowserIn::AlbumView(Box::new(msg))
                        });

                    self.history_widget
                        .push(Views::Album(album.widget().clone()));
                    self.album_views.push(album);
                    self.content
                        .set_child(Some(self.history_widget.last().unwrap().widget()));
                    self.back_btn.set_sensitive(true);
                }
            },
        }
    }
}

impl Browser {
    fn deactivate_all_buttons(&self) {
        self.artists_btn.set_active(false);
        self.albums_btn.set_active(false);
        self.tracks_btn.set_active(false);
        self.playlists_btn.set_active(false);
        self.dashboard_btn.set_active(false);
    }
}
