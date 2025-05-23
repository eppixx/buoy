use gettextrs::gettext;
use relm4::{
    component,
    gtk::{
        self,
        prelude::{BoxExt, ButtonExt, EditableExt, EntryExt, GridExt, OrientableExt, WidgetExt},
    },
};

use crate::{config, settings::Settings};

#[derive(Debug, Default, Clone)]
pub struct LoginForm {}

#[derive(Debug)]
pub enum LoginFormIn {
    AuthClicked,
    UriChanged,
    FormChanged,
    ResetClicked,
}

#[derive(Debug)]
pub enum LoginFormOut {
    LoggedIn,
    DisplayToast(String),
}

#[component(pub, async)]
impl relm4::component::AsyncComponent for LoginForm {
    type Init = ();
    type Input = LoginFormIn;
    type Output = LoginFormOut;
    type CommandOutput = ();

    async fn init(
        _init: Self::Init,
        root: Self::Root,
        sender: relm4::AsyncComponentSender<Self>,
    ) -> relm4::component::AsyncComponentParts<Self> {
        let model = LoginForm::default();
        let widgets = view_output!();

        //init widgets
        {
            let settings = Settings::get().lock().unwrap();
            if let Some(uri) = &settings.login_uri {
                widgets.uri.set_text(uri);
            }
            if let Some(user) = &settings.login_username {
                widgets.user.set_text(user);
            }
        }

        relm4::component::AsyncComponentParts { model, widgets }
    }

    view! {
        gtk::Box {
            set_widget_name: "login-form",
            set_orientation: gtk::Orientation::Vertical,
            set_spacing: 30,

            gtk::Label {
                add_css_class: granite::STYLE_CLASS_H3_LABEL,
                set_label: &gettext("Login to a Subsonic server"),
                set_halign: gtk::Align::Center,
            },

            gtk::Grid {
                set_row_spacing: 7,
                set_column_spacing: 7,

                attach[0, 0, 1, 1] = &gtk::Label {
                    set_label: &gettext("Server address"),
                    set_halign: gtk::Align::End,
                },
                attach[1, 0, 1, 1]: uri = &gtk::Entry {
                    set_hexpand: true,
                    set_placeholder_text: Some("http(s)://..."),
                    connect_changed => LoginFormIn::UriChanged,
                    connect_changed => LoginFormIn::FormChanged,
                },
                attach[0, 1, 1, 1] = &gtk::Label {
                    set_label: &gettext("User name"),
                    set_halign: gtk::Align::End,
                },
                attach[1, 1, 1, 1]: user = &gtk::Entry {
                    connect_changed => LoginFormIn::FormChanged,
                },
                attach[0, 2, 1, 1] = &gtk::Label {
                    set_label: &gettext("Password"),
                    set_halign: gtk::Align::End,
                },
                attach[1, 2, 1, 1]: password = &gtk::PasswordEntry {
                    connect_changed => LoginFormIn::FormChanged,
                },
            },

            gtk::CenterBox {
                #[wrap(Some)]
                set_start_widget = &gtk::Button {
                    add_css_class: "destructive-action",
                    set_label: &gettext("Reset login data"),
                    connect_clicked => LoginFormIn::ResetClicked,
                },
                #[wrap(Some)]
                set_end_widget = &gtk::Box {
                    set_orientation: gtk::Orientation::Horizontal,
                    set_spacing: 5,

                    append: login_btn = &gtk::Button {
                        set_label: &gettext("Login"),
                        set_sensitive: false,
                        connect_clicked => LoginFormIn::AuthClicked,
                    }
                },
            }
        }
    }

    async fn update_with_view(
        &mut self,
        widgets: &mut Self::Widgets,
        msg: Self::Input,
        sender: relm4::AsyncComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match msg {
            LoginFormIn::AuthClicked => {
                let auth = submarine::auth::AuthBuilder::new(widgets.user.text(), "0.16.1")
                    .client_name(config::APP_NAME)
                    .hashed(&widgets.password.text());
                let hash = auth.hash.clone();
                let salt = auth.salt.clone();
                let client = submarine::Client::new(&widgets.uri.text(), auth);
                match client.ping().await {
                    Ok(_) => {
                        {
                            let mut settings = Settings::get().lock().unwrap();
                            settings.login_uri = Some(widgets.uri.text().to_string());
                            settings.login_username = Some(widgets.user.text().to_string());
                            settings.login_hash = Some(hash);
                            settings.login_salt = Some(salt);
                            if let Err(e) = settings.save() {
                                sender
                                    .output(LoginFormOut::DisplayToast(format!(
                                        "error while saving settings: {e}"
                                    )))
                                    .unwrap();
                            }
                        }
                        sender.output(LoginFormOut::LoggedIn).unwrap();
                    }
                    Err(e) => {
                        use submarine::SubsonicError;
                        let error_str = match e {
                            SubsonicError::Connection(_) => {
                                gettext("Connection error. Is the address valid?")
                            }
                            SubsonicError::NoServerFound => {
                                gettext("Subsonic server not found. Is the address correct")
                            }
                            SubsonicError::Server(_) => gettext("Username or password is wrong"),
                            e => format!("{}: {e}", gettext("Login error")),
                        };
                        sender
                            .output(LoginFormOut::DisplayToast(error_str))
                            .unwrap();
                    }
                }
            }
            LoginFormIn::ResetClicked => {
                widgets.uri.set_text("");
                widgets.user.set_text("");
                widgets.password.set_text("");

                let mut settings = Settings::get().lock().unwrap();
                settings.login_uri = None;
                settings.login_username = None;
                settings.login_hash = None;
                if let Err(e) = settings.save() {
                    sender
                        .output(LoginFormOut::DisplayToast(format!(
                            "error while saving settings: {e}"
                        )))
                        .unwrap();
                }
            }
            LoginFormIn::UriChanged => match url::Url::parse(&widgets.uri.text()) {
                Ok(_) => {
                    widgets.uri.set_secondary_icon_name(None);
                }
                Err(e) => {
                    widgets.uri.set_secondary_icon_name(Some("dialog-error"));
                    let error_str = match e {
                        url::ParseError::EmptyHost => &gettext("Address is empty"),
                        url::ParseError::InvalidPort => &gettext("Port is invalid"),
                        url::ParseError::InvalidIpv4Address => &gettext("Invalid IPv4 address"),
                        url::ParseError::InvalidIpv6Address => &gettext("Invalid IPv6 address"),
                        url::ParseError::InvalidDomainCharacter => {
                            &gettext("Address contains invalid character")
                        }
                        e => &format!("{}: {e}", &gettext("Address is invalid")),
                    };
                    widgets.uri.set_secondary_icon_tooltip_text(Some(error_str));
                }
            },
            LoginFormIn::FormChanged => {
                //check form if input text is ok
                let sensitive = !widgets.user.text().is_empty()
                    && !widgets.password.text().is_empty()
                    && url::Url::parse(&widgets.uri.text()).is_ok();
                widgets.login_btn.set_sensitive(sensitive);
            }
        }
    }
}
