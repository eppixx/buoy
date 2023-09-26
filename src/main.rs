use gtk::prelude::{BoxExt, ButtonExt, GtkWindowExt, OrientableExt, ScaleButtonExt};
use relm4::{
    component::{AsyncComponent, AsyncComponentController, AsyncController},
    gtk::{
        self,
        gio::SimpleAction,
        prelude::{ActionMapExt, ApplicationExt},
        traits::{GtkApplicationExt, PopoverExt, WidgetExt},
    },
    Component, ComponentController, ComponentParts, ComponentSender, Controller, RelmApp,
    SimpleComponent,
};

use crate::components::{
    browser::Browser,
    equalizer::{Equalizer, EqualizerOut},
    login_form::{LoginForm, LoginFormOut},
    play_controls::PlayControlIn,
    play_controls::{PlayControl, PlayControlOut},
    play_info::PlayInfo,
    play_info::PlayInfoIn,
    queue::QueueOut,
    queue::{Queue, QueueIn},
    seekbar::SeekbarOut,
    seekbar::{Seekbar, SeekbarCurrent, SeekbarIn},
};
use crate::{
    client::Client,
    play_state::PlayState,
    playback::{Playback, PlaybackOutput},
    settings::Settings,
};

pub mod cache;
pub mod client;
mod components;
pub mod css;
mod factory;
mod play_state;
mod playback;
pub mod settings;
pub mod types;

struct App {
    playback: Playback,

    login_form: AsyncController<LoginForm>,
    queue: Controller<Queue>,
    play_controls: Controller<PlayControl>,
    seekbar: Controller<Seekbar>,
    play_info: Controller<PlayInfo>,
    browser: Controller<Browser>,
    equalizer: Controller<Equalizer>,

    main_stack: gtk::Stack,
    equalizer_btn: gtk::MenuButton,
    volume_btn: gtk::VolumeButton,
    config_btn: gtk::MenuButton,
}

#[derive(Debug)]
enum AppIn {
    ResetLogin,
    DeleteCache,
    PlayControlOutput(PlayControlOut),
    Seekbar(SeekbarOut),
    VolumeChange(f64),
    Playback(PlaybackOutput),
    LoginForm(LoginFormOut),
    Equalizer(EqualizerOut),
    Queue(Box<QueueOut>),
}

#[relm4::component]
impl SimpleComponent for App {
    type Input = AppIn;
    type Output = ();
    type Init = ();

    // Initialize the UI.
    fn init(
        _init: Self::Init,
        root: &Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let (playback_sender, receiver) =
            gtk::glib::MainContext::channel(gtk::glib::Priority::default());
        let playback = Playback::new(&playback_sender).unwrap();

        let login_form: AsyncController<LoginForm> = LoginForm::builder()
            .launch(())
            .forward(sender.input_sender(), AppIn::LoginForm);
        let queue: Controller<Queue> = Queue::builder()
            .launch(())
            .forward(sender.input_sender(), |msg| AppIn::Queue(Box::new(msg)));
        let play_controls = PlayControl::builder()
            .launch(PlayState::Pause) // TODO change to previous state
            .forward(sender.input_sender(), AppIn::PlayControlOutput);
        let seekbar = Seekbar::builder()
            .launch(Some(SeekbarCurrent::new(1000 * 60, None))) // TODO change to previous state
            .forward(sender.input_sender(), AppIn::Seekbar);
        let play_info = PlayInfo::builder()
            .launch(None) // TODO change to previous state
            .detach();
        let browser = Browser::builder().launch(()).detach();
        let equalizer = Equalizer::builder()
            .launch(())
            .forward(sender.input_sender(), AppIn::Equalizer);

        let model = App {
            playback,

            login_form,
            queue,
            play_controls,
            seekbar,
            play_info,
            browser,
            equalizer,

            main_stack: gtk::Stack::default(),
            volume_btn: gtk::VolumeButton::default(),
            equalizer_btn: gtk::MenuButton::default(),
            config_btn: gtk::MenuButton::default(),
        };

        let widgets = view_output!();

        //init widgets
        {
            let settings = Settings::get().lock().unwrap();
            model.volume_btn.set_value(settings.volume);
        }

        receiver.attach(None, move |msg| {
            sender.input(AppIn::Playback(msg));
            gtk::prelude::Continue(true)
        });

        {
            let client = Client::get().lock().unwrap();
            model.config_btn.set_sensitive(client.inner.is_some());
            model.equalizer_btn.set_sensitive(client.inner.is_some());
            model.volume_btn.set_sensitive(client.inner.is_some());

            match &client.inner {
                Some(_client) => model.main_stack.set_visible_child_name("logged-in"),
                None => model.main_stack.set_visible_child_name("login-form"),
            }
        }

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>) {
        match msg {
            AppIn::PlayControlOutput(PlayControlOut::Next) => {
                _ = self.queue.sender().send(QueueIn::PlayNext);
            }
            AppIn::PlayControlOutput(PlayControlOut::Previous) => {
                _ = self.queue.sender().send(QueueIn::PlayPrevious);
            }
            AppIn::PlayControlOutput(PlayControlOut::Status(status)) => {
                match status {
                    PlayState::Pause => self.playback.pause().unwrap(),
                    PlayState::Play => self.playback.play().unwrap(),
                    PlayState::Stop => self.playback.stop().unwrap(),
                };
                self.queue.sender().send(QueueIn::NewState(status)).unwrap();
            }
            AppIn::Seekbar(msg) => match msg {
                SeekbarOut::SeekDragged(seek_in_ms) => self.playback.set_position(seek_in_ms),
            },
            AppIn::VolumeChange(value) => {
                self.playback.set_volume(value);
                let mut settings = Settings::get().lock().unwrap();
                settings.volume = value;
                settings.save();
            }
            AppIn::Playback(playback) => {
                match playback {
                    PlaybackOutput::TrackEnd => {} //TODO play next
                    PlaybackOutput::Seek(ms) => {
                        self.seekbar.emit(SeekbarIn::SeekTo(ms));
                        self.play_controls
                            .emit(PlayControlIn::NewState(PlayState::Play));
                    }
                }
            }
            AppIn::LoginForm(client) => match client {
                LoginFormOut::LoggedIn => {
                    self.main_stack.set_visible_child_name("logged-in");
                    self.config_btn.set_sensitive(true);
                    self.equalizer_btn.set_sensitive(true);
                    self.volume_btn.set_sensitive(true);
                }
            },
            AppIn::Equalizer(_changed) => {
                self.playback.sync_equalizer();
            }
            AppIn::ResetLogin => {
                let mut settings = Settings::get().lock().unwrap();
                settings.reset_login();
                self.main_stack.set_visible_child_name("login-form");
                self.config_btn.set_sensitive(false);
                self.equalizer_btn.set_sensitive(false);
                self.volume_btn.set_sensitive(false);
            }
            AppIn::Queue(msg) => match *msg {
                QueueOut::Play(child) => {
                    // update playcontrol
                    self.play_info.emit(PlayInfoIn::NewState(child.clone()));

                    // set playback
                    let client = Client::get().lock().unwrap().inner.clone().unwrap();
                    match client.stream_url(
                        child.id,
                        None,
                        None::<&str>,
                        None,
                        None::<&str>,
                        None,
                        None,
                    ) {
                        Ok(url) => {
                            self.playback.set_track(url);
                            if let Some(length) = child.duration {
                                self.seekbar.emit(SeekbarIn::NewRange(length as i64 * 1000));
                            } else {
                                self.seekbar.emit(SeekbarIn::NewRange(0));
                            }
                            self.playback.play().unwrap();
                        }
                        Err(_) => {} //TODO error handling
                    }
                }
            },
            AppIn::DeleteCache => {
                //TODO delete cache
                tracing::error!("cache button pressed");
            }
        }
    }

    view! {
        #[root]
        gtk::Window {
            add_css_class: "main-window",
            set_title: Some("Bouy"),
            set_default_width: 900,
            set_default_height: 700,

            #[wrap(Some)]
            set_titlebar = &gtk::WindowHandle {
                gtk::Box {
                    add_css_class: "window-titlebar",

                    gtk::WindowControls {
                        set_side: gtk::PackType::Start,
                    },

                    //title
                    gtk::Label {
                        set_markup: "<span weight=\"bold\">Bouy</span>",
                        set_hexpand: true,
                    },

                    append = &model.equalizer_btn.clone() -> gtk::MenuButton {
                        set_icon_name: "media-eq-symbolic",
                        set_focus_on_click: false,
                        #[wrap(Some)]
                        set_popover: equalizer_popover = &gtk::Popover {
                            model.equalizer.widget(),
                        },
                    },

                    append = &model.volume_btn.clone() -> gtk::VolumeButton {
                        set_focus_on_click: false,
                        connect_value_changed[sender] => move |_scale, value| {
                            sender.input(AppIn::VolumeChange(value));
                        }
                    },

                    append = &model.config_btn.clone() -> gtk::MenuButton {
                        set_icon_name: "open-menu-symbolic",
                        set_focus_on_click: false,

                        #[wrap(Some)]
                        set_popover: popover = &gtk::Popover {
                            set_position: gtk::PositionType::Right,

                            gtk::Box {
                                add_css_class: "config-menu",
                                set_orientation: gtk::Orientation::Vertical,
                                set_spacing: 15,

                                gtk::Button {
                                    add_css_class: "destructive-action",
                                    set_label: "Logout from Server",
                                    connect_clicked => AppIn::ResetLogin,
                                },
                                gtk::Button {
                                    add_css_class: "destructive-action",
                                    set_label: "Delete cache",
                                    connect_clicked => AppIn::DeleteCache,
                                }
                            },
                        },
                    },

                    gtk::WindowControls {
                        set_side: gtk::PackType::End,
                    },
                },
            },

            model.main_stack.clone() -> gtk::Stack {
                add_css_class: "main-box",
                set_transition_type: gtk::StackTransitionType::Crossfade,
                set_transition_duration: 200,

                add_child = &gtk::WindowHandle {
                    set_hexpand: true,
                    set_vexpand: true,
                    set_halign: gtk::Align::Center,
                    set_valign: gtk::Align::Center,

                    gtk::Box {
                        set_hexpand: true,
                        set_vexpand: true,
                        model.login_form.widget() {}
                    }
                } -> {
                    set_name: "login-form",
                },
                add_child = &gtk::Box {
                    add_css_class: "main-box",
                    set_orientation: gtk::Orientation::Vertical,

                    gtk::WindowHandle {
                        gtk::Box {
                            set_spacing: 5,

                            model.play_info.widget(),
                            gtk::Box {
                                set_orientation: gtk::Orientation::Vertical,
                                set_spacing: 12,
                                set_valign: gtk::Align::Center,

                                append: model.play_controls.widget(),
                                append: model.seekbar.widget(),
                            }
                        },
                    },
                    gtk::Paned {
                        add_css_class: "main-paned",
                        // set_wide_handle: true,
                        set_position: 300, // TODO set from previous state

                        set_start_child: Some(model.queue.widget()),
                        set_end_child: Some(model.browser.widget()),
                    },
                } -> {
                    set_name: "logged-in",
                },
            },
        }
    }

    fn shutdown(&mut self, _widgets: &mut Self::Widgets, _output: relm4::Sender<Self::Output>) {
        tracing::error!("shutdown called");
        self.playback.shutdown().unwrap();
        Settings::get().lock().unwrap().save();
        // Cache::get().lock().await.save();
    }
}

fn main() -> anyhow::Result<()> {
    //enable logging
    tracing_subscriber::fmt()
        .with_span_events(tracing_subscriber::fmt::format::FmtSpan::FULL)
        .with_max_level(tracing::Level::INFO)
        .init();

    //init settings
    {
        let _settings = Settings::get().lock().unwrap();
    }

    let application = relm4::main_application();

    // quit action
    let quit = SimpleAction::new("quit", None);
    let app = application.clone();
    quit.connect_activate(move |_action, _parameter| {
        app.quit();
    });
    application.set_accels_for_action("app.quit", &["<Primary>Q"]);
    application.add_action(&quit);

    //relaod css action
    let reload_css = SimpleAction::new("reload_css", None);
    reload_css.connect_activate(move |_action, _parameter| {
        css::setup_css().unwrap();
    });
    application.set_accels_for_action("app.reload_css", &["<Primary><Shift>C"]);
    application.add_action(&reload_css);

    let app = RelmApp::new("com.github.eppixx.bouy");
    css::setup_css()?;
    app.run::<App>(());
    Ok(())
}
