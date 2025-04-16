use relm4::gtk::{self, prelude::WidgetExt};

use crate::css::DragState;

pub mod album_row;
pub mod album_track_row;
pub mod artist_row;
pub mod playlist_row;
pub mod queue_song_row;
pub mod track_row;

fn get_list_item_widget(widget: &impl gtk::prelude::IsA<gtk::Widget>) -> Option<gtk::Widget> {
    let b = widget.parent()?;
    let column_view_cell = b.parent()?;
    column_view_cell.parent()
}

pub struct SetupFinished(bool);

pub trait DragIndicatable {
    fn child_widget(&self) -> &Option<impl gtk::prelude::IsA<gtk::Widget>>;

    fn add_drag_indicator_top(&self) {
        if let Some(widget) = &self.child_widget() {
            if let Some(list_item) = get_list_item_widget(widget) {
                DragState::drop_shadow_top(&list_item);
            }
        }
    }

    fn add_drag_indicator_bottom(&self) {
        if let Some(widget) = &self.child_widget() {
            if let Some(list_item) = get_list_item_widget(widget) {
                DragState::drop_shadow_bottom(&list_item);
            }
        }
    }

    fn reset_drag_indicators(&self) {
        if let Some(widget) = &self.child_widget() {
            if let Some(list_item) = get_list_item_widget(widget) {
                DragState::reset(&list_item);
            }
        }
    }
}
