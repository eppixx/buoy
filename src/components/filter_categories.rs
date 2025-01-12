use gettextrs::gettext;
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
    DurationMin,
    DurationSec,
    BitRate,
    AlbumCount,
}

impl Category {
    fn translate(&self) -> String {
        match self {
            Self::Favorite => gettext("Favorite"),
            Self::Title => gettext("Title"),
            Self::Year => gettext("Year"),
            Self::Cd => gettext("CD"),
            Self::TrackNumber => gettext("Track Number"),
            Self::Artist => gettext("Artist"),
            Self::Album => gettext("Album"),
            Self::Genre => gettext("Genre"),
            Self::DurationMin => gettext("Length (min)"),
            Self::DurationSec => gettext("Length (sec)"),
            Self::BitRate => gettext("Bit Rate"),
            Self::AlbumCount => gettext("Album Count"),
        }
    }
}

impl std::fmt::Display for Category {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Favorite => write!(f, "Favorite"),
            Self::Title => write!(f, "Title"),
            Self::Year => write!(f, "Year"),
            Self::Cd => write!(f, "CD"),
            Self::TrackNumber => write!(f, "Track Number"),
            Self::Artist => write!(f, "Artist"),
            Self::Album => write!(f, "Album"),
            Self::Genre => write!(f, "Genre"),
            Self::DurationMin => write!(f, "Length (min)"),
            Self::DurationSec => write!(f, "Length (sec)"),
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
            "CD" => Ok(Self::Cd),
            "Track Number" => Ok(Self::TrackNumber),
            "Artist" => Ok(Self::Artist),
            "Album" => Ok(Self::Album),
            "Genre" => Ok(Self::Genre),
            "Length (min)" => Ok(Self::DurationMin),
            "Length (sec)" => Ok(Self::DurationSec),
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
            Self::DurationSec,
            Self::BitRate,
        ];
        store_from_category(&categories)
    }

    pub fn artists() -> gio::ListStore {
        let categories = [Self::Artist, Self::AlbumCount];
        store_from_category(&categories)
    }

    pub fn albums() -> gio::ListStore {
        let categories = [
            Self::Album,
            Self::Artist,
            Self::Year,
            Self::Cd,
            Self::Genre,
            Self::DurationMin,
        ];
        store_from_category(&categories)
    }

    pub fn factory() -> gtk::SignalListItemFactory {
        use glib::object::Cast;
        use granite::prelude::CastNone;

        let factory = gtk::SignalListItemFactory::new();
        factory.connect_setup(move |_, list_item| {
            let label = gtk::Label::new(Some(&gettext("Category")));
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
            label.set_label(&boxed.borrow::<Category>().translate());
        });

        factory
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_self(state: Category) {
        assert_eq!(Ok(&state), Category::try_from(state.translate()).as_ref());
    }

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
        test_self(Category::DurationSec);
        test_self(Category::DurationMin);
        test_self(Category::BitRate);
    }
}
