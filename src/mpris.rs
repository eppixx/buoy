use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use zbus::interface;

use crate::Args;
use crate::{
    client::Client,
    components::sequence_button_impl::{repeat::Repeat, shuffle::Shuffle},
    play_state::PlayState,
    player::Command,
};

/// converts the internally used values to the mpris used ones and vice versa
pub trait MprisString {
    fn to_mpris_string(&self) -> String;
    /// on unusable input it defaults to Normal
    fn from_mpris_string(value: impl AsRef<str>) -> Self;
}

#[derive(Debug, Default)]
struct Info {
    can_next: bool,
    can_previous: bool,
    can_play: bool,
    volume: f64,
    state: PlayState,
    song: Option<submarine::data::Child>,
    loop_status: Repeat,
    shuffle: Shuffle,
    song_position: i64, // in microseconds
}

#[derive(Debug)]
pub struct Mpris {
    info: Arc<Mutex<Info>>,
    sender: async_channel::Sender<DataChanged>,
}

enum DataChanged {
    Metadata,
    Playback,
    CanPlayNext,
    CanPlayPrev,
    CanPlay,
    Volume,
    Repeat,
    Shuffle,
    SongPosition,
}

impl Mpris {
    pub async fn new(
        args: &Rc<RefCell<Args>>,
    ) -> anyhow::Result<(Mpris, async_channel::Receiver<MprisOut>)> {
        let (sender, receiver) = async_channel::unbounded();
        let info = Arc::new(Mutex::new(Info::default()));
        let root = Root {
            sender: sender.clone(),
        };
        let player = Player {
            sender: sender.clone(),
            info: info.clone(),
        };
        let connection = zbus::conn::Builder::session()?
            .name(format!(
                "org.mpris.MediaPlayer2.{}",
                args.borrow().alternative_title
            ))?
            .serve_at("/org/mpris/MediaPlayer2", root)?
            .serve_at("/org/mpris/MediaPlayer2", player)?
            .build()
            .await?;

        let server = connection.object_server();
        let interface = server
            .interface::<_, Player>("/org/mpris/MediaPlayer2")
            .await?;

        let (sender, rec) = async_channel::unbounded();
        relm4::gtk::glib::spawn_future_local(async move {
            let interface_ref = interface.get().await;
            let ctx = interface.signal_emitter();
            while let Ok(msg) = rec.recv().await {
                let result = match msg {
                    DataChanged::Metadata => interface_ref.metadata_changed(ctx).await,
                    DataChanged::Playback => interface_ref.playback_status_changed(ctx).await,
                    DataChanged::CanPlayNext => interface_ref.can_go_next_changed(ctx).await,
                    DataChanged::CanPlayPrev => interface_ref.can_go_previous_changed(ctx).await,
                    DataChanged::CanPlay => interface_ref.can_play_changed(ctx).await,
                    DataChanged::Volume => interface_ref.volume_changed(ctx).await,
                    DataChanged::Repeat => interface_ref.loop_status_changed(ctx).await,
                    DataChanged::Shuffle => interface_ref.shuffle_changed(ctx).await,
                    DataChanged::SongPosition => interface_ref.position_changed(ctx).await,
                };
                if let Err(e) = result {
                    tracing::error!("error while interacting with dbus: {e:?}");
                }
            }
        });

        Ok((Mpris { info, sender }, receiver))
    }

    pub fn can_play_next(&mut self, state: bool) {
        self.info.lock().unwrap().can_next = state;
        self.sender.try_send(DataChanged::CanPlayNext).unwrap();
    }

    pub fn can_play_previous(&mut self, state: bool) {
        self.info.lock().unwrap().can_previous = state;
        self.sender.try_send(DataChanged::CanPlayPrev).unwrap();
    }

    pub fn can_play(&mut self, state: bool) {
        self.info.lock().unwrap().can_play = state;
        self.sender.try_send(DataChanged::CanPlay).unwrap();
    }

    pub fn set_volume(&mut self, volume: f64) {
        self.info.lock().unwrap().volume = volume;
        self.sender.try_send(DataChanged::Volume).unwrap();
    }

    pub fn set_state(&mut self, state: PlayState) {
        self.info.lock().unwrap().state = state;
        self.sender.try_send(DataChanged::Playback).unwrap();
    }

    pub async fn set_song(&mut self, song: Option<submarine::data::Child>) {
        self.info.lock().unwrap().song = song;
        self.sender.try_send(DataChanged::Metadata).unwrap();
    }

    pub fn set_loop_status(&mut self, repeat: Repeat) {
        self.info.lock().unwrap().loop_status = repeat;
        self.sender.try_send(DataChanged::Repeat).unwrap();
    }

    pub fn set_shuffle(&mut self, shuffle: Shuffle) {
        self.info.lock().unwrap().shuffle = shuffle;
        self.sender.try_send(DataChanged::Shuffle).unwrap();
    }

    pub fn set_position(&mut self, position: i64) {
        self.info.lock().unwrap().song_position = position;
        self.sender.try_send(DataChanged::SongPosition).unwrap();
    }
}

pub struct Root {
    sender: async_channel::Sender<MprisOut>,
}

#[derive(Debug)]
pub enum MprisOut {
    WindowQuit,
    Player(Command),
}

// implements https://specifications.freedesktop.org/mpris-spec/latest/Media_Player.html
#[interface(name = "org.mpris.MediaPlayer2")]
impl Root {
    fn raise(&self) {}

    fn quit(&self) {
        self.sender.try_send(MprisOut::WindowQuit).unwrap();
    }

    #[zbus(property)]
    fn can_quit(&self) -> bool {
        true
    }

    #[zbus(property)]
    fn can_raise(&self) -> bool {
        false
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
    info: Arc<Mutex<Info>>,
}

// implementes https://specifications.freedesktop.org/mpris-spec/latest/Player_Interface.html
#[interface(name = "org.mpris.MediaPlayer2.Player")]
impl Player {
    fn next(&self) {
        self.sender
            .try_send(MprisOut::Player(Command::Next))
            .unwrap();
    }

    fn previous(&self) {
        self.sender
            .try_send(MprisOut::Player(Command::Previous))
            .unwrap();
    }

    fn pause(&self) {
        self.sender
            .try_send(MprisOut::Player(Command::Stop))
            .unwrap();
    }

    fn play_pause(&self) {
        self.sender
            .try_send(MprisOut::Player(Command::PlayPause))
            .unwrap();
    }

    fn stop(&self) {
        self.sender
            .try_send(MprisOut::Player(Command::Stop))
            .unwrap();
    }

    fn play(&self) {
        self.sender
            .try_send(MprisOut::Player(Command::Play))
            .unwrap();
    }

    /// * `offset` - Position relative to current position to seek to in microseconds
    fn seek(&self, offset: i64) {
        self.info.lock().unwrap().song_position += offset;
        self.sender
            .try_send(MprisOut::Player(Command::SetSongPosition(
                self.info.lock().unwrap().song_position,
            )))
            .unwrap();
    }

    /// * `index` - Index id of the track to set to
    /// * `pos` - Position to seek to in micoseconds
    fn set_position(&self, index: i32, pos: i64) {
        //TODO check index
        self.info.lock().unwrap().song_position = pos;
        self.sender
            .try_send(MprisOut::Player(Command::SetSongPosition(pos)))
            .unwrap();
    }

    fn open_uri(&self, _uri: &str) {}

    #[zbus(property)]
    pub fn playback_status(&self) -> zvariant::Value {
        zvariant::Value::new(self.info.lock().unwrap().state.to_mpris_string())
    }

    //None, Track, Playlist
    #[zbus(property)]
    pub fn loop_status(&self) -> zvariant::Value {
        zvariant::Value::new(self.info.lock().unwrap().loop_status.to_mpris_string())
    }

    #[zbus(property)]
    fn set_loop_status(&mut self, loop_status: &str) {
        let repeat = Repeat::from_mpris_string(loop_status);
        self.info.lock().unwrap().loop_status = repeat.clone();
        self.sender
            .try_send(MprisOut::Player(Command::Repeat(repeat)))
            .unwrap();
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
        let shuffle = self.info.lock().unwrap().shuffle == Shuffle::Shuffle;
        zvariant::Value::new(shuffle)
    }

    #[zbus(property)]
    fn set_shuffle(&mut self, shuffle: bool) {
        let shuffle = if shuffle {
            Shuffle::Shuffle
        } else {
            Shuffle::Sequential
        };
        self.info.lock().unwrap().shuffle = shuffle.clone();
        self.sender
            .try_send(MprisOut::Player(Command::Shuffle(shuffle)))
            .unwrap();
    }

    /// specifications: https://www.freedesktop.org/wiki/Specifications/mpris-spec/metadata/
    #[zbus(property)]
    pub fn metadata(&self) -> zvariant::Value {
        let mut map = HashMap::new();
        if let Some(song) = &self.info.lock().unwrap().song {
            use zvariant::Value;
            map.insert("mpris:trackid", Value::new(String::from(&song.id)));
            map.insert("xesam:title", Value::new(String::from(&song.title)));
            if let Some(duration) = song.duration {
                // from sec to ms
                map.insert("mpris:length", Value::new(duration));
            }
            if let Some(artist) = &song.artist {
                map.insert("xesam:albumArtist", Value::new(String::from(artist)));
            }
            if let Some(album) = &song.album {
                map.insert("xesam:album", Value::new(String::from(album)));
            }
            if let Some(artist) = &song.artist {
                map.insert("xesam:artist", Value::new(vec![String::from(artist)]));
            }
            if let Some(number) = song.disc_number {
                map.insert("xesam:discNumber", Value::new(number));
            }
            if let Some(number) = song.track {
                map.insert("xesam:trackNumber", Value::new(number));
            }
            if let Some(id) = &song.cover_art {
                let client = Client::get().unwrap();
                if let Ok(url) = client.get_cover_art_url(id, Some(100)) {
                    map.insert("mpris:artUrl", Value::new(url.to_string()));
                }
            }
            if let Some(count) = song.play_count {
                map.insert("xesam:useCount", Value::new(count));
            }
        }
        zvariant::Value::new(map)
    }

    //ranges from 0.0 to 1.0
    #[zbus(property)]
    pub fn volume(&self) -> zvariant::Value {
        zvariant::Value::new(self.info.lock().unwrap().volume)
    }

    #[zbus(property)]
    pub fn set_volume(&mut self, volume: f64) {
        self.sender
            .try_send(MprisOut::Player(Command::Volume(volume)))
            .unwrap()
    }

    //time im microseconds
    #[zbus(property)]
    fn position(&self) -> zvariant::Value {
        zvariant::Value::new(self.info.lock().unwrap().song_position)
    }

    #[zbus(property)]
    fn minimum_rate(&self) -> zvariant::Value {
        zvariant::Value::new(1.0f64)
    }

    #[zbus(property)]
    fn maximum_rate(&self) -> zvariant::Value {
        zvariant::Value::new(1.0f64)
    }

    #[zbus(property)]
    pub fn can_go_next(&self) -> zvariant::Value {
        zvariant::Value::new(self.info.lock().unwrap().can_next)
    }

    #[zbus(property)]
    pub fn can_go_previous(&self) -> zvariant::Value {
        zvariant::Value::new(self.info.lock().unwrap().can_previous)
    }

    /// if a playable track is possible to play, not if a track is playing/pausing
    #[zbus(property)]
    pub fn can_play(&self) -> zvariant::Value {
        zvariant::Value::new(self.info.lock().unwrap().can_play)
    }

    #[zbus(property)]
    pub fn can_pause(&self) -> zvariant::Value {
        zvariant::Value::new(true)
    }

    #[zbus(property)]
    pub fn can_seek(&self) -> zvariant::Value {
        zvariant::Value::new(true)
    }

    #[zbus(property)]
    fn can_control(&self) -> zvariant::Value {
        zvariant::Value::new(true)
    }
}
