use relm4::gtk::traits::WidgetExt;

pub fn setup_css() {
    let data = "
.padd-item {
margin-top: 1px;
margin-bottom: 1px;
}
.drag-indicator-top {
border-top: 1px solid Gray;
margin-bottom: 1px;
}

.drag-indicator-bottom {
border-bottom: 1px solid Gray;
margin-top: 1px;
}

.destructive-button-spacer {
margin-left: 15px;
}
";
    relm4::set_global_css(data);
}

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
