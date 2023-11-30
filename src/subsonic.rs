use futures::StreamExt;
use relm4::gtk;
use serde::{Deserialize, Serialize};

use std::{collections::HashMap, io::Read};

use crate::client::Client;

const PREFIX: &str = "Buoy";
const MUSIC_INFOS: &str = "Music-Infos";
const COVER_CACHE: &str = "Covers";

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct Subsonic {
    artists: Vec<submarine::data::ArtistId3>,
    album_list: Vec<submarine::data::Child>,
    // scan_status: submarine::data::ScanStatus,
    #[serde(skip)]
    covers: HashMap<String, Option<gtk::Image>>,
    #[serde(skip)]
    cached_images: HashMap<String, Option<Vec<u8>>>,
}

impl Subsonic {
    pub async fn load_or_create() -> anyhow::Result<Self> {
        match Self::load() {
            Ok(subsonic) => Ok(subsonic),
            Err(_e) => {
                tracing::warn!("no cache found");
                //load new from server
                let subsonic = Self::new().await?;
                // subsonic.save()?;
                Ok(subsonic)
            }
        }
    }

    pub fn load() -> anyhow::Result<Self> {
        let xdg_dirs = xdg::BaseDirectories::with_prefix(PREFIX)?;
        let cache_path = xdg_dirs
            .place_cache_file(MUSIC_INFOS)
            .expect("cannot create cache directory");
        let mut content = String::new();
        let mut file = std::fs::File::open(cache_path)?;
        file.read_to_string(&mut content)?;
        tracing::info!("loaded subsonic cache");
        let mut result = toml::from_str::<Self>(&content)?;
        {
            let xdg_dirs = xdg::BaseDirectories::with_prefix(PREFIX).unwrap();
            let cache_path = xdg_dirs
                .place_cache_file(COVER_CACHE)
                .expect("cannot create cache directory");

            let cache = match std::fs::File::open(cache_path) {
                Ok(mut file) => {
                    // load file content
                    let mut content = vec![];
                    file.read_to_end(&mut content).unwrap();
                    postcard::from_bytes::<HashMap<String, Option<Vec<u8>>>>(&content).unwrap()
                }
                _ => HashMap::default(),
            };
            tracing::error!("len of cache: {}", cache.len());
            result.cached_images = cache;
        }
        result.covers = result
            .cached_images
            .iter()
            .map(|(id, b)| match b {
                None => (id.into(), None),
                Some(b) => {
                    let bytes = gtk::glib::Bytes::from(b);
                    let stream = gtk::gio::MemoryInputStream::from_bytes(&bytes);
                    match gtk::gdk_pixbuf::Pixbuf::from_stream(&stream, gtk::gio::Cancellable::NONE)
                    {
                        Ok(pixbuf) => (id.into(), Some(gtk::Image::from_pixbuf(Some(&pixbuf)))),
                        _ => (id.into(), None),
                    }
                }
            })
            .collect::<HashMap<String, Option<gtk::Image>>>();
        tracing::error!("len of pixbuf: {}", result.covers.len());
        Ok(result)
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

        let mut result = Self {
            artists,
            album_list,
            cached_images: HashMap::default(),
            covers: HashMap::default(),
        };
        result.cached_images = result.load_all_covers().await;

        tracing::info!("finished loading subsonic info");
        Ok(result)
    }

    pub fn save(&self) -> anyhow::Result<()> {
        tracing::info!("saving subsonic music info");
        let cache = toml::to_string(self).unwrap();
        let xdg_dirs = xdg::BaseDirectories::with_prefix(PREFIX)?;
        let cache_path = xdg_dirs
            .place_cache_file(MUSIC_INFOS)
            .expect("cannot create cache directory");
        std::fs::write(cache_path, cache).unwrap();

        tracing::info!("saving cover cache");
        let cache = postcard::to_allocvec(&self.cached_images).unwrap();
        let xdg_dirs = xdg::BaseDirectories::with_prefix(PREFIX).unwrap();
        let cache_path = xdg_dirs
            .place_cache_file(COVER_CACHE)
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

    pub fn cover(&self, id: impl AsRef<str>) -> Option<&gtk::Image> {
        match self.covers.get(id.as_ref()) {
            None => None,
            Some(None) => None,
            Some(pix) => pix.as_ref(),
        }
    }

    pub async fn load_all_covers(&mut self) -> HashMap<String, Option<Vec<u8>>> {
        let client = Client::get().lock().unwrap().inner.clone().unwrap();
        let mut covers = std::collections::HashMap::new();

        //load album art
        tracing::info!("fetching album art from server");
        let mut tasks = vec![];
        for album in &self.album_list {
            if let Some(cover) = &album.cover_art {
                tasks.push(async move {
                    let client = Client::get().lock().unwrap().inner.clone().unwrap();
                    (
                        album.id.clone(),
                        client.get_cover_art(cover, Some(200)).await,
                    )
                });
            }
        }

        tracing::info!("number of albums to fetch {}", tasks.len());
        let stream = futures::stream::iter(tasks)
            .buffered(100)
            .collect::<Vec<_>>();
        let results = stream.await;

        // let results = futures::future::join_all(tasks).await;
        for (id, cover) in results {
            match cover {
                Ok(cover) => _ = covers.insert(id.clone(), Some(cover)),
                Err(e) => {
                    tracing::error!("error fetching: {e}");
                    let client = Client::get().lock().unwrap().inner.clone().unwrap();
                    match client.get_cover_art(&id, Some(200)).await {
                        Ok(cover) => _ = covers.insert(id, Some(cover)),
                        Err(e) => tracing::error!("refetching resulted in error {e}"),
                    }
                }
            }
        }

        //load artist art
        tracing::info!("fetchung artist art from server");
        tracing::info!("number of artists to fetch {}", self.artists.len());
        for artist in &self.artists {
            tracing::warn!("new art");
            match &artist.cover_art {
                None => _ = covers.insert(artist.id.clone(), None),
                Some(cover) => match client.get_cover_art(cover, Some(200)).await {
                    Ok(cover) => _ = covers.insert(artist.id.clone(), Some(cover)),
                    Err(e) => tracing::error!("error fetching:{e}"),
                },
            }
        }

        //TODO load playlist art

        covers
    }
}
