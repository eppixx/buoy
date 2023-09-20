use components::{play_controls::PlayControlIn, queue::QueueOut, seekbar::SeekbarOut};
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
    play_controls::{PlayControl, PlayControlOut},
    play_info::PlayInfo,
    queue::{Queue, QueueIn},
    seekbar::{Seekbar, SeekbarCurrent, SeekbarIn},
};
use crate::{
    client::Client,
    play_state::PlayState,
    playback::{Playback, PlaybackOutput},
    settings::Settings,
};

pub mod client;
mod components;
pub mod css;
mod factory;
mod play_state;
mod playback;
pub mod settings;
pub mod types;

struct AppModel {
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
enum AppMsg {
    ResetLogin,
    PlayControlOutput(PlayControlOut),
    Seekbar(SeekbarOut),
    VolumeChange(f64),
    Playback(PlaybackOutput),
    LoginForm(LoginFormOut),
    Equalizer(EqualizerOut),
    Queue(QueueOut),
}

#[relm4::component]
impl SimpleComponent for AppModel {
    type Input = AppMsg;
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
            .forward(sender.input_sender(), AppMsg::LoginForm);
        let queue: Controller<Queue> = Queue::builder()
            .launch(())
            .forward(sender.input_sender(), AppMsg::Queue);
        let play_controls = PlayControl::builder()
            .launch(PlayState::Pause) // TODO change to previous state
            .forward(sender.input_sender(), AppMsg::PlayControlOutput);
        let seekbar = Seekbar::builder()
            .launch(Some(SeekbarCurrent::new(1000 * 60, None))) // TODO change to previous state
            .forward(sender.input_sender(), AppMsg::Seekbar);
        let play_info = PlayInfo::builder()
            .launch(None) // TODO change to previous state
            .detach();
        let browser = Browser::builder().launch(()).detach();
        let equalizer = Equalizer::builder()
            .launch(())
            .forward(sender.input_sender(), AppMsg::Equalizer);

        let model = AppModel {
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
            sender.input(AppMsg::Playback(msg));
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
            AppMsg::PlayControlOutput(PlayControlOut::Next) => {
                _ = self.queue.sender().send(QueueIn::PlayNext);
            }
            AppMsg::PlayControlOutput(PlayControlOut::Previous) => {
                _ = self.queue.sender().send(QueueIn::PlayPrevious);
            }
            AppMsg::PlayControlOutput(PlayControlOut::Status(status)) => {
                match status {
                    PlayState::Pause => self.playback.pause().unwrap(),
                    PlayState::Play => self.playback.play().unwrap(),
                    PlayState::Stop => self.playback.stop().unwrap(),
                };
                self.queue.sender().send(QueueIn::NewState(status)).unwrap();
            }
            AppMsg::Seekbar(msg) => match msg {
                SeekbarOut::SeekDragged(seek_in_ms) => self.playback.set_position(seek_in_ms),
            },
            AppMsg::VolumeChange(value) => {
                self.playback.set_volume(value);
                let mut settings = Settings::get().lock().unwrap();
                settings.volume = value;
                settings.save();
            }
            AppMsg::Playback(playback) => {
                match playback {
                    PlaybackOutput::TrackEnd => {} //TODO play next
                    PlaybackOutput::Seek(ms) => {
                        self.seekbar.emit(SeekbarIn::SeekTo(ms));
                        self.play_controls
                            .emit(PlayControlIn::NewState(PlayState::Play));
                    }
                }
            }
            AppMsg::LoginForm(client) => match client {
                LoginFormOut::LoggedIn => {
                    self.main_stack.set_visible_child_name("logged-in");
                    self.config_btn.set_sensitive(true);
                    self.equalizer_btn.set_sensitive(true);
                    self.volume_btn.set_sensitive(true);
                }
            },
            AppMsg::Equalizer(changed) => {
                self.playback.sync_equalizer();
            }
            AppMsg::ResetLogin => {
                let mut settings = Settings::get().lock().unwrap();
                settings.reset_login();
                self.main_stack.set_visible_child_name("login-form");
                self.config_btn.set_sensitive(false);
                self.equalizer_btn.set_sensitive(false);
                self.volume_btn.set_sensitive(false);
            }
            AppMsg::Queue(msg) => match msg {
                QueueOut::Play(id, length) => {
                    let client = Client::get().lock().unwrap().inner.clone().unwrap();
                    match client.stream_url(
                        id.inner(),
                        None,
                        None::<&str>,
                        None,
                        None::<&str>,
                        None,
                        None,
                    ) {
                        Ok(url) => {
                            self.playback.set_track(url);
                            self.seekbar.emit(SeekbarIn::NewRange(length));
                            self.playback.play().unwrap();
                        }
                        Err(_) => {} //TODO error handling
                    }
                }
            },
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
                            sender.input(AppMsg::VolumeChange(value));
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

                                gtk::Button {
                                    add_css_class: "destructive-action",
                                    set_label: "Logout from Server",
                                    connect_clicked => AppMsg::ResetLogin,
                                },
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
        self.playback.shutdown().unwrap();
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
    app.run::<AppModel>(());
    Ok(())
}
