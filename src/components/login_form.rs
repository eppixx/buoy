use relm4::gtk::traits::{
    BoxExt, ButtonExt, EditableExt, EntryExt, GridExt, OrientableExt, WidgetExt,
};
use relm4::{component, gtk, RelmWidgetExt};

use crate::settings::Settings;

#[derive(Debug, Default, Clone)]
pub struct LoginForm {
    uri: gtk::Entry,
    user: gtk::Entry,
    password: gtk::PasswordEntry,
    error_icon: gtk::Image,
    login_btn: gtk::Button,
}

#[derive(Debug)]
pub enum LoginFormInput {
    AuthClicked,
    UriChanged,
    FormChanged,
    ResetClicked,
}

#[derive(Debug)]
pub enum LoginFormOutput {
    LoggedIn(submarine::Client),
}

#[component(pub, async)]
impl relm4::component::AsyncComponent for LoginForm {
    type Input = LoginFormInput;
    type Output = LoginFormOutput;
    type Init = ();
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
                model.uri.set_text(uri);
            }
            if let Some(user) = &settings.login_username {
                model.user.set_text(user);
            }
        }

        relm4::component::AsyncComponentParts { model, widgets }
    }

    view! {
        gtk::Box {
            add_css_class: "login-form",
            set_orientation: gtk::Orientation::Vertical,
            set_spacing: 30,

            gtk::Label {
                add_css_class: "h3",
                set_label: "Login to a Subsonic server",
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
                    connect_changed => LoginFormInput::UriChanged,
                    connect_changed => LoginFormInput::FormChanged,
                },
                attach[0, 1, 1, 1] = &gtk::Label {
                    set_label: "User name",
                    set_halign: gtk::Align::End,
                },
                attach[1, 1, 1, 1] = &model.user.clone() {
                    connect_changed => LoginFormInput::FormChanged,
                },
                attach[0, 2, 1, 1] = &gtk::Label {
                    set_label: "Password",
                    set_halign: gtk::Align::End,
                },
                attach[1, 2, 1, 1] = &model.password.clone() {
                    connect_changed => LoginFormInput::FormChanged,
                },
            },

            gtk::CenterBox {
                #[wrap(Some)]
                set_start_widget = &gtk::Button {
                    add_css_class: "destructive-action",
                    set_label: "Reset login data",
                    connect_clicked => LoginFormInput::ResetClicked,
                },
                #[wrap(Some)]
                set_end_widget = &gtk::Box {
                    set_orientation: gtk::Orientation::Horizontal,
                    set_spacing: 5,

                    model.error_icon.clone() -> gtk::Image {
                        set_icon_name: Some("dialog-error"),
                        set_visible: false,
                        // set_visible: true,
                    },
                    model.login_btn.clone() -> gtk::Button {
                        set_label: "Login",
                        set_sensitive: false,
                        connect_clicked => LoginFormInput::AuthClicked,
                    }
                },
            }
        }
    }

    async fn update(
        &mut self,
        msg: Self::Input,
        sender: relm4::AsyncComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match msg {
            LoginFormInput::AuthClicked => {
                let auth = submarine::auth::AuthBuilder::new(self.user.text(), "0.16.1")
                    .client_name("Bouy")
                    .hashed(&self.password.text());
                let hash = auth.hash.clone();
                let salt = auth.salt.clone();
                let client = submarine::Client::new(&self.uri.text(), auth);
                match client.ping().await {
                    Ok(_) => {
                        {
                            let mut settings = Settings::get().lock().unwrap();
                            settings.login_uri = Some(self.uri.text().to_string());
                            settings.login_username = Some(self.user.text().to_string());
                            settings.login_hash = Some(hash);
                            settings.login_salt = Some(salt);
                            settings.save();
                        }
                        sender.output(LoginFormOutput::LoggedIn(client)).unwrap();
                    }
                    Err(e) => {
                        use submarine::SubsonicError;
                        let error_str = match e {
                            SubsonicError::Connection(_) => {
                                "Connection error. Is the address valid?"
                            }
                            SubsonicError::NoServerFound => {
                                "Subsonic server not found. Is the address correct"
                            }
                            SubsonicError::Server(_) => "Username or password is wrong",
                            _ => "Login error",
                        };
                        println!("error: {error_str}");
                        self.error_icon.set_visible(true);
                        self.error_icon.set_tooltip(error_str);
                    }
                }
            }
            LoginFormInput::ResetClicked => {
                self.uri.set_text("");
                self.user.set_text("");
                self.password.set_text("");

                let mut settings = Settings::get().lock().unwrap();
                settings.login_uri = None;
                settings.login_username = None;
                settings.login_hash = None;
                settings.save();
            }
            LoginFormInput::UriChanged => match url::Url::parse(&self.uri.text()) {
                Ok(_) => {
                    self.uri.set_secondary_icon_name(None);
                }
                Err(e) => {
                    self.uri.set_secondary_icon_name(Some("dialog-error"));
                    let error_str = match e {
                        url::ParseError::EmptyHost => "Address is empty",
                        url::ParseError::InvalidPort => "Port is invalid",
                        url::ParseError::InvalidIpv4Address => "Invalid IPv4 address",
                        url::ParseError::InvalidIpv6Address => "Invalid IPv6 address",
                        url::ParseError::InvalidDomainCharacter => {
                            "Address contains invalid character"
                        }
                        _ => "Address is invalid",
                    };
                    self.uri.set_secondary_icon_tooltip_text(Some(error_str));
                }
            },
            LoginFormInput::FormChanged => {
                self.error_icon.set_visible(false);
                //check form if input text is ok
                let sensitive = !self.user.text().is_empty()
                    && !self.password.text().is_empty()
                    && url::Url::parse(&self.uri.text()).is_ok();
                self.login_btn.set_sensitive(sensitive);
            }
        }
    }
}
