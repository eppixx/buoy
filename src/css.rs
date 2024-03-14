use relm4::gtk::prelude::WidgetExt;

pub struct DragState {}

impl DragState {
    pub fn reset<W: WidgetExt>(widget: &mut W) {
        widget.remove_css_class("drag-indicator-top");
        widget.remove_css_class("drag-indicator-bottom");
        widget.add_css_class("padd-item");
    }

    pub fn drop_shadow_top<W: WidgetExt>(widget: &mut W) {
        widget.remove_css_class("drag-indicator-bottom");
        widget.remove_css_class("padd-item");
        widget.add_css_class("drag-indicator-top");
    }

    pub fn drop_shadow_bottom<W: WidgetExt>(widget: &mut W) {
        widget.remove_css_class("drag-indicator-top");
        widget.remove_css_class("padd-item");
        widget.add_css_class("drag-indicator-bottom");
    }
}
