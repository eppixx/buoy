use gstreamer as gst;
use gstreamer::prelude::*;
use relm4::gtk;

use std::cell::RefCell;
use std::rc::Rc;

use crate::settings::Settings;

#[derive(Debug)]
pub struct Playback {
    pipeline: gstreamer::Pipeline,
    source: gstreamer::Element,
    volume: gstreamer::Element,
    equalizer: gstreamer::Element,
}

const TICK: u64 = 250;

#[derive(Debug)]
pub enum PlaybackOut {
    TrackEnd,
    Seek(i64), // in ms
}

impl Playback {
    pub fn new(
        sender: &async_channel::Sender<PlaybackOut>,
        // settings: &Arc<RwLock<settings::Settings>>,
    ) -> anyhow::Result<Self> {
        gst::init()?;

        // Create the empty pipeline
        let pipeline = gst::Pipeline::with_name("playback");
        // Create the elements
        let source = gst::ElementFactory::make_with_name("uridecodebin", Some("source"))?;
        let convert = gst::ElementFactory::make_with_name("audioconvert", Some("convert"))?;
        let volume = gst::ElementFactory::make_with_name("volume", Some("volume"))?;
        let equalizer =
            gst::ElementFactory::make_with_name("equalizer-10bands", Some("equalizer"))?;
        let sink = gst::ElementFactory::make_with_name("autoaudiosink", Some("sink"))?;

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
                tracing::warn!("audio is no raw type, but {} - ignoring.", new_pad_type);
                return;
            }

            let res = src_pad.link(&sink_pad);
            if res.is_err() {
                tracing::error!("type is {} but link failed.", new_pad_type);
            }
        });

        //check for pipline messages
        let  send = sender.clone();
        let bus = pipeline.bus().unwrap();
        std::thread::spawn(move || {
            for msg in bus.iter_timed(gst::ClockTime::NONE) {
                use gstreamer::MessageView;
                match msg.view() {
                    MessageView::Eos(..) => send.try_send(PlaybackOut::TrackEnd).unwrap(),
                    MessageView::StreamStart(..) => send.try_send(PlaybackOut::Seek(0)).unwrap(),
                    _ => {}
                }
            }
        });

        //callback for seekbar
        let stamp = Rc::new(RefCell::new(pipeline.query_position::<gst::ClockTime>()));
        let pipeline_weak = pipeline.downgrade();
        let send = sender.clone();
        gtk::glib::source::timeout_add_local(std::time::Duration::from_millis(TICK), move || {
            let pipeline = match pipeline_weak.upgrade() {
                Some(pipeline) => pipeline,
                None => return gtk::glib::ControlFlow::Continue,
            };

            //dont send messages when not playing a stream
            if pipeline.current_state() != gst::State::Playing {
								return gtk::glib::ControlFlow::Continue;
            }

            let current = pipeline.query_position::<gst::ClockTime>();
            if current != *stamp.borrow() {
                let seconds = match current {
                    Some(clock) => clock.seconds() as i64,
                    None => 0,
                };
                send.try_send(PlaybackOut::Seek(seconds * 1000)).unwrap();
                stamp.replace(current);
            }

						gtk::glib::ControlFlow::Continue
        });

        let mut play = Self {
            pipeline,
            source,
            volume,
            equalizer,
        };

        play.sync_equalizer();
        play.sync_volume();
        Ok(play)
    }

    pub fn set_track(&mut self, uri: impl AsRef<str>) {
        self.stop().unwrap(); //TODO error handling
        self.source.set_property("uri", uri.as_ref());
    }

    pub fn play(&mut self) -> anyhow::Result<()> {
        self.pipeline.set_state(gstreamer::State::Playing)?;
        Ok(())
    }
    pub fn pause(&mut self) -> anyhow::Result<()> {
        self.pipeline.set_state(gstreamer::State::Paused)?;
        Ok(())
    }

    pub fn stop(&mut self) -> anyhow::Result<()> {
        self.pipeline.set_state(gstreamer::State::Ready)?;
        Ok(())
    }

    pub fn shutdown(&mut self) -> anyhow::Result<()> {
        self.pipeline.set_state(gstreamer::State::Null)?;
        Ok(())
    }

    pub fn set_position(&self, position: i64) {
        self.pipeline
            .seek_simple(
                gst::SeekFlags::FLUSH | gst::SeekFlags::KEY_UNIT,
                position as u64 * gst::ClockTime::MSECOND,
            )
            .expect("seek failed");
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
        self.equalizer.set_property(&format!("band{}", band), value);
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
