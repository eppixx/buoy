use relm4::gtk::{self, prelude::WidgetExt};

pub struct DragState {}

impl DragState {
    pub fn reset<W: gtk::prelude::IsA<gtk::Widget>>(widget: &W) {
        widget.remove_css_class("drag-indicator-top");
        widget.remove_css_class("drag-indicator-bottom");
        widget.add_css_class("padd-item");
    }

    pub fn drop_shadow_top<W: gtk::prelude::IsA<gtk::Widget>>(widget: &W) {
        widget.remove_css_class("drag-indicator-bottom");
        widget.remove_css_class("padd-item");
        widget.add_css_class("drag-indicator-top");
    }

    pub fn drop_shadow_bottom<W: gtk::prelude::IsA<gtk::Widget>>(widget: &W) {
        widget.remove_css_class("drag-indicator-top");
        widget.remove_css_class("padd-item");
        widget.add_css_class("drag-indicator-bottom");
    }
}
