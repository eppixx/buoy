use std::{cell::RefCell, rc::Rc};

use itertools::Itertools;
use relm4::{
    component::AsyncComponentController,
    gtk::{
        self,
        prelude::{BoxExt, OrientableExt, WidgetExt},
    },
    Component, ComponentController,
};

use super::{
    album_element::AlbumElementInit,
    album_view::{AlbumView, AlbumViewIn, AlbumViewInit, AlbumViewOut},
    albums_view::{AlbumsView, AlbumsViewIn, AlbumsViewOut},
    artist_view::{ArtistView, ArtistViewIn, ArtistViewOut},
    artists_view::{ArtistsView, ArtistsViewIn, ArtistsViewOut},
    dashboard::{Dashboard, DashboardIn, DashboardOut},
    playlists_view::{PlaylistsView, PlaylistsViewIn, PlaylistsViewOut},
};
use crate::{client::Client, subsonic::Subsonic, types::Droppable};

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
            Self::Dashboard(w)
            | Self::Artists(w)
            | Self::Artist(w)
            | Self::Albums(w)
            | Self::Album(w)
            | Self::Tracks(w)
            | Self::Playlists(w) => w,
        }
    }
}

#[derive(Debug)]
pub struct Browser {
    subsonic: Rc<RefCell<Subsonic>>,
    history_widget: Vec<Views>,
    content: gtk::Viewport,

    dashboards: Vec<relm4::Controller<Dashboard>>,
    artistss: Vec<relm4::component::AsyncController<ArtistsView>>,
    albumss: Vec<relm4::component::Controller<AlbumsView>>,
    album_views: Vec<relm4::Controller<AlbumView>>,
    artist_views: Vec<relm4::Controller<ArtistView>>,
    playlists_views: Vec<relm4::Controller<PlaylistsView>>,
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
    Dashboard(DashboardOut),
    AlbumsView(AlbumsViewOut),
    AlbumView(Box<AlbumViewOut>),
    ArtistsView(ArtistsViewOut),
    ArtistView(Box<ArtistViewOut>),
    PlaylistsView(PlaylistsViewOut),
    NewPlaylist(String, Vec<submarine::data::Child>),
    FavoriteAlbum(String, bool),
    FavoriteArtist(String, bool),
    FavoriteSong(String, bool),
}

#[derive(Debug)]
pub enum BrowserOut {
    AppendToQueue(Droppable),
    ReplaceQueue(Droppable),
    InsertAfterCurrentInQueue(Droppable),
    BackButtonSensitivity(bool),
    DisplayToast(String),
    ChangedView,
    FavoriteAlbumClicked(String, bool),
    FavoriteArtistClicked(String, bool),
    FavoriteSongClicked(String, bool),
}

#[relm4::component(async, pub)]
impl relm4::component::AsyncComponent for Browser {
    type Init = Rc<RefCell<Subsonic>>;
    type Input = BrowserIn;
    type Output = BrowserOut;
    type CommandOutput = ();

    async fn init(
        init: Self::Init,
        root: Self::Root,
        sender: relm4::AsyncComponentSender<Self>,
    ) -> relm4::component::AsyncComponentParts<Self> {
        let model = Self {
            subsonic: init,
            history_widget: vec![],
            content: gtk::Viewport::default(),

            dashboards: vec![],
            artistss: vec![],
            albumss: vec![],
            album_views: vec![],
            artist_views: vec![],
            playlists_views: vec![],
        };
        let widgets = view_output!();

        sender.input(BrowserIn::DashboardClicked);
        sender.input(BrowserIn::BackClicked);

        relm4::component::AsyncComponentParts { model, widgets }
    }

    view! {
        gtk::Box {
            add_css_class: "browser",
            set_orientation: gtk::Orientation::Vertical,

            append = &model.content.clone() -> gtk::Viewport {
                add_css_class: "browser-content",
            }
        }
    }

    async fn update(
        &mut self,
        msg: Self::Input,
        sender: relm4::AsyncComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match msg {
            BrowserIn::SearchChanged(search) => {
                for view in &self.dashboards {
                    view.emit(DashboardIn::SearchChanged(search.clone()));
                }
                for view in &self.artistss {
                    view.emit(ArtistsViewIn::SearchChanged(search.clone()));
                }
                for view in &self.albumss {
                    view.emit(AlbumsViewIn::SearchChanged(search.clone()));
                }
                for view in &self.album_views {
                    view.emit(AlbumViewIn::SearchChanged(search.clone()));
                }
                for view in &self.artist_views {
                    view.emit(ArtistViewIn::SearchChanged(search.clone()));
                }
                for view in &self.playlists_views {
                    view.emit(PlaylistsViewIn::SearchChanged(search.clone()));
                }
                //TODO search for tracks
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
                            Views::Playlists(_) => _ = self.playlists_views.pop(),
                        },
                    }

                    //change view
                    if let Some(view) = self.history_widget.last() {
                        self.content.set_child(Some(view.widget()));
                    }
                }

                //change back button sensitivity
                if self.history_widget.len() == 1 {
                    sender
                        .output(BrowserOut::BackButtonSensitivity(false))
                        .expect("main window.gone");
                }
            }
            BrowserIn::DashboardClicked => {
                if let Some(&Views::Dashboard(_)) = self.history_widget.last() {
                    return;
                }

                let dashboard: relm4::Controller<Dashboard> = Dashboard::builder()
                    .launch(self.subsonic.clone())
                    .forward(sender.input_sender(), BrowserIn::Dashboard);
                self.history_widget
                    .push(Views::Dashboard(dashboard.widget().clone()));
                self.dashboards.push(dashboard);
                self.content
                    .set_child(Some(self.history_widget.last().unwrap().widget()));
                sender
                    .output(BrowserOut::BackButtonSensitivity(true))
                    .expect("main window.gone");
            }
            BrowserIn::ArtistsClicked => {
                if let Some(&Views::Artists(_)) = self.history_widget.last() {
                    return;
                }

                let artists: relm4::component::AsyncController<ArtistsView> =
                    ArtistsView::builder()
                        .launch(self.subsonic.clone())
                        .forward(sender.input_sender(), BrowserIn::ArtistsView);
                self.history_widget
                    .push(Views::Artists(artists.widget().clone()));
                self.artistss.push(artists);
                self.content
                    .set_child(Some(self.history_widget.last().unwrap().widget()));
                sender
                    .output(BrowserOut::BackButtonSensitivity(true))
                    .expect("main window gone");
            }
            BrowserIn::AlbumsClicked => {
                if let Some(&Views::Albums(_)) = self.history_widget.last() {
                    return;
                }

                let albums: relm4::component::Controller<AlbumsView> = AlbumsView::builder()
                    .launch(self.subsonic.clone())
                    .forward(sender.input_sender(), BrowserIn::AlbumsView);
                self.history_widget
                    .push(Views::Albums(albums.widget().clone()));
                self.albumss.push(albums);
                self.content
                    .set_child(Some(self.history_widget.last().unwrap().widget()));
                sender
                    .output(BrowserOut::BackButtonSensitivity(true))
                    .expect("main window gone");
            }
            BrowserIn::TracksClicked => {
                //TODO
            }
            BrowserIn::PlaylistsClicked => {
                if let Some(&Views::Playlists(_)) = self.history_widget.last() {
                    return;
                }

                let playlists: relm4::Controller<PlaylistsView> = PlaylistsView::builder()
                    .launch(self.subsonic.clone())
                    .forward(sender.input_sender(), BrowserIn::PlaylistsView);
                self.history_widget
                    .push(Views::Playlists(playlists.widget().clone()));
                self.playlists_views.push(playlists);
                self.content
                    .set_child(Some(self.history_widget.last().unwrap().widget()));
                sender
                    .output(BrowserOut::BackButtonSensitivity(true))
                    .expect("main window gone");
            }
            BrowserIn::Dashboard(output) => match output {
                DashboardOut::ClickedAlbum(id) => {
                    sender.input(BrowserIn::AlbumsView(AlbumsViewOut::Clicked(id)))
                }
                DashboardOut::DisplayToast(title) => sender
                    .output(BrowserOut::DisplayToast(title))
                    .expect("sending failed"),
                DashboardOut::FavoriteClicked(id, state) => sender
                    .output(BrowserOut::FavoriteAlbumClicked(id, state))
                    .expect("sending failed"),
            },
            BrowserIn::AlbumsView(msg) => match msg {
                AlbumsViewOut::Clicked(id) => {
                    sender
                        .output(BrowserOut::ChangedView)
                        .expect("sending failed");

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
                    sender
                        .output(BrowserOut::BackButtonSensitivity(true))
                        .expect("main window.gone");
                }
                AlbumsViewOut::DisplayToast(title) => sender
                    .output(BrowserOut::DisplayToast(title))
                    .expect("sending failed"),
                AlbumsViewOut::FavoriteClicked(id, state) => sender
                    .output(BrowserOut::FavoriteAlbumClicked(id, state))
                    .expect("sending failed"),
            },
            BrowserIn::ArtistsView(msg) => match msg {
                ArtistsViewOut::ClickedArtist(id) => {
                    sender
                        .output(BrowserOut::ChangedView)
                        .expect("sending failed");

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
                    sender
                        .output(BrowserOut::BackButtonSensitivity(true))
                        .expect("main window.gone");
                }
                ArtistsViewOut::DisplayToast(msg) => sender
                    .output(BrowserOut::DisplayToast(msg))
                    .expect("sending failed"),
                ArtistsViewOut::FavoriteClicked(id, state) => sender
                    .output(BrowserOut::FavoriteArtistClicked(id, state))
                    .expect("sending failed"),
            },
            BrowserIn::AlbumView(msg) => match *msg {
                AlbumViewOut::AppendAlbum(drop) => {
                    sender.output(BrowserOut::AppendToQueue(drop)).unwrap();
                }
                AlbumViewOut::InsertAfterCurrentPLayed(drop) => sender
                    .output(BrowserOut::InsertAfterCurrentInQueue(drop))
                    .unwrap(),
                AlbumViewOut::DisplayToast(title) => sender
                    .output(BrowserOut::DisplayToast(title))
                    .expect("sending failed"),
                AlbumViewOut::FavoriteClicked(id, state) => sender
                    .output(BrowserOut::FavoriteAlbumClicked(id, state))
                    .expect("sending failed"),
            },
            BrowserIn::ArtistView(msg) => match *msg {
                ArtistViewOut::AlbumClicked(id) => {
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
                    sender
                        .output(BrowserOut::BackButtonSensitivity(true))
                        .expect("sending failed");
                }
                ArtistViewOut::DisplayToast(title) => sender
                    .output(BrowserOut::DisplayToast(title))
                    .expect("sending failed"),
                ArtistViewOut::FavoriteAlbumClicked(id, state) => sender
                    .output(BrowserOut::FavoriteAlbumClicked(id, state))
                    .expect("sending failed"),
                ArtistViewOut::FavoriteArtistClicked(id, state) => sender
                    .output(BrowserOut::FavoriteArtistClicked(id, state))
                    .expect("sending failed"),
            },
            BrowserIn::PlaylistsView(msg) => match msg {
                PlaylistsViewOut::DisplayToast(title) => sender
                    .output(BrowserOut::DisplayToast(title))
                    .expect("sending failed"),
                PlaylistsViewOut::AppendToQueue(list) => {
                    let drop = Droppable::Playlist(Box::new(list));
                    sender
                        .output(BrowserOut::AppendToQueue(drop))
                        .expect("sending failed");
                }
                PlaylistsViewOut::AddToQueue(list) => {
                    let drop = Droppable::Playlist(Box::new(list));
                    sender
                        .output(BrowserOut::InsertAfterCurrentInQueue(drop))
                        .expect("sending failed");
                }
                PlaylistsViewOut::ReplaceQueue(list) => {
                    let drop = Droppable::Playlist(Box::new(list));
                    sender
                        .output(BrowserOut::ReplaceQueue(drop))
                        .expect("sending failed");
                }
                PlaylistsViewOut::DeletePlaylist(index, list) => {
                    //delete playlist from server
                    let client = Client::get().unwrap();
                    if let Err(e) = client.delete_playlist(list.base.id.clone()).await {
                        sender
                            .output(BrowserOut::DisplayToast(format!(
                                "could not delete playlist from server: {e:?}"
                            )))
                            .expect("sending failed");
                        return;
                    }

                    //delete paylist from subsonic cache
                    self.subsonic.borrow_mut().delete_playlist(&list);

                    //update views
                    for view in &self.playlists_views {
                        view.emit(PlaylistsViewIn::DeletePlaylist(index.clone()));
                    }
                }
                PlaylistsViewOut::CreatePlaylist => {
                    sender.input(BrowserIn::NewPlaylist(String::from("New Playlist"), vec![]))
                }
            },
            BrowserIn::NewPlaylist(name, list) => {
                const CHUNKS: usize = 100;
                let client = Client::get().unwrap();
                let ids = list.iter().map(|track| track.id.clone()).collect();

                //decide wether to create a playlist whole or in chunks
                let list = if list.len() < CHUNKS {
                    match client.create_playlist(name, ids).await {
                        Err(e) => {
                            sender
                                .output(BrowserOut::DisplayToast(format!(
                                    "could not create playlist on server: {e:?}"
                                )))
                                .expect("sending failed");
                            return;
                        }
                        Ok(list) => list,
                    }
                } else {
                    tracing::info!("create a new playlist on server in chunks");
                    let first: Vec<_> = ids.iter().take(CHUNKS).collect();
                    let mut playlist = match client.create_playlist(name, first).await {
                        Err(e) => {
                            sender
                                .output(BrowserOut::DisplayToast(format!(
                                    "could not create playlist on server: {e:?}"
                                )))
                                .expect("sending failed");
                            return;
                        }
                        Ok(list) => list,
                    };
                    let mut index = 0;
                    for ids in &ids.iter().skip(CHUNKS).chunks(CHUNKS) {
                        index += 1;
                        if let Err(e) = client
                            .update_playlist(
                                playlist.base.id.clone(),
                                None::<String>,
                                None::<String>,
                                None,
                                ids.collect(),
                                vec![],
                            )
                            .await
                        {
                            sender
                                .output(BrowserOut::DisplayToast(format!(
                                    "could not update playlist on server: {e:?}"
                                )))
                                .expect("sending failed");
                            break;
                        }

                        let mut part: Vec<_> = list
                            .iter()
                            .skip(index * CHUNKS)
                            .take(CHUNKS)
                            .cloned()
                            .collect();
                        playlist.base.song_count += part.len() as i32;
                        playlist.entry.append(&mut part);
                    }

                    playlist
                };

                //update playlists in subsonic
                self.subsonic.borrow_mut().push_playlist(&list);

                //show new playlists in views
                for view in &self.playlists_views {
                    view.emit(PlaylistsViewIn::NewPlaylist(list.clone()));
                }
            }
            BrowserIn::FavoriteSong(id, state) => {
                //notify all views with songs in them
                for view in &self.album_views {
                    view.emit(AlbumViewIn::FavoritedSong(id.clone(), state));
                }
                for view in &self.playlists_views {
                    // view.emit(PlaylistsViewIn::Favorited(id, state));
                }
                //TODO add for track view
            }
            BrowserIn::FavoriteAlbum(id, state) => {
                //notify all views with albums in them
                for view in &self.dashboards {
                    view.emit(DashboardIn::Favorited(id.clone(), state));
                }
                for view in &self.albumss {
                    view.emit(AlbumsViewIn::Favorited(id.clone(), state));
                }
                for view in &self.album_views {
                    view.emit(AlbumViewIn::FavoritedAlbum(id.clone(), state));
                }
                for view in &self.artist_views {
                    view.emit(ArtistViewIn::FavoritedAlbum(id.clone(), state));
                }
            }
            BrowserIn::FavoriteArtist(id, state) => {
                //notify all views with artists in them
                for view in &self.artist_views {
                    view.emit(ArtistViewIn::FavoritedArtist(id.clone(), state));
                }
                for view in &self.artistss {
                    view.emit(ArtistsViewIn::Favorited(id.clone(), state));
                }
            }
        }
    }
}
