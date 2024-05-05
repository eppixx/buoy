use serde::{Deserialize, Serialize};

use std::{
    io::Read,
    sync::{Mutex, OnceLock},
};

use crate::client::Client;

const PREFIX: &str = "Buoy";
const FILE_NAME: &str = "config.toml";

/// Stores all main settings of the window, queue and login information
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Settings {
    #[serde(default = "default_window_width")]
    pub window_width: i32,
    #[serde(default = "default_window_height")]
    pub window_height: i32,
    #[serde(default)] //defaults to false
    pub window_maximized: bool,
    #[serde(default = "default_paned_position")]
    pub paned_position: i32,

    #[serde(default)] //defaults to Vec::new()
    pub queue_ids: Vec<submarine::data::Child>,
    #[serde(default)] //defaults to None
    pub queue_current: Option<usize>,
    #[serde(default)] //defaults to 0.0f64
    pub queue_seek: f64,

    #[serde(default)] //defaults to None
    pub login_uri: Option<String>,
    #[serde(default)] //defaults to None
    pub login_username: Option<String>,
    #[serde(default)] //defaults to None
    pub login_hash: Option<String>,
    #[serde(default)] //defaults to None
    pub login_salt: Option<String>,

    #[serde(default = "default_volume")]
    pub volume: f64,

    #[serde(default)] //defaults to false
    pub equalizer_enabled: bool,
    #[serde(default)] //defaults to [0.0f64; 10]
    pub equalizer_bands: [f64; 10],
}

fn default_window_width() -> i32 {
    1200
}

fn default_window_height() -> i32 {
    900
}

fn default_paned_position() -> i32 {
    400
}

fn default_volume() -> f64 {
    0.75
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            window_width: 1200,
            window_height: 900,
            window_maximized: false,
            paned_position: 400,
            queue_ids: vec![],
            queue_current: None,
            queue_seek: 0.0f64,
            login_uri: Default::default(),
            login_username: Default::default(),
            login_hash: Default::default(),
            login_salt: Default::default(),
            volume: 0.75,
            equalizer_enabled: false,
            equalizer_bands: [0.0f64; 10],
        }
    }
}

// used singleton from https://stackoverflow.com/questions/27791532/how-do-i-create-a-global-mutable-singleton
impl Settings {
    pub fn get() -> &'static Mutex<Settings> {
        static SETTING: OnceLock<Mutex<Settings>> = OnceLock::new();
        let xdg_dirs = xdg::BaseDirectories::with_prefix(PREFIX).unwrap();
        let config_path = xdg_dirs
            .place_config_file(FILE_NAME)
            .expect("cannot create configuration directory");
        let mut config_file = match std::fs::File::open(&config_path) {
            Ok(file) => file,
            Err(_) => std::fs::File::create(config_path).unwrap(),
        };
        let mut content = String::new();
        config_file.read_to_string(&mut content).unwrap_or_default();
        let setting = toml::from_str::<Settings>(&content).unwrap_or_default();
        SETTING.get_or_init(|| Mutex::new(setting))
    }

    pub fn save(&self) {
        let settings = toml::to_string(self).unwrap();
        let xdg_dirs = xdg::BaseDirectories::with_prefix(PREFIX).unwrap();
        let config_path = xdg_dirs
            .place_config_file(FILE_NAME)
            .expect("cannot create configuration directory");
        std::fs::write(config_path, settings).unwrap();
    }

    pub fn reset_login(&mut self) {
        self.login_uri = None;
        self.login_username = None;
        self.login_hash = None;
        self.login_salt = None;
        self.save();
    }

    /// pings server with current settings and checks if they are correct
    pub async fn valid_login(&self) -> bool {
        if let Some(client) = Client::get() {
            // let ping = futures::executor::block_on(async move { client.ping().await });
            let ping = client.ping().await;
            tracing::error!("{ping:?}");
            ping.is_ok()
        } else {
            false
        }
    }
}
