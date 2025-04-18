use std::{cell::RefCell, rc::Rc};

use gettextrs::gettext;
use granite::prelude::{SettingsExt, ToastExt};
use relm4::{
    actions::AccelsPlus,
    gtk::{
        self,
        prelude::{ApplicationExt, BoxExt, ButtonExt, GtkWindowExt, OrientableExt, WidgetExt},
    },
    prelude::AsyncComponentController,
    RelmWidgetExt,
};

use crate::{
    app::{App, AppIn, AppOut},
    components::login_form::{LoginForm, LoginFormOut},
    config,
    gtk_helper::stack::StackExt,
    mpris::{Mpris, MprisOut},
    playback::{Playback, PlaybackOut},
    settings::Settings,
    views::ClickableViews,
    Args,
};

enum Content {
    Loading,
    Login,
    App,
    NoConnection,
}

impl std::fmt::Display for Content {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Loading => write!(f, "Loading"),
            Self::Login => write!(f, "Login"),
            Self::App => write!(f, "App"),
            Self::NoConnection => write!(f, "NoConnection"),
        }
    }
}

impl TryFrom<String> for Content {
    type Error = String;

    fn try_from(value: String) -> Result<Self, String> {
        match value.as_ref() {
            "Loading" => Ok(Self::Loading),
            "Login" => Ok(Self::Login),
            "App" => Ok(Self::App),
            "NoConnection" => Ok(Self::NoConnection),
            e => Err(format!("\"{e}\" is not a valid Content")),
        }
    }
}

#[derive(Debug)]
pub struct MainWindow {
    args: Rc<RefCell<Args>>,
    playback: Rc<RefCell<Playback>>,
    mpris: Rc<RefCell<Mpris>>,

    content: gtk::Viewport,
    login_form: relm4::component::AsyncController<LoginForm>,
    app: Rc<RefCell<Option<relm4::component::AsyncController<App>>>>,
}

#[derive(Debug)]
pub enum MainWindowIn {
    Playback(PlaybackOut),
    Mpris(MprisOut),
    ShowApp,
    ShowLogin,
    ShowNoConnection,
    App(AppOut),
    LoginForm(LoginFormOut),
    Logout,
    RetryLogin,
    DisplayToast(String),
}

relm4::new_action_group!(WindowActionGroup, "win");
relm4::new_stateless_action!(QuitAction, WindowActionGroup, "quit-app");
relm4::new_stateless_action!(ActivateSearchAction, WindowActionGroup, "activate-search");
relm4::new_stateless_action!(SwitchToDashboard, WindowActionGroup, "switch-to-dashboard");
relm4::new_stateless_action!(SwitchToArtists, WindowActionGroup, "switch-to-artists");
relm4::new_stateless_action!(SwitchToAlbums, WindowActionGroup, "switch-to-albums");
relm4::new_stateless_action!(SwitchToTracks, WindowActionGroup, "switch-to-tracks");
relm4::new_stateless_action!(SwitchToPlaylists, WindowActionGroup, "switch-to-playlists");

#[relm4::component(async, pub)]
impl relm4::component::AsyncComponent for MainWindow {
    type Init = Rc<RefCell<Args>>;
    type Input = MainWindowIn;
    type Output = ();
    type CommandOutput = ();

    fn init_loading_widgets(root: Self::Root) -> Option<relm4::loading_widgets::LoadingWidgets> {
        relm4::view! {
            #[local]
            root {
                set_widget_name: "main-window",
                set_default_width: Settings::get().lock().unwrap().window_width,
                set_default_height: Settings::get().lock().unwrap().window_height,
                set_maximized: Settings::get().lock().unwrap().window_maximized,

                //remove the titlebar and add WindowControl to the other widgets
                #[wrap(Some)]
                set_titlebar = &gtk::HeaderBar {
                    add_css_class: granite::STYLE_CLASS_FLAT,
                    add_css_class: granite::STYLE_CLASS_DEFAULT_DECORATION,
                    set_show_title_buttons: false,
                    set_visible: false,
                },

                #[name(spinner)]
                gtk::Spinner {
                    start: (),
                    set_halign: gtk::Align::Center,
                    set_valign: gtk::Align::Center,
                    set_vexpand: true,
                    set_hexpand: true,
                }
            }
        }
        Some(relm4::loading_widgets::LoadingWidgets::new(root, spinner))
    }

    async fn init(
        args: Self::Init,
        root: Self::Root,
        sender: relm4::AsyncComponentSender<Self>,
    ) -> relm4::component::AsyncComponentParts<Self> {
        // decide if dark or white style; also watch if style changes
        let gtk_settings = gtk::Settings::default().expect("Unable to get the GtkSettings object");
        let granite_settings = granite::Settings::default();
        gtk_settings.set_gtk_application_prefer_dark_theme(
            granite_settings.prefers_color_scheme() == granite::SettingsColorScheme::Dark,
        );
        granite_settings.connect_prefers_color_scheme_notify(move |granite_settings| {
            gtk_settings.set_gtk_application_prefer_dark_theme(
                granite_settings.prefers_color_scheme() == granite::SettingsColorScheme::Dark,
            );
        });

        let (playback, receiver) = Playback::new().unwrap();
        let (mpris, mpris_receiver) = Mpris::new(&args).await.unwrap();

        let login_form = LoginForm::builder()
            .launch(())
            .forward(sender.input_sender(), MainWindowIn::LoginForm);

        let model = Self {
            args,
            playback: Rc::new(RefCell::new(playback)),
            mpris: Rc::new(RefCell::new(mpris)),

            content: gtk::Viewport::default(),
            login_form,
            app: Rc::new(RefCell::new(None)),
        };
        let widgets = view_output!();
        gtk::Window::set_default_icon_name(config::APP_ID);

        // set window shortcuts
        let application = relm4::main_application();
        application.set_accelerators_for_action::<QuitAction>(&["<Primary>Q"]);
        application.set_accelerators_for_action::<ActivateSearchAction>(&["<Primary>F"]);
        application.set_accelerators_for_action::<SwitchToDashboard>(&["<Primary>1", "<Primary>D"]);
        application.set_accelerators_for_action::<SwitchToArtists>(&["<Primary>2", "<Primary>R"]);
        application.set_accelerators_for_action::<SwitchToAlbums>(&["<Primary>3", "<Primary>L"]);
        application.set_accelerators_for_action::<SwitchToTracks>(&["<Primary>4", "<Primary>T"]);
        application.set_accelerators_for_action::<SwitchToPlaylists>(&["<Primary>5", "<Primary>P"]);
        let app = application.clone();

        let quit_action: relm4::actions::RelmAction<QuitAction> =
            relm4::actions::RelmAction::new_stateless(move |_| {
                tracing::info!("keyboard shortcut quit called");
                app.quit();
            });
        let app = model.app.clone();
        let activate_search: relm4::actions::RelmAction<ActivateSearchAction> =
            relm4::actions::RelmAction::new_stateless(move |_| {
                if let Some(ref app) = *app.borrow() {
                    tracing::info!("keyboard shortcut open search bar");
                    app.emit(AppIn::SearchActivate(true));
                }
            });
        let app = model.app.clone();
        let switch_to_dashboard: relm4::actions::RelmAction<SwitchToDashboard> =
            relm4::actions::RelmAction::new_stateless(move |_| {
                if let Some(ref app) = *app.borrow() {
                    tracing::info!("keyboard shortcut switch to dashboard");
                    app.emit(AppIn::ClickedNavigationBtn(ClickableViews::Dashboard));
                }
            });
        let app = model.app.clone();
        let switch_to_artists: relm4::actions::RelmAction<SwitchToArtists> =
            relm4::actions::RelmAction::new_stateless(move |_| {
                if let Some(ref app) = *app.borrow() {
                    tracing::info!("keyboard shortcut switch to artists");
                    app.emit(AppIn::ClickedNavigationBtn(ClickableViews::Artists));
                }
            });
        let app = model.app.clone();
        let switch_to_albums: relm4::actions::RelmAction<SwitchToAlbums> =
            relm4::actions::RelmAction::new_stateless(move |_| {
                if let Some(ref app) = *app.borrow() {
                    tracing::info!("keyboard shortcut switch to albums");
                    app.emit(AppIn::ClickedNavigationBtn(ClickableViews::Albums));
                }
            });
        let app = model.app.clone();
        let switch_to_tracks: relm4::actions::RelmAction<SwitchToTracks> =
            relm4::actions::RelmAction::new_stateless(move |_| {
                if let Some(ref app) = *app.borrow() {
                    tracing::info!("keyboard shortcut switch to tracks");
                    app.emit(AppIn::ClickedNavigationBtn(ClickableViews::Tracks));
                }
            });
        let app = model.app.clone();
        let switch_to_playlists: relm4::actions::RelmAction<SwitchToPlaylists> =
            relm4::actions::RelmAction::new_stateless(move |_| {
                if let Some(ref app) = *app.borrow() {
                    tracing::info!("keyboard shortcut switch to playlists");
                    app.emit(AppIn::ClickedNavigationBtn(ClickableViews::Playlists));
                }
            });

        let mut group = relm4::actions::RelmActionGroup::<WindowActionGroup>::new();
        group.add_action(quit_action);
        group.add_action(activate_search);
        group.add_action(switch_to_dashboard);
        group.add_action(switch_to_artists);
        group.add_action(switch_to_albums);
        group.add_action(switch_to_tracks);
        group.add_action(switch_to_playlists);
        group.register_for_widget(&widgets.main_window);

        {
            let settings = Settings::get().lock().unwrap().clone();
            widgets.main_window.set_maximized(settings.window_maximized);

            // decide which content to show
            if !settings.login_set() {
                tracing::info!("show login form");
                sender.input(MainWindowIn::ShowLogin);
            } else if settings.valid_login().await {
                tracing::info!("show app");
                sender.input(MainWindowIn::ShowApp);
            } else {
                tracing::info!("show no connection screen");
                sender.input(MainWindowIn::ShowNoConnection);
            }
        }

        //setup mpris
        let sender_mpris = sender.clone();
        gtk::glib::spawn_future_local(async move {
            while let Ok(msg) = mpris_receiver.recv().await {
                sender_mpris.input(MainWindowIn::Mpris(msg));
            }
        });

        // setup playback
        gtk::glib::spawn_future_local(async move {
            while let Ok(msg) = receiver.recv().await {
                sender.input(MainWindowIn::Playback(msg));
            }
        });

        relm4::component::AsyncComponentParts { model, widgets }
    }

    view! {
        main_window = gtk::Window {

            gtk::Overlay {
                add_overlay: toasts = &granite::Toast,

                #[wrap(Some)]
                set_child: stack = &gtk::Stack {
                    add_css_class: "main-box",
                    set_transition_type: gtk::StackTransitionType::Crossfade,
                    set_transition_duration: 200,

                    // use this as default while loading
                    add_enumed[Content::Loading] = &gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,

                        gtk::HeaderBar {
                            add_css_class: granite::STYLE_CLASS_FLAT,
                            add_css_class: granite::STYLE_CLASS_DEFAULT_DECORATION,
                        },
                        gtk::WindowHandle {
                            set_vexpand: true,
                            set_valign: gtk::Align::Center,
                            set_halign: gtk::Align::Center,

                            gtk::Box {
                                set_orientation: gtk::Orientation::Vertical,
                                set_spacing: 10,

                                gtk::Spinner {
                                    start: ()
                                },
                                gtk::Label {
                                    set_text: &gettext("Loading information from server"),
                                    add_css_class: granite::STYLE_CLASS_H2_LABEL,
                                }
                            }
                        }
                    },
                    add_enumed[Content::Login] = &gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,

                        gtk::HeaderBar {
                            add_css_class: granite::STYLE_CLASS_FLAT,
                            add_css_class: granite::STYLE_CLASS_DEFAULT_DECORATION,
                        },

                        gtk::WindowHandle {
                            set_vexpand: true,

                            model.login_form.widget() {
                                set_halign: gtk::Align::Center,
                                set_valign: gtk::Align::Center,
                            }
                        }
                    },
                    add_enumed[Content::App] = &model.content.clone() -> gtk::Viewport {},
                    add_enumed[Content::NoConnection] = &gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,

                        gtk::HeaderBar {
                            add_css_class: granite::STYLE_CLASS_FLAT,
                            add_css_class: granite::STYLE_CLASS_DEFAULT_DECORATION,
                        },

                        gtk::WindowHandle {
                            set_hexpand: true,
                            set_vexpand: true,
                            set_halign: gtk::Align::Center,
                            set_valign: gtk::Align::Center,

                            gtk::Box {
                                set_margin_all: 15,
                                set_spacing: 20,
                                set_halign: gtk::Align::Center,

                                gtk::Image {
                                    set_icon_name: Some("network-error"),
                                    set_pixel_size: 64,
                                },

                                gtk::Box {
                                    set_orientation: gtk::Orientation::Vertical,
                                    set_spacing: 20,
                                    set_margin_all: 7,

                                    gtk::Label {
                                        add_css_class: granite::STYLE_CLASS_H2_LABEL,
                                        set_halign: gtk::Align::Start,
                                        set_text: &gettext("Can't connect to subsonic server"),
                                    },
                                    gtk::Label {
                                        add_css_class: granite::STYLE_CLASS_H3_LABEL,
                                        set_wrap: true,
                                        set_text: &gettext("Make sure the connection to the server is available. You might want to try later or connect to another server"),
                                    },
                                    gtk::Box {
                                        set_halign: gtk::Align::End,
                                        set_spacing: 10,

                                        gtk::Button {
                                            set_label: &gettext("Retry connecting"),
                                            connect_clicked => MainWindowIn::RetryLogin,
                                        },
                                        gtk::Button {
                                            add_css_class: "destructive-action",
                                            set_label: &gettext("Logout"),
                                            connect_clicked => MainWindowIn::Logout,
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
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
            MainWindowIn::Playback(msg) => {
                if let Some(ref app) = *self.app.borrow() {
                    app.emit(AppIn::Playback(msg));
                }
            }
            MainWindowIn::Mpris(msg) => {
                if let Some(ref app) = *self.app.borrow() {
                    app.emit(AppIn::Mpris(msg));
                }
            }
            MainWindowIn::ShowApp => {
                //reset app
                self.app.replace(None);
                self.content.set_child(None::<&gtk::Box>);

                //load app
                let app = App::builder()
                    .launch((self.args.clone(), self.mpris.clone(), self.playback.clone()))
                    .forward(sender.input_sender(), MainWindowIn::App);
                self.content.set_child(Some(app.widget()));
                self.app.replace(Some(app));
                widgets.stack.set_visible_child_enum(&Content::App);
            }
            MainWindowIn::ShowLogin => {
                let login = LoginForm::builder()
                    .launch(())
                    .forward(sender.input_sender(), MainWindowIn::LoginForm);
                self.content.set_child(Some(login.widget()));
                widgets.stack.set_visible_child_enum(&Content::Login);
            }
            MainWindowIn::App(msg) => match msg {
                AppOut::Logout => sender.input(MainWindowIn::ShowLogin),
                AppOut::Reload => sender.input(MainWindowIn::ShowApp),
                AppOut::DisplayToast(msg) => sender.input(MainWindowIn::DisplayToast(msg)),
            },
            MainWindowIn::LoginForm(msg) => match msg {
                LoginFormOut::LoggedIn => sender.input(MainWindowIn::ShowApp),
                LoginFormOut::DisplayToast(msg) => sender.input(MainWindowIn::DisplayToast(msg)),
            },
            MainWindowIn::ShowNoConnection => {
                widgets.stack.set_visible_child_enum(&Content::NoConnection)
            }
            MainWindowIn::Logout => {
                tracing::info!("logging out");

                if let Some(ref app) = *self.app.borrow() {
                    app.emit(AppIn::ClearCache);
                }

                let mut settings = Settings::get().lock().unwrap();
                if let Err(e) = settings.reset_login() {
                    sender.input(MainWindowIn::DisplayToast(format!("error on logout: {e}")));
                }
                crate::client::Client::get_mut().lock().unwrap().reset();

                sender.input(MainWindowIn::ShowLogin);
            }
            MainWindowIn::RetryLogin => {
                tracing::info!("retry login");
                widgets.stack.set_visible_child_enum(&Content::Loading);
                {
                    let settings = Settings::get().lock().unwrap().clone();
                    widgets.main_window.set_maximized(settings.window_maximized);

                    // decide which content to show
                    if !settings.login_set() {
                        tracing::info!("show login form");
                        sender.input(MainWindowIn::ShowLogin);
                    } else if settings.valid_login().await {
                        tracing::info!("show app");
                        sender.input(MainWindowIn::ShowApp);
                    } else {
                        tracing::info!("show error screen");
                        sender.input(MainWindowIn::ShowNoConnection);
                    }
                }
            }
            MainWindowIn::DisplayToast(title) => {
                tracing::error!(title);
                widgets.toasts.set_title(&title);
                widgets.toasts.send_notification();
            }
        }
    }

    fn shutdown(&mut self, widgets: &mut Self::Widgets, _sender: relm4::Sender<Self::Output>) {
        tracing::info!("shutdown MainWindow");
        if let Err(e) = self.playback.borrow_mut().shutdown() {
            tracing::error!("error on playback shutdown: {e}");
        }

        //save window state
        let mut settings = Settings::get().lock().unwrap();
        settings.window_width = widgets.main_window.default_width();
        settings.window_height = widgets.main_window.default_height();
        settings.window_maximized = widgets.main_window.is_maximized();
        if let Err(e) = settings.save() {
            tracing::error!("error while saving: {e}");
        }
    }
}
