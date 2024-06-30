use crate::{components::sequence_button::Sequence, mpris::MprisString};

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub enum Repeat {
    #[default]
    Normal,
    One,
    All,
}

impl Sequence for Repeat {
    fn current(&self) -> &str {
        match self {
            Self::Normal => "media-playlist-no-repeat-symbolic",
            Self::One => "media-playlist-repeat-song",
            Self::All => "media-playlist-repeat-symbolic",
        }
    }

    fn next(&mut self) {
        *self = match self {
            Self::Normal => Self::One,
            Self::One => Self::All,
            Self::All => Self::Normal,
        };
    }

    fn tooltip(&self) -> Option<&str> {
        match self {
            Self::Normal => Some("no repeat"),
            Self::One => Some("repeat current song"),
            Self::All => Some("repeat queue"),
        }
    }
}

impl MprisString for Repeat {
    fn to_mpris_string(&self) -> String {
        match self {
            Self::Normal => String::from("None"),
            Self::One => String::from("Track"),
            Self::All => String::from("Playlist"),
        }
    }

    fn from_mpris_string(value: impl AsRef<str>) -> Self {
        match value.as_ref() {
            "Track" => Self::One,
            "Playlist" => Self::All,
            // otherwise Normal, includes "None"
            _ => Self::Normal,
        }
    }
}
