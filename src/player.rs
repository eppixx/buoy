use crate::components::sequence_button_impl::{repeat::Repeat, shuffle::Shuffle};

#[derive(Debug)]
pub enum Command {
    Next,
    Previous,
    Play,
    Pause,
    PlayPause,
    Stop,
    /// in ms
    SetSongPosition(i64),
    /// ranges from 0.0f64 to 1.0f64
    Volume(f64),
    Repeat(Repeat),
    Shuffle(Shuffle),
}
