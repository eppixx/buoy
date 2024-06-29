use std::{cell::RefCell, rc::Rc};

use granite::prelude::{SettingsExt, ToastExt};
use gtk::prelude::{BoxExt, ButtonExt, GtkWindowExt, OrientableExt, ScaleButtonExt};
use relm4::{
    actions::AccelsPlus,
    component::{AsyncComponentController, AsyncController},
    gtk::{
        self,
        prelude::{ApplicationExt, EditableExt, PopoverExt, ToggleButtonExt, WidgetExt},
    },
    Component, ComponentController, Controller, RelmWidgetExt,
};

use crate::{
    client::Client,
    mpris::MprisOut,
    play_state::PlayState,
    playback::{Playback, PlaybackOut},
    player::Command,
    settings::Settings,
};
use crate::{
    components::{
        browser::{Browser, BrowserIn, BrowserOut},
        equalizer::{Equalizer, EqualizerOut},
        login_form::{LoginForm, LoginFormOut},
        play_controls::{PlayControl, PlayControlIn, PlayControlOut},
        play_info::{PlayInfo, PlayInfoIn, PlayInfoOut},
        queue::{Queue, QueueIn, QueueOut},
        seekbar::{Seekbar, SeekbarCurrent, SeekbarIn, SeekbarOut},
    },
    window_state::{NavigationMode, WindowState},
};
use crate::{mpris::Mpris, subsonic::Subsonic};

#[derive(Debug)]
pub struct App {
    playback: Playback,
    subsonic: Rc<RefCell<Subsonic>>,
    mpris: Mpris,

    login_form: AsyncController<LoginForm>,
    queue: Controller<Queue>,
    play_controls: Controller<PlayControl>,
    seekbar: Controller<Seekbar>,
    play_info: Controller<PlayInfo>,
    browser: Controller<Browser>,
    equalizer: Controller<Equalizer>,

    main_stack: gtk::Stack,
    back_btn: gtk::Button,
    search_btn: gtk::ToggleButton,
    search_stack: gtk::Stack,
    search: gtk::SearchEntry,
    volume_btn: gtk::VolumeButton,
    toasts: granite::Toast,
}

impl App {
    fn recalculate_mpris_next_prev(&mut self) {
        let can_prev = self.queue.model().can_play_previous();
        self.play_controls
            .emit(PlayControlIn::DisablePrevious(can_prev));
        self.mpris.can_play_previous(can_prev);
        let can_next = self.queue.model().can_play_next();
        self.play_controls
            .emit(PlayControlIn::DisableNext(can_next));
        self.mpris.can_play_next(can_next);
    }
}

#[derive(Debug)]
pub enum AppIn {
    ResetLogin,
    DeleteCache,
    PlayControlOutput(PlayControlOut),
    Seekbar(SeekbarOut),
    Playback(PlaybackOut),
    LoginForm(LoginFormOut),
    Equalizer(EqualizerOut),
    Queue(Box<QueueOut>),
    Browser(BrowserOut),
    PlayInfo(PlayInfoOut),
    DisplayToast(String),
    Navigation(NavigationMode),
    Mpris(MprisOut),
    Player(Command),
}

#[relm4::widget_template(pub)]
impl relm4::WidgetTemplate for LoadingState {
    view! {
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,

            gtk::HeaderBar {
                add_css_class: granite::STYLE_CLASS_FLAT,
                add_css_class: granite::STYLE_CLASS_DEFAULT_DECORATION,
            },
            gtk::Box {
                set_hexpand: true,
                set_vexpand: true,
                set_halign: gtk::Align::Center,
                set_valign: gtk::Align::Center,
                set_orientation: gtk::Orientation::Vertical,
                set_spacing: 10,

                #[name = "label"]
                gtk::Label {
                    add_css_class: granite::STYLE_CLASS_H3_LABEL,
                    set_text: "loading subsonic information from server",
                },
                #[name = "spinner"]
                gtk::Spinner {
                    start: (),
                    set_halign: gtk::Align::Center,
                }
            }
        }
    }
}

relm4::new_action_group!(WindowActionGroup, "win");
relm4::new_stateless_action!(QuitAction, WindowActionGroup, "quit-app");
relm4::new_stateless_action!(ActivateSearchAction, WindowActionGroup, "activate-search");

#[relm4::component(async, pub)]
impl relm4::component::AsyncComponent for App {
    type Init = ();
    type Input = AppIn;
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

                #[wrap(Some)]
                set_titlebar = &gtk::HeaderBar {
                    add_css_class: granite::STYLE_CLASS_FLAT,
                    add_css_class: granite::STYLE_CLASS_DEFAULT_DECORATION,
                    set_show_title_buttons: false,
                    set_visible: false,
                },

                #[name(loading)]
                gtk::Box {
                    #[template]
                    LoadingState {}
                }
            }
        }
        Some(relm4::loading_widgets::LoadingWidgets::new(
            root.clone(),
            loading,
        ))
    }

    // Initialize the UI.
    async fn init(
        _init: Self::Init,
        root: Self::Root,
        sender: relm4::AsyncComponentSender<Self>,
    ) -> relm4::component::AsyncComponentParts<Self> {
        let (playback_sender, receiver) = async_channel::unbounded();
        let mut playback = Playback::new(&playback_sender).unwrap();

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
                        seekbar = Some(SeekbarCurrent::new(i64::from(duration) * 1000, None));
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
            let client = Client::get().unwrap();
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
                    if let Err(e) = playback.set_track(url) {
                        sender.input(AppIn::DisplayToast(format!("could not set track: {e}")));
                    }
                }
                Err(e) => {
                    sender.input(AppIn::DisplayToast(format!(
                        "could not fetch stream url: {e:?}"
                    )));
                }
            }
            playback.pause().unwrap();
        }

        tracing::info!("start loading subsonic information");
        let subsonic = Subsonic::load_or_create().await.unwrap_or_default();
        let subsonic = std::rc::Rc::new(std::cell::RefCell::new(subsonic));
        tracing::info!("finished loaded subsonic information");

        let login_form: AsyncController<LoginForm> = LoginForm::builder()
            .launch(())
            .forward(sender.input_sender(), AppIn::LoginForm);
        let queue: Controller<Queue> = Queue::builder()
            .launch((subsonic.clone(), queue, queue_index))
            .forward(sender.input_sender(), |msg| AppIn::Queue(Box::new(msg)));
        let play_controls = PlayControl::builder()
            .launch(controls)
            .forward(sender.input_sender(), AppIn::PlayControlOutput);
        let seekbar = Seekbar::builder()
            .launch(seekbar)
            .forward(sender.input_sender(), AppIn::Seekbar);
        let play_info = PlayInfo::builder()
            .launch((subsonic.clone(), current_song))
            .forward(sender.input_sender(), AppIn::PlayInfo);
        let browser = Browser::builder()
            .launch(subsonic.clone())
            .forward(sender.input_sender(), AppIn::Browser);
        let equalizer = Equalizer::builder()
            .launch(())
            .forward(sender.input_sender(), AppIn::Equalizer);

        let (mpris_sender, mpris_receiver) = async_channel::unbounded();
        let mpris = crate::mpris::Mpris::new(&mpris_sender).await.unwrap();

        let mut model = App {
            playback,
            subsonic,
            mpris,

            login_form,
            queue,
            play_controls,
            seekbar,
            play_info,
            browser,
            equalizer,

            main_stack: gtk::Stack::default(),
            back_btn: gtk::Button::default(),
            search_btn: gtk::ToggleButton::default(),
            search_stack: gtk::Stack::default(),
            search: gtk::SearchEntry::default(),
            volume_btn: gtk::VolumeButton::default(),
            toasts: granite::Toast::default(),
        };

        let browser_sender = model.browser.sender().clone();
        let widgets = view_output!();
        tracing::info!("loaded main window");

        // set application shortcuts
        // quit
        let application = relm4::main_application();
        application.set_accelerators_for_action::<QuitAction>(&["<Primary>Q"]);
        let app = application.clone();
        let quit_action: relm4::actions::RelmAction<QuitAction> =
            relm4::actions::RelmAction::new_stateless(move |_| {
                tracing::info!("quit called");
                app.quit();
            });
        application.set_accelerators_for_action::<ActivateSearchAction>(&["<Primary>F"]);
        let search_btn = model.search_btn.clone();
        let activate_search_action: relm4::actions::RelmAction<ActivateSearchAction> =
            relm4::actions::RelmAction::new_stateless(move |_| {
                tracing::info!("activate search called");
                search_btn.set_active(true);
            });

        let mut group = relm4::actions::RelmActionGroup::<WindowActionGroup>::new();
        group.add_action(quit_action);
        group.add_action(activate_search_action);
        group.register_for_widget(&widgets.main_window);

        //init widgets
        {
            let settings = Settings::get().lock().unwrap();
            model.volume_btn.set_value(settings.volume);
            model.mpris.set_volume(settings.volume);

            // playcontrol
            if model.queue.model().songs().is_empty() {
                model.play_controls.emit(PlayControlIn::Disable);
            }
        }

        //setup mpris
        let sender_mpris = sender.clone();
        gtk::glib::spawn_future_local(async move {
            while let Ok(msg) = mpris_receiver.recv().await {
                sender_mpris.input(AppIn::Mpris(msg));
            }
        });

        gtk::glib::spawn_future_local(async move {
            while let Ok(msg) = receiver.recv().await {
                sender.input(AppIn::Playback(msg));
            }
        });

        {
            let client = Client::get_mut().lock().unwrap();

            match &client.inner {
                Some(_client) => model
                    .main_stack
                    .set_visible_child_name(WindowState::Main.to_str()),
                None => model
                    .main_stack
                    .set_visible_child_name(WindowState::LoginForm.to_str()),
            }
        }

        relm4::component::AsyncComponentParts { model, widgets }
    }

    view! {
        #[root]
        main_window = gtk::Window {
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

            model.main_stack.clone() -> gtk::Stack {
                add_css_class: "main-box",
                set_transition_type: gtk::StackTransitionType::Crossfade,
                set_transition_duration: 200,

                add_named[Some(WindowState::Main.to_str())] = &gtk::Box {
                    add_css_class: "main-box",
                    set_orientation: gtk::Orientation::Vertical,

                    #[name = "paned"]
                    gtk::Paned {
                        add_css_class: "main-paned",
                        set_position: Settings::get().lock().unwrap().paned_position,
                        set_shrink_start_child: false,
                        set_resize_start_child: false,
                        set_shrink_end_child: false,

                        #[wrap(Some)]
                        set_start_child = &gtk::Box {
                            add_css_class: granite::STYLE_CLASS_SIDEBAR,
                            set_orientation: gtk::Orientation::Vertical,
                            set_spacing: 15,

                            append = &gtk::WindowHandle {
                                gtk::Box {
                                    set_orientation: gtk::Orientation::Vertical,
                                    set_spacing: 12,

                                    gtk::HeaderBar {
                                        add_css_class: granite::STYLE_CLASS_FLAT,
                                        add_css_class: granite::STYLE_CLASS_DEFAULT_DECORATION,
                                        set_show_title_buttons: false,
                                        pack_start = &gtk::WindowControls {
                                            set_side: gtk::PackType::Start,
                                        }
                                    },
                                    model.play_info.widget(),
                                    model.play_controls.widget(),
                                    model.seekbar.widget(),
                                },
                            },

                            model.queue.widget(),
                        },

                        #[wrap(Some)]
                        set_end_child = &gtk::Box {
                            set_orientation: gtk::Orientation::Vertical,

                            gtk::HeaderBar {
                                add_css_class: granite::STYLE_CLASS_FLAT,
                                add_css_class: granite::STYLE_CLASS_DEFAULT_DECORATION,
                                set_show_title_buttons: false,
                                set_halign: gtk::Align::Fill,

                                pack_start = &gtk::Box {
                                    model.back_btn.clone() {
                                        set_icon_name: "go-previous-symbolic",
                                        add_css_class: "destructive-button-spacer",

                                        connect_clicked[browser_sender] => move |_| {
                                            browser_sender.emit(BrowserIn::BackClicked);
                                        }
                                    },

                                },

                                #[wrap(Some)]
                                set_title_widget = &gtk::Box {
                                    set_hexpand: true,
                                    set_halign: gtk::Align::Center,
                                    set_spacing: 15,

                                    model.search_btn.clone() -> gtk::ToggleButton {
                                        add_css_class: "browser-navigation-button",
                                        set_icon_name: "system-search-symbolic",

                                        connect_toggled[sender] => move |button| {
                                            match button.is_active() {
                                                true => sender.input(AppIn::Navigation(NavigationMode::Search)),
                                                false => sender.input(AppIn::Navigation(NavigationMode::Normal)),
                                            }
                                        }
                                    },

                                    model.search_stack.clone() -> gtk::Stack {
                                        add_named[Some(NavigationMode::Normal.to_str())] = &gtk::Box {
                                            gtk::Button {
                                                add_css_class: "browser-navigation-button",
                                                set_icon_name: "go-home-symbolic",
                                                set_tooltip: "Go to dashboard",
                                                connect_clicked[browser_sender] => move |_| {
                                                    browser_sender.emit(BrowserIn::DashboardClicked);
                                                }
                                            },
                                            gtk::Button {
                                                add_css_class: "browser-navigation-button",
                                                set_icon_name: "avatar-default-symbolic",
                                                set_tooltip: "Show Artists",
                                                connect_clicked[browser_sender] => move |_| {
                                                    browser_sender.emit(BrowserIn::ArtistsClicked);
                                                }
                                            },
                                            gtk::Button {
                                                add_css_class: "browser-navigation-button",
                                                set_icon_name: "media-optical-cd-audio-symbolic",
                                                set_tooltip: "Show Albums",
                                                connect_clicked[browser_sender] => move |_| {
                                                    browser_sender.emit(BrowserIn::AlbumsClicked);
                                                }
                                            },
                                            gtk::Button {
                                                add_css_class: "browser-navigation-button",
                                                set_icon_name: "audio-x-generic-symbolic",
                                                set_tooltip: "Show Tracks",
                                                connect_clicked[browser_sender] => move |_| {
                                                    browser_sender.emit(BrowserIn::TracksClicked);
                                                }
                                            },
                                            gtk::Button {
                                                add_css_class: "browser-navigation-button",
                                                set_icon_name: "playlist-symbolic",
                                                set_tooltip: "Show playlists",
                                                connect_clicked[browser_sender] => move|_| {
                                                    browser_sender.emit(BrowserIn::PlaylistsClicked);
                                                }
                                            },
                                        },
                                        add_named[Some(NavigationMode::Search.to_str())] = &model.search.clone() -> gtk::SearchEntry {
                                            set_placeholder_text: Some("Search..."),
                                            connect_search_changed[browser_sender] => move |w| {
                                                browser_sender.emit(BrowserIn::SearchChanged(w.text().to_string()));
                                            }
                                        },
                                    }
                                },

                                pack_end = &gtk::Box {
                                    set_hexpand: true,
                                    set_halign: gtk::Align::End,
                                    set_spacing: 5,

                                    gtk::MenuButton {
                                        set_icon_name: "media-eq-symbolic",
                                        set_focus_on_click: false,
                                        #[wrap(Some)]
                                        set_popover = &gtk::Popover {
                                            model.equalizer.widget(),
                                        },
                                    },

                                    model.volume_btn.clone() {
                                        set_focus_on_click: false,
                                        connect_value_changed[sender] => move |_scale, value| {
                                            sender.input(AppIn::Player(Command::Volume(value)));
                                        }
                                    },

                                    gtk::MenuButton {
                                        set_icon_name: "open-menu-symbolic",
                                        set_focus_on_click: false,

                                        #[wrap(Some)]
                                        set_popover = &gtk::Popover {
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
                                    }
                                },
                            },

                            gtk::Overlay {
                                set_child: Some(model.browser.widget()),
                                add_overlay: &model.toasts.clone(),
                            }
                        }
                    }
                },
                add_named[Some(WindowState::LoginForm.to_str())] = &gtk::WindowHandle {
                    gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,

                        gtk::HeaderBar {
                            add_css_class: granite::STYLE_CLASS_FLAT,
                            add_css_class: granite::STYLE_CLASS_DEFAULT_DECORATION,
                        },

                        model.login_form.widget() {
                            set_hexpand: true,
                            set_vexpand: true,
                            set_halign: gtk::Align::Center,
                            set_valign: gtk::Align::Center,
                        }
                    }
                },
                add_named[Some(WindowState::Loading.to_str())] = &gtk::WindowHandle {
                    #[template]
                    LoadingState {
                        #[template_child]
                        label {
                            set_text: "please restart application",
                        },
                        #[template_child]
                        spinner {
                            set_visible: false,
                        }
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
            AppIn::PlayControlOutput(input) => match input {
                PlayControlOut::Next => sender.input(AppIn::Player(Command::Next)),
                PlayControlOut::Previous => sender.input(AppIn::Player(Command::Previous)),
                PlayControlOut::Play => sender.input(AppIn::Player(Command::Play)),
                PlayControlOut::Pause => sender.input(AppIn::Player(Command::Pause)),
            },
            AppIn::Seekbar(msg) => match msg {
                SeekbarOut::SeekDragged(seek_in_ms) => {
                    if let Err(e) = self.playback.set_position(seek_in_ms) {
                        sender.input(AppIn::DisplayToast(format!("seek failed: {e:?}")));
                    }
                }
            },
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
                    self.main_stack
                        .set_visible_child_name(WindowState::Loading.to_str());
                }
            },
            AppIn::Equalizer(_changed) => {
                self.playback.sync_equalizer();
            }
            AppIn::ResetLogin => {
                let mut settings = Settings::get().lock().unwrap();
                settings.reset_login();
                self.main_stack
                    .set_visible_child_name(WindowState::LoginForm.to_str());
                sender.input(AppIn::DeleteCache);
            }
            AppIn::DeleteCache => {
                if let Err(e) = self.subsonic.borrow_mut().delete_cache() {
                    sender.input(AppIn::DisplayToast(format!(
                        "error while deleting cache: {e:?}"
                    )));
                } else {
                    sender.input(AppIn::DisplayToast(String::from(
                        "Deleted cache\nPlease restart to reload the cache",
                    )));
                }
                self.main_stack
                    .set_visible_child_name(WindowState::Loading.to_str());
            }
            AppIn::Queue(msg) => match *msg {
                QueueOut::Play(child) => {
                    // set playback track
                    let client = Client::get().unwrap();
                    match client.stream_url(
                        child.clone().id,
                        None,
                        None::<&str>,
                        None,
                        None::<&str>,
                        None,
                        None,
                    ) {
                        Ok(url) => {
                            if let Err(e) = self.playback.set_track(url) {
                                sender.input(AppIn::DisplayToast(format!(
                                    "could not set track: {e}"
                                )));
                                return;
                            }
                        }
                        Err(e) => {
                            sender.input(AppIn::DisplayToast(format!(
                                "could not find song streaming url: {e:?}"
                            )));
                            return;
                        }
                    }

                    // playback play
                    if let Err(e) = self.playback.play() {
                        sender.input(AppIn::DisplayToast(format!(
                            "could set playback to play: {e:?}"
                        )));
                    }

                    // update seekbar
                    if let Some(length) = child.duration {
                        self.seekbar
                            .emit(SeekbarIn::NewRange(i64::from(length) * 1000));
                    } else {
                        self.seekbar.emit(SeekbarIn::NewRange(0));
                    }

                    // update playcontrol
                    self.play_info
                        .emit(PlayInfoIn::NewState(Box::new(Some(*child.clone()))));
                    self.mpris.set_song(Some(*child)).await;
                    self.recalculate_mpris_next_prev();
                    self.mpris.set_state(PlayState::Play);
                }
                QueueOut::Stop => {
                    if let Err(e) = self.playback.stop() {
                        sender.input(AppIn::DisplayToast(format!(
                            "error while stopping playback: {e}"
                        )));
                    }
                    self.play_info.emit(PlayInfoIn::NewState(Box::new(None)));
                    self.mpris.set_song(None).await;
                    self.play_controls
                        .emit(PlayControlIn::NewState(PlayState::Stop));
                    self.seekbar.emit(SeekbarIn::Disable);
                }
                QueueOut::QueueEmpty => self.play_controls.emit(PlayControlIn::Disable),
                QueueOut::QueueNotEmpty => self.play_controls.emit(PlayControlIn::Enable),
                QueueOut::Player(cmd) => sender.input(AppIn::Player(cmd)),
                QueueOut::DisplayToast(title) => sender.input(AppIn::DisplayToast(title)),
            },
            AppIn::Browser(msg) => match msg {
                BrowserOut::AppendToQueue(drop) => self.queue.emit(QueueIn::Append(drop)),
                BrowserOut::ReplaceQueue(drop) => self.queue.emit(QueueIn::Replace(drop)),
                BrowserOut::InsertAfterCurrentInQueue(drop) => {
                    self.queue.emit(QueueIn::InsertAfterCurrentlyPlayed(drop));
                }
                BrowserOut::BackButtonSensitivity(status) => self.back_btn.set_sensitive(status),
                BrowserOut::DisplayToast(title) => sender.input(AppIn::DisplayToast(title)),
            },
            AppIn::PlayInfo(msg) => match msg {
                PlayInfoOut::DisplayToast(title) => sender.input(AppIn::DisplayToast(title)),
            },
            AppIn::DisplayToast(title) => {
                tracing::error!(title);
                self.toasts.set_title(&title);
                self.toasts.send_notification();
            }
            AppIn::Navigation(NavigationMode::Normal) => {
                self.browser.emit(BrowserIn::SearchChanged(String::new()));
                self.search_stack
                    .set_visible_child_name(NavigationMode::Normal.to_str());
                self.back_btn.set_visible(true);
            }
            AppIn::Navigation(NavigationMode::Search) => {
                self.browser
                    .emit(BrowserIn::SearchChanged(self.search.text().to_string()));
                self.search_stack
                    .set_visible_child_name(NavigationMode::Search.to_str());
                self.search.grab_focus();
                self.back_btn.set_visible(false);
            }
            AppIn::Mpris(MprisOut::Player(cmd)) => sender.input(AppIn::Player(cmd)),
            AppIn::Mpris(msg) => sender.input(AppIn::DisplayToast(format!("mpris msg: {msg:?}"))),
            AppIn::Player(cmd) => match cmd {
                //TODO
                Command::Next => {
                    if !self.queue.model().can_play_next() {
                        return;
                    }
                    self.queue.emit(QueueIn::PlayNext);
                    self.recalculate_mpris_next_prev();
                    self.mpris.set_state(PlayState::Play);
                }
                Command::Previous => {
                    if !self.queue.model().can_play_previous() {
                        return;
                    }
                    self.queue.emit(QueueIn::PlayPrevious);
                    self.recalculate_mpris_next_prev();
                    self.mpris.set_state(PlayState::Play);
                }
                Command::Play => {
                    if !self.queue.model().can_play() {
                        return;
                    }

                    if let Err(e) = self.playback.play() {
                        sender.input(AppIn::DisplayToast(format!(
                            "could not play playback: {e:?}"
                        )));
                    }
                    self.mpris.can_play(true);
                    self.recalculate_mpris_next_prev();
                    self.mpris.set_state(PlayState::Play);
                }
                Command::Pause => {
                    if let Err(e) = self.playback.pause() {
                        sender.input(AppIn::DisplayToast(format!(
                            "could not pause playback: {e:?}"
                        )));
                    }
                    self.play_controls
                        .emit(PlayControlIn::NewState(PlayState::Pause));
                    self.queue.emit(QueueIn::NewState(PlayState::Pause));
                    self.mpris.set_state(PlayState::Pause);
                }
                Command::PlayPause => match self.playback.is_playing() {
                    PlayState::Stop | PlayState::Pause => {
                        sender.input(AppIn::Player(Command::Play))
                    }
                    PlayState::Play => sender.input(AppIn::Player(Command::Pause)),
                },
                Command::Stop => {
                    if let Err(e) = self.playback.stop() {
                        sender.input(AppIn::DisplayToast(format!(
                            "could not stop playback: {e:?}"
                        )));
                    }
                    self.play_controls
                        .emit(PlayControlIn::NewState(PlayState::Stop));
                    self.queue.emit(QueueIn::NewState(PlayState::Stop));
                    self.mpris.set_state(PlayState::Stop);
                }
                Command::Seek(offset) => {}
                Command::Volume(volume) => {
                    self.playback.set_volume(volume);
                    self.volume_btn.set_value(volume);
                    self.mpris.set_volume(volume);
                    let mut settings = Settings::get().lock().unwrap();
                    settings.volume = volume;
                    settings.save();
                }
                Command::Repeat(repeat) => {
                    self.mpris.set_loop_status(repeat.clone());
                    self.queue.emit(QueueIn::SetRepeat(repeat));
                }
            },
        }
    }

    fn shutdown(&mut self, widgets: &mut Self::Widgets, _output: relm4::Sender<Self::Output>) {
        tracing::info!("shutdown called");
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
}
