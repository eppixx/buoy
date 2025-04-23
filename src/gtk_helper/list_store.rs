pub trait ListStoreExt {
    /// takes a Slice and creates a `ListStore` in a generic fashion; to be used in a `gtk::DropDown` as a store
    fn from_slice<T: Clone + 'static>(slice: &[T]) -> Self;
}

impl ListStoreExt for relm4::gtk::gio::ListStore {
    fn from_slice<T: Clone + 'static>(slice: &[T]) -> Self {
        use relm4::gtk::{gio, glib};
        let store = gio::ListStore::new::<glib::BoxedAnyObject>();
        for piece in slice {
            store.append(&glib::BoxedAnyObject::new(piece.clone()));
        }
        store
    }
}
