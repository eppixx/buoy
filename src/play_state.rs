use crate::mpris::MprisString;

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub enum PlayState {
    Play,
    Pause,
    #[default]
    Stop,
}

impl MprisString for PlayState {
    fn to_mpris_string(&self) -> String {
        match self {
            Self::Play => String::from("Playing"),
            Self::Pause => String::from("Paused"),
            Self::Stop => String::from("Stopped"),
        }
    }

    fn from_mpris_string(value: impl AsRef<str>) -> Self {
        match value.as_ref() {
            "Playing" => Self::Play,
            "Paused" => Self::Pause,
            _ => Self::Stop,
        }
    }
}
