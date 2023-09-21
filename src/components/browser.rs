use relm4::{
    component::{AsyncComponent, AsyncComponentController},
    gtk::{
        self,
        traits::{BoxExt, ButtonExt, EditableExt, OrientableExt, ToggleButtonExt, WidgetExt},
    },
    Component, ComponentController, RelmWidgetExt,
};

use super::{
    album_view::AlbumViewOut,
    albums_view::{AlbumsView, AlbumsViewOut},
    artists_view::{ArtistsView, ArtistsViewOut},
    dashboard::DashboardOutput,
};
use crate::{
    components::{album_view::AlbumView, dashboard::Dashboard},
    types::Id,
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
    albumss: Vec<relm4::component::AsyncController<AlbumsView>>,
    album_views: Vec<relm4::Controller<AlbumView>>,
}

#[derive(Debug)]
pub enum BrowserInput {
    SearchChanged(String),
    BackClicked,
    DashboardClicked,
    ArtistsClicked,
    AlbumsClicked,
    TracksClicked,
    PlaylistsClicked,
    Dashboard(DashboardOutput),
    AlbumsView(AlbumsViewOut),
    AlbumView(AlbumViewOut),
    ArtistsView(ArtistsViewOut),
}

#[relm4::component(pub)]
impl relm4::SimpleComponent for Browser {
    type Input = BrowserInput;
    type Output = ();
    type Init = ();

    fn init(
        _path: Self::Init,
        root: &Self::Root,
        sender: relm4::ComponentSender<Self>,
    ) -> relm4::ComponentParts<Self> {
        let model = Self {
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
        };
        let widgets = view_output!();

        //TODO swtich default view
        sender.input(BrowserInput::DashboardClicked);
        sender.input(BrowserInput::AlbumsClicked);

        relm4::ComponentParts { model, widgets }
    }

    view! {
        gtk::Box {
            add_css_class: "browser",
            set_orientation: gtk::Orientation::Vertical,

            gtk::Box {
                add_css_class: "pathbar",

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
                    add_css_class: "pathbar-space",
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
                    add_css_class: "pathbar-space",
                },

                gtk::SearchEntry {
                    set_placeholder_text: Some("Search..."),
                    grab_focus: (),
                    connect_search_changed[sender] => move |w| {
                        sender.input(BrowserInput::SearchChanged(w.text().to_string()));
                    }
                }
            },

            //TODO implement stack of view here
            append = &model.content.clone() -> gtk::Viewport {
                add_css_class: "browser-content",
                // set_hexpand: true,
                // set_vexpand: true,
            }
        }
    }

    fn update(&mut self, msg: Self::Input, sender: relm4::ComponentSender<Self>) {
        match msg {
            BrowserInput::SearchChanged(search) => {
                tracing::warn!("new search {search}");
            }
            BrowserInput::BackClicked => {
                if self.history_widget.len() > 1 {
                    // remove current view from history
                    let _ = self.history_widget.pop();

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
            BrowserInput::DashboardClicked => {
                self.deactivate_all_buttons();
                self.dashboard_btn.set_active(true);

                let dashboard: relm4::Controller<Dashboard> = Dashboard::builder()
                    .launch(())
                    .forward(sender.input_sender(), BrowserInput::Dashboard);
                self.history_widget
                    .push(Views::Dashboard(dashboard.widget().clone()));
                self.dashboards.push(dashboard);
                self.content
                    .set_child(Some(self.history_widget.last().unwrap().widget()));
                self.back_btn.set_sensitive(true);
            }
            BrowserInput::ArtistsClicked => {
                //TODO rename to ArtistsClicke
                self.deactivate_all_buttons();
                self.artists_btn.set_active(true);

                let artists: relm4::component::AsyncController<ArtistsView> =
                    ArtistsView::builder()
                        .launch(())
                        .forward(sender.input_sender(), BrowserInput::ArtistsView);
                self.history_widget
                    .push(Views::Artists(artists.widget().clone()));
                self.artistss.push(artists);
                self.content
                    .set_child(Some(self.history_widget.last().unwrap().widget()));
                self.back_btn.set_sensitive(true);
            }
            BrowserInput::AlbumsClicked => {
                // TODO rename to AlbumsClicked
                self.deactivate_all_buttons();
                self.albums_btn.set_active(true);

                let albums: relm4::component::AsyncController<AlbumsView> = AlbumsView::builder()
                    .launch(())
                    .forward(sender.input_sender(), BrowserInput::AlbumsView);
                self.history_widget
                    .push(Views::Albums(albums.widget().clone()));
                self.albumss.push(albums);
                self.content
                    .set_child(Some(self.history_widget.last().unwrap().widget()));
                self.back_btn.set_sensitive(true);
            }
            BrowserInput::TracksClicked => {
                //TODO
            }
            BrowserInput::PlaylistsClicked => {
                //TODO
            }
            BrowserInput::Dashboard(output) => {
                //TODO react to output
            }
            BrowserInput::AlbumsView(msg) => match msg {
                AlbumsViewOut::ClickedAlbum(id) => {
                    tracing::error!("received click in browser");
                    self.deactivate_all_buttons();
                    let album: relm4::Controller<AlbumView> = AlbumView::builder()
                        .launch(Id::album(id.inner()))
                        .forward(sender.input_sender(), BrowserInput::AlbumView);

                    self.history_widget
                        .push(Views::Album(album.widget().clone()));
                    self.album_views.push(album);
                    self.content
                        .set_child(Some(self.history_widget.last().unwrap().widget()));
                    self.back_btn.set_sensitive(true);
                }
            },
            BrowserInput::ArtistsView(msg) => match msg {
                ArtistsViewOut::ClickedArtist(id) => {
                    tracing::error!("clicked album");
                    // sender.input(BrowserInput::NewView(View::Id(id)))
                }
            },
            BrowserInput::AlbumView(msg) => match msg {
                AlbumViewOut::AppendAlbum(id) => {} //TODO append to queue
                AlbumViewOut::InsertAfterCurrentPLayed(id) => {} //TODO insert in queue
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
