use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use std::{collections::HashMap, io::Read, sync::OnceLock};

use crate::client::Client;

const PREFIX: &str = "Bouy";
const FILE_NAME: &str = "images";

#[derive(Debug, Default, Deserialize, Serialize, Clone)]
pub struct Cache {
    pub covers: HashMap<String, Vec<u8>>,
}

impl Cache {
    pub async fn cover(&mut self, id: &str) -> Option<Vec<u8>> {
        //TODO remove clone
        if let Some(buffer) = self.covers.get(id) {
            return Some(buffer.clone());
        }

        let client = Client::get().lock().unwrap().inner.clone().unwrap();
        match client.get_cover_art(id, Some(200)).await {
            Ok(buffer) => {
                self.covers.insert(id.to_string(), buffer.clone());
                self.save(); // TODO save at closing app
                Some(self.covers.get(id).unwrap().clone())
            }
            Err(_) => None, // TODO error handling
        }
    }

    pub fn save(&self) {
        let cache = postcard::to_allocvec(self).unwrap();
        let xdg_dirs = xdg::BaseDirectories::with_prefix(PREFIX).unwrap();
        let cache_path = xdg_dirs
            .place_cache_file(FILE_NAME)
            .expect("cannot create cache directory");
        std::fs::write(cache_path, cache).unwrap();
    }

    pub fn remove() {
        let xdg_dirs = xdg::BaseDirectories::with_prefix(PREFIX).unwrap();
        if let Some(cache_path) = xdg_dirs.find_cache_file(FILE_NAME) {
            std::fs::remove_file(cache_path).unwrap();
        }
    }

    // used singleton from https://stackoverflow.com/questions/27791532/how-do-i-create-a-global-mutable-singleton
    pub fn get() -> &'static Mutex<Cache> {
        static CACHE: OnceLock<Mutex<Cache>> = OnceLock::new();

        match CACHE.get() {
            Some(cache) => cache,
            None => {
                let xdg_dirs = xdg::BaseDirectories::with_prefix(PREFIX).unwrap();
                let cache_path = xdg_dirs
                    .place_cache_file(FILE_NAME)
                    .expect("cannot create configuration directory");

                let cache = match std::fs::File::open(cache_path) {
                    Ok(mut file) => {
                        // load file content
                        let mut content = vec![];
                        file.read_to_end(&mut content).unwrap();
                        postcard::from_bytes::<Cache>(&content).unwrap_or_default()
                    }
                    Err(_) => Cache::default(),
                };
                return CACHE.get_or_init(|| Mutex::new(cache));
            }
        }
    }
}
