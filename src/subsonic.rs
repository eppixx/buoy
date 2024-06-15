use futures::StreamExt;
use relm4::gtk;
use serde::{Deserialize, Serialize};

use std::{collections::HashMap, io::Read};

use crate::{client::Client, subsonic_cover::SubsonicCovers};

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
    #[serde(skip)]
    pub coverss: SubsonicCovers,
}

impl Subsonic {
    // this is the main way to create a Subsonic object
    pub async fn load_or_create() -> anyhow::Result<Self> {
        match Self::load().await {
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

    pub async fn load() -> anyhow::Result<Self> {
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
        tracing::info!("loaded {} covers from local chache", result.covers.len());

        let ids: Vec<String> = result
            .album_list
            .iter()
            .filter_map(|album| album.cover_art.clone())
            .chain(
                result
                    .artists
                    .iter()
                    .filter_map(|artist| artist.cover_art.clone()),
            )
            .collect();
        result.coverss.work(ids).await;
        Ok(result)
    }

    pub async fn new() -> anyhow::Result<Self> {
        tracing::info!("create subsonic cache");
        let client = Client::get().unwrap();

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
            coverss: SubsonicCovers::default(),
        };
        result.coverss.work(vec![]).await;
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
        // TODO fetch playlist covers
        // .chain::<Vec<String, &String>>(self.playlists.iter().filter_map(|playlist| {
        //     if let Some(cover) = &playlist.cover_art {
        //         Some((playlist.id.clone(), cover))
        //     } else {
        //         None
        //     }
        // }))
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
