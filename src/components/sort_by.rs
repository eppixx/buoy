use relm4::gtk::{self, gio, glib, prelude::ListItemExt};

use crate::common::store_from_category;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SortBy {
    Alphabetical,
    AlphabeticalRev,
    Release,
    ReleaseRev,
    RecentlyAdded,
    RecentlyAddedRev,
    MostAlbums,
    MostAlbumsRev,
}

impl std::fmt::Display for SortBy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Alphabetical => write!(f, "A-Z"),
            Self::AlphabeticalRev => write!(f, "Z-A"),
            Self::Release => write!(f, "Newest Release"),
            Self::ReleaseRev => write!(f, "Oldest Release"),
            Self::RecentlyAdded => write!(f, "Recently added"),
            Self::RecentlyAddedRev => write!(f, "Longest available"),
            Self::MostAlbums => write!(f, "Most Albums"),
            Self::MostAlbumsRev => write!(f, "Least Albums"),
        }
    }
}

impl TryFrom<String> for SortBy {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.as_ref() {
            "A-Z" => Ok(Self::Alphabetical),
            "Z-A" => Ok(Self::AlphabeticalRev),
            "Newest Release" => Ok(Self::Release),
            "Oldest Release" => Ok(Self::ReleaseRev),
            "Recently added" => Ok(Self::RecentlyAdded),
            "Longest available" => Ok(Self::RecentlyAddedRev),
            "Most Albums" => Ok(Self::MostAlbums),
            "Least Albums" => Ok(Self::MostAlbumsRev),
            e => Err(format!("\"{e}\" is not a SortBy")),
        }
    }
}

impl SortBy {
    pub fn artists_store() -> gio::ListStore {
        let categories = [
            Self::Alphabetical,
            Self::AlphabeticalRev,
            Self::MostAlbums,
            Self::MostAlbumsRev,
        ];
        store_from_category(&categories)
    }

    pub fn albums_store() -> gio::ListStore {
        let categories = [
            Self::Alphabetical,
            Self::AlphabeticalRev,
            Self::Release,
            Self::ReleaseRev,
            Self::RecentlyAdded,
            Self::RecentlyAddedRev,
        ];
        store_from_category(&categories)
    }

    pub fn factory() -> gtk::SignalListItemFactory {
        use glib::object::Cast;
        use granite::prelude::CastNone;

        let factory = gtk::SignalListItemFactory::new();
        factory.connect_setup(move |_, list_item| {
            let label = gtk::Label::new(Some("Selection"));
            list_item
                .downcast_ref::<gtk::ListItem>()
                .expect("is not a ListItem")
                .set_child(Some(&label));
        });
        factory.connect_bind(move |_, list_item| {
            // get BoxedAnyObject from ListItem
            let boxed = list_item
                .downcast_ref::<gtk::ListItem>()
                .expect("ist not a ListItem")
                .item()
                .and_downcast::<glib::BoxedAnyObject>()
                .expect("item is not a BoxedAnyObject");
            // get label from ListItem
            let label = list_item
                .downcast_ref::<gtk::ListItem>()
                .expect("is not a ListItem")
                .child()
                .and_downcast::<gtk::Label>()
                .expect("is not a Label");

            // set label from String
            let s = format!("{}", boxed.borrow::<Self>());
            label.set_label(&s);
        });

        factory
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gtk_helper::stack::test_self;

    #[test]
    fn sort_by_enum_conversion() {
        test_self(SortBy::Alphabetical);
        test_self(SortBy::AlphabeticalRev);
        test_self(SortBy::Release);
        test_self(SortBy::ReleaseRev);
        test_self(SortBy::RecentlyAdded);
        test_self(SortBy::RecentlyAddedRev);
        test_self(SortBy::MostAlbums);
        test_self(SortBy::MostAlbumsRev);
    }
}
