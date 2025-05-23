use std::{collections::HashMap, io::Cursor};

use relm4::gtk::{self, gdk};
use serde::{Deserialize, Serialize};

use crate::client::Client;

const COVER_SIZE: Option<i32> = Some(200);
const PREFIX: &str = "Buoy";
const COVER_CACHE: &str = "cover-cache";
static CONCURRENT_COVER_RELOAD: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(50);

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct SubsonicCovers {
    /// the raw buffers that are send from server
    buffers: HashMap<String, Option<Vec<u8>>>,
}

#[derive(Default, Debug)]
pub enum Response {
    /// there is no image on server
    #[default]
    Empty,
    /// downloaded image from server
    Loaded(gdk::Texture),
    /// in the process of loading from server
    Processing(async_channel::Receiver<(String, Option<gdk::Texture>, Option<Vec<u8>>)>),
}

impl SubsonicCovers {
    pub fn cover_raw(&self, id: &str) -> Option<Vec<u8>> {
        match self.buffers.get(id) {
            None => None,
            Some(buffer) => buffer.clone(),
        }
    }

    pub fn cover(&mut self, id: &str) -> Response {
        match self.buffers.get(id) {
            Some(Some(buffer)) => {
                // converting buffer to image
                let bytes = gtk::glib::Bytes::from(buffer);
                match gdk::Texture::from_bytes(&bytes) {
                    Ok(texture) => Response::Loaded(texture),
                    Err(e) => {
                        // could not convert to image
                        tracing::warn!("converting buffer to Pixbuf: {e} for {id}");
                        Response::Empty
                    }
                }
            }
            // there is no buffer for id
            Some(None) => Response::Empty,
            // id is missing in cache
            None => {
                let client = Client::get().unwrap();
                let (sender, receiver) = async_channel::unbounded();
                let id = id.to_owned();

                let _handle = tokio::spawn(async move {
                    let permit = CONCURRENT_COVER_RELOAD.acquire().await.unwrap();
                    let buffer = match client.get_cover_art(&id, COVER_SIZE).await {
                        Ok(buffer) => buffer,
                        Err(e) => {
                            tracing::warn!("error fetching cover {id}: {e}");
                            return;
                        }
                    };
                    drop(permit);
                    let bytes = gtk::glib::Bytes::from(&buffer);
                    let (texture, buffer) = match gdk::Texture::from_bytes(&bytes) {
                        Ok(texture) => (Some(texture), Some(buffer)),
                        Err(e) => {
                            // could not convert to image
                            tracing::warn!("converting buffer to Pixbuf: {e} for {id}");
                            (None, None)
                        }
                    };
                    match sender.send((id, texture, buffer)).await {
                        Ok(()) => {}
                        Err(e) => tracing::error!("sending error: {e}"),
                    }
                });
                Response::Processing(receiver)
            }
        }
    }

    pub fn cover_icon(&self, id: &str) -> Option<gdk::Texture> {
        match self.cover_raw(id) {
            None => None,
            Some(buffer) => match image::load_from_memory(&buffer) {
                Err(e) => {
                    tracing::warn!("converting buffer to image: {e}");
                    None
                }
                Ok(image) => {
                    let thumb = image.thumbnail(32, 32);
                    let mut writer = Cursor::new(vec![]);
                    let color_type = image::ExtendedColorType::from(thumb.color());
                    if let Err(e) = image::write_buffer_with_format(
                        &mut writer,
                        thumb.as_bytes(),
                        thumb.width(),
                        thumb.height(),
                        color_type,
                        image::ImageFormat::Png,
                    ) {
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
            },
        }
    }

    pub fn cover_update(&mut self, id: &str, buffer: Option<Vec<u8>>) {
        self.buffers.insert(String::from(id), buffer);
    }

    pub fn save(&self) -> anyhow::Result<()> {
        let cache: Vec<u8> = postcard::to_allocvec(self)?;

        let cache_path = dirs::cache_dir()
            .ok_or(std::io::Error::other("cant create cache dir"))?
            .join(PREFIX)
            .join(COVER_CACHE);
        std::fs::write(cache_path, cache)?;

        Ok(())
    }

    pub fn load(&mut self) -> anyhow::Result<()> {
        let cache_path = dirs::cache_dir()
            .ok_or(std::io::Error::other("cant create cache dir"))?
            .join(PREFIX)
            .join(COVER_CACHE);
        let content = std::fs::read(cache_path)?;
        tracing::info!("loaded subsonic cover cache");
        let result = postcard::from_bytes::<Self>(&content)?;

        self.buffers = result.buffers;
        tracing::info!("count of loaded covers {}", self.buffers.len());
        Ok(())
    }

    pub fn delete_cache(&self) -> anyhow::Result<()> {
        let cache_path = dirs::cache_dir()
            .ok_or(std::io::Error::other("cant create cache dir"))?
            .join(PREFIX)
            .join(COVER_CACHE);
        std::fs::remove_file(cache_path)?;
        Ok(())
    }
}
