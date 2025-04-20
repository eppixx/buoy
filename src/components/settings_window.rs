use gettextrs::gettext;
use relm4::{
    gtk::{
        self,
        prelude::{BoxExt, ButtonExt, GtkWindowExt, OrientableExt, WidgetExt},
    },
    RelmWidgetExt,
};

use crate::settings::Settings;

#[derive(Debug)]
pub struct SettingsWindow {}

#[derive(Debug)]
pub enum SettingsWindowIn {
    Show,
}

#[derive(Debug)]
pub enum SettingsWindowOut {
    ClearCache,
    Logout,
}

#[relm4::component(pub)]
impl relm4::component::Component for SettingsWindow {
    type Init = ();
    type Input = SettingsWindowIn;
    type Output = SettingsWindowOut;
    type CommandOutput = ();

    fn init(
        _: Self::Init,
        root: Self::Root,
        sender: relm4::ComponentSender<Self>,
    ) -> relm4::ComponentParts<Self> {
        let model = Self {};
        let widgets = view_output!();

        relm4::ComponentParts { model, widgets }
    }

    view! {
        #[name = "settings_window"]
        gtk::Window {
            set_widget_name: "settings-window",
            set_modal: true,
            set_hide_on_close: true,

            #[wrap(Some)]
            set_titlebar = &gtk::HeaderBar {
                add_css_class: granite::STYLE_CLASS_FLAT,
                add_css_class: granite::STYLE_CLASS_DEFAULT_DECORATION,
                set_show_title_buttons: true,
                set_visible: true,

                #[wrap(Some)]
                set_title_widget = &gtk::Label {
                    set_label: &gettext("Settings"),
                }
            },

            gtk::WindowHandle {
                gtk::Box {
                    set_widget_name: "config-menu",
                    set_orientation: gtk::Orientation::Vertical,
                    set_margin_all: 15,
                    set_spacing: 15,

                    gtk::CenterBox {
                        set_tooltip: &gettext("Wether or not send desktop notifications"),

                        #[wrap(Some)]
                        set_start_widget = &gtk::Label {
                            set_text: &gettext("Send desktop notifications\nwhen in background"),
                        },
                        #[wrap(Some)]
                        set_end_widget = &gtk::Switch {
                            set_state: Settings::get().lock().unwrap().send_notifications,
                            connect_state_set => move |_switch, value| {
                                Settings::get().lock().unwrap().send_notifications = value;
                                gtk::glib::signal::Propagation::Proceed
                            }
                        },
                    },

                    gtk::CenterBox {
                        set_tooltip: &gettext("Updates play count, played timestamp on server and the now playing page in the web app"),

                        #[wrap(Some)]
                        set_start_widget = &gtk::Label {
                            set_text: &gettext("Scrobble to server"),
                        },
                        #[wrap(Some)]
                        set_end_widget = &gtk::Switch {
                            set_state: Settings::get().lock().unwrap().scrobble,

                            connect_state_set => move |_switch, value| {
                                Settings::get().lock().unwrap().scrobble = value;
                                gtk::glib::signal::Propagation::Proceed
                            }
                        },
                    },

                    gtk::CenterBox {
                        set_tooltip: &gettext("How much of a song needs to be played to be scrobbled to server in percent"),

                        #[wrap(Some)]
                        set_start_widget = &gtk::Label {
                            set_text: &gettext("Scrobble threshold"),
                        },
                        #[wrap(Some)]
                        set_end_widget = &gtk::SpinButton {
                            set_width_request: 100,
                            set_range: (1f64, 100f64),
                            set_increments: (1f64, 1f64),
                            set_digits: 0,
                            set_value: Settings::get().lock().unwrap().scrobble_threshold as f64,

                            connect_value_changed => move |button| {
                                Settings::get().lock().unwrap().scrobble_threshold = button.value() as u32;
                            }
                        },
                    },
                    gtk::Separator {},
                    gtk::Box {
                        set_halign: gtk::Align::End,
                        gtk::Button {
                            add_css_class: "destructive-action",
                            set_label: &gettext("Delete cache"),
                            set_tooltip: &gettext("Deletes the local cache of Covers and Metadata of music. They will be redownloaded from the server on the next start"),

                            connect_clicked[sender] => move |_btn| {
                                sender.output(Self::Output::ClearCache).unwrap();
                            }
                        }
                    },
                    gtk::Box {
                        set_halign: gtk::Align::End,
                        gtk::Button {
                            add_css_class: "destructive-action",
                            set_label: &gettext("Logout from Server"),
                            set_tooltip: &gettext("Logging out will delete the cache and also require to login again to listen to music"),

                            connect_clicked[sender] => move |_btn| {
                                sender.output(Self::Output::Logout).unwrap();
                            }
                        },

                    },
                }
            }
        }
    }

    fn update_with_view(
        &mut self,
        widgets: &mut Self::Widgets,
        msg: Self::Input,
        _sender: relm4::ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match msg {
            SettingsWindowIn::Show => {
                widgets.settings_window.set_visible(true);
                widgets.settings_window.present();
            }
        }
    }
}
