use relm4::gtk::{self, gio, glib, prelude::ListItemExt};

use crate::common::store_from_category;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Category {
    Favorite,
    Title,
    Year,
    Cd,
    TrackNumber,
    Artist,
    Album,
    Genre,
    Duration,
    BitRate,
    AlbumCount,
}

impl std::fmt::Display for Category {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Favorite => write!(f, "Favorite"),
            Self::Title => write!(f, "Title"),
            Self::Year => write!(f, "Year"),
            Self::Cd => write!(f, "Cd"),
            Self::TrackNumber => write!(f, "Track Number"),
            Self::Artist => write!(f, "Artist"),
            Self::Album => write!(f, "Album"),
            Self::Genre => write!(f, "Genre"),
            Self::Duration => write!(f, "Duration"),
            Self::BitRate => write!(f, "Bit Rate"),
            Self::AlbumCount => write!(f, "Album Count"),
        }
    }
}

impl TryFrom<String> for Category {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.as_ref() {
            "Favorite" => Ok(Self::Favorite),
            "Title" => Ok(Self::Title),
            "Year" => Ok(Self::Year),
            "Cd" => Ok(Self::Cd),
            "Track Number" => Ok(Self::TrackNumber),
            "Artist" => Ok(Self::Artist),
            "Album" => Ok(Self::Album),
            "Genre" => Ok(Self::Genre),
            "Duration" => Ok(Self::Duration),
            "Bit Rate" => Ok(Self::BitRate),
            "Album Count" => Ok(Self::AlbumCount),
            e => Err(format!("\"{e}\" is not a State")),
        }
    }
}

impl Category {
    pub fn tracks() -> gio::ListStore {
        let categories = [
            Self::Title,
            Self::Year,
            Self::Cd,
            Self::TrackNumber,
            Self::Artist,
            Self::Album,
            Self::Genre,
            Self::Duration,
            Self::BitRate,
        ];
        store_from_category(&categories)
    }

    pub fn artists() -> gio::ListStore {
        let categories = [Self::Artist, Self::AlbumCount];
        store_from_category(&categories)
    }

    pub fn albums_view() -> gio::ListStore {
        let categories = [
            Self::Album,
            Self::Artist,
            Self::Year,
            Self::Cd,
            Self::Genre,
            Self::Duration,
        ];
        store_from_category(&categories)
    }

    pub fn factory() -> gtk::SignalListItemFactory {
        use glib::object::Cast;
        use granite::prelude::CastNone;

        let factory = gtk::SignalListItemFactory::new();
        factory.connect_setup(move |_, list_item| {
            let label = gtk::Label::new(Some("Category"));
            list_item
                .downcast_ref::<gtk::ListItem>()
                .expect("is not a ListItem")
                .set_child(Some(&label));
        });
        factory.connect_bind(move |_, list_item| {
            // get BoxedAnyObject from ListItem
            let boxed = list_item
                .downcast_ref::<gtk::ListItem>()
                .expect("is not a ListItem")
                .item()
                .and_downcast::<glib::BoxedAnyObject>()
                .expect("item is not a BoxedAnyObject");
            // get label from ListItem
            let label = list_item
                .downcast_ref::<gtk::ListItem>()
                .expect("is not a ListItem")
                .child()
                .and_downcast::<gtk::Label>()
                .expect("child has to be a Label");
            // set label from category
            label.set_label(&boxed.borrow::<Category>().to_string());
        });

        factory
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gtk_helper::stack::test_self;

    #[test]
    fn category_enum_conversion() {
        test_self(Category::Favorite);
        test_self(Category::Title);
        test_self(Category::Year);
        test_self(Category::Cd);
        test_self(Category::TrackNumber);
        test_self(Category::Artist);
        test_self(Category::Album);
        test_self(Category::Genre);
        test_self(Category::Duration);
        test_self(Category::BitRate);
    }
}
