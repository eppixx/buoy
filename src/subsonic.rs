use futures::StreamExt;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use std::{collections::HashMap, io::Read};

use crate::{client::Client, subsonic_cover, subsonic_cover::SubsonicCovers};

const PREFIX: &str = "Buoy";
const MUSIC_INFOS: &str = "Music-Infos";

#[derive(Error, Debug)]
enum Error {
    #[error("Settings does not exists yet")]
    NoClient,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Subsonic {
    scan_status: Option<i64>,
    artists: Vec<submarine::data::ArtistId3>,
    album_list: Vec<submarine::data::Child>,
    playlists: Vec<submarine::data::PlaylistWithSongs>,
    #[serde(skip)]
    covers: SubsonicCovers,
}

impl Subsonic {
    // this is the main way to create a Subsonic object
    pub async fn load_or_create() -> anyhow::Result<Self> {
        let current_scan_status = {
            let client = Client::get().ok_or(Error::NoClient)?;
            client.get_scan_status().await?
        };

        let mut subsonic = match Self::load().await {
            Ok(subsonic) => {
                if subsonic.scan_status == current_scan_status.count {
                    tracing::info!("scan status is current; load cached info");
                    subsonic
                } else {
                    tracing::info!("scan_status changed; reload info");
                    Self::new().await?
                }
            }
            Err(_e) => {
                tracing::warn!("no cache found or cache is malformed");
                //load new from server
                Self::new().await?
            }
        };
        subsonic.work().await?;
        Ok(subsonic)
    }

    pub async fn load() -> anyhow::Result<Self> {
        let xdg_dirs = xdg::BaseDirectories::with_prefix(PREFIX)?;
        let cache_path = xdg_dirs
            .place_cache_file(MUSIC_INFOS)
            .expect("cannot create cache directory");
        let mut content = String::new();
        let mut file = std::fs::File::open(cache_path)?;
        file.read_to_string(&mut content)?;
        tracing::info!("loaded subsonic cache");
        let result = toml::from_str::<Self>(&content)?;

        Ok(result)
    }

    pub async fn new() -> anyhow::Result<Self> {
        tracing::info!("create subsonic cache");
        let client = Client::get().unwrap();

        //fetch scan status
        let scan_status = client.get_scan_status().await?;

        //fetch artists
        tracing::info!("fetching artists");
        let indexes = client.get_artists(None).await?;
        let artists = indexes.into_iter().flat_map(|i| i.artist).collect();

        //fetch album_list
        tracing::info!("fetching album_list");
        let album_list: Vec<submarine::data::Child> = {
            let mut albums = vec![];
            let mut offset = 0;
            loop {
                let mut part = client
                    .get_album_list2(
                        submarine::api::get_album_list::Order::AlphabeticalByName,
                        Some(500),
                        Some(offset),
                        None::<&str>,
                    )
                    .await?;
                if part.len() < 500 || part.is_empty() {
                    albums.append(&mut part);
                    break;
                } else {
                    albums.append(&mut part);
                    offset += 500;
                }
            }
            albums
        };

        //fetch playlists
        tracing::info!("fetching playlists");
        let playlists = {
            let mut playlist_list = vec![];
            let playlists = client.get_playlists(None::<&str>).await?;
            for playlist in playlists {
                let list = client.get_playlist(playlist.id).await?;
                playlist_list.push(list);
            }
            playlist_list
        };

        let result = Self {
            scan_status: scan_status.count,
            artists,
            album_list,
            playlists,
            covers: SubsonicCovers::default(),
        };

        result.save()?;

        tracing::info!("finished loading subsonic info");
        Ok(result)
    }

    async fn work(&mut self) -> anyhow::Result<()> {
        let ids: Vec<String> = self
            .album_list
            .iter()
            .filter_map(|album| album.cover_art.clone())
            .chain(
                self.artists
                    .iter()
                    .filter_map(|artist| artist.cover_art.clone()),
            )
            .chain(
                self.playlists
                    .iter()
                    .filter_map(|list| list.base.cover_art.clone()),
            )
            .collect();
        self.covers.work(ids).await;

        self.covers.save()?;
        Ok(())
    }

    fn save(&self) -> anyhow::Result<()> {
        tracing::info!("saving subsonic music info");
        let cache = toml::to_string(self).unwrap();
        let xdg_dirs = xdg::BaseDirectories::with_prefix(PREFIX)?;
        let cache_path = xdg_dirs
            .place_cache_file(MUSIC_INFOS)
            .expect("cannot create cache directory");
        std::fs::write(cache_path, cache).unwrap();

        Ok(())
    }

    pub fn artists(&self) -> &Vec<submarine::data::ArtistId3> {
        &self.artists
    }

    pub fn find_artist(&self, id: impl AsRef<str>) -> Option<submarine::data::ArtistId3> {
        self.artists
            .iter()
            .find(|artist| artist.id == id.as_ref())
            .cloned()
    }

    pub fn albums(&self) -> &Vec<submarine::data::Child> {
        &self.album_list
    }

    pub fn find_album(&self, id: impl AsRef<str>) -> Option<submarine::data::Child> {
        self.album_list
            .iter()
            .find(|album| album.id == id.as_ref())
            .cloned()
    }

    pub fn album_of_song(&self, child: &submarine::data::Child) -> Option<submarine::data::Child> {
        if let Some(album) = &child.album_id {
            self.find_album(album)
        } else {
            None
        }
    }

    pub fn favorite_song(&mut self, id: impl AsRef<str>, state: bool) {
        for playlist in &mut self.playlists {
            for song in &mut playlist.entry {
                if song.id == id.as_ref() {
                    match state {
                        true => song.starred = Some(chrono::offset::Local::now().into()),
                        false => song.starred = None,
                    }
                }
            }
        }
        self.save().expect("saving failed");
    }

    pub fn favorite_album(&mut self, id: impl AsRef<str>, state: bool) {
        for album in &mut self.album_list {
            if album.id == id.as_ref() {
                match state {
                    true => album.starred = Some(chrono::offset::Local::now().into()),
                    false => album.starred = None,
                }
            }
        }
        self.save().expect("saving failed");
    }

    pub fn favorite_artist(&mut self, id: impl AsRef<str>, state: bool) {
        for artist in &mut self.artists {
            if artist.id == id.as_ref() {
                match state {
                    true => artist.starred = Some(chrono::offset::Local::now().into()),
                    false => artist.starred = None,
                }
            }
        }
        self.save().expect("saving failed");
    }

    pub fn playlists(&self) -> &Vec<submarine::data::PlaylistWithSongs> {
        &self.playlists
    }

    pub fn push_playlist(&mut self, list: &submarine::data::PlaylistWithSongs) {
        self.playlists.push(list.clone());
        self.save().expect("saving failed");
    }

    pub fn delete_playlist(&mut self, list: &submarine::data::PlaylistWithSongs) {
        self.playlists
            .retain(|candidate| candidate.base.id != list.base.id);
        self.save().expect("saving failed");
    }

    pub fn cover_raw(&self, id: &str) -> Option<Vec<u8>> {
        self.covers.cover_raw(id)
    }

    pub fn cover(&mut self, id: &str) -> subsonic_cover::Response {
        self.covers.cover(id)
    }

    pub fn cover_icon(&self, id: &str) -> Option<relm4::gtk::gdk::Texture> {
        self.covers.cover_icon(id)
    }

    pub fn delete_cache(&mut self) -> anyhow::Result<()> {
        // delete stored covers
        self.covers.delete_cache()?;

        // delete music info
        let xdg_dirs = xdg::BaseDirectories::with_prefix(PREFIX)?;
        let cache_path = xdg_dirs
            .place_cache_file(MUSIC_INFOS)
            .expect("cannot create cache directory");
        std::fs::remove_file(cache_path)?;

        Ok(())
    }

    pub async fn load_all_covers(&mut self) -> HashMap<String, Option<Vec<u8>>> {
        const COVER_SIZE: Option<i32> = Some(200);
        let mut covers = std::collections::HashMap::new();

        //load album art
        tracing::info!("fetching album art from server");
        let mut tasks = vec![];

        // create tasks
        for (id, cover) in self
            .artists
            .iter()
            .filter_map(|artist| {
                artist
                    .cover_art
                    .as_ref()
                    .map(|cover| (artist.id.clone(), cover))
            })
            .chain::<Vec<(String, &String)>>(
                self.album_list
                    .iter()
                    .filter_map(|album| {
                        album
                            .cover_art
                            .as_ref()
                            .map(|cover| (album.id.clone(), cover))
                    })
                    .collect::<Vec<(String, &String)>>(),
            )
            .chain::<Vec<(String, &String)>>(
                self.playlists
                    .iter()
                    .filter_map(|playlist| {
                        playlist
                            .base
                            .cover_art
                            .as_ref()
                            .map(|cover| (playlist.base.id.clone(), cover))
                    })
                    .collect::<Vec<(String, &String)>>(),
            )
        {
            tasks.push(async move {
                let client = Client::get().unwrap();
                (id.clone(), client.get_cover_art(cover, COVER_SIZE).await)
            });
        }

        // buffer tasks so only 100 will be simultaniously loaded
        tracing::info!("number of albums to fetch {}", tasks.len());
        let stream = futures::stream::iter(tasks)
            .buffered(100)
            .collect::<Vec<_>>();
        let results = stream.await;

        // actual fetch
        for (id, cover) in results {
            match cover {
                Ok(cover) => _ = covers.insert(id.clone(), Some(cover)),
                Err(e) => {
                    tracing::error!("error fetching: {e}");
                    let client = Client::get().unwrap();
                    match client.get_cover_art(&id, COVER_SIZE).await {
                        Ok(cover) => _ = covers.insert(id, Some(cover)),
                        Err(e) => tracing::error!("refetching resulted in error {e}"),
                    }
                }
            }
        }

        covers
    }
}
