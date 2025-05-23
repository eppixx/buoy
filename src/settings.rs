use std::{
    io::Read,
    sync::{Mutex, OnceLock},
};

use serde::{Deserialize, Serialize};

use crate::{
    client::Client,
    components::sequence_button_impl::{repeat::Repeat, shuffle::Shuffle},
};

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

    #[serde(default)]
    pub queue_jump_to_new_song: bool, // jump to new song

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
    #[serde(default = "default_mute")]
    pub mute: bool,

    #[serde(default)]
    pub repeat: Repeat,
    #[serde(default)]
    pub shuffle: Shuffle,

    #[serde(default)]
    pub send_notifications: bool,

    #[serde(default)]
    pub queue_animations: bool,

    #[serde(default)]
    pub scrobble: bool,
    #[serde(default = "default_scrobble_threshold")]
    pub scrobble_threshold: u32,

    #[serde(default)] //defaults to false
    pub equalizer_enabled: bool,
    #[serde(default)] //defaults to [0.0f64; 10]
    pub equalizer_bands: [f64; 10],

    #[serde(skip)]
    pub search_active: bool,
    #[serde(default)]
    pub search_text: String,
    #[serde(default)]
    pub fuzzy_search: bool,
    #[serde(default)]
    pub case_sensitive: bool,

    #[serde(default = "default_cover_size")]
    pub cover_size: i32,

    #[serde(default = "default_download_warning_threshold")]
    pub download_warning_threshold: usize,

    #[serde(default = "default_drag_time_timeout")]
    pub drag_time_timeout_ms: u64,

    #[serde(default = "default_save_interval_secs")]
    pub save_interval_secs: u64,

    #[serde(default = "default_dashboard_line_items")]
    pub dashboard_line_items: usize,
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

fn default_mute() -> bool {
    true
}

fn default_scrobble_threshold() -> u32 {
    80
}

fn default_cover_size() -> i32 {
    150
}

fn default_download_warning_threshold() -> usize {
    100
}

fn default_drag_time_timeout() -> u64 {
    1000
}

fn default_save_interval_secs() -> u64 {
    120
}

fn default_dashboard_line_items() -> usize {
    10
}

// used singleton from https://stackoverflow.com/questions/27791532/how-do-i-create-a-global-mutable-singleton
impl Settings {
    pub fn get() -> &'static Mutex<Settings> {
        static SETTING: OnceLock<Mutex<Settings>> = OnceLock::new();
        if let Some(setting) = SETTING.get() {
            return setting;
        }
        let config_path = dirs::config_dir()
            .expect("cant create config dir")
            .join(PREFIX)
            .join(FILE_NAME);
        let mut config_file = match std::fs::File::open(&config_path) {
            Ok(file) => file,
            Err(_) => std::fs::File::create(config_path).expect("could not create config file"),
        };
        let mut content = String::new();
        config_file.read_to_string(&mut content).unwrap_or_default();
        tracing::info!("loaded settings from file or created default settings");
        let setting = toml::from_str::<Settings>(&content)
            .expect("not all members of Settings are defaulted");
        SETTING.get_or_init(|| Mutex::new(setting))
    }

    pub fn save(&self) -> anyhow::Result<()> {
        let config_path = dirs::config_dir()
            .ok_or(std::io::Error::other("cant create config dir"))?
            .join(PREFIX)
            .join(FILE_NAME);

        let settings = toml::to_string(self)?;
        std::fs::write(config_path, settings)?;
        Ok(())
    }

    pub fn reset_login(&mut self) -> anyhow::Result<()> {
        self.login_uri = None;
        self.login_username = None;
        self.login_hash = None;
        self.login_salt = None;
        self.save()?;
        Ok(())
    }

    pub fn login_set(&self) -> bool {
        self.login_uri.is_some()
            && self.login_username.is_some()
            && self.login_hash.is_some()
            && self.login_salt.is_some()
    }

    /// pings server with current settings and checks if they are correct
    pub async fn valid_login(&self) -> bool {
        if let Some(client) = Client::get() {
            let ping = client.ping().await;
            ping.is_ok()
        } else {
            false
        }
    }
}
