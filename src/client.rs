use std::sync::{Mutex, OnceLock};

use crate::settings::Settings;

#[derive(Debug, Default)]
pub struct Client {
    pub inner: Option<submarine::Client>,
}

// used singleton from https://stackoverflow.com/questions/27791532/how-do-i-create-a-global-mutable-singleton
impl Client {
    pub fn get() -> &'static Mutex<Client> {
        static CLIENT: OnceLock<Mutex<Client>> = OnceLock::new();
        match CLIENT.get() {
            Some(client) => return client,
            None => {
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
                        client_name: String::from("Bouy"),
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
}
