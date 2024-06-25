use zbus::{interface, Connection, Result};

use std::sync::{Arc, Mutex, RwLock};

struct Greeter {
    name: String,
    sender: async_channel::Sender<MprisOut>,
}

#[interface(name = "org.zbus.MyGreeter1")]
impl Greeter {
    async fn say_hello(&self, name: &str) -> String {
        format!("Hello {}!", name)
    }

    // Rude!
    async fn go_away(
        &self,
        #[zbus(signal_context)] ctxt: zbus::SignalContext<'_>,
    ) -> zbus::fdo::Result<()> {
        Self::greeted_everyone(&ctxt).await?;
        self.sender
            .try_send(MprisOut::DisplayToast(String::from("hello from mpris")))
            .unwrap();

        Ok(())
    }

    /// A "GreeterName" property.
    #[zbus(property)]
    async fn greeter_name(&self) -> &str {
        &self.name
    }

    /// A setter for the "GreeterName" property.
    ///
    /// Additionally, a `greeter_name_changed` method has been generated for you if you need to
    /// notify listeners that "GreeterName" was updated. It will be automatically called when
    /// using this setter.
    #[zbus(property)]
    async fn set_greeter_name(&mut self, name: String) {
        self.name = name;
    }

    /// A signal; the implementation is provided by the macro.
    #[zbus(signal)]
    async fn greeted_everyone(ctxt: &zbus::SignalContext<'_>) -> Result<()>;
}

#[derive(Debug)]
pub struct Mpris {
    // root: zbus::Connection,
    greeter: zbus::Connection,
}

impl Mpris {
    pub async fn new(sender: &async_channel::Sender<MprisOut>) -> anyhow::Result<Mpris> {
        let root = Root::new(sender).await?;
        // let root_connection = zbus::conn::Builder::session()?
        //     .name("org.mpris.MediaPlayer2.buoy")?
        //     .serve_at("/org/mpris/MediaPlayer2/buoy", root)?
        //     .build()
        //     .await?;

        // let mpris = Mpris {
        //     root: root_connection,
        // };
        let greeter = Greeter {
            name: "GreeterName".to_string(),
            sender: sender.clone(),
        };
        let connection = zbus::conn::Builder::session()?
            .name("org.mpris.MediaPlayer2.buoy")?
            .serve_at("/org/mpris/MediaPlayer2", greeter)?
            .build()
            .await?;

        // Ok(Mpris { root: connection, })
        Ok(Mpris {
            greeter: connection,
        })
    }
}

pub struct Root {
    sender: async_channel::Sender<MprisOut>,
}

#[derive(Debug)]
pub enum MprisOut {
    WindowRaise,
    WindowQuit,
    DisplayToast(String),
    Next,
    Previous,
    Pause,
    PlayPause,
    Play,
    Stop,
}

impl Root {
    async fn new(sender: &async_channel::Sender<MprisOut>) -> anyhow::Result<Self> {
        let result = Self {
            sender: sender.clone(),
        };

        Ok(result)
    }
}

#[interface(name = "org.mpris.MediaPlayer2")]
impl Root {
    fn raise(&self) {
        self.sender.try_send(MprisOut::WindowRaise).unwrap();
    }

    fn quit(&self) {
        self.sender.try_send(MprisOut::WindowQuit).unwrap();
    }

    #[dbus_interface(property)]
    fn can_quit(&self) -> bool {
        true
    }

    #[dbus_interface(property)]
    fn fullscreen(&self) -> bool {
        false
    }

    #[dbus_interface(property)]
    fn can_set_fullscreen(&self) -> bool {
        false
    }

    #[dbus_interface(property)]
    fn can_raise(&self) -> bool {
        true
    }

    #[dbus_interface(property)]
    fn has_track_list(&self) -> bool {
        true
    }

    #[dbus_interface(property)]
    fn identity(&self) -> &str {
        "buoy"
    }

    #[dbus_interface(property)]
    fn supported_uri_schemes(&self) -> Vec<&str> {
        vec![]
    }

    #[dbus_interface(property)]
    fn supported_mime_types(&self) -> Vec<&str> {
        vec![]
    }
}

struct Player {
    sender: async_channel::Sender<MprisOut>,
}

#[interface(name = "org.mpris.MediaPlayer2.Player")]
impl Player {
    fn next(&self) {
        self.sender.try_send(MprisOut::Next).unwrap();
    }

    fn previous(&self) {
        self.sender.try_send(MprisOut::Previous).unwrap();
    }

    fn pause(&self) {
        self.sender.try_send(MprisOut::Pause).unwrap();
    }

    fn play_pause(&self) {
        self.sender.try_send(MprisOut::PlayPause).unwrap();
    }

    fn play(&self) {
        self.sender.try_send(MprisOut::Play).unwrap();
    }

    fn stop(&self) {
        self.sender.try_send(MprisOut::Stop).unwrap();
    }

    // fn seek(&self, offset: i32) {
    //     self.sender
    //         .try_send(app_action::AppAction::Queue(
    //             app_action::QueueRequest::SeekOffset(offset),
    //         ))
    //         .unwrap();
    // }

    // fn set_position(&self, _index: i32, pos: i32) {
    //     //TODO implement changing index
    //     self.sender
    //         .try_send(app_action::AppAction::Queue(
    //             app_action::QueueRequest::Seek(pos),
    //         ))
    //         .unwrap();
    // }

    // fn open_uri(&self, _uri: &str) {}

    // //Playing, Paused, Stopped
    // #[dbus_interface(property)]
    // pub fn playback_status(&self) -> zvariant::Value {
    //     zvariant::Value::new(String::from(&self.settings.read().unwrap().queue_state))
    // }

    // //None, Track, Playlist
    // #[dbus_interface(property)]
    // pub fn loop_status(&self) -> zvariant::Value {
    //     zvariant::Value::new(String::from(self.settings.read().unwrap().queue_repeat()))
    // }

    // #[dbus_interface(property)]
    // fn set_loop_status(&mut self, loop_status: &str) {
    //     let mut settings = self.settings.write().unwrap();
    //     settings.set_queue_repeat(repeat::Repeat::from(loop_status));
    // }

    // //playback speed; 1.0 is normal speed, 0.5 is half speed
    // #[dbus_interface(property)]
    // fn rate(&self) -> f64 {
    //     1.0
    // }

    // //dont support changing playback speed
    // #[dbus_interface(property)]
    // fn set_rate(&mut self, _rate: f64) {}

    // #[dbus_interface(property)]
    // pub fn shuffle(&self) -> zvariant::Value {
    //     let settings = self.settings.read().unwrap();
    //     match settings.queue_random() {
    //         shuffle::Shuffle::Sequential => zvariant::Value::new(false),
    //         shuffle::Shuffle::Random => zvariant::Value::new(true),
    //     }
    // }

    // #[dbus_interface(property)]
    // fn set_shuffle(&mut self, shuffle: bool) {
    //     let mut settings = self.settings.write().unwrap();
    //     let random = match shuffle {
    //         true => shuffle::Shuffle::Random,
    //         false => shuffle::Shuffle::Sequential,
    //     };
    //     settings.set_queue_random(random);
    // }

    // #[dbus_interface(property)]
    // pub fn metadata(&self) -> zvariant::Value {
    //     let mut map = HashMap::new();
    //     let track = self.playing_track.lock().unwrap();
    //     if let Some(track) = &*track {
    //         use zvariant::Value;
    //         map.insert("mpris:trackid", Value::new(String::from(track.id())));
    //         // from sec to ms
    //         map.insert("mpris:length", Value::new(track.length() * 1000));
    //         map.insert("xesam:title", Value::new(String::from(track.title())));
    //         if let Some(album) = &track.album() {
    //             map.insert("xesam:album", Value::new(String::from(album)));
    //         }
    //         if let Some(artist) = &track.artist() {
    //             map.insert("xesam:albumArtist", Value::new(String::from(artist)));
    //         }
    //         if let Some(artist) = &track.artist() {
    //             map.insert("xesam:artist", Value::new(vec![String::from(artist)]));
    //         }
    //         if let Some(number) = &track.disc_number() {
    //             map.insert("xesam:discNumber", zvariant::Value::new(*number));
    //         }
    //         if let Some(number) = &track.track_number() {
    //             map.insert("xesam:trackNumber", zvariant::Value::new(*number));
    //         }
    //         if let Some(url) = &self.cover_url {
    //             map.insert("mpris:artUrl", Value::new(url));
    //         }
    //         //TODO
    //         // map.insert("xesam:useCount", zvariant::Value::new(5));
    //         // map.insert("xesam:genre", zvariant::Value::new(vec!["Blues", "Phonk"]));
    //     }

    //     zvariant::Value::new(map)
    // }

    // //ranges from 0.0 to 1.0
    // #[dbus_interface(property)]
    // pub fn volume(&self) -> zvariant::Value {
    //     zvariant::Value::new(self.volume)
    // }

    // #[dbus_interface(property)]
    // pub fn set_volume(&mut self, volume: f64) {
    //     self.settings.write().unwrap().set_volume(volume);
    // }

    // //time im mircoseconds
    // #[dbus_interface(property)]
    // fn position(&self) -> zvariant::Value {
    //     // TODO
    //     zvariant::Value::new(5000)
    // }

    // #[dbus_interface(property)]
    // fn minimum_rate(&self) -> zvariant::Value {
    //     zvariant::Value::new(1.0f64)
    // }

    // #[dbus_interface(property)]
    // fn maximum_rate(&self) -> zvariant::Value {
    //     zvariant::Value::new(1.0f64)
    // }

    // #[dbus_interface(property)]
    // pub fn can_go_next(&self) -> zvariant::Value {
    //     zvariant::Value::new(!self.queue_empty)
    // }

    // #[dbus_interface(property)]
    // pub fn can_go_previous(&self) -> zvariant::Value {
    //     zvariant::Value::new(!self.queue_empty)
    // }

    // #[dbus_interface(property)]
    // pub fn can_play(&self) -> zvariant::Value {
    //     let play = match self.settings.read().unwrap().queue_state {
    //         queue_state::State::Pause(_) => true,
    //         queue_state::State::Stop if self.queue_empty => true,
    //         _ => false,
    //     };
    //     zvariant::Value::new(play)
    // }

    // #[dbus_interface(property)]
    // pub fn can_pause(&self) -> zvariant::Value {
    //     if let queue_state::State::Play(_) = self.settings.read().unwrap().queue_state {
    //         return zvariant::Value::new(true);
    //     }

    //     zvariant::Value::new(false)
    // }

    #[dbus_interface(property)]
    pub fn can_seek(&self) -> zvariant::Value {
        zvariant::Value::new(false)
    }

    #[dbus_interface(property)]
    fn can_control(&self) -> zvariant::Value {
        zvariant::Value::new(true)
    }

    // #[dbus_interface(signal)]
    // pub async fn seeked(&self, ctxt: &zbus::SignalContext<'_>) -> zbus::Result<()> {
    //     Ok(())
    //     // TODO
    // }
}
