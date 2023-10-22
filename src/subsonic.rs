use serde::{Deserialize, Serialize};

use std::{collections::HashMap, io::Read};

use crate::{client::Client, types::Id};

const PREFIX: &str = "Buoy";
const FILE_NAME: &str = "Cache";

#[derive(Debug, Default, Deserialize, Serialize, Clone)]
pub struct Subsonic {
    artists: Vec<submarine::data::ArtistId3>,
    album_list: Vec<submarine::data::Child>,
    albums: HashMap<String, submarine::data::AlbumWithSongsId3>,
    // covers: HashMap<String, Vec<u8>>,
}

impl Subsonic {
    pub async fn load_or_create() -> anyhow::Result<Self> {
        match Self::load() {
            Ok(subsonic) => Ok(subsonic),
            Err(_e) => {
                dbg!(_e);
                tracing::warn!("no cache found");
                //load new from server
                let subsonic = Self::new().await?;
                subsonic.save()?;
                Ok(subsonic)
            }
        }
    }

    pub fn load() -> anyhow::Result<Self> {
        let xdg_dirs = xdg::BaseDirectories::with_prefix(PREFIX)?;
        let cache_path = xdg_dirs
            .place_cache_file(FILE_NAME)
            .expect("cannot create cache directory");
        let mut content = String::new();
        let mut file = std::fs::File::open(cache_path)?;
        file.read_to_string(&mut content)?;
        Ok(toml::from_str::<Self>(&content)?)
    }

    pub async fn new() -> anyhow::Result<Self> {
        tracing::info!("create subsonic cache");
        let client = Client::get().lock().unwrap().inner.clone().unwrap();

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

        // fetch albums
        tracing::info!("fetching albums");
        let mut albums = HashMap::new();
        for album in album_list.iter() {
            let new_album = match client.get_album(&album.id).await {
                Ok(album) => album,
                Err(e) => {
                    tracing::error!("error fetching id {}: {e}", &album.id);
                    continue;
                }
            };
            albums.insert(album.id.clone(), new_album);
        }

        let result = Self {
            artists,
            album_list,
            albums,
            // covers: HashMap::new(),
        };

        Ok(result)
    }

    fn save(&self) -> anyhow::Result<()> {
        tracing::info!("saving subsonic info");
        // let cache = postcard::to_allocvec(self).unwrap();
        let cache = toml::to_string(self).unwrap();
        let xdg_dirs = xdg::BaseDirectories::with_prefix(PREFIX)?;
        let cache_path = xdg_dirs
            .place_cache_file(FILE_NAME)
            .expect("cannot create cache directory");
        std::fs::write(cache_path, cache).unwrap();
        Ok(())
    }

    pub fn artists(&self) -> &Vec<submarine::data::ArtistId3> {
        &self.artists
    }

    pub fn albums(&self) -> &Vec<submarine::data::Child> {
        &self.album_list
    }

    // pub fn album(&mut self, id: &Id) -> Option<&submarine::data::AlbumWithSongsId3> {
    //     self.albums.get(id.inner())
    // }
}
