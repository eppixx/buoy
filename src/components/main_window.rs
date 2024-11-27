use std::{cell::RefCell, rc::Rc};

use granite::prelude::SettingsExt;
use relm4::{
    actions::AccelsPlus,
    gtk::{
        self,
        prelude::{ApplicationExt, BoxExt, GtkWindowExt, OrientableExt, WidgetExt},
    },
    prelude::AsyncComponentController,
};

use crate::{
    app::{App, AppIn, AppOut},
    components::login_form::LoginForm,
    gtk_helper::stack::StackExt,
    mpris::{Mpris, MprisOut},
    playback::{Playback, PlaybackOut},
    settings::Settings,
    Args,
};

use super::login_form::LoginFormOut;

enum Content {
    Loading,
    Login,
    App,
    Error,
}

impl std::fmt::Display for Content {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Loading => write!(f, "Loading"),
            Self::Login => write!(f, "Login"),
            Self::App => write!(f, "App"),
            Self::Error => write!(f, "Error"),
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
            "Error" => Ok(Self::Error),
            e => Err(format!("\"{e}\" is not a valid content")),
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
    app: Option<relm4::component::AsyncController<App>>,
}

#[derive(Debug)]
pub enum MainWindowIn {
    Playback(PlaybackOut),
    Mpris(MprisOut),
    ShowApp,
    ShowLogin,
    ShowErrorScreen,
    App(AppOut),
    LoginForm(LoginFormOut),
}

relm4::new_action_group!(WindowActionGroup, "win");
relm4::new_stateless_action!(QuitAction, WindowActionGroup, "quit-app");

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
                add_css_class: "main-window",
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
        let granite_settings =
            granite::Settings::default().expect("Unable to get the Granite settings object");
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
            app: None,
        };
        let widgets = view_output!();
        gtk::Window::set_default_icon_name("com.github.eppixx.buoy");

        // set window shortcuts
        let application = relm4::main_application();
        application.set_accelerators_for_action::<QuitAction>(&["<Primary>Q"]);
        let app = application.clone();
        let quit_action: relm4::actions::RelmAction<QuitAction> =
            relm4::actions::RelmAction::new_stateless(move |_| {
                tracing::info!("quit called");
                app.quit();
            });
        let mut group = relm4::actions::RelmActionGroup::<WindowActionGroup>::new();
        group.add_action(quit_action);
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
                tracing::info!("show error screen");
                sender.input(MainWindowIn::ShowErrorScreen);
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
            #[name = "stack"]
            gtk::Stack {
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
                                set_text: "loading information from server",
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
                add_enumed[Content::Error] = &gtk::Box {
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

                        gtk::Label {
                            add_css_class: granite::STYLE_CLASS_H3_LABEL,
                            set_text: "an error occured, you might have no connetion to your server or the server is down, if none of this is the case try deleting your cache and settings and try a new setup",
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
                if let Some(app) = &self.app {
                    app.emit(AppIn::Playback(msg));
                }
            }
            MainWindowIn::Mpris(msg) => {
                if let Some(app) = &self.app {
                    app.emit(AppIn::Mpris(msg));
                }
            }
            MainWindowIn::ShowApp => {
                let app = App::builder()
                    .launch((self.args.clone(), self.mpris.clone(), self.playback.clone()))
                    .forward(sender.input_sender(), MainWindowIn::App);
                self.content.set_child(None::<&gtk::Box>);
                self.content.set_child(Some(app.widget()));
                self.app = Some(app);
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
            },
            MainWindowIn::LoginForm(LoginFormOut::LoggedIn) => sender.input(MainWindowIn::ShowApp),
            MainWindowIn::ShowErrorScreen => widgets.stack.set_visible_child_enum(&Content::Error),
        }
    }

    fn shutdown(&mut self, widgets: &mut Self::Widgets, _sender: relm4::Sender<Self::Output>) {
        tracing::info!("shutdown MainWindow");
        self.playback.borrow_mut().shutdown().unwrap();

        //save window state
        let mut settings = Settings::get().lock().unwrap();
        settings.window_width = widgets.main_window.default_width();
        settings.window_height = widgets.main_window.default_height();
        settings.window_maximized = widgets.main_window.is_maximized();
        settings.save();
    }
}
