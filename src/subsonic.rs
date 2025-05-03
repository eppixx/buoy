use std::collections::HashMap;

use futures::StreamExt;
use serde::{Deserialize, Serialize};

use crate::{client::Client, subsonic_cover, subsonic_cover::SubsonicCovers};

const PREFIX: &str = "Buoy";
const MUSIC_INFOS: &str = "Music-Infos";

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct Subsonic {
    scan_status: Option<i64>,
    artists: Vec<submarine::data::ArtistId3>,
    album_list: Vec<submarine::data::Child>,
    tracks: Vec<submarine::data::Child>,
    playlists: Vec<submarine::data::PlaylistWithSongs>,
    #[serde(skip)]
    covers: SubsonicCovers,
}

impl Subsonic {
    pub async fn new() -> anyhow::Result<Self> {
        tracing::info!("create subsonic cache");
        let client = Client::get().unwrap();

        //fetch scan status
        let scan_status = client.get_scan_status().await?;

        //fetch artists
        tracing::info!("fetching artists");
        let indexes = client.get_artists(None).await?;
        let artists: Vec<_> = indexes.into_iter().flat_map(|i| i.artist).collect();
        tracing::info!("fetched {} artists", artists.len());

        //fetch album_list
        tracing::info!("fetching albums");
        let album_list: Vec<submarine::data::Child> = {
            let mut albums = vec![];
            let mut offset = 0;
            loop {
                match client
                    .get_album_list(
                        submarine::api::get_album_list::Order::AlphabeticalByName,
                        Some(500),
                        Some(offset),
                        None::<&str>,
                    )
                    .await
                {
                    Err(e) => {
                        tracing::error!("error while fetching albums: {e}");
                    }
                    Ok(mut part) => {
                        if part.len() < 500 || part.is_empty() {
                            albums.append(&mut part);
                            break;
                        } else {
                            albums.append(&mut part);
                            offset += 500;
                        }
                    }
                }
            }
            albums
        };
        tracing::info!("fetched {} albums", album_list.len());

        //fetch tracks
        tracing::info!("fetching tracks");
        let tasks: Vec<_> = album_list
            .iter()
            .map(|album| async move {
                let client = Client::get().unwrap();
                tracing::info!("start loading album {}", album.title);
                match client.get_album(&album.id).await {
                    Ok(album) => album.song,
                    Err(e) => {
                        tracing::error!("error fetching album {}: {e}", album.title);
                        vec![]
                    }
                }
            })
            .collect();
        //buffer futures to not overwhelm server and client
        // based on: https://stackoverflow.com/questions/70871368/limiting-the-number-of-concurrent-futures-in-join-all
        let stream = futures::stream::iter(tasks)
            .buffer_unordered(50)
            .collect::<Vec<_>>();
        let tracks = stream.await;
        let tracks: Vec<submarine::data::Child> = tracks.into_iter().flatten().collect();
        tracing::info!("fetched {} tracks", tracks.len());

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
        tracing::info!("fetched {} playlists", playlists.len());

        let result = Self {
            scan_status: scan_status.count,
            artists,
            album_list,
            tracks,
            playlists,
            covers: SubsonicCovers::default(),
        };

        result.save()?;

        tracing::info!("finished loading subsonic info");
        Ok(result)
    }

    // this is the main way to create a Subsonic object
    pub async fn load_or_create() -> anyhow::Result<Self> {
        let current_scan_status = {
            let client = match Client::get() {
                None => {
                    tracing::warn!("no client found");
                    return Ok(Self::default());
                }
                Some(client) => client,
            };
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

        let _ = subsonic.covers.load();
        Ok(subsonic)
    }

    pub async fn load() -> anyhow::Result<Self> {
        let cache_path = dirs::cache_dir()
            .ok_or(std::io::Error::new(
                std::io::ErrorKind::Other,
                "cant create cache dir",
            ))?
            .join(PREFIX)
            .join(MUSIC_INFOS);
        let content = tokio::fs::read(cache_path).await?;
        tracing::info!("loaded subsonic cache");
        let mut reader = content.as_slice();
        let mut deserializer = rmp_serde::Deserializer::new(&mut reader);
        let mut result = Self::deserialize(&mut deserializer)?;

        // update smart playlists
        let client = Client::get().unwrap();
        let mut modified_list = false;
        for playlist in result.playlists.iter_mut() {
            if Self::is_smart_playlist(&playlist.base) {
                *playlist = client
                    .get_playlist(&playlist.base.id)
                    .await?;
                modified_list = true;
            }
        }
        if modified_list {
            tracing::info!("updated smart playlist(s)");
        }

        tracing::info!("loaded subsonic cache done");
        Ok(result)
    }

    pub fn save(&self) -> anyhow::Result<()> {
        tracing::info!("saving cover cache");
        self.covers.save()?;

        tracing::info!("saving subsonic music info");
        let mut cache = vec![];
        let mut serializer = rmp_serde::Serializer::new(&mut cache);
        self.serialize(&mut serializer)?;
        let cache_path = dirs::cache_dir()
            .ok_or(std::io::Error::new(
                std::io::ErrorKind::Other,
                "cant create cache dir",
            ))?
            .join(PREFIX)
            .join(MUSIC_INFOS);
        std::fs::write(cache_path, cache)?;

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

    pub fn songs_of_album(&self, id: impl AsRef<str>) -> Vec<submarine::data::Child> {
        self.tracks()
            .iter()
            .filter(|track| track.album_id.as_deref() == Some(id.as_ref()))
            .cloned()
            .collect()
    }

    pub fn songs_of_artist(&self, id: impl AsRef<str>) -> Vec<submarine::data::Child> {
        self.album_list
            .iter()
            .filter(|album| album.artist_id.as_deref() == Some(id.as_ref()))
            .flat_map(|album| self.songs_of_album(&album.id))
            .collect()
    }

    pub fn tracks(&self) -> &Vec<submarine::data::Child> {
        &self.tracks
    }

    pub fn find_track(&self, id: impl AsRef<str>) -> Option<submarine::data::Child> {
        self.tracks
            .iter()
            .find(|track| track.id == id.as_ref())
            .cloned()
    }

    pub fn tracks_from_album_id3(
        &self,
        album: &submarine::data::AlbumId3,
    ) -> Vec<&submarine::data::Child> {
        self.tracks
            .iter()
            .filter(|track| track.album_id.as_deref() == Some(&album.id))
            .collect()
    }

    pub fn tracks_from_album(
        &self,
        album: &submarine::data::Child,
    ) -> Vec<&submarine::data::Child> {
        self.tracks
            .iter()
            .filter(|track| track.album_id.as_deref() == Some(&album.id))
            .collect()
    }

    pub fn albums_from_artist(
        &self,
        artist: &submarine::data::ArtistId3,
    ) -> Vec<&submarine::data::Child> {
        self.album_list
            .iter()
            .filter(|album| album.artist_id.as_deref() == Some(&artist.id))
            .collect()
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
        for track in &mut self.tracks {
            if track.id == id.as_ref() {
                match state {
                    true => track.starred = Some(chrono::offset::Local::now().into()),
                    false => track.starred = None,
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

    pub fn rename_playlist(&mut self, candidate: &submarine::data::Playlist) {
        for list in &mut self.playlists {
            if list.base.id == candidate.id {
                list.base.name = candidate.name.clone();
            }
        }
        self.save().expect("saving failed");
    }

    pub fn move_playlist(&mut self, src_index: usize, target_index: usize) {
        let item = self.playlists.remove(src_index);

        let adjusted_index = if src_index < target_index {
            target_index - 1
        } else {
            target_index
        };

        self.playlists.insert(adjusted_index, item);
        self.save().expect("saving failed");
    }

    pub fn replace_playlist(&mut self, list: &submarine::data::PlaylistWithSongs) {
        let Some(playlist) = self
            .playlists
            .iter_mut()
            .find(|playlist| playlist.base.id == list.base.id)
        else {
            tracing::error!("tried to replace playlist, but no playlist to replace");
            return;
        };

        playlist.entry = list.entry.clone();
        playlist.base = list.base.clone();
        self.save().expect("saving failed");
    }

    pub fn increment_play_counter(&mut self, candidate: &submarine::data::Child) {
        for track in &mut self.tracks {
            if track.id == candidate.id {
                track.play_count.map(|count| count + 1);
            }
        }
        for playlist in &mut self.playlists {
            for track in &mut playlist.entry {
                if track.id == candidate.id {
                    track.play_count.map(|count| count + 1);
                }
            }
        }
        self.save().expect("saving failed");
    }

    pub fn cover_raw(&self, id: &str) -> Option<Vec<u8>> {
        self.covers.cover_raw(id)
    }

    pub fn cover(&mut self, id: &str) -> subsonic_cover::Response {
        self.covers.cover(id)
    }

    pub fn cover_update(&mut self, id: &str, buffer: Option<Vec<u8>>) {
        self.covers.cover_update(id, buffer);
    }

    pub fn cover_icon(&self, id: &str) -> Option<relm4::gtk::gdk::Texture> {
        self.covers.cover_icon(id)
    }

    pub fn delete_cache(&mut self) -> anyhow::Result<()> {
        // delete stored covers
        self.covers.delete_cache()?;

        // delete music info
        let cache_path = dirs::cache_dir()
            .ok_or(std::io::Error::new(
                std::io::ErrorKind::Other,
                "cant find cache info",
            ))?
            .join(PREFIX)
            .join(MUSIC_INFOS);
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

    // check if playlist is a generated Playlist
    // there may be different telling signs for different servers
    pub fn is_smart_playlist(list: &submarine::data::Playlist) -> bool {
        // check comments of playlist
        let signs_comment = [
            "Auto-imported", // imported playlist from navidrome
        ];
        if signs_comment
            .iter()
            .any(|sign| list.comment.as_deref().unwrap_or_default().contains(sign))
        {
            return true;
        }

        false
    }
}
