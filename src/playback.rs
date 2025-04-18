use std::{
    cell::RefCell,
    rc::Rc,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
};

use gstreamer::{self as gst, prelude::*};
use relm4::gtk;

use crate::{play_state::PlayState, settings::Settings};

#[derive(Debug)]
pub struct Playback {
    pipeline: gst::Pipeline,
    source: gst::Element,
    volume: gst::Element,
    equalizer: gst::Element,
    track_set: Arc<AtomicBool>,
    scrobbled: Arc<Mutex<Scrobbled>>, //needed for threshold
}

const TICK: u64 = 250; // update rate for Seekbar

#[derive(Debug)]
pub enum PlaybackOut {
    TrackEnd,
    SongPosition(i64), // in ms
    ScrobbleThresholdReached,
}

#[derive(Debug)]
pub enum Scrobbled {
    ScrobbleTriggered,
    SetTo(Option<gstreamer::format::Percent>),
}

impl Playback {
    pub fn new() -> anyhow::Result<(Self, async_channel::Receiver<PlaybackOut>)> {
        let (sender, receiver) = async_channel::unbounded();
        gst::init()?;

        // Create the empty pipeline
        let pipeline = gst::Pipeline::with_name("playback");
        // Create the elements
        let source = gst::ElementFactory::make_with_name("uridecodebin", Some("source"))?;
        source.set_property("download", true);
        let convert = gst::ElementFactory::make_with_name("audioconvert", Some("convert"))?;
        let volume = gst::ElementFactory::make_with_name("volume", Some("volume"))?;
        let equalizer =
            gst::ElementFactory::make_with_name("equalizer-10bands", Some("equalizer"))?;
        let sink = gst::ElementFactory::make_with_name("autoaudiosink", Some("sink"))?;
        let track_set = Arc::new(AtomicBool::new(false));
        let scrobbled = Arc::new(Mutex::new(Scrobbled::SetTo(None)));

        // build the pipeline
        pipeline.add_many([&source, &convert, &volume, &equalizer, &sink])?;
        gst::Element::link_many([&convert, &volume, &equalizer, &sink])
            .expect("Elements could not be linked.");

        // Connect the pad-added signal
        let conv = convert;
        source.connect_pad_added(move |_, src_pad| {
            let sink_pad = conv
                .static_pad("sink")
                .expect("Failed to get static sink pad from convert");
            // if sink_pad.is_linked() {
            //     warn!("We are already linked. Ignoring.");
            //     return;
            // }

            let new_pad_caps = src_pad
                .current_caps()
                .expect("Failed to get caps of new pad.");
            let new_pad_struct = new_pad_caps
                .structure(0)
                .expect("Failed to get first structure of caps.");
            let new_pad_type = new_pad_struct.name();

            let is_audio = new_pad_type.starts_with("audio/x-raw");
            if !is_audio {
                tracing::warn!("audio is no raw type, but {new_pad_type} - ignoring.");
                return;
            }

            let res = src_pad.link(&sink_pad);
            if res.is_err() {
                tracing::error!("type is {new_pad_type} but link failed.");
            }
        });

        //check for pipline messages
        let send = sender.clone();
        let bus = pipeline.bus().unwrap();
        let track = track_set.clone();
        std::thread::spawn(move || {
            for msg in bus.iter_timed(gst::ClockTime::NONE) {
                use gstreamer::MessageView;
                if let MessageView::Eos(..) = msg.view() {
                    track.store(false, Ordering::Relaxed);
                    send.try_send(PlaybackOut::TrackEnd).unwrap();
                }
            }
        });

        //callback for seekbar
        let stamp = Rc::new(RefCell::new(pipeline.query_position::<gst::ClockTime>()));
        let pipeline_weak = pipeline.downgrade();
        let send = sender.clone();
        let scrobble = scrobbled.clone();
        gtk::glib::source::timeout_add_local(std::time::Duration::from_millis(TICK), move || {
            let Some(pipeline) = pipeline_weak.upgrade() else {
                return gtk::glib::ControlFlow::Continue;
            };

            //dont send messages when not playing a stream
            if pipeline.current_state() != gst::State::Playing {
                return gtk::glib::ControlFlow::Continue;
            }

            let current = pipeline.query_position::<gst::ClockTime>();
            // is not paused since last tick
            if current != *stamp.borrow() {
                let seconds = match current {
                    Some(clock) => clock.seconds() as i64,
                    None => 0,
                };
                send.try_send(PlaybackOut::SongPosition(seconds * 1000))
                    .unwrap();
                stamp.replace(current);

                let mut lock = scrobble.lock().unwrap();
                match *lock {
                    Scrobbled::ScrobbleTriggered => {}
                    Scrobbled::SetTo(None) => {
                        tracing::error!("Scrobble is set to None while playing song");
                    }
                    Scrobbled::SetTo(Some(time)) => {
                        if let Some(position) = pipeline.query_position::<gst::format::Percent>() {
                            if position.percent() - time.percent()
                                >= Settings::get().lock().unwrap().scrobble_threshold
                            {
                                *lock = Scrobbled::ScrobbleTriggered;
                                send.try_send(PlaybackOut::ScrobbleThresholdReached)
                                    .unwrap();
                            }
                        }
                    }
                }
                drop(lock);
            }

            gtk::glib::ControlFlow::Continue
        });

        let mut play = Self {
            pipeline,
            source,
            volume,
            equalizer,
            track_set,
            scrobbled,
        };

        play.sync_equalizer();
        play.sync_volume();
        Ok((play, receiver))
    }

    pub fn set_track(&mut self, uri: impl AsRef<str>) -> anyhow::Result<()> {
        self.stop()?;
        self.track_set.store(true, Ordering::Relaxed);
        self.source.set_property("uri", uri.as_ref());

        let mut lock = self.scrobbled.lock().unwrap();
        *lock = Scrobbled::SetTo(Some(0.percent()));

        Ok(())
    }

    pub fn is_track_set(&self) -> bool {
        self.track_set.load(Ordering::Relaxed)
    }

    pub fn play(&mut self) -> anyhow::Result<()> {
        self.pipeline.set_state(gst::State::Playing)?;
        Ok(())
    }

    pub fn pause(&mut self) -> anyhow::Result<()> {
        self.pipeline.set_state(gst::State::Paused)?;
        Ok(())
    }

    pub fn is_playing(&self) -> PlayState {
        match self.pipeline.state(None) {
            (_, gst::State::Playing, _) => PlayState::Play,
            (_, gst::State::Paused, _) => PlayState::Pause,
            _ => PlayState::Stop,
        }
    }

    pub fn stop(&mut self) -> anyhow::Result<()> {
        self.track_set.store(false, Ordering::Relaxed);
        self.pipeline.set_state(gst::State::Ready)?;

        let mut lock = self.scrobbled.lock().unwrap();
        *lock = Scrobbled::SetTo(None);
        Ok(())
    }

    pub fn shutdown(&mut self) -> anyhow::Result<()> {
        self.pipeline.set_state(gst::State::Null)?;
        Ok(())
    }

    /// position in ms
    pub fn set_position(&mut self, position: i64) -> anyhow::Result<()> {
        let pos = position as u64 * gst::ClockTime::MSECOND;

        // https://gstreamer.freedesktop.org/documentation/additional/design/seeking.html?gi-language=c
        self.pipeline.seek_simple(
            gst::SeekFlags::SEGMENT | gst::SeekFlags::FLUSH | gst::SeekFlags::KEY_UNIT,
            pos,
        )?;

        // TODO find out why self.pipeline.query_position returns None
        // workaround calculate percent ourself
        if let Some(total) = self.pipeline.query_duration::<gst::ClockTime>() {
            let percent = position as f32 / (total.mseconds() as f32);
            let mut lock = self.scrobbled.lock().unwrap();
            *lock = Scrobbled::SetTo(Some(percent.percent_ratio()));
        }

        Ok(())
    }

    /// returns position in seconds
    pub fn _position(&self) -> i32 {
        match self.pipeline.query_position::<gst::ClockTime>() {
            Some(clock) => clock.seconds() as i32,
            None => 0,
        }
    }

    /// returns duration of playback in seconds
    pub fn _duration(&self) -> i32 {
        match self.source.query_duration::<gst::ClockTime>() {
            Some(clock) => clock.seconds() as i32,
            None => 0,
        }
    }

    pub fn set_band(&self, band: usize, value: f64) {
        let value = value.clamp(-10.0, 10.0);
        self.equalizer.set_property(&format!("band{band}"), value);
    }

    pub fn set_volume(&self, value: f64) {
        let volume = value.clamp(0.0, 1.0);
        self.volume.set_property("volume", volume.powi(2));
    }

    pub fn sync_equalizer(&mut self) {
        let settings = Settings::get().lock().unwrap();
        for (i, band) in settings.equalizer_bands.iter().enumerate() {
            match settings.equalizer_enabled {
                true => self.set_band(i, *band),
                false => self.set_band(i, 0.0),
            }
        }
    }

    pub fn sync_volume(&mut self) {
        let settings = Settings::get().lock().unwrap();
        self.volume.set_property("volume", settings.volume);
    }
}
