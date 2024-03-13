use crate::components::sequence_button::Sequence;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Repeat {
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
