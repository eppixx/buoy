use std::{cell::RefCell, rc::Rc};

use gettextrs::gettext;
use itertools::Itertools;
use relm4::{
    gtk::{
        self,
        prelude::{BoxExt, OrientableExt, WidgetExt},
    },
    prelude::AsyncComponentController,
    Component, ComponentController,
};

use crate::{
    client::Client,
    components::{
        album_view::{AlbumView, AlbumViewIn, AlbumViewOut},
        albums_view::{AlbumsView, AlbumsViewIn, AlbumsViewOut},
        artist_view::{ArtistView, ArtistViewIn, ArtistViewOut},
        artists_view::{ArtistsView, ArtistsViewIn, ArtistsViewOut},
        dashboard::{Dashboard, DashboardIn, DashboardOut},
        playlists_view::{PlaylistsView, PlaylistsViewIn, PlaylistsViewOut},
        tracks_view::{TracksView, TracksViewIn, TracksViewOut},
    },
    subsonic::Subsonic,
    types::{Droppable, Id},
    views,
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

    dashboard: relm4::Controller<Dashboard>,
    artists: Option<relm4::component::Controller<ArtistsView>>,
    albums: Option<relm4::component::Controller<AlbumsView>>,
    tracks: Option<relm4::component::Controller<TracksView>>,
    album_views: Vec<relm4::Controller<AlbumView>>,
    artist_views: Vec<relm4::Controller<ArtistView>>,
    playlists_views: Vec<relm4::component::AsyncController<PlaylistsView>>,
}

#[derive(Debug)]
pub enum BrowserIn {
    SearchChanged(String),
    GoBack,
    ShowDashboard,
    ShowArtists,
    ShowArtist(Id),
    ShowAlbums,
    ShowTracks,
    ShowPlaylists,
    ShowAlbum(Id),
    Dashboard(DashboardOut),
    AlbumsView(AlbumsViewOut),
    AlbumView(Box<AlbumViewOut>),
    TracksView(TracksViewOut),
    ArtistsView(ArtistsViewOut),
    ArtistView(Box<ArtistViewOut>),
    PlaylistsView(PlaylistsViewOut),
    RenamePlaylist(submarine::data::Playlist),
    NewPlaylist(String, Vec<submarine::data::Child>),
    UpdateFavoriteAlbum(String, bool),
    UpdateFavoriteArtist(String, bool),
    UpdateFavoriteSong(String, bool),
    UpdatePlayCountSong(String, Option<i64>),
    UpdatePlayCountAlbum(String, Option<i64>),
    InsertSongsToPlaylist(u32, Vec<submarine::data::Child>),
}

#[derive(Debug)]
pub enum BrowserOut {
    AppendToQueue(Droppable),
    ReplaceQueue(Droppable),
    InsertAfterCurrentInQueue(Droppable),
    BackButtonSensitivity(bool),
    DisplayToast(String),
    FavoriteAlbumClicked(String, bool),
    FavoriteArtistClicked(String, bool),
    FavoriteSongClicked(String, bool),
    Download(Droppable),
    ChangedViewTo(views::Views),
}

#[relm4::component(async, pub)]
impl relm4::component::AsyncComponent for Browser {
    type Init = Rc<RefCell<Subsonic>>;
    type Input = BrowserIn;
    type Output = BrowserOut;
    type CommandOutput = ();

    async fn init(
        subsonic: Self::Init,
        root: Self::Root,
        sender: relm4::AsyncComponentSender<Self>,
    ) -> relm4::component::AsyncComponentParts<Self> {
        let model = Self {
            subsonic: subsonic.clone(),
            history_widget: vec![],
            content: gtk::Viewport::default(),

            dashboard: Dashboard::builder()
                .launch(subsonic)
                .forward(sender.input_sender(), BrowserIn::Dashboard),
            artists: None,
            albums: None,
            tracks: None,
            album_views: vec![],
            artist_views: vec![],
            playlists_views: vec![],
        };
        let widgets = view_output!();

        sender.input(BrowserIn::ShowDashboard);
        sender.input(BrowserIn::GoBack);

        relm4::component::AsyncComponentParts { model, widgets }
    }

    view! {
        gtk::Box {
            set_widget_name: "browser",
            set_orientation: gtk::Orientation::Vertical,

            append = &model.content.clone() -> gtk::Viewport {}
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
                self.dashboard.emit(DashboardIn::FilterChanged);
                if let Some(artists) = &self.artists {
                    artists.emit(ArtistsViewIn::FilterChanged);
                }
                if let Some(albums) = &self.albums {
                    albums.emit(AlbumsViewIn::FilterChanged);
                }
                if let Some(tracks) = &self.tracks {
                    tracks.emit(TracksViewIn::FilterChanged);
                }
                //TODO change to filter changed
                for view in &self.album_views {
                    view.emit(AlbumViewIn::FilterChanged(search.clone()));
                }
                for view in &self.artist_views {
                    view.emit(ArtistViewIn::FilterChanged(search.clone()));
                }
                for view in &self.playlists_views {
                    view.emit(PlaylistsViewIn::FilterChanged(search.clone()));
                }
            }
            BrowserIn::GoBack => {
                if self.history_widget.len() > 1 {
                    // remove current view from history
                    match self.history_widget.pop() {
                        None => {}
                        Some(view) => match view {
                            // these are singletons
                            Views::Dashboard(_)
                            | Views::Artists(_)
                            | Views::Albums(_)
                            | Views::Tracks(_) => {}
                            // these are not
                            Views::Artist(_) => _ = self.artist_views.pop(),
                            Views::Album(_) => _ = self.album_views.pop(),
                            Views::Playlists(_) => _ = self.playlists_views.pop(),
                        },
                    }

                    //change view
                    if let Some(view) = self.history_widget.last() {
                        self.content.set_child(Some(view.widget()));
                        sender
                            .output(BrowserOut::ChangedViewTo(view.into()))
                            .unwrap();
                    }
                }

                //change back button sensitivity
                if self.history_widget.len() == 1 {
                    sender
                        .output(BrowserOut::BackButtonSensitivity(false))
                        .expect("main window.gone");
                }
            }
            BrowserIn::ShowDashboard => {
                if let Some(&Views::Dashboard(_)) = self.history_widget.last() {
                    return;
                }

                sender
                    .output(BrowserOut::ChangedViewTo(views::Views::Clickable(
                        views::ClickableViews::Dashboard,
                    )))
                    .unwrap();
                self.history_widget
                    .push(Views::Dashboard(self.dashboard.widget().clone()));
                self.content.set_child(Some(self.dashboard.widget()));
                sender
                    .output(BrowserOut::BackButtonSensitivity(true))
                    .unwrap();
            }
            BrowserIn::ShowArtists => {
                if let Some(&Views::Artists(_)) = self.history_widget.last() {
                    return;
                }

                if self.artists.is_none() {
                    self.artists = Some(
                        ArtistsView::builder()
                            .launch(self.subsonic.clone())
                            .forward(sender.input_sender(), BrowserIn::ArtistsView),
                    );
                }

                sender
                    .output(BrowserOut::ChangedViewTo(views::Views::Clickable(
                        views::ClickableViews::Artists,
                    )))
                    .unwrap();
                self.history_widget.push(Views::Artists(
                    self.artists.as_ref().unwrap().widget().clone(),
                ));
                self.content
                    .set_child(Some(self.history_widget.last().unwrap().widget()));
                sender
                    .output(BrowserOut::BackButtonSensitivity(true))
                    .unwrap();
            }
            BrowserIn::ShowArtist(id) => {
                let Some(artist) = self.subsonic.borrow().find_artist(id) else {
                    sender
                        .output(BrowserOut::DisplayToast(String::from(
                            "clicked artist not found",
                        )))
                        .unwrap();
                    return;
                };
                let artist: relm4::Controller<ArtistView> = ArtistView::builder()
                    .launch((self.subsonic.clone(), Id::artist(&artist.id)))
                    .forward(sender.input_sender(), |msg| {
                        BrowserIn::ArtistView(Box::new(msg))
                    });

                sender
                    .output(BrowserOut::ChangedViewTo(views::Views::Artist))
                    .unwrap();
                self.history_widget
                    .push(Views::Artist(artist.widget().clone()));
                self.artist_views.push(artist);
                self.content
                    .set_child(Some(self.history_widget.last().unwrap().widget()));
                sender
                    .output(BrowserOut::BackButtonSensitivity(true))
                    .unwrap();
            }
            BrowserIn::ShowAlbums => {
                if let Some(&Views::Albums(_)) = self.history_widget.last() {
                    return;
                }

                sender
                    .output(BrowserOut::ChangedViewTo(views::Views::Clickable(
                        views::ClickableViews::Albums,
                    )))
                    .unwrap();
                if self.albums.is_none() {
                    self.albums = Some(
                        AlbumsView::builder()
                            .launch(self.subsonic.clone())
                            .forward(sender.input_sender(), BrowserIn::AlbumsView),
                    );
                }

                self.history_widget.push(Views::Albums(
                    self.albums.as_ref().unwrap().widget().clone(),
                ));
                self.content
                    .set_child(Some(self.history_widget.last().unwrap().widget()));
                sender
                    .output(BrowserOut::BackButtonSensitivity(true))
                    .unwrap();
            }
            BrowserIn::ShowAlbum(id) => {
                let album: relm4::Controller<AlbumView> = AlbumView::builder()
                    .launch((self.subsonic.clone(), id))
                    .forward(sender.input_sender(), |msg| {
                        BrowserIn::AlbumView(Box::new(msg))
                    });

                sender
                    .output(BrowserOut::ChangedViewTo(views::Views::Album))
                    .unwrap();
                self.history_widget
                    .push(Views::Album(album.widget().clone()));
                self.content.set_child(Some(album.widget()));
                self.album_views.push(album);
                sender
                    .output(BrowserOut::BackButtonSensitivity(true))
                    .unwrap();
            }
            BrowserIn::ShowTracks => {
                if let Some(&Views::Tracks(_)) = self.history_widget.last() {
                    return;
                }

                if self.tracks.is_none() {
                    self.tracks = Some(
                        TracksView::builder()
                            .launch(self.subsonic.clone())
                            .forward(sender.input_sender(), BrowserIn::TracksView),
                    );
                }

                sender
                    .output(BrowserOut::ChangedViewTo(views::Views::Clickable(
                        views::ClickableViews::Tracks,
                    )))
                    .unwrap();
                self.history_widget.push(Views::Tracks(
                    self.tracks.as_ref().unwrap().widget().clone(),
                ));
                self.content
                    .set_child(Some(self.history_widget.last().unwrap().widget()));
                sender
                    .output(BrowserOut::BackButtonSensitivity(true))
                    .unwrap();
            }
            BrowserIn::ShowPlaylists => {
                if let Some(&Views::Playlists(_)) = self.history_widget.last() {
                    return;
                }

                let playlists: relm4::component::AsyncController<PlaylistsView> =
                    PlaylistsView::builder()
                        .launch(self.subsonic.clone())
                        .forward(sender.input_sender(), BrowserIn::PlaylistsView);
                sender
                    .output(BrowserOut::ChangedViewTo(views::Views::Clickable(
                        views::ClickableViews::Playlists,
                    )))
                    .unwrap();
                self.history_widget
                    .push(Views::Playlists(playlists.widget().clone()));
                self.playlists_views.push(playlists);
                self.content
                    .set_child(Some(self.history_widget.last().unwrap().widget()));
                sender
                    .output(BrowserOut::BackButtonSensitivity(true))
                    .unwrap();
            }
            BrowserIn::Dashboard(output) => match output {
                DashboardOut::ClickedAlbum(id) => {
                    match self.subsonic.borrow().find_album(id.as_ref()) {
                        None => {
                            tracing::error!("clicked album {id} not found");
                        }
                        Some(album) => {
                            sender.input(BrowserIn::AlbumsView(AlbumsViewOut::ClickedAlbum(
                                Id::album(&album.id),
                            )));
                        }
                    }
                }
                DashboardOut::DisplayToast(title) => {
                    sender.output(BrowserOut::DisplayToast(title)).unwrap();
                }
                DashboardOut::FavoriteClicked(id, state) => sender
                    .output(BrowserOut::FavoriteAlbumClicked(id, state))
                    .unwrap(),
            },
            BrowserIn::AlbumsView(msg) => match msg {
                AlbumsViewOut::ClickedAlbum(id) => {
                    let album: relm4::Controller<AlbumView> = AlbumView::builder()
                        .launch((self.subsonic.clone(), id))
                        .forward(sender.input_sender(), |msg| {
                            BrowserIn::AlbumView(Box::new(msg))
                        });

                    sender
                        .output(BrowserOut::ChangedViewTo(views::Views::Album))
                        .unwrap();
                    self.history_widget
                        .push(Views::Album(album.widget().clone()));
                    self.content.set_child(Some(album.widget()));
                    self.album_views.push(album);
                    sender
                        .output(BrowserOut::BackButtonSensitivity(true))
                        .unwrap();
                }
                AlbumsViewOut::DisplayToast(title) => {
                    sender.output(BrowserOut::DisplayToast(title)).unwrap();
                }
                AlbumsViewOut::FavoriteClicked(id, state) => sender
                    .output(BrowserOut::FavoriteAlbumClicked(id, state))
                    .unwrap(),
                AlbumsViewOut::AddToQueue(drop) => sender
                    .output(BrowserOut::InsertAfterCurrentInQueue(drop))
                    .unwrap(),
                AlbumsViewOut::AppendToQueue(drop) => {
                    sender.output(BrowserOut::AppendToQueue(drop)).unwrap()
                }
                AlbumsViewOut::ReplaceQueue(drop) => {
                    sender.output(BrowserOut::ReplaceQueue(drop)).unwrap()
                }
                AlbumsViewOut::ClickedArtist(id) => sender.input(BrowserIn::ShowArtist(id)),
            },
            BrowserIn::ArtistsView(msg) => match msg {
                ArtistsViewOut::ClickedArtist(id) => sender.input(BrowserIn::ShowArtist(id)),
                ArtistsViewOut::DisplayToast(msg) => {
                    sender.output(BrowserOut::DisplayToast(msg)).unwrap();
                }
                ArtistsViewOut::FavoriteClicked(id, state) => sender
                    .output(BrowserOut::FavoriteArtistClicked(id, state))
                    .unwrap(),
                ArtistsViewOut::AddToQueue(drop) => sender
                    .output(BrowserOut::InsertAfterCurrentInQueue(drop))
                    .unwrap(),
                ArtistsViewOut::AppendToQueue(drop) => {
                    sender.output(BrowserOut::AppendToQueue(drop)).unwrap()
                }
                ArtistsViewOut::ReplaceQueue(drop) => {
                    sender.output(BrowserOut::ReplaceQueue(drop)).unwrap()
                }
            },
            BrowserIn::AlbumView(msg) => match *msg {
                AlbumViewOut::AppendAlbum(drop) => {
                    sender.output(BrowserOut::AppendToQueue(drop)).unwrap();
                }
                AlbumViewOut::InsertAfterCurrentPlayed(drop) => sender
                    .output(BrowserOut::InsertAfterCurrentInQueue(drop))
                    .unwrap(),
                AlbumViewOut::ReplaceQueue(drop) => {
                    sender.output(BrowserOut::ReplaceQueue(drop)).unwrap();
                }
                AlbumViewOut::DisplayToast(title) => {
                    sender.output(BrowserOut::DisplayToast(title)).unwrap();
                }
                AlbumViewOut::FavoriteAlbumClicked(id, state) => sender
                    .output(BrowserOut::FavoriteAlbumClicked(id, state))
                    .unwrap(),
                AlbumViewOut::FavoriteSongClicked(id, state) => sender
                    .output(BrowserOut::FavoriteSongClicked(id, state))
                    .unwrap(),
                AlbumViewOut::Download(drop) => sender.output(BrowserOut::Download(drop)).unwrap(),
                AlbumViewOut::ArtistClicked(id) => sender.input(BrowserIn::ShowArtist(id)),
            },
            BrowserIn::ArtistView(msg) => match *msg {
                ArtistViewOut::AlbumClicked(id) => {
                    let album: relm4::Controller<AlbumView> = AlbumView::builder()
                        .launch((self.subsonic.clone(), id))
                        .forward(sender.input_sender(), |msg| {
                            BrowserIn::AlbumView(Box::new(msg))
                        });

                    sender
                        .output(BrowserOut::ChangedViewTo(views::Views::Album))
                        .unwrap();
                    self.history_widget
                        .push(Views::Album(album.widget().clone()));
                    self.content.set_child(Some(album.widget()));
                    self.album_views.push(album);
                    sender
                        .output(BrowserOut::BackButtonSensitivity(true))
                        .unwrap();
                }
                ArtistViewOut::AppendArtist(drop) => {
                    sender.output(BrowserOut::AppendToQueue(drop)).unwrap();
                }
                ArtistViewOut::InsertAfterCurrentPlayed(drop) => sender
                    .output(BrowserOut::InsertAfterCurrentInQueue(drop))
                    .unwrap(),
                ArtistViewOut::ReplaceQueue(drop) => {
                    sender.output(BrowserOut::ReplaceQueue(drop)).unwrap();
                }
                ArtistViewOut::DisplayToast(title) => {
                    sender.output(BrowserOut::DisplayToast(title)).unwrap();
                }
                ArtistViewOut::FavoriteAlbumClicked(id, state) => sender
                    .output(BrowserOut::FavoriteAlbumClicked(id, state))
                    .unwrap(),
                ArtistViewOut::FavoriteArtistClicked(id, state) => sender
                    .output(BrowserOut::FavoriteArtistClicked(id, state))
                    .unwrap(),
                ArtistViewOut::Download(drop) => sender.output(BrowserOut::Download(drop)).unwrap(),
            },
            BrowserIn::TracksView(msg) => match msg {
                TracksViewOut::DisplayToast(msg) => {
                    sender.output(BrowserOut::DisplayToast(msg)).unwrap();
                }
                TracksViewOut::AddToQueue(drop) => sender
                    .output(BrowserOut::InsertAfterCurrentInQueue(drop))
                    .unwrap(),
                TracksViewOut::AppendToQueue(drop) => {
                    sender.output(BrowserOut::AppendToQueue(drop)).unwrap();
                }
                TracksViewOut::ReplaceQueue(drop) => {
                    sender.output(BrowserOut::ReplaceQueue(drop)).unwrap();
                }
                TracksViewOut::Download(drop) => sender.output(BrowserOut::Download(drop)).unwrap(),
                TracksViewOut::FavoriteClicked(id, state) => sender
                    .output(BrowserOut::FavoriteSongClicked(id, state))
                    .unwrap(),
                TracksViewOut::ClickedArtist(id) => sender.input(BrowserIn::ShowArtist(id)),
                TracksViewOut::ClickedAlbum(id) => sender.input(BrowserIn::ShowAlbum(id)),
            },
            BrowserIn::PlaylistsView(msg) => match msg {
                PlaylistsViewOut::DisplayToast(title) => {
                    sender.output(BrowserOut::DisplayToast(title)).unwrap();
                }
                PlaylistsViewOut::AppendToQueue(list) => {
                    let drop = Droppable::Playlist(Box::new(list));
                    sender.output(BrowserOut::AppendToQueue(drop)).unwrap();
                }
                PlaylistsViewOut::AddToQueue(list) => {
                    let drop = Droppable::Playlist(Box::new(list));
                    sender
                        .output(BrowserOut::InsertAfterCurrentInQueue(drop))
                        .unwrap();
                }
                PlaylistsViewOut::ReplaceQueue(list) => {
                    let drop = Droppable::Playlist(Box::new(list));
                    sender.output(BrowserOut::ReplaceQueue(drop)).unwrap();
                }
                PlaylistsViewOut::DeletePlaylist(index, list) => {
                    //delete playlist from server
                    let client = Client::get().unwrap();
                    if let Err(e) = client.delete_playlist(list.base.id.clone()).await {
                        sender
                            .output(BrowserOut::DisplayToast(format!(
                                "could not delete playlist from server: {e:?}"
                            )))
                            .unwrap();
                        return;
                    }

                    //delete paylist from subsonic cache
                    self.subsonic.borrow_mut().delete_playlist(&list);

                    //update views
                    for view in &self.playlists_views {
                        view.emit(PlaylistsViewIn::DeletePlaylist(index.clone()));
                    }
                }
                PlaylistsViewOut::CreateEmptyPlaylist => {
                    sender.input(BrowserIn::NewPlaylist(gettext("New Playlist"), vec![]));
                }
                PlaylistsViewOut::CreatePlaylist(drop) => {
                    let songs = drop.get_songs(&self.subsonic);
                    sender.input(BrowserIn::NewPlaylist(gettext("New Playlist"), songs));
                }

                PlaylistsViewOut::Download(drop) => {
                    sender.output(BrowserOut::Download(drop)).unwrap();
                }
                PlaylistsViewOut::FavoriteClicked(id, state) => sender
                    .output(BrowserOut::FavoriteSongClicked(id, state))
                    .unwrap(),
                PlaylistsViewOut::ClickedArtist(id) => sender.input(BrowserIn::ShowArtist(id)),
                PlaylistsViewOut::ClickedAlbum(id) => sender.input(BrowserIn::ShowAlbum(id)),
                PlaylistsViewOut::RenamePlaylist(list) => {
                    sender.input(BrowserIn::RenamePlaylist(list))
                }
            },
            BrowserIn::RenamePlaylist(list) => {
                // change server
                let client = Client::get().unwrap();
                if let Err(e) = client
                    .update_playlist(
                        &list.id,
                        Some(list.name.clone()),
                        None::<&str>,
                        None,
                        Vec::<&str>::new(),
                        vec![],
                    )
                    .await
                {
                    sender
                        .output(BrowserOut::DisplayToast(format!(
                            "could not update playlist on server: {e:?}"
                        )))
                        .unwrap();
                    return;
                }

                // change local cache
                self.subsonic.borrow_mut().rename_playlist(&list);
            }
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
                                .unwrap();
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
                                .unwrap();
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
                                .unwrap();
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
            BrowserIn::UpdateFavoriteSong(id, state) => {
                //notify all views with songs in them
                for view in &self.album_views {
                    view.emit(AlbumViewIn::UpdateFavoriteSong(id.clone(), state));
                }
                for view in &self.playlists_views {
                    view.emit(PlaylistsViewIn::UpdateFavoriteSong(id.clone(), state));
                }
                if let Some(tracks) = &self.tracks {
                    tracks.emit(TracksViewIn::UpdateFavoriteSong(id, state));
                }
            }
            BrowserIn::UpdateFavoriteAlbum(id, state) => {
                //notify all views with albums in them
                self.dashboard
                    .emit(DashboardIn::UpdateFavoriteAlbum(id.clone(), state));
                if let Some(albums) = &self.albums {
                    albums.emit(AlbumsViewIn::UpdateFavoriteAlbum(id.clone(), state));
                }
                for view in &self.album_views {
                    view.emit(AlbumViewIn::UpdateFavoriteAlbum(id.clone(), state));
                }
                for view in &self.artist_views {
                    view.emit(ArtistViewIn::UpdateFavoriteAlbum(id.clone(), state));
                }
            }
            BrowserIn::UpdateFavoriteArtist(id, state) => {
                //notify all views with artists in them
                for view in &self.artist_views {
                    view.emit(ArtistViewIn::UpdateFavoriteArtist(id.clone(), state));
                }
                if let Some(artists) = &self.artists {
                    artists.emit(ArtistsViewIn::UpdateFavoriteArtist(id, state));
                }
            }
            BrowserIn::UpdatePlayCountSong(id, play_count) => {
                for view in &self.album_views {
                    view.emit(AlbumViewIn::UpdatePlayCountSong(id.clone(), play_count));
                }
                for view in &self.playlists_views {
                    view.emit(PlaylistsViewIn::UpdatePlayCountSong(id.clone(), play_count));
                }
                if let Some(tracks) = &self.tracks {
                    tracks.emit(TracksViewIn::UpdatePlayCountSong(id.clone(), play_count));
                }
            }
            BrowserIn::UpdatePlayCountAlbum(id, play_count) => {
                if let Some(albums) = &self.albums {
                    albums.emit(AlbumsViewIn::UpdatePlayCountAlbum(id.clone(), play_count));
                }
            }
            BrowserIn::InsertSongsToPlaylist(index, songs) => {
                for view in &self.playlists_views {
                    view.emit(PlaylistsViewIn::InsertSongsTo(index, songs.clone()));
                }
            }
        }
    }
}
