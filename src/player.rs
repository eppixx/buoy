#[derive(Debug)]
pub enum Command {
    Next,
    Previous,
    Play,
    Pause,
    PlayPause,
    Stop,
    /// in seconds
    Seek(i64),
    /// ranges from 0.0f64 to 1.0f64
    Volume(f64),
}
