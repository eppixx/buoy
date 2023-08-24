use crate::components::sequence_button::Sequence;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Shuffle {
    Sequential,
    Shuffle,
}

impl Sequence for Shuffle {
    fn current(&self) -> &str {
        match self {
            Self::Sequential => "media-playlist-consecutive-symbolic",
            Self::Shuffle => "media-playlist-shuffle-symbolic",
        }
    }

    fn next(&mut self) {
        *self = match self {
            Self::Sequential => Self::Shuffle,
            Self::Shuffle => Self::Sequential,
        };
    }

    fn tooltip(&self) -> Option<&str> {
        match self {
            Self::Sequential => Some("sequential order"),
            Self::Shuffle => Some("random order"),
        }
    }
}
