use std::{
    collections::HashMap,
    sync::{mpsc::Receiver, Arc, Mutex},
};

use futures::StreamExt;
use relm4::gtk;

use crate::client::Client;

const COVER_SIZE: Option<i32> = Some(200);
const CONCURRENT_FETCH: usize = 300;

#[derive(Default, Debug)]
pub struct SubsonicCovers {
    // the raw buffers that are send from server
    buffers: Arc<Mutex<HashMap<String, Option<Vec<u8>>>>>,
    // coverted from buffers, can be copied
    covers: HashMap<String, Option<gtk::gdk_pixbuf::Pixbuf>>,
}

#[derive(Default, Debug)]
pub enum Response {
    // there is no image on server
    #[default]
    Empty,
    // downloaded image from server
    Loaded(gtk::gdk_pixbuf::Pixbuf),
}

impl SubsonicCovers {
    pub async fn work(&self, start_requests: Vec<String>) {
        let buffers = self.buffers.clone();

        //build futures
        let tasks: Vec<_> = start_requests
            .iter()
            .map(|id| async move {
                let client = Client::get().unwrap();
                let cover = client.get_cover_art(id, COVER_SIZE).await.unwrap();
                (id, cover)
            })
            .collect();

        //buffer futures to not overwhelm server and client
        // based on: https://stackoverflow.com/questions/70871368/limiting-the-number-of-concurrent-futures-in-join-all
        let stream = futures::stream::iter(tasks)
            .buffer_unordered(CONCURRENT_FETCH)
            .collect::<Vec<_>>();
        let results = stream.await;

        for (id, cover) in results {
            buffers
                .lock()
                .unwrap()
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
                match self.buffers.lock().unwrap().get(id) {
                    Some(Some(buffer)) => {
                        // converting buffer to image
                        let bytes = gtk::glib::Bytes::from(buffer);
                        let stream = gtk::gio::MemoryInputStream::from_bytes(&bytes);
                        match gtk::gdk_pixbuf::Pixbuf::from_stream(
                            &stream,
                            gtk::gio::Cancellable::NONE,
                        ) {
                            Ok(pixbuf) => {
                                tracing::error!("loaded cached cover {id}");
                                self.covers.insert(id.into(), Some(pixbuf.clone()));
                                Response::Loaded(pixbuf)
                            }
                            Err(e) => {
                                // could not convert to image
                                tracing::error!("converting buffer to Pixbuf: {e}");
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
}
