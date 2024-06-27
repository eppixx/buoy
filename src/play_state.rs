#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub enum PlayState {
    Play,
    Pause,
    #[default]
    Stop,
}

impl PlayState {
    pub fn to_mpris_string(&self) -> String {
        match self {
            Self::Play => String::from("Playing"),
            Self::Pause => String::from("Paused"),
            Self::Stop => String::from("Stopped"),
        }
    }
}
