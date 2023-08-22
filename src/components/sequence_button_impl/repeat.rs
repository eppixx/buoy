use crate::components::sequence_button::Sequence;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Repeat {
    Normal,
    RepeatOne,
    RepeatAll,
}

impl Sequence for Repeat {
    fn current(&self) -> &str {
        match self {
            Self::Normal => "media-playlist-no-repeat-symbolic",
            Self::RepeatOne => "media-playlist-repeat-song",
            Self::RepeatAll => "media-playlist-repeat-symbolic",
        }
    }

    fn next(&mut self) {
        *self = match self {
            Self::Normal => Self::RepeatOne,
            Self::RepeatOne => Self::RepeatAll,
            Self::RepeatAll => Self::Normal,
        };
    }

    fn tooltip(&self) -> &str {
        match self {
            Self::Normal => "no repeat",
            Self::RepeatOne => "repeat current song",
            Self::RepeatAll => "repeat queue",
        }
    }
}
