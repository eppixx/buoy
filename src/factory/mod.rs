use relm4::gtk::{self, glib, prelude::WidgetExt};

pub mod album_row;
pub mod album_track_row;
pub mod artist_row;
pub mod playlist_row;
pub mod queue_song;
pub mod track_row;

fn get_list_item_widget(widget: &impl glib::object::IsA<gtk::Widget>) -> Option<gtk::Widget> {
    let b = widget.parent()?;
    let column_view_cell = b.parent()?;
    column_view_cell.parent()
}

pub struct SetupFinished(bool);


