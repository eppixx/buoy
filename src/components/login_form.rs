use relm4::gtk::traits::{BoxExt, ButtonExt, EntryExt, GridExt, OrientableExt, WidgetExt, EditableExt};
use relm4::{component, gtk};

use crate::settings::Settings;

#[derive(Debug, Default, Clone)]
pub struct LoginForm {
    uri: gtk::Entry,
    user: gtk::Entry,
    password: gtk::PasswordEntry,
}

#[derive(Debug)]
pub enum LoginFormOutput {
    LoggedIn,
    LoggedOut,
}

#[component(pub)]
impl relm4::SimpleComponent for LoginForm {
    type Input = ();
    type Output = LoginFormOutput;
    type Init = ();

    fn init(
        _init: Self::Init,
        root: &Self::Root,
        _sender: relm4::ComponentSender<Self>,
    ) -> relm4::ComponentParts<Self> {
        let model = LoginForm::default();
        let widgets = view_output!();

        //init widgets
        {
            let settings = Settings::get().lock().unwrap();
            if let Some(uri) = &settings.login_uri {
                model.uri.set_text(&uri);
            }
            if let Some(user) = &settings.login_username {
                model.user.set_text(&user);
            }
            if let Some(pw) = &settings.login_password {
                model.password.set_text(&pw);
            }
        }

        relm4::ComponentParts { model, widgets }
    }

    view! {
        gtk::Box {
            add_css_class: "login-form",
            set_orientation: gtk::Orientation::Vertical,
            set_spacing: 30,

            gtk::Label {
                add_css_class: "h3",
                set_label: "Login",
                set_halign: gtk::Align::Center,
            },

            gtk::Grid {
                set_row_spacing: 7,
                set_column_spacing: 7,

                attach[0, 0, 1, 1] = &gtk::Label {
                    set_label: "Server address",
                    set_halign: gtk::Align::End,
                },
                attach[1, 0, 1, 1] = &model.uri.clone() {
                    set_placeholder_text: Some("http(s)://..."),
                    // TODO validate uri
                },
                attach[0, 1, 1, 1] = &gtk::Label {
                    set_label: "User name",
                    set_halign: gtk::Align::End,
                },
                attach[1, 1, 1, 1] = &model.user.clone() {
                },
                attach[0, 2, 1, 1] = &gtk::Label {
                    set_label: "Password",
                    set_halign: gtk::Align::End,
                },
                attach[1, 2, 1, 1] = &model.password.clone() {
                },
            },

            gtk::CenterBox {
                #[wrap(Some)]
                set_start_widget = &gtk::Button {
                    add_css_class: "destructive-action",
                    set_label: "Reset login data",
                    connect_clicked[model] => move |_| {
                        model.uri.set_text("");
                        model.user.set_text("");
                        model.password.set_text("");

                        let mut settings = Settings::get().lock().unwrap();
                        settings.login_uri = None;
                        settings.login_username = None;
                        settings.login_password = None;
                        settings.save();
                    }
                },
                #[wrap(Some)]
                set_end_widget = &gtk::Button {
                    set_label: "Login",
                    //TODO check login data
                    //save them
                }
            }
        }
    }
}
