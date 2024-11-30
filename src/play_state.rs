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

impl std::fmt::Display for PlayState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Play => write!(f, "Play"),
            Self::Pause => write!(f, "Pause"),
            Self::Stop => write!(f, "Stop"),
        }
    }
}

impl TryFrom<String> for PlayState {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.as_ref() {
            "Play" => Ok(PlayState::Play),
            "Pause" => Ok(PlayState::Pause),
            "Stop" => Ok(PlayState::Stop),
            e => Err(format!("{e} does not match a PlayState")),
        }
    }
}
