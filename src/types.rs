use std::{cell::RefCell, rc::Rc};

use relm4::gtk::glib;

use crate::{
    factory::{playlist_row::PlaylistUid, queue_song_row::QueueUid},
    subsonic::Subsonic,
};

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Hash, glib::Boxed)]
#[boxed_type(name = "MediaId")]
pub enum Id {
    Song(String),
    Artist(String),
    Album(String),
    Playlist(String),
}

impl Id {
    const DELIMITER: char = ':';

    pub fn song(id: impl Into<String>) -> Self {
        Self::Song(id.into())
    }

    pub fn album(id: impl Into<String>) -> Self {
        Self::Album(id.into())
    }

    pub fn artist(id: impl Into<String>) -> Self {
        Self::Artist(id.into())
    }

    pub fn playlist(id: impl Into<String>) -> Self {
        Self::Playlist(id.into())
    }

    pub fn inner(&self) -> &str {
        match self {
            Self::Song(id) | Self::Artist(id) | Self::Album(id) | Self::Playlist(id) => id,
        }
    }

    pub fn kind(&self) -> String {
        match self {
            Self::Song(_) => String::from("Song"),
            Self::Artist(_) => String::from("Artist"),
            Self::Album(_) => String::from("Album"),
            Self::Playlist(_) => String::from("Playlist"),
        }
    }

    pub fn serialize(&self) -> String {
        match self {
            Self::Song(id) => format!("song{}{id}", Self::DELIMITER),
            Self::Artist(id) => format!("artist{}{id}", Self::DELIMITER),
            Self::Album(id) => format!("album{}{id}", Self::DELIMITER),
            Self::Playlist(id) => format!("playlist{}{id}", Self::DELIMITER),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, glib::Boxed)]
#[boxed_type(name = "Droppable")]
pub enum Droppable {
    Queue(Vec<submarine::data::Child>),
    QueueSongs(Vec<QueueUid>),
    Child(Box<submarine::data::Child>),
    AlbumWithSongs(Box<submarine::data::AlbumWithSongsId3>),
    Album(Box<submarine::data::AlbumId3>),
    AlbumChild(Box<submarine::data::Child>),
    ArtistWithAlbums(Box<submarine::data::ArtistWithAlbumsId3>),
    Artist(Box<submarine::data::ArtistId3>),
    Playlist(Box<submarine::data::PlaylistWithSongs>),
    PlaylistItems(Vec<PlaylistUid>),
}

impl Droppable {
    pub fn get_songs(&self, subsonic: &Rc<RefCell<Subsonic>>) -> Vec<submarine::data::Child> {
        match &self {
            Droppable::Queue(ids) => ids.clone(),
            Droppable::QueueSongs(songs) => songs.iter().map(|song| song.child.clone()).collect(),
            Droppable::Child(c) => vec![*c.clone()],
            Droppable::AlbumWithSongs(album) => album.song.clone(),
            Droppable::Artist(artist) => {
                let subsonic = subsonic.borrow();
                let albums = subsonic.albums_from_artist(artist);
                albums
                    .iter()
                    .flat_map(|a| subsonic.tracks_from_album(a))
                    .cloned()
                    .collect()
            }
            Droppable::ArtistWithAlbums(artist) => {
                let subsonic = subsonic.borrow();
                artist
                    .album
                    .iter()
                    .flat_map(|a| subsonic.tracks_from_album_id3(a))
                    .cloned()
                    .collect()
            }
            Droppable::Playlist(playlist) => playlist.entry.clone(),
            Droppable::AlbumChild(child) => subsonic
                .borrow()
                .tracks_from_album(child)
                .into_iter()
                .cloned()
                .collect(),
            Droppable::Album(album) => subsonic
                .borrow()
                .tracks_from_album_id3(album)
                .into_iter()
                .cloned()
                .collect(),
            Droppable::PlaylistItems(items) => {
                items.iter().map(|song| song.child.clone()).collect()
            }
        }
    }
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub enum IdConversionError {
    IdIsEmpty,
    IncorrectSeperators,
    UnrecognizedType,
}

impl TryFrom<&str> for Id {
    type Error = IdConversionError;

    fn try_from<'a>(value: &'a str) -> Result<Self, Self::Error> {
        let parts: Vec<&str> = value.split_terminator(Self::DELIMITER).collect();

        let test = |value: &'a str, parts: Vec<&'a str>| -> Result<&'a str, Self::Error> {
            if parts.len() == 1 && value.ends_with(Self::DELIMITER) {
                Err(IdConversionError::IdIsEmpty)
            } else if parts.len() != 2 || value.ends_with(Self::DELIMITER) {
                Err(IdConversionError::IncorrectSeperators)
            } else {
                Ok(parts[1])
            }
        };

        match parts.first() {
            Some(&"song") => Ok(Self::song(test(value, parts)?)),
            Some(&"artist") => Ok(Self::artist(test(value, parts)?)),
            Some(&"album") => Ok(Self::album(test(value, parts)?)),
            Some(&"playlist") => Ok(Self::playlist(test(value, parts)?)),
            _ => Err(IdConversionError::UnrecognizedType),
        }
    }
}

impl TryFrom<&String> for Id {
    type Error = IdConversionError;

    fn try_from(value: &String) -> Result<Self, Self::Error> {
        Id::try_from(value.as_str())
    }
}

impl AsRef<str> for Id {
    fn as_ref(&self) -> &str {
        self.inner()
    }
}

impl std::fmt::Display for Id {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Id: {{{}:{}}}", self.kind(), self.inner())
    }
}

#[cfg(test)]
mod tests {
    use super::{Id, IdConversionError};

    #[test]
    fn convert() -> anyhow::Result<()> {
        let test_oracle = vec![
            Id::Song(String::from("77777")),
            Id::Artist(String::from("33333")),
        ];
        for test in &test_oracle {
            let string = test.serialize();
            let reverse = Id::try_from(&string);
            match reverse {
                Err(_) => panic!(),
                Ok(id) => assert_eq!(test, &id),
            }
        }

        Ok(())
    }

    #[test]
    fn conver_error() {
        let test_oracle = vec![
            ("sdf:44444", IdConversionError::UnrecognizedType),
            (":44444", IdConversionError::UnrecognizedType),
            ("55555", IdConversionError::UnrecognizedType),
            ("song:555:", IdConversionError::IncorrectSeperators),
            ("song:555:dsfsdf", IdConversionError::IncorrectSeperators),
            ("song:", IdConversionError::IdIsEmpty),
        ];

        for test in test_oracle {
            assert_eq!(
                Id::try_from(test.0),
                Err(test.1.clone()),
                "testing {} and {:?}",
                test.0,
                test.1
            );
        }
    }
}
