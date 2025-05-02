use relm4::{
    gtk::{
        self,
        prelude::{BoxExt, GtkApplicationExt, GtkWindowExt, OrientableExt, WidgetExt},
    },
    RelmWidgetExt,
};

#[relm4::widget_template(pub)]
impl relm4::WidgetTemplate for WarningDialog {
    view! {
        dialog = gtk::Window {
            set_modal: true,
            set_transient_for: Some(&relm4::main_application().windows()[0]),

            #[wrap(Some)]
            set_titlebar = &gtk::HeaderBar {
                add_css_class: granite::STYLE_CLASS_FLAT,
                add_css_class: granite::STYLE_CLASS_DEFAULT_DECORATION,
                set_show_title_buttons: false,
                set_visible: false,
            },

            gtk::WindowHandle {
                gtk::Box {
                    set_margin_all: 15,
                    set_spacing: 20,

                    gtk::Image {
                        set_icon_name: Some("dialog-warning"),
                        set_pixel_size: 64,
                    },

                    gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_spacing: 20,

                        gtk::Box {
                            set_orientation: gtk::Orientation::Vertical,
                            set_spacing: 5,

                            gtk::Label {
                                set_label: "Warning",
                                add_css_class: granite::STYLE_CLASS_H2_LABEL,
                                set_halign: gtk::Align::Start,
                            },

                            append: warning_text = &gtk::Label {}
                        },
                        gtk::Box {
                            set_halign: gtk::Align::End,
                            set_spacing: 10,

                            append: cancel_btn = &gtk::Button {},
                            append: proceed_btn = &gtk::Button {}
                        }
                    }
                }
            }
        }
    }
}
