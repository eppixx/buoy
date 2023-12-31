use cache::Cache;
use components::play_info::PlayInfoOut;
use gtk::prelude::{BoxExt, ButtonExt, GtkWindowExt, OrientableExt, ScaleButtonExt};
use relm4::{
    actions::AccelsPlus,
    component::{AsyncComponentController, AsyncController},
    gtk::{
        self,
        prelude::ApplicationExt,
        traits::{PopoverExt, WidgetExt},
    },
    Component, ComponentController, Controller, RelmApp,
};

use crate::components::{
    browser::{Browser, BrowserOut},
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
    playback::{Playback, PlaybackOut},
    settings::Settings,
};

pub mod cache;
pub mod client;
pub mod common;
mod components;
pub mod css;
mod factory;
mod play_state;
mod playback;
pub mod settings;
pub mod subsonic;
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
    Playback(PlaybackOut),
    LoginForm(LoginFormOut),
    Equalizer(EqualizerOut),
    Queue(Box<QueueOut>),
    Browser(BrowserOut),
    PlayInfo(PlayInfoOut),
}

relm4::new_action_group!(WindowActionGroup, "win");
relm4::new_stateless_action!(QuitAction, WindowActionGroup, "quit-app");
relm4::new_stateless_action!(ReloadCssAction, WindowActionGroup, "reload-css");

#[relm4::component(async)]
impl relm4::component::AsyncComponent for App {
    type Init = ();
    type Input = AppIn;
    type Output = ();
    type CommandOutput = ();

    // Initialize the UI.
    async fn init(
        _init: Self::Init,
        root: Self::Root,
        sender: relm4::AsyncComponentSender<Self>,
    ) -> relm4::component::AsyncComponentParts<Self> {
        let subsonic = subsonic::Subsonic::load_or_create().await.unwrap();
        let subsonic = std::rc::Rc::new(std::cell::RefCell::new(subsonic));

        let (playback_sender, receiver) =
            gtk::glib::MainContext::channel(gtk::glib::Priority::default());
        let mut playback = Playback::new(&playback_sender).unwrap();

        // load from settings
        let (queue, queue_index, current_song, seekbar, controls) = {
            let settings = Settings::get().lock().unwrap();
            //queue
            let queue = settings.queue_ids.clone();
            let queue_index = settings.queue_current;

            // play info
            let current_song = if let Some(index) = settings.queue_current {
                settings.queue_ids.get(index).cloned()
            } else {
                None
            };

            // seekbar
            let mut seekbar = None;
            if let Some(index) = settings.queue_current {
                if let Some(song) = settings.queue_ids.get(index) {
                    if let Some(duration) = song.duration {
                        seekbar = Some(SeekbarCurrent::new(
                            duration as i64 * 1000,
                            None,
                            // Some(settings.queue_seek as i64),
                        ));
                    }
                }
            };

            //TODO set playback seek

            // controls
            let controls = match settings.queue_current {
                Some(_) => PlayState::Pause,
                None => PlayState::Stop,
            };
            (queue, queue_index, current_song, seekbar, controls)
        };

        // set playback song from settings
        if let Some(child) = &current_song {
            let client = Client::get().lock().unwrap().inner.clone().unwrap();
            match client.stream_url(
                &child.id,
                None,
                None::<&str>,
                None,
                None::<&str>,
                None,
                None,
            ) {
                Ok(url) => {
                    playback.set_track(url);
                }
                Err(_) => {} //TODO error handling
            }
            playback.pause().unwrap();
        }

        let login_form: AsyncController<LoginForm> = LoginForm::builder()
            .launch(())
            .forward(sender.input_sender(), AppIn::LoginForm);
        let queue: Controller<Queue> = Queue::builder()
            .launch((queue, queue_index))
            .forward(sender.input_sender(), |msg| AppIn::Queue(Box::new(msg)));
        let play_controls = PlayControl::builder()
            .launch(controls)
            .forward(sender.input_sender(), AppIn::PlayControlOutput);
        let seekbar = Seekbar::builder()
            .launch(seekbar)
            .forward(sender.input_sender(), AppIn::Seekbar);
        let play_info = PlayInfo::builder()
            .launch(current_song)
            .forward(sender.input_sender(), AppIn::PlayInfo);
        let browser = Browser::builder()
            .launch(subsonic)
            .forward(sender.input_sender(), AppIn::Browser);
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

        // set application shortcuts
        // quit
        let application = relm4::main_application();
        application.set_accelerators_for_action::<QuitAction>(&["<Primary>Q"]);
        let app = application.clone();
        let quit_action: relm4::actions::RelmAction<QuitAction> =
            relm4::actions::RelmAction::new_stateless(move |_| {
                tracing::error!("quit called");
                app.quit();
            });
        // reload css
        application.set_accelerators_for_action::<ReloadCssAction>(&["<Primary><Shift>C"]);
        let reload_css_action: relm4::actions::RelmAction<ReloadCssAction> =
            relm4::actions::RelmAction::new_stateless(move |_| {
                tracing::error!("reload css");
                css::setup_css().unwrap();
            });

        let mut group = relm4::actions::RelmActionGroup::<WindowActionGroup>::new();
        group.add_action(quit_action);
        group.add_action(reload_css_action);
        group.register_for_widget(&widgets.main_window);

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

        relm4::component::AsyncComponentParts { model, widgets }
    }

    async fn update(
        &mut self,
        msg: Self::Input,
        _sender: relm4::AsyncComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match msg {
            AppIn::PlayControlOutput(input) => match input {
                PlayControlOut::Next => self.queue.emit(QueueIn::PlayNext),
                PlayControlOut::Previous => self.queue.emit(QueueIn::PlayPrevious),
                PlayControlOut::Status(status) => {
                    match status {
                        PlayState::Pause => self.playback.pause().unwrap(),
                        PlayState::Play => self.playback.play().unwrap(),
                        PlayState::Stop => self.playback.stop().unwrap(),
                    }
                    self.queue.emit(QueueIn::NewState(status));
                }
            },
            AppIn::Seekbar(msg) => match msg {
                SeekbarOut::SeekDragged(seek_in_ms) => self.playback.set_position(seek_in_ms),
            },
            AppIn::VolumeChange(value) => {
                self.playback.set_volume(value);
                let mut settings = Settings::get().lock().unwrap();
                settings.volume = value;
                settings.save();
            }
            AppIn::Playback(playback) => match playback {
                PlaybackOut::TrackEnd => self.queue.emit(QueueIn::PlayNext),
                PlaybackOut::Seek(ms) => {
                    self.seekbar.emit(SeekbarIn::SeekTo(ms));
                    self.play_controls
                        .emit(PlayControlIn::NewState(PlayState::Play));
                }
            },
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
                    self.play_info
                        .emit(PlayInfoIn::NewState(Box::new(Some(*child.clone()))));

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
                QueueOut::Stop => {
                    self.playback.stop().unwrap(); //TODO error handling
                    self.play_controls
                        .emit(PlayControlIn::NewState(PlayState::Stop));
                    self.seekbar.emit(SeekbarIn::Disable);
                }
                QueueOut::QueueEmpty => self.play_controls.emit(PlayControlIn::Disable),
                QueueOut::QueueNotEmpty => self.play_controls.emit(PlayControlIn::Enable),
            },
            AppIn::DeleteCache => Cache::remove(),
            AppIn::Browser(msg) => match msg {
                BrowserOut::AppendToQueue(drop) => self.queue.emit(QueueIn::Append(drop)),
                BrowserOut::InsertAfterCurrentInQueue(drop) => {
                    self.queue.emit(QueueIn::InsertAfterCurrentlyPlayed(drop))
                }
            },
            AppIn::PlayInfo(msg) => match msg {},
        }
    }

    fn shutdown(&mut self, widgets: &mut Self::Widgets, _output: relm4::Sender<Self::Output>) {
        tracing::error!("shutdown called");
        self.playback.shutdown().unwrap();

        let mut settings = Settings::get().lock().unwrap();

        //save queue
        settings.queue_ids = self.queue.model().songs();
        settings.queue_current = self
            .queue
            .model()
            .playing_index()
            .as_ref()
            .map(|i| i.current_index());
        settings.queue_seek = self.seekbar.model().current();

        //save window state
        settings.window_width = widgets.main_window.default_width();
        settings.window_height = widgets.main_window.default_height();
        settings.window_maximized = widgets.main_window.is_maximized();
        settings.paned_position = widgets.paned.position();
        settings.save();
    }

    view! {
        #[root]
        main_window = gtk::Window {
            add_css_class: "main-window",
            set_title: Some("Buoy"),
            set_default_width: Settings::get().lock().unwrap().window_width,
            set_default_height: Settings::get().lock().unwrap().window_height,
            set_maximized: Settings::get().lock().unwrap().window_maximized,

            #[wrap(Some)]
            set_titlebar = &gtk::WindowHandle {
                gtk::Box {
                    add_css_class: "window-titlebar",

                    gtk::WindowControls {
                        set_side: gtk::PackType::Start,
                    },

                    //title
                    gtk::Label {
                        set_markup: "<span weight=\"bold\">Buoy</span>",
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
                    #[name = "paned"]
                    gtk::Paned {
                        add_css_class: "main-paned",
                        set_position: Settings::get().lock().unwrap().paned_position,
                        set_shrink_start_child: true,

                        set_start_child: Some(model.queue.widget()),
                        set_end_child: Some(model.browser.widget()),
                    },
                } -> {
                    set_name: "logged-in",
                },
            },
        }
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

    let app = RelmApp::new("com.github.eppixx.bouy");
    css::setup_css()?;
    app.run_async::<App>(());
    Ok(())
}
