use serde::{Deserialize, Serialize};

use crate::components::sequence_button::Sequence;

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Shuffle {
    #[default]
    Sequential,
    Shuffle,
}

impl std::str::FromStr for Shuffle {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "media-playlist-consecutive-symbolic" => Ok(Shuffle::Sequential),
            "media-playlist-shuffle-symbolic" => Ok(Shuffle::Shuffle),
            _ => Err(()),
        }
    }
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
