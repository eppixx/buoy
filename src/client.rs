use std::sync::{Mutex, OnceLock};

use crate::{config, settings::Settings};

#[derive(Debug, Default)]
pub struct Client {
    pub inner: Option<submarine::Client>,
}

// used singleton from https://stackoverflow.com/questions/27791532/how-do-i-create-a-global-mutable-singleton
impl Client {
    pub fn get() -> Option<submarine::Client> {
        static CLIENT: OnceLock<Mutex<Client>> = OnceLock::new();
        if let Some(client) = CLIENT.get() {
            client.lock().unwrap().inner.clone()
        } else {
            let settings = Settings::get().lock().unwrap();
            if let (Some(uri), Some(user), Some(hash), Some(salt)) = (
                &settings.login_uri,
                &settings.login_username,
                &settings.login_hash,
                &settings.login_salt,
            ) {
                let auth = submarine::auth::Auth {
                    user: user.clone(),
                    version: String::from("0.16.1"),
                    client_name: String::from(config::APP_NAME),
                    hash: hash.clone(),
                    salt: salt.clone(),
                };
                let client = Client {
                    inner: Some(submarine::Client::new(uri, auth)),
                };
                let client = CLIENT.get_or_init(|| Mutex::new(client));
                client.lock().unwrap().inner.clone()
            } else {
                tracing::warn!("no login config found");
                // no settings for login found; show LoginForm
                None
            }
        }
    }

    pub fn get_mut() -> &'static Mutex<Client> {
        static CLIENT: OnceLock<Mutex<Client>> = OnceLock::new();
        if let Some(client) = CLIENT.get() {
            client
        } else {
            let settings = Settings::get().lock().unwrap();
            if let (Some(uri), Some(user), Some(hash), Some(salt)) = (
                &settings.login_uri,
                &settings.login_username,
                &settings.login_hash,
                &settings.login_salt,
            ) {
                let auth = submarine::auth::Auth {
                    user: user.clone(),
                    version: String::from("0.16.1"),
                    client_name: String::from(config::APP_NAME),
                    hash: hash.clone(),
                    salt: salt.clone(),
                };
                let client = Client {
                    inner: Some(submarine::Client::new(uri, auth)),
                };
                CLIENT.get_or_init(|| Mutex::new(client))
            } else {
                CLIENT.get_or_init(|| Mutex::new(Client::default()))
            }
        }
    }
}
