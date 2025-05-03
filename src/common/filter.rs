use std::cmp::Ordering;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TextRelation {
    Contains,
    ContainsNot,
    ExactNot,
    Exact,
}

#[derive(Debug, Clone)]
pub enum Filter {
    Favorite(Option<bool>),
    Title(TextRelation, String),
    Year(Ordering, i32),
    Cd(Ordering, i32),
    TrackNumber(Ordering, usize),
    Artist(TextRelation, String),
    Album(TextRelation, String),
    Genre(TextRelation, String),
    BitRate(Ordering, usize),
    DurationSec(Ordering, i32),
    DurationMin(Ordering, i32),
    AlbumCount(Ordering, i32),
}

impl Filter {
    pub fn match_artist(&self, artist: &submarine::data::ArtistId3) -> bool {
        match self {
            //TODO add matching for regular expressions
            Filter::Favorite(None) => {}
            Filter::Favorite(Some(state)) => {
                if *state != artist.starred.is_some() {
                    return false;
                }
            }
            Filter::Artist(_, value) if value.is_empty() => {}
            Filter::Artist(relation, value) => match relation {
                TextRelation::ExactNot if value == &artist.name => return false,
                TextRelation::Exact if value != &artist.name => return false,
                TextRelation::ContainsNot if artist.name.contains(value) => return false,
                TextRelation::Contains if !artist.name.contains(value) => return false,
                _ => {} // filter matches
            },
            Filter::AlbumCount(order, value) => {
                if artist.album_count.cmp(value) != *order {
                    return false;
                }
            }
            _ => unreachable!("there are filters that shouldnt be"),
        }
        true
    }

    pub fn match_album(&self, album: &submarine::data::Child) -> bool {
        match self {
            //TODO add matching for regular expressions
            Filter::Favorite(None) => {}
            Filter::Favorite(Some(state)) => {
                if *state != album.starred.is_some() {
                    return false;
                }
            }
            Filter::Album(_, value) if value.is_empty() => {} // filter matches
            Filter::Album(relation, value) => match relation {
                TextRelation::ExactNot if Some(value) == album.album.as_ref() => return false,
                TextRelation::Exact if Some(value) != album.album.as_ref() => return false,
                TextRelation::ContainsNot => {
                    if let Some(album) = &album.album {
                        if album.contains(value) {
                            return false;
                        }
                    }
                }
                TextRelation::Contains => {
                    if let Some(album) = &album.album {
                        if !album.contains(value) {
                            return false;
                        }
                    } else {
                        return false;
                    }
                }
                _ => {} // filter matches
            },
            Filter::Artist(_, value) if value.is_empty() => {} // filter matches
            Filter::Artist(relation, value) => match relation {
                TextRelation::ExactNot if Some(value) == album.artist.as_ref() => return false,
                TextRelation::Exact if Some(value) != album.artist.as_ref() => return false,
                TextRelation::ContainsNot => {
                    if let Some(artist) = &album.artist {
                        if artist.contains(value) {
                            return false;
                        }
                    }
                }
                TextRelation::Contains => {
                    if let Some(artist) = &album.artist {
                        if !artist.contains(value) {
                            return false;
                        }
                    } else {
                        return false;
                    }
                }
                _ => {} // filter matches
            },
            Filter::Year(order, value) => {
                if let Some(year) = &album.year {
                    if year.cmp(value) != *order {
                        return false;
                    }
                } else {
                    return false;
                }
            }
            Filter::Cd(_, 0) => {
                if album.disc_number.is_some() {
                    return false;
                }
            }
            Filter::Cd(order, value) => {
                if let Some(disc) = &album.disc_number {
                    if disc.cmp(value) != *order {
                        return false;
                    }
                } else {
                    return false;
                }
            }
            Filter::Genre(_, value) if value.is_empty() => {}
            Filter::Genre(relation, value) => match relation {
                TextRelation::ExactNot if Some(value) == album.genre.as_ref() => return false,
                TextRelation::Exact if Some(value) != album.genre.as_ref() => return false,
                TextRelation::ContainsNot => {
                    if let Some(genre) = &album.genre {
                        if genre.contains(value) {
                            return false;
                        }
                    }
                }
                TextRelation::Contains => {
                    if let Some(genre) = &album.genre {
                        if !genre.contains(value) {
                            return false;
                        }
                    } else {
                        return false;
                    }
                }
                _ => {} // filter matches
            },
            Filter::DurationMin(order, value) => {
                let value = value * 60;
                if let Some(duration) = &album.duration {
                    if duration.cmp(&value) != *order {
                        return false;
                    }
                } else {
                    return false;
                }
            }
            _ => unreachable!("there are filters that shouldnt be"),
        }
        true
    }

    pub fn match_track(&self, track: &submarine::data::Child) -> bool {
        match self {
            //TODO add matching for regular expressions
            Filter::Favorite(None) => {}
            Filter::Favorite(Some(state)) => {
                if *state != track.starred.is_some() {
                    return false;
                }
            }
            Filter::Title(_, value) if value.is_empty() => {} // filter matches
            Filter::Title(relation, value) => match relation {
                TextRelation::ExactNot if value == &track.title => return false,
                TextRelation::Exact if value != &track.title => return false,
                TextRelation::ContainsNot if track.title.contains(value) => return false,
                TextRelation::Contains if !track.title.contains(value) => return false,
                _ => {} // filter matches
            },
            Filter::Album(_, value) if value.is_empty() => {} // filter matches
            Filter::Album(relation, value) => match relation {
                TextRelation::ExactNot if Some(value) == track.album.as_ref() => return false,
                TextRelation::Exact if Some(value) != track.album.as_ref() => return false,
                TextRelation::ContainsNot => {
                    if let Some(album) = &track.album {
                        if album.contains(value) {
                            return false;
                        }
                    }
                }
                TextRelation::Contains => {
                    if let Some(album) = &track.album {
                        if !album.contains(value) {
                            return false;
                        }
                    } else {
                        return false;
                    }
                }
                _ => {} // filter matches
            },
            Filter::Artist(_, value) if value.is_empty() => {} // filter matches
            Filter::Artist(relation, value) => match relation {
                TextRelation::ExactNot if Some(value) == track.artist.as_ref() => return false,
                TextRelation::Exact if Some(value) != track.artist.as_ref() => return false,
                TextRelation::ContainsNot => {
                    if let Some(artist) = &track.artist {
                        if artist.contains(value) {
                            return false;
                        }
                    }
                }
                TextRelation::Contains => {
                    if let Some(artist) = &track.artist {
                        if !artist.contains(value) {
                            return false;
                        }
                    } else {
                        return false;
                    }
                }
                _ => {} // filter matches
            },
            Filter::Year(order, value) => {
                if let Some(year) = &track.year {
                    if year.cmp(value) != *order {
                        return false;
                    }
                } else {
                    return false;
                }
            }
            Filter::Cd(order, value) => {
                if let Some(disc) = &track.disc_number {
                    if disc.cmp(value) != *order {
                        return false;
                    }
                } else {
                    return false;
                }
            }
            Filter::Genre(_, value) if value.is_empty() => {}
            Filter::Genre(relation, value) => match relation {
                TextRelation::ExactNot if Some(value) == track.genre.as_ref() => return false,
                TextRelation::Exact if Some(value) != track.genre.as_ref() => return false,
                TextRelation::ContainsNot => {
                    if let Some(genre) = &track.genre {
                        if genre.contains(value) {
                            return false;
                        }
                    }
                }
                TextRelation::Contains => {
                    if let Some(genre) = &track.genre {
                        if !genre.contains(value) {
                            return false;
                        }
                    } else {
                        return false;
                    }
                }
                _ => {} // filter matches
            },
            Filter::DurationMin(order, value) => {
                if let Some(duration) = &track.duration {
                    if duration.cmp(value) != *order {
                        return false;
                    }
                } else {
                    return false;
                }
            }
            Filter::BitRate(order, value) => {
                if let Some(bitrate) = &track.bit_rate {
                    if bitrate.cmp(&(*value as i32)) != *order {
                        return false;
                    }
                } else {
                    return false;
                }
            }
            _ => unreachable!("there are filters that shouldnt be"),
        }

        true
    }
}
