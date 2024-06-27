use std::collections::HashMap;

use zbus::interface;

#[derive(Debug)]
pub struct Mpris {
    _connection: zbus::Connection,
}

impl Mpris {
    pub async fn new(sender: &async_channel::Sender<MprisOut>) -> anyhow::Result<Mpris> {
        let root = Root::new(&sender.clone());
        let player = Player::new(sender);
        let connection = zbus::conn::Builder::session()?
            .name("org.mpris.MediaPlayer2.buoy")?
            .serve_at("/org/mpris/MediaPlayer2", root)?
            .serve_at("/org/mpris/MediaPlayer2", player)?
            .build()
            .await?;
        Ok(Mpris { _connection: connection, })
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
    Player(Command),
}

impl Root {
    fn new(sender: &async_channel::Sender<MprisOut>) -> Self {
        Self {
            sender: sender.clone(),
        }
    }
}

// implements https://specifications.freedesktop.org/mpris-spec/latest/Media_Player.html
#[interface(name = "org.mpris.MediaPlayer2")]
impl Root {
    fn raise(&self) {
        self.sender.try_send(MprisOut::WindowRaise).unwrap();
    }

    fn quit(&self) {
        self.sender.try_send(MprisOut::WindowQuit).unwrap();
    }

    #[zbus(property)]
    fn can_quit(&self) -> bool {
        true
    }

    #[zbus(property)]
    fn can_raise(&self) -> bool {
        true
    }

    #[zbus(property)]
    fn has_track_list(&self) -> bool {
        true
    }

    #[zbus(property)]
    fn identity(&self) -> &str {
        "Buoy"
    }

    #[zbus(property)]
    fn desktop_entry(&self) -> &str {
        "buoy"
    }

    #[zbus(property)]
    fn supported_uri_schemes(&self) -> Vec<&str> {
        vec![]
    }

    #[zbus(property)]
    fn supported_mime_types(&self) -> Vec<&str> {
        vec![]
    }
}

struct Player {
    sender: async_channel::Sender<MprisOut>,
}

impl Player {
    fn new(sender: &async_channel::Sender<MprisOut>) -> Self {
        Self {
            sender: sender.clone(),
        }
    }
}

// implementes https://specifications.freedesktop.org/mpris-spec/latest/Player_Interface.html
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

    fn stop(&self) {
        self.sender
            .try_send(MprisOut::Player(Command::Stop))
            .unwrap();
    }

    fn play(&self) {
        self.sender.try_send(MprisOut::Play).unwrap();
    }

    /// * `offset` - Position relative to current position to seek to in microseconds
    fn seek(&self, offset: i32) {
        // self.sender.try_send(MprisOut::Seek(offset)).unwrap();
        //TODO
    }

    /// * `index` - Index id of the track to set to
    /// * `pos` - Position to seek to in micoseconds
    fn set_position(&self, index: i32, pos: i32) {
        // self.sender.try_send(MprisOut::SetSeekPosition(index, pos)).unwrap()
        //TODO
    }

    fn open_uri(&self, _uri: &str) {}

    //Playing, Paused, Stopped
    #[zbus(property)]
    pub fn playback_status(&self) -> zvariant::Value {
        // zvariant::Value::new(String::from(&self.settings.read().unwrap().queue_state))
        //TODO
        zvariant::Value::new("Playing")
    }

    //None, Track, Playlist
    #[zbus(property)]
    pub fn loop_status(&self) -> zvariant::Value {
        // zvariant::Value::new(String::from(self.settings.read().unwrap().queue_repeat()))
        //TODO
        zvariant::Value::new(false)
    }

    #[zbus(property)]
    fn set_loop_status(&mut self, loop_status: &str) {
        // zvariant::Value::new(String::from(self.settings.read().unwrap().queue_repeat()))
        //TODO
    }

    //playback speed; 1.0 is normal speed, 0.5 is half speed
    #[zbus(property)]
    fn rate(&self) -> f64 {
        1.0
    }

    #[zbus(property)]
    fn set_rate(&self, _rate: f64) {}

    #[zbus(property)]
    pub fn shuffle(&self) -> zvariant::Value {
        //TODO
        zvariant::Value::new(false)
    }

    #[zbus(property)]
    fn set_shuffle(&mut self, shuffle: bool) {
        // let mut settings = self.settings.write().unwrap();
        // let random = match shuffle {
        //     true => shuffle::Shuffle::Random,
        //     false => shuffle::Shuffle::Sequential,
        // };
        // settings.set_queue_random(random);
        //TODO
    }

    #[zbus(property)]
    pub fn metadata(&self) -> zvariant::Value {
        let mut map = HashMap::new();
        // let track = self.playing_track.lock().unwrap();
        // if let Some(track) = &*track {
        //     use zvariant::Value;
        //     map.insert("mpris:trackid", Value::new(String::from(track.id())));
        //     // from sec to ms
        //     map.insert("mpris:length", Value::new(track.length() * 1000));
        //     map.insert("xesam:title", Value::new(String::from(track.title())));
        //     if let Some(album) = &track.album() {
        //         map.insert("xesam:album", Value::new(String::from(album)));
        //     }
        //     if let Some(artist) = &track.artist() {
        //         map.insert("xesam:albumArtist", Value::new(String::from(artist)));
        //     }
        //     if let Some(artist) = &track.artist() {
        //         map.insert("xesam:artist", Value::new(vec![String::from(artist)]));
        //     }
        //     if let Some(number) = &track.disc_number() {
        //         map.insert("xesam:discNumber", zvariant::Value::new(*number));
        //     }
        //     if let Some(number) = &track.track_number() {
        //         map.insert("xesam:trackNumber", zvariant::Value::new(*number));
        //     }
        //     if let Some(url) = &self.cover_url {
        //         map.insert("mpris:artUrl", Value::new(url));
        //     }
        //     //TODO
        //     // map.insert("xesam:useCount", zvariant::Value::new(5));
        //     // map.insert("xesam:genre", zvariant::Value::new(vec!["Blues", "Phonk"]));
        // }

        //TODO
        map.insert("xesam:title", zvariant::Value::new(String::from("Test title")));
        zvariant::Value::new(map)
    }

    //ranges from 0.0 to 1.0
    #[dbus_interface(property)]
    pub fn volume(&self) -> zvariant::Value {
        // zvariant::Value::new(self.volume)
        zvariant::Value::new(1.0f64)
    }

    #[dbus_interface(property)]
    pub fn set_volume(&mut self, volume: f64) {
        // self.settings.write().unwrap().set_volume(volume);
        //TODO
    }

    //time im mircoseconds
    #[dbus_interface(property)]
    fn position(&self) -> zvariant::Value {
        // TODO
        zvariant::Value::new(5000)
    }

    #[dbus_interface(property)]
    fn minimum_rate(&self) -> zvariant::Value {
        zvariant::Value::new(1.0f64)
    }

    #[dbus_interface(property)]
    fn maximum_rate(&self) -> zvariant::Value {
        zvariant::Value::new(1.0f64)
    }

    #[dbus_interface(property)]
    pub fn can_go_next(&self) -> zvariant::Value {
        // TODO
        zvariant::Value::new(true)
    }

    #[dbus_interface(property)]
    pub fn can_go_previous(&self) -> zvariant::Value {
        // TODO
        zvariant::Value::new(true)
    }

    #[dbus_interface(property)]
    pub fn can_play(&self) -> zvariant::Value {
        // TODO
        // let play = match self.settings.read().unwrap().queue_state {
        //     queue_state::State::Pause(_) => true,
        //     queue_state::State::Stop if self.queue_empty => true,
        //     _ => false,
        // };
        // zvariant::Value::new(play)
        zvariant::Value::new(true)
    }

    #[dbus_interface(property)]
    pub fn can_pause(&self) -> zvariant::Value {
        // TODO
        // if let queue_state::State::Play(_) = self.settings.read().unwrap().queue_state {
        //     return zvariant::Value::new(true);
        // }

        zvariant::Value::new(false)
    }

    #[dbus_interface(property)]
    pub fn can_seek(&self) -> zvariant::Value {
        // TODO
        zvariant::Value::new(false)
    }

    #[dbus_interface(property)]
    fn can_control(&self) -> zvariant::Value {
        // TODO
        zvariant::Value::new(true)
    }
}
