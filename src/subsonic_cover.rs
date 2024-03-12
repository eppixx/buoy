use std::{
    collections::{HashMap, HashSet},
    sync::{
        mpsc::{Receiver, Sender},
        Arc, Mutex,
    },
    thread,
    time::Duration,
};

use futures::StreamExt;
use relm4::gtk;

use crate::{client::Client, types::Id};

const COVER_SIZE: Option<i32> = Some(200);
const CONCURRENT_FETCH: usize = 100;

#[derive(Default, Debug)]
pub struct SubsonicCovers {
    // the raw buffers that are send from server
    buffers: Arc<Mutex<HashMap<String, Option<Vec<u8>>>>>,
    // coverted from buffers, can be copied
    covers: HashMap<String, Option<gtk::gdk_pixbuf::Pixbuf>>,
    // requests
    requests: Arc<Mutex<HashSet<String>>>,
    // stored senders tat wait for response in form of a cover
    senders: Arc<Mutex<HashMap<String, Vec<Sender<Result<(), ()>>>>>>,
}

#[derive(Default, Debug)]
pub enum Response {
    // there is no image on server
    #[default]
    Empty,
    // downloaded image from server
    Loaded(gtk::gdk_pixbuf::Pixbuf),
    // currently requesting image from server
    InLoading(Receiver<Result<(), ()>>),
}

impl SubsonicCovers {
    pub async fn work(&self, start_requests: Option<Vec<String>>) {
        let requests = self.requests.clone();
        let buffers = self.buffers.clone();
        // let senders = self.senders.clone();

        if let Some(start_requests) = start_requests {
            for id in start_requests.into_iter() {
                requests.lock().unwrap().insert(id);
            }
        }


				let client = Client::get().unwrap();
				let ids = vec![
						String::from("mf-1b0e80b2e518b237700d9d0651077035_611688cd"),
						String::from("mf-52e172abed657b4d57a58ebbcc111257_611688cb"),
						String::from("mf-8c2cc7cb8f0932a075f2d16bb71307dc_611688ce"),
				];
				for id in ids {
						let cover = client.get_cover_art(&id, COVER_SIZE).await.unwrap();
						buffers
								.lock()
								.unwrap()
								.entry(id.clone())
								.and_modify(|buf| *buf = Some(cover.clone()))
								.or_insert(Some(cover.clone()));
				}


        // // handle hint taken from
        // // https://stackoverflow.com/questions/61292425/how-to-run-an-asynchronous-task-from-a-non-main-thread-in-tokio
        // let handle = tokio::runtime::Handle::current();
				// // tracing::error!("{handle:?}");
				// std::thread::spawn(move || {

				// // });
        // handle.block_on(async move {
				// 		let client = Client::get().unwrap();
				// 		tracing::error!("dsfsdfs");
				// 		let license =  client.get_license().await;
				// 		tracing::error!("{license:?} dsfsdfs");

        //     loop {
        //         // build queue of tasks and clear requests
        //         let mut tasks = vec![];
        //         {
        //             let mut requests = requests.lock().unwrap();
        //             for id in requests.iter() {
        //                 let id = id.clone();
        //                 tasks.push(async move {
        //                     let client = Client::get().unwrap();
        //                     tracing::info!("fetching cover: {}", &id);
        //                     let cover = client.get_cover_art(id.inner(), COVER_SIZE).await;
        //                     (id.clone(), cover)
        //                 });
        //             }
        //             requests.clear();
        //         }

        //         // work queue
        //         // based on: https://stackoverflow.com/questions/70871368/limiting-the-number-of-concurrent-futures-in-join-all
        //         tracing::error!("starting worker thread {}", tasks.len());
        //         let stream = futures::stream::iter(tasks)
        //             .buffer_unordered(CONCURRENT_FETCH)
        //             .collect::<Vec<_>>();
        //         let buffers = buffers.clone();
        //         let senders = senders.clone();
				// 				let handle = tokio::runtime::Handle::current();
				// 				handle.block_on(async move {
        //         // let join_handle = tokio::spawn(async move {
        //             let results = stream.await;

        //             for (id, cover) in results {
        //                 match &cover {
        //                     Ok(cover) => {
        //                         tracing::error!("found a cover");
        //                         buffers
        //                             .lock()
        //                             .unwrap()
        //                             .entry(id.clone())
        //                             .and_modify(|buf| *buf = Some(cover.clone()))
        //                             .or_insert(Some(cover.clone()));
        //                     }
        //                     Err(e) => {
        //                         tracing::error!("error in fetching cover {id}: {e} - retry");
        //                         // retry fetching
        //                         let client = Client::get().unwrap();
        //                         match client.get_cover_art(id.inner(), COVER_SIZE).await {
        //                             Ok(cover) => {
        //                                 buffers
        //                                     .lock()
        //                                     .unwrap()
        //                                     .entry(id.clone())
        //                                     .and_modify(|buf| *buf = Some(cover.clone()))
        //                                     .or_insert(Some(cover.clone()));
        //                             }
        //                             Err(e) => {
        //                                 tracing::error!(
        //                                     "refetching cover {id} resulted in error {e}"
        //                                 );
        //                             }
        //                         }
        //                     }
        //                 };
        //                 {
        //                     // send ok to receiver and clear senders
        //                     tracing::error!("send ok to receivers");
        //                     let mut senders = senders.lock().unwrap();
        //                     for sender in senders.get(&id).unwrap().iter() {
        //                         sender.send(Ok(()));
        //                     }
        //                     senders.remove(&id);
        //                 }
        //             }
        //         });
        //         // let _ = join_handle.await;
				// 						// });

        //         //wait for new requests
        //         thread::sleep(Duration::from_millis(200));
        //     }
        // });
				// });
				// let _ = join.await;
    }

    pub fn cover(&mut self, id: &str) -> Response {
				tracing::error!("request cover {id}");
				tracing::error!("covers stored: {:?}", self.covers);
        match self.covers.get(id) {
            Some(Some(cover)) => {
								tracing::error!("returned loaded cover");
								Response::Loaded(cover.clone())
						}
            Some(None) => Response::Empty,
            None => {
                // check if buffer exists
                match self.buffers.lock().unwrap().get(id) {
                    // a request was made and it hasn't been converted yet
                    Some(Some(buffer)) => {
												tracing::warn!("found buffer");
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
																tracing::error!("{:?}", self.covers);
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
                    // a request was send but nothing came back
                    Some(None) => {
                        self.covers.insert(id.into(), None);
                        Response::Empty
                    }
                    // there is no request made, doing it now
                    None => {
                        // create request
                        {
                            let mut requests = self.requests.lock().unwrap();
                            requests.insert(id.into());
                        }

                        // store sender
                        let (sender, receiver) = std::sync::mpsc::channel();
                        self.senders
                            .lock()
                            .unwrap()
                            .entry(id.into())
                            .and_modify(|senders| senders.push(sender.clone()))
                            .or_insert(vec![sender]);

                        // send receiver
                        Response::InLoading(receiver)
                    }
                }
            }
        }
    }
}
