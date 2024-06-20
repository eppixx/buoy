use futures::StreamExt;
use relm4::gtk;
use serde::{Deserialize, Serialize};

use std::collections::HashMap;

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
    covers: HashMap<String, Option<gtk::gdk_pixbuf::Pixbuf>>,
}

#[derive(Default, Debug)]
pub enum Response {
    /// there is no image on server
    #[default]
    Empty,
    /// downloaded image from server
    Loaded(gtk::gdk_pixbuf::Pixbuf),
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
                        let stream = gtk::gio::MemoryInputStream::from_bytes(&bytes);
                        match gtk::gdk_pixbuf::Pixbuf::from_stream(
                            &stream,
                            gtk::gio::Cancellable::NONE,
                        ) {
                            Ok(pixbuf) => {
                                self.covers.insert(id.into(), Some(pixbuf.clone()));
                                Response::Loaded(pixbuf)
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
