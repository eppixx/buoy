use gettextrs::gettext;
use relm4::{
    gtk::{self, prelude::WidgetExt},
    RelmWidgetExt,
};

use crate::css::DragState;

pub mod album_row;
pub mod album_track_row;
pub mod artist_row;
pub mod artist_song_row;
pub mod playlist_element;
pub mod playlist_row;
pub mod queue_song_row;
pub mod track_row;

fn get_list_item_widget(widget: &impl gtk::prelude::IsA<gtk::Widget>) -> Option<gtk::Widget> {
    let b = widget.parent()?;
    let column_view_cell = b.parent()?;
    column_view_cell.parent()
}

fn create_fav_btn() -> gtk::Button {
    let fav_btn = gtk::Button::new();
    fav_btn.set_tooltip(&gettext("Click to (un)favorite song"));
    fav_btn.set_focus_on_click(false);
    fav_btn.add_css_class("flat");
    fav_btn
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
