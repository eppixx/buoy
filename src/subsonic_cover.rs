use futures::StreamExt;
use relm4::gtk::{self, gdk};
use serde::{Deserialize, Serialize};

use std::{collections::HashMap, io::Cursor};

use crate::client::Client;

const COVER_SIZE: Option<i32> = Some(200);
const CONCURRENT_FETCH: usize = 100;
const PREFIX: &str = "Buoy";
const COVER_CACHE: &str = "cover-cache";

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct SubsonicCovers {
    /// the raw buffers that are send from server
    buffers: HashMap<String, Option<Vec<u8>>>,
    /// converted from buffers, can be copied
    #[serde(skip)]
    covers: HashMap<String, Option<gdk::Texture>>,
}

#[derive(Default, Debug)]
pub enum Response {
    /// there is no image on server
    #[default]
    Empty,
    /// downloaded image from server
    Loaded(gdk::Texture),
}

impl SubsonicCovers {
    pub async fn work(&mut self, start_requests: Vec<String>) {
        // try to load covers from cache
        if self.load().is_ok() {
            return;
        }

        //build futures
        let tasks: Vec<_> = start_requests
            .iter()
            .map(|id| async move {
                let client = Client::get().unwrap();
                let cover = client.get_cover_art(id, COVER_SIZE).await.unwrap();
                (id, cover)
            })
            .collect();
        tracing::info!("start fetching {} covers", tasks.len());

        //buffer futures to not overwhelm server and client
        // based on: https://stackoverflow.com/questions/70871368/limiting-the-number-of-concurrent-futures-in-join-all
        let stream = futures::stream::iter(tasks)
            .buffer_unordered(CONCURRENT_FETCH)
            .collect::<Vec<_>>();
        let results = stream.await;

        for (id, cover) in results {
            self.buffers
                .entry(id.clone())
                .and_modify(|buf| *buf = Some(cover.clone()))
                .or_insert(Some(cover.clone()));
        }
        tracing::info!("fetched all covers");
    }

    pub fn cover_raw(&self, id: &str) -> Option<Vec<u8>> {
        match self.buffers.get(id) {
            None => None,
            Some(buffer) => buffer.clone(),
        }
    }

    pub fn cover(&mut self, id: &str) -> Response {
        match self.covers.get(id) {
            Some(Some(cover)) => {
                //return cached cover
                Response::Loaded(cover.clone())
            }
            Some(None) => Response::Empty,
            None => {
                match self.buffers.get(id) {
                    Some(Some(buffer)) => {
                        // converting buffer to image
                        let bytes = gtk::glib::Bytes::from(buffer);
                        match gdk::Texture::from_bytes(&bytes) {
                            Ok(texture) => {
                                self.covers.insert(id.into(), Some(texture.clone()));
                                Response::Loaded(texture)
                            }
                            Err(e) => {
                                // could not convert to image
                                tracing::warn!("converting buffer to Pixbuf: {e} for {id}");
                                self.covers.insert(id.into(), None);
                                Response::Empty
                            }
                        }
                    }
                    // id is missing or there is no buffer for id
                    _ => {
                        self.covers.insert(id.into(), None);
                        Response::Empty
                    }
                }
            }
        }
    }

    pub fn cover_icon(&mut self, id: &str) -> Option<gdk::Texture> {
        match self.cover_raw(id) {
            None => None,
            Some(buffer) => {
                match image::load_from_memory(&buffer) {
                    Err(e) => {
                        tracing::warn!("converting buffer to image: {e}");
                        None
                    }
                    Ok(image) => {
                        let thumb = image.thumbnail(32, 32);
                        let mut writer = Cursor::new(vec![]);
                        let color_type = image::ExtendedColorType::from(thumb.color());
                        if let Err(e) = image::write_buffer_with_format(&mut writer, thumb.as_bytes(), thumb.width(), thumb.height(), color_type, image::ImageFormat::Png) {
                            tracing::warn!("converting thumbnail to png: {e:?}");
                            return None;
                        }
                        let bytes = gtk::glib::Bytes::from(&writer.into_inner());
                        match gdk::Texture::from_bytes(&bytes) {
                            Ok(texture) => Some(texture),
                            Err(e) => {
                                tracing::warn!("converting buffer to icon: {e}");
                                None
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn save(&self) -> anyhow::Result<()> {
        let xdg_dirs = xdg::BaseDirectories::with_prefix(PREFIX)?;
        let cache: Vec<u8> = postcard::to_allocvec(self).unwrap();
        let cache_path = xdg_dirs
            .place_cache_file(COVER_CACHE)
            .expect("cannot create cache directory");
        std::fs::write(cache_path, cache).unwrap();

        Ok(())
    }

    fn load(&mut self) -> anyhow::Result<()> {
        let xdg_dirs = xdg::BaseDirectories::with_prefix(PREFIX)?;
        let cache_path = xdg_dirs
            .place_cache_file(COVER_CACHE)
            .expect("cannot create cache directory");
        let content = std::fs::read(cache_path)?;
        tracing::info!("loaded subsonic cover cache");
        let result = postcard::from_bytes::<Self>(&content)?;

        //convert buffers to textures
        for (id, buffer) in &result.buffers {
            match buffer {
                None => _ = self.covers.insert(id.into(), None),
                Some(buffer) => {
                    let bytes = gtk::glib::Bytes::from(buffer);
                    match gdk::Texture::from_bytes(&bytes) {
                        Ok(tex) => {
                            _ = self.covers.insert(id.into(), Some(tex));
                        }
                        Err(e) => {
                            tracing::warn!("error while converting buffer from cache: {e}");
                            self.covers.insert(id.into(), None);
                        }
                    }
                }
            }
        }
        println!("size of covers {}", self.covers.len());

        self.buffers = result.buffers;
        Ok(())
    }

    pub fn delete_cache(&self) -> anyhow::Result<()> {
        let xdg_dirs = xdg::BaseDirectories::with_prefix(PREFIX)?;
        let cache_path = xdg_dirs
            .place_cache_file(COVER_CACHE)
            .expect("cannot create cache directory");
        std::fs::remove_file(cache_path)?;
        Ok(())
    }
}
