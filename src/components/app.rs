use std::{cell::RefCell, rc::Rc};

use gettextrs::gettext;
use gtk::prelude::{BoxExt, ButtonExt, CheckButtonExt, OrientableExt};
use relm4::{
    component::{AsyncComponentController, AsyncController},
    gtk::{
        self, gdk,
        prelude::{
            ApplicationExt, EditableExt, EventControllerExt, GestureSingleExt, GtkApplicationExt,
            GtkWindowExt, PopoverExt, ToggleButtonExt, WidgetExt,
        },
    },
    Component, ComponentController, Controller, RelmWidgetExt,
};

use crate::{
    client::Client,
    common::{
        play_state::PlayState,
        player::Command,
        types::Droppable,
        views::{ClickableViews, Views},
    },
    components::{
        browser::{Browser, BrowserIn, BrowserOut},
        equalizer::{Equalizer, EqualizerOut},
        play_controls::{PlayControl, PlayControlIn, PlayControlOut},
        play_info::{PlayInfo, PlayInfoIn, PlayInfoOut},
        queue::{Queue, QueueIn, QueueOut},
        seekbar::{Seekbar, SeekbarIn, SeekbarOut},
        settings_window::{SettingsWindow, SettingsWindowIn, SettingsWindowOut},
        volume_button::{VolumeButton, VolumeButtonIn, VolumeButtonOut},
    },
    config,
    download::Download,
    mpris::{Mpris, MprisOut},
    playback::{Playback, PlaybackOut},
    settings::Settings,
    subsonic::Subsonic,
    Args,
};

#[derive(Debug)]
pub struct App {
    playback: Rc<RefCell<Playback>>,
    subsonic: Rc<RefCell<Subsonic>>,
    mpris: Rc<RefCell<Mpris>>,

    queue: Controller<Queue>,
    play_controls: Controller<PlayControl>,
    seekbar: Controller<Seekbar>,
    play_info: Controller<PlayInfo>,
    browser: AsyncController<Browser>,
    equalizer: Controller<Equalizer>,
    volume_button: Controller<VolumeButton>,
    settings_window: Controller<SettingsWindow>,
}

#[derive(Debug)]
pub enum AppIn {
    Logout,
    ClearCache,
    PlayControlOutput(PlayControlOut),
    Seekbar(SeekbarOut),
    Playback(PlaybackOut),
    Equalizer(EqualizerOut),
    Queue(Box<QueueOut>),
    Browser(BrowserOut),
    PlayInfo(PlayInfoOut),
    DisplayToast(String),
    DesktopNotification,
    Mpris(MprisOut),
    Player(Command),
    FavoriteAlbumClicked(String, bool),
    FavoriteArtistClicked(String, bool),
    FavoriteSongClicked(String, bool),
    SearchActivate(bool),
    SearchChanged,
    Download(Droppable),
    ClickedNavigationBtn(ClickableViews),
    DisableBigCoverOverlay,
    LoadBigCoverPicture(String),
    UpdateCanPlayNextOrPrev,
    BackPressed,
    OpenSettings,
    SettingsWindow(SettingsWindowOut),
    VolumeButton(VolumeButtonOut),
}

#[derive(Debug)]
pub enum AppOut {
    Logout,
    Reload,
    DisplayToast(String),
}

#[relm4::component(async, pub)]
impl relm4::component::AsyncComponent for App {
    type Init = (Rc<RefCell<Args>>, Rc<RefCell<Mpris>>, Rc<RefCell<Playback>>);
    type Input = AppIn;
    type Output = AppOut;
    type CommandOutput = ();

    fn init_loading_widgets(root: Self::Root) -> Option<relm4::loading_widgets::LoadingWidgets> {
        relm4::view! {
            #[local]
            root {
                #[name(loading)]
                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_hexpand: true,

                    gtk::WindowHandle {
                        set_vexpand: true,
                        set_valign: gtk::Align::Center,
                        set_halign: gtk::Align::Center,

                        gtk::Box {
                            set_orientation: gtk::Orientation::Vertical,
                            set_spacing: 10,

                            gtk::Spinner {
                                set_height_request: 50,
                                start: ()
                            },
                            gtk::Label {
                                set_text: &gettext("loading information from server"),
                                add_css_class: granite::STYLE_CLASS_H2_LABEL,
                            }
                        }
                    }
                }
            }
        }
        Some(relm4::loading_widgets::LoadingWidgets::new(
            root.clone(),
            loading,
        ))
    }

    // Initialize the UI
    async fn init(
        (args, mpris, playback): Self::Init,
        root: Self::Root,
        sender: relm4::AsyncComponentSender<Self>,
    ) -> relm4::component::AsyncComponentParts<Self> {
        let time_startup = std::time::Instant::now();

        tracing::info!("start loading subsonic information");
        let subsonic = Subsonic::load_or_create().await.unwrap_or_default();
        let subsonic = std::rc::Rc::new(std::cell::RefCell::new(subsonic));
        tracing::info!("finished loaded subsonic information");

        let queue: Controller<Queue> = Queue::builder()
            .launch(subsonic.clone())
            .forward(sender.input_sender(), |msg| AppIn::Queue(Box::new(msg)));
        let play_controls = PlayControl::builder()
            .launch(PlayState::Stop)
            .forward(sender.input_sender(), AppIn::PlayControlOutput);
        let seekbar = Seekbar::builder()
            .launch(None)
            .forward(sender.input_sender(), AppIn::Seekbar);
        let play_info = PlayInfo::builder()
            .launch((subsonic.clone(), None))
            .forward(sender.input_sender(), AppIn::PlayInfo);
        let browser = Browser::builder()
            .launch(subsonic.clone())
            .forward(sender.input_sender(), AppIn::Browser);
        let equalizer = Equalizer::builder()
            .launch(())
            .forward(sender.input_sender(), AppIn::Equalizer);
        let volume_button = VolumeButton::builder()
            .launch(())
            .forward(sender.input_sender(), AppIn::VolumeButton);
        let settings_window = SettingsWindow::builder()
            .launch(())
            .forward(sender.input_sender(), AppIn::SettingsWindow);

        let model = App {
            playback: playback.clone(),
            subsonic,
            mpris,

            queue,
            play_controls,
            seekbar,
            play_info,
            browser,
            equalizer,
            volume_button,
            settings_window,
        };

        let equalizer_popover = gtk::Popover::default();
        equalizer_popover.set_child(Some(model.equalizer.widget()));
        let volume_popover = gtk::Popover::default();
        volume_popover.set_child(Some(model.volume_button.widget()));
        let volume_button_sender = model.volume_button.sender();
        let last_volume_scroll_event = Rc::new(RefCell::new(0));
        let widgets = view_output!();
        equalizer_popover.set_parent(&widgets.equalizer_btn);
        volume_popover.set_parent(&widgets.volume_btn);

        tracing::info!("loaded main window");

        //init widgets
        {
            let settings = Settings::get().lock().unwrap();
            model.mpris.borrow_mut().set_volume(settings.volume);

            // playcontrol
            if model.queue.model().songs().is_empty() {
                model.play_controls.emit(PlayControlIn::Disable);
            }
        }

        let current = model.queue.model().current();
        if let Some((_, row)) = &current {
            let child = row.item().clone();

            // set playback song
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
                    if let Err(e) = playback.borrow_mut().set_track(url) {
                        sender.input(AppIn::DisplayToast(format!("could not set track: {e}")));
                    }
                }
                Err(e) => {
                    sender.input(AppIn::DisplayToast(format!(
                        "could not fetch stream url: {e:?}"
                    )));
                }
            }
            if let Err(e) = playback.borrow_mut().pause() {
                sender.input(AppIn::DisplayToast(format!("error pausing: {e}")));
            }

            // set play info
            model
                .play_info
                .emit(PlayInfoIn::NewState(Box::new(Some(child.clone()))));

            // set controls
            model
                .play_controls
                .emit(PlayControlIn::NewState(PlayState::Pause));

            // set seekbar
            if let Some(duration) = &child.duration {
                model
                    .seekbar
                    .emit(SeekbarIn::NewRange(i64::from(*duration) * 1000));
            }
        } else {
            // set controls
            model
                .play_controls
                .emit(PlayControlIn::NewState(PlayState::Stop));
        }

        //regularly save
        let library = model.subsonic.clone();
        let send = sender.clone();
        gtk::glib::spawn_future_local(async move {
            loop {
                let timeout = Settings::get().lock().unwrap().save_interval_secs;
                tokio::time::sleep(std::time::Duration::from_secs(timeout)).await;
                tracing::info!("periodic save");
                if let Err(e) = library.borrow().save() {
                    send.input(AppIn::DisplayToast(format!("error saving library: {e}")));
                }
            }
        });

        if args.borrow().time_startup {
            let shutdown = std::time::Instant::now();
            let duration = shutdown - time_startup;
            tracing::info!("startup time was {duration:?}");
            relm4::main_application().quit();
        }

        //setup some stuff delayed
        let queue = model.queue.sender().clone();
        gtk::glib::spawn_future_local(async move {
            // delay
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;

            // scroll to current
            queue.emit(QueueIn::DirectJumpToCurrent);
        });

        relm4::component::AsyncComponentParts { model, widgets }
    }

    view! {
        #[root]
        main_window = gtk::Box {
            set_widget_name: "app",
            set_orientation: gtk::Orientation::Vertical,

            append: paned = &gtk::Paned {
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

                            gtk::Overlay {
                                #[wrap(Some)]
                                set_child = model.play_info.widget() {
                                    set_margin_top: 10,
                                },

                                add_overlay = &gtk::Box {
                                    set_margin_top: 3,
                                    set_margin_start: 3,
                                    set_valign: gtk::Align::Start,
                                    set_halign: gtk::Align::Start,

                                    gtk::WindowControls {
                                        set_valign: gtk::Align::Start,
                                        set_side: gtk::PackType::Start,
                                    },
                                },
                            },

                            model.play_controls.widget(),
                            model.seekbar.widget(),
                        },
                    },

                    model.queue.widget(),
                },

                #[wrap(Some)]
                set_end_child = &gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_margin_top: 3,
                    set_margin_horizontal: 3,

                    gtk::WindowHandle {
                        gtk::CenterBox {
                            set_halign: gtk::Align::Fill,

                            #[wrap(Some)]
                            set_start_widget = &gtk::Box {
                                append: back_btn = &gtk::Button {
                                    set_icon_name: "go-previous-symbolic",
                                    add_css_class: "flat",
                                    add_css_class: "size24",
                                    add_css_class: "round-button",
                                    add_css_class: "destructive-button-spacer",
                                    set_tooltip: &gettext("Go back to previous page"),

                                    connect_clicked => AppIn::BackPressed,
                                },

                            },

                            #[wrap(Some)]
                            set_center_widget = &gtk::Box {
                                set_widget_name: "navigation-buttons",
                                set_hexpand: true,
                                set_halign: gtk::Align::Center,
                                set_spacing: 15,

                                append: search_btn = &gtk::ToggleButton {
                                    add_css_class: "flat",
                                    add_css_class: "size24",
                                    add_css_class: "round-button",
                                    set_icon_name: "system-search-symbolic",
                                    set_tooltip: &gettext("Open search bar"),

                                    connect_toggled[sender] => move |button| {
                                        match button.is_active() {
                                            true => sender.input(AppIn::SearchActivate(true)),
                                            false => sender.input(AppIn::SearchActivate(false)),
                                        }
                                    }
                                },

                                gtk::Box {
                                    append: dashboard_btn = &gtk::ToggleButton {
                                        add_css_class: "flat",
                                        add_css_class: "circular",
                                        set_tooltip: &gettext("Go to dashboard"),
                                        connect_clicked => AppIn::ClickedNavigationBtn(ClickableViews::Dashboard),

                                        gtk::Box {
                                            set_spacing: 3,

                                            gtk::Image {
                                                set_icon_name: Some("go-home-symbolic"),
                                            },
                                            append: dashboard_rvl = &gtk::Revealer {
                                                set_transition_duration: 200,
                                                set_transition_type: gtk::RevealerTransitionType::SlideRight,

                                                gtk::Label {
                                                    set_text: &gettext("Dashboard"),
                                                }
                                            }
                                        }
                                    },
                                    append: artists_btn = &gtk::ToggleButton {
                                        add_css_class: "flat",
                                        add_css_class: "circular",
                                        set_tooltip: &gettext("Show artists"),
                                        connect_clicked => AppIn::ClickedNavigationBtn(ClickableViews::Artists),

                                        gtk::Box {
                                            set_spacing: 3,

                                            gtk::Image {
                                                set_icon_name: Some("avatar-default-symbolic"),
                                            },
                                            append: artists_rvl = &gtk::Revealer {
                                                set_transition_duration: 200,
                                                set_transition_type: gtk::RevealerTransitionType::SlideRight,

                                                gtk::Label {
                                                    set_text: &gettext("Artists"),
                                                }
                                            }
                                        }
                                    },
                                    append: artist_rvl = &gtk::Revealer {
                                        set_transition_duration: 200,
                                        set_transition_type: gtk::RevealerTransitionType::SlideRight,

                                        #[wrap(Some)]
                                        set_child: artist_btn = &gtk::ToggleButton {
                                            add_css_class: "flat",
                                            add_css_class: "circular",

                                            gtk::Box {
                                                set_spacing: 3,

                                                gtk::Label {
                                                    set_text: &gettext("Artist"),
                                                }
                                            }
                                        }
                                    },
                                    append: albums_btn = &gtk::ToggleButton {
                                        add_css_class: "flat",
                                        add_css_class: "circular",
                                        set_tooltip: &gettext("Show albums"),
                                        connect_clicked => AppIn::ClickedNavigationBtn(ClickableViews::Albums),

                                        gtk::Box {
                                            set_spacing: 3,

                                            gtk::Image {
                                                set_icon_name: Some("media-optical-cd-audio-symbolic"),
                                            },
                                            append: albums_rvl = &gtk::Revealer {
                                                set_transition_duration: 200,
                                                set_transition_type: gtk::RevealerTransitionType::SlideRight,

                                                gtk::Label {
                                                    set_text: &gettext("Albums"),
                                                }
                                            }
                                        }
                                    },
                                    append: album_rvl = &gtk::Revealer {
                                        set_transition_duration: 200,
                                        set_transition_type: gtk::RevealerTransitionType::SlideRight,

                                        #[wrap(Some)]
                                        set_child: album_btn = &gtk::ToggleButton {
                                            add_css_class: "flat",
                                            add_css_class: "circular",

                                            gtk::Box {
                                                set_spacing: 3,

                                                gtk::Label {
                                                    set_text: &gettext("Album"),
                                                }
                                            }
                                        }
                                    },
                                    append: tracks_btn = &gtk::ToggleButton {
                                        add_css_class: "flat",
                                        add_css_class: "circular",
                                        set_tooltip: &gettext("Show tracks"),
                                        connect_clicked => AppIn::ClickedNavigationBtn(ClickableViews::Tracks),

                                        gtk::Box {
                                            set_spacing: 3,

                                            gtk::Image {
                                                set_icon_name: Some("audio-x-generic-symbolic"),
                                            },
                                            append: tracks_rvl = &gtk::Revealer {
                                                set_transition_duration: 200,
                                                set_transition_type: gtk::RevealerTransitionType::SlideRight,

                                                gtk::Label {
                                                    set_text: &gettext("Tracks"),
                                                }
                                            }
                                        }
                                    },
                                    append: playlists_btn = &gtk::ToggleButton {
                                        add_css_class: "flat",
                                        add_css_class: "circular",
                                        set_tooltip: &gettext("Show playlists"),
                                        connect_clicked => AppIn::ClickedNavigationBtn(ClickableViews::Playlists),

                                        // switch views when dragging over playlists button
                                        add_controller = gtk::DropTarget {
                                            set_actions: gdk::DragAction::MOVE | gdk::DragAction::COPY,
                                            set_types: &[<Droppable as gtk::prelude::StaticType>::static_type()],

                                            connect_enter[sender] => move |_controller, _x, _y| {
                                                sender.input(AppIn::ClickedNavigationBtn(ClickableViews::Playlists));
                                                gdk::DragAction::COPY
                                            }
                                        },

                                        gtk::Box {
                                            set_spacing: 3,

                                            gtk::Image {
                                                set_icon_name: Some("playlist-symbolic"),
                                            },
                                            append: playlists_rvl = &gtk::Revealer {
                                                set_transition_duration: 200,
                                                set_transition_type: gtk::RevealerTransitionType::SlideRight,

                                                gtk::Label {
                                                    set_text: &gettext("Playlists"),
                                                }
                                            },
                                        }
                                    },
                                }
                            },

                            #[wrap(Some)]
                            set_end_widget = &gtk::Box {
                                set_hexpand: true,
                                set_halign: gtk::Align::End,
                                set_spacing: 5,

                                append: equalizer_btn = &gtk::Button {
                                    add_css_class: "flat",
                                    add_css_class: "size24",
                                    add_css_class: "round-button",
                                    set_icon_name: "media-eq-symbolic",
                                    set_tooltip: &gettext("Open Equalizer settings"),

                                    connect_clicked[equalizer_popover] => move |_btn| {
                                        equalizer_popover.show();
                                    }
                                },

                                append: volume_btn = &gtk::Button {
                                    add_css_class: "flat",
                                    add_css_class: "size24",
                                    add_css_class: "round-button",
                                    set_tooltip: &gettext("Change volume by opening, scrolling or middle clicking"),

                                    connect_clicked[volume_popover] => move |_btn| {
                                        volume_popover.show();
                                    },

                                    // change volume on scrolling
                                    add_controller = gtk::EventControllerScroll {
                                        set_flags: gtk::EventControllerScrollFlags::VERTICAL,

                                        connect_scroll[last_volume_scroll_event, volume_button_sender] => move |controller, _x, y| {
                                            // make sure scroll events are spaced
                                            // otherwise the app locks up
                                            const TIMEOUT: u32 = 70;
                                            let event_time = controller.current_event_time();
                                            if event_time - *last_volume_scroll_event.borrow() < TIMEOUT {
                                                return gtk::glib::signal::Propagation::Proceed;
                                            }
                                            *last_volume_scroll_event.borrow_mut() = event_time;

                                            if y < 0.0 {
                                                volume_button_sender.emit(VolumeButtonIn::Increase);
                                            } else {
                                                volume_button_sender.emit(VolumeButtonIn::Decrease);
                                            }

                                            gtk::glib::signal::Propagation::Proceed
                                        }
                                    },

                                    // mute on middle click
                                    add_controller = gtk::GestureClick {
                                        set_button: 2, // middle click

                                        connect_released[volume_button_sender] => move |_controller, _number_of_clicks, _x, _y| {
                                            volume_button_sender.emit(VolumeButtonIn::MuteToggle);
                                        }
                                    }

                                },

                                gtk::Button {
                                    add_css_class: "flat",
                                    add_css_class: "size24",
                                    add_css_class: "round-button",
                                    set_icon_name: "open-menu-symbolic",
                                    set_tooltip: &gettext("Open settings"),

                                    connect_clicked => AppIn::OpenSettings,
                                },

                                gtk::WindowControls {
                                    set_side: gtk::PackType::End,
                                }
                            },
                        },
                    },

                    gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,

                        append: search_bar = &gtk::Revealer {
                            set_transition_duration: 200,
                            set_transition_type: gtk::RevealerTransitionType::SlideUp,
                            set_margin_top: 3,

                            gtk::Box {
                                set_spacing: 10,
                                set_halign: gtk::Align::Center,
                                set_margin_vertical: 2,

                                append: search = &gtk::SearchEntry {
                                    set_placeholder_text: Some(&gettext("Search...")),
                                    set_text: &Settings::get().lock().unwrap().search_text,
                                    set_tooltip: &gettext("Enter your search here"),
                                    connect_search_changed => AppIn::SearchChanged,
                                    add_controller = gtk::EventControllerKey {
                                        connect_key_pressed[sender] => move |_, key, _, _modifier| {
                                            if key == gtk::gdk::Key::Escape {
                                                sender.input(AppIn::SearchActivate(false));
                                            }
                                            gtk::glib::signal::Propagation::Proceed
                                        }
                                    }
                                },
                                gtk::CheckButton {
                                    set_label: Some(&gettext("Use fuzzy search")),
                                    set_tooltip: &gettext("Shows close and similar search results if activated"),
                                    set_active: Settings::get().lock().unwrap().fuzzy_search,

                                    connect_toggled[sender] => move |btn| {
                                        Settings::get().lock().unwrap().fuzzy_search = btn.is_active();
                                        sender.input(AppIn::SearchChanged);
                                    }
                                },
                                gtk::CheckButton {
                                    set_label: Some(&gettext("Use case sensitivity")),
                                    set_tooltip: &gettext("Ignores case sensitivity in search term and results"),
                                    set_active: Settings::get().lock().unwrap().case_sensitive,

                                    connect_toggled[sender] => move |btn| {
                                        Settings::get().lock().unwrap().case_sensitive = btn.is_active();
                                        sender.input(AppIn::SearchChanged);

                                    }
                                }
                            }
                        },

                        gtk::Overlay {
                            #[wrap(Some)]
                            set_child = model.browser.widget(),

                            // overlay for showing the cover in the original size
                            add_overlay: big_cover_overlay = &gtk::Revealer {
                                set_transition_type: gtk::RevealerTransitionType::Crossfade,
                                set_transition_duration: 1000,
                                set_visible: false,

                                // disable overlay when it is clicked
                                add_controller = gtk::GestureClick {
                                    connect_pressed[sender] => move |_ctrl, _btn, _x, _y| {
                                        sender.input(AppIn::DisableBigCoverOverlay);
                                    },
                                },

                                #[wrap(Some)]
                                set_child: big_cover_picture = &gtk::Picture {}
                            }
                        }
                    },
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
            AppIn::PlayControlOutput(input) => match input {
                PlayControlOut::Player(cmd) => sender.input(AppIn::Player(cmd)),
            },
            AppIn::Seekbar(msg) => match msg {
                SeekbarOut::SeekDragged(seek_in_ms) => {
                    if let Err(e) = self.playback.borrow_mut().set_position(seek_in_ms) {
                        sender.input(AppIn::DisplayToast(format!("seek failed: {e:?}")));
                    }
                }
            },
            AppIn::Playback(playback) => match playback {
                PlaybackOut::TrackEnd => sender.input(AppIn::Player(Command::Next)),
                PlaybackOut::SongPosition(ms) => {
                    sender.input(AppIn::Player(Command::SetSongPosition(ms)));
                }
                PlaybackOut::ScrobbleThresholdReached => {
                    if Settings::get().lock().unwrap().scrobble {
                        let child = match self.queue.model().current() {
                            Some((_index, row)) => row.item().clone(),
                            None => {
                                return;
                            }
                        };

                        let client = Client::get().unwrap();
                        if let Err(e) = client.scrobble(vec![(&child.id, None)], Some(true)).await {
                            sender.input(AppIn::DisplayToast(format!(
                                "could not scrobble to server: {e:?}"
                            )));
                            return;
                        }

                        // update subsonic cache
                        self.subsonic.borrow_mut().increment_play_counter(&child);

                        // update played counter in app
                        let play_count = match &child.play_count {
                            None => Some(1),
                            Some(i) => Some(i + 1),
                        };
                        self.queue
                            .emit(QueueIn::UpdatePlayCountSong(child.id.clone(), play_count));
                        self.browser
                            .emit(BrowserIn::UpdatePlayCountSong(child.id.clone(), play_count));

                        // check if play count album changed
                        let Some(album) = self.subsonic.borrow().album_of_song(&child) else {
                            return;
                        };
                        match client.get_album(&album.id).await {
                            Err(e) => {
                                sender.input(AppIn::DisplayToast(format!(
                                    "could not find album: {e:?}"
                                )));
                            }
                            Ok(server_album) => {
                                if server_album.base.play_count != album.play_count {
                                    self.browser.emit(BrowserIn::UpdatePlayCountAlbum(
                                        album.id.clone(),
                                        server_album.base.play_count,
                                    ));
                                }
                            }
                        }
                    }
                }
            },
            AppIn::Equalizer(msg) => match msg {
                EqualizerOut::Changed => self.playback.borrow_mut().sync_equalizer(),
                EqualizerOut::DisplayToast(msg) => sender.input(AppIn::DisplayToast(msg)),
            },
            AppIn::Logout => sender.output(AppOut::Logout).unwrap(),
            AppIn::ClearCache => {
                if let Err(e) = self.subsonic.borrow_mut().delete_cache() {
                    sender.input(AppIn::DisplayToast(format!(
                        "error while deleting cache: {e:?}"
                    )));
                } else {
                    sender.input(AppIn::DisplayToast(gettext(
                        "Deleted cache\nPlease restart to reload the cache",
                    )));
                }
                self.queue.emit(QueueIn::Clear);

                sender.output(AppOut::Reload).unwrap();
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
                            if let Err(e) = self.playback.borrow_mut().set_track(url) {
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
                    if let Err(e) = self.playback.borrow_mut().play() {
                        sender.input(AppIn::DisplayToast(format!(
                            "could not set playback to play: {e:?}"
                        )));
                    }

                    sender.input(AppIn::DesktopNotification);

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

                    self.mpris.borrow_mut().set_song(Some(*child));
                    self.mpris.borrow_mut().set_state(PlayState::Play);
                }
                QueueOut::QueueEmpty => {
                    if let Err(e) = self.playback.borrow_mut().stop() {
                        sender.input(AppIn::DisplayToast(format!("{e}")));
                    }
                    self.play_info.emit(PlayInfoIn::NewState(Box::new(None)));
                    self.play_controls.emit(PlayControlIn::Disable);
                    self.mpris.borrow_mut().can_play(false);
                }
                QueueOut::QueueNotEmpty => {
                    self.play_controls.emit(PlayControlIn::Enable);
                    self.mpris.borrow_mut().can_play(true);
                }
                QueueOut::Player(cmd) => sender.input(AppIn::Player(cmd)),
                QueueOut::CreatePlaylist => {
                    self.browser.emit(BrowserIn::NewPlaylist(
                        gettext("New playlist from Queue"),
                        self.queue.model().songs(),
                    ));
                }
                QueueOut::DisplayToast(title) => sender.input(AppIn::DisplayToast(title)),
                QueueOut::FavoriteClicked(id, state) => {
                    sender.input(AppIn::FavoriteSongClicked(id, state));
                }
                QueueOut::QueueSongsChanged => {
                    sender.input(AppIn::UpdateCanPlayNextOrPrev);
                }
                QueueOut::SongChanged => {
                    sender.input(AppIn::UpdateCanPlayNextOrPrev);
                    if Settings::get().lock().unwrap().queue_jump_to_new_song {
                        self.queue.emit(QueueIn::JumpToCurrent);
                    }
                }
            },
            AppIn::Browser(msg) => match msg {
                BrowserOut::AppendToQueue(drop) => self.queue.emit(QueueIn::Append(drop)),
                BrowserOut::ReplaceQueue(drop) => self.queue.emit(QueueIn::Replace(drop)),
                BrowserOut::InsertAfterCurrentInQueue(drop) => {
                    self.queue.emit(QueueIn::InsertAfterCurrentlyPlayed(drop));
                }
                BrowserOut::BackButtonSensitivity(status) => widgets.back_btn.set_sensitive(status),
                BrowserOut::DisplayToast(title) => sender.input(AppIn::DisplayToast(title)),
                BrowserOut::FavoriteAlbumClicked(id, state) => {
                    sender.input(AppIn::FavoriteAlbumClicked(id, state));
                }
                BrowserOut::FavoriteArtistClicked(id, state) => {
                    sender.input(AppIn::FavoriteArtistClicked(id, state));
                }
                BrowserOut::FavoriteSongClicked(id, state) => {
                    sender.input(AppIn::FavoriteSongClicked(id, state));
                }
                BrowserOut::Download(drop) => sender.input(AppIn::Download(drop)),
                BrowserOut::ChangedViewTo(view) => {
                    sender.input(AppIn::SearchActivate(false));
                    match view {
                        Views::Clickable(view) => sender.input(AppIn::ClickedNavigationBtn(view)),
                        Views::Artist => {
                            reset_navigation_btns(widgets);
                            widgets.artist_rvl.set_reveal_child(true);
                            widgets.artist_btn.set_active(true);
                        }
                        Views::Album => {
                            reset_navigation_btns(widgets);
                            widgets.album_rvl.set_reveal_child(true);
                            widgets.album_btn.set_active(true);
                        }
                    }
                }
            },
            AppIn::PlayInfo(msg) => match msg {
                PlayInfoOut::DisplayToast(title) => sender.input(AppIn::DisplayToast(title)),
                PlayInfoOut::ShowAlbum(id) => self.browser.emit(BrowserIn::ShowAlbum(id)),
                PlayInfoOut::ShowArtist(id) => self.browser.emit(BrowserIn::ShowArtist(id)),
                PlayInfoOut::CoverClicked(cover_id) => {
                    sender.input(AppIn::LoadBigCoverPicture(cover_id))
                }
            },
            AppIn::DisplayToast(title) => {
                sender.output(AppOut::DisplayToast(title)).unwrap();
            }
            AppIn::DesktopNotification => {
                let song = self.queue.model().current();
                if let Some((_i, song)) = song {
                    show_desktop_notification(&self.subsonic, song.item().clone(), sender).await;
                }
            }
            AppIn::Mpris(msg) => match msg {
                MprisOut::Player(cmd) => sender.input(AppIn::Player(cmd)),
                MprisOut::WindowQuit => relm4::main_application().quit(),
            },
            AppIn::Player(cmd) => match cmd {
                Command::Next => {
                    if !self.queue.model().can_play_next() {
                        sender.input(AppIn::Player(Command::Stop));
                        return;
                    }
                    self.queue.emit(QueueIn::PlayNext);
                    self.mpris.borrow_mut().set_state(PlayState::Play);
                }
                Command::Previous => {
                    if !self.queue.model().can_play_previous() {
                        return;
                    }
                    self.queue.emit(QueueIn::PlayPrevious);
                    self.mpris.borrow_mut().set_state(PlayState::Play);
                }
                Command::Play => {
                    if !self.queue.model().can_play() {
                        return;
                    }
                    // play the next song if no song is played
                    if self.queue.model().current().is_none() {
                        sender.input(AppIn::Player(Command::Next));
                        return;
                    }

                    if let Err(e) = self.playback.borrow_mut().play() {
                        sender.input(AppIn::DisplayToast(format!(
                            "could not play playback: {e:?}"
                        )));
                    }
                    self.mpris.borrow_mut().set_state(PlayState::Play);
                }
                Command::Pause => {
                    if let Err(e) = self.playback.borrow_mut().pause() {
                        sender.input(AppIn::DisplayToast(format!(
                            "could not pause playback: {e:?}"
                        )));
                    }
                    self.play_controls
                        .emit(PlayControlIn::NewState(PlayState::Pause));
                    self.queue.emit(QueueIn::NewState(PlayState::Pause));
                    self.mpris.borrow_mut().set_state(PlayState::Pause);
                }
                Command::PlayPause => match self.playback.borrow_mut().is_playing() {
                    PlayState::Stop | PlayState::Pause => {
                        sender.input(AppIn::Player(Command::Play));
                    }
                    PlayState::Play => sender.input(AppIn::Player(Command::Pause)),
                },
                Command::Stop => {
                    if let Err(e) = self.playback.borrow_mut().stop() {
                        sender.input(AppIn::DisplayToast(format!(
                            "could not stop playback: {e:?}"
                        )));
                    }
                    self.play_info.emit(PlayInfoIn::NewState(Box::new(None)));
                    self.play_controls
                        .emit(PlayControlIn::NewState(PlayState::Stop));
                    self.queue.emit(QueueIn::NewState(PlayState::Stop));
                    self.seekbar.emit(SeekbarIn::Disable);
                    self.mpris.borrow_mut().set_state(PlayState::Stop);
                    self.mpris.borrow_mut().can_play_previous(false);
                    self.play_controls
                        .emit(PlayControlIn::DisablePrevious(false));
                }
                Command::SetSongPosition(pos_ms) => {
                    // sanitiy check
                    if pos_ms < 0 {
                        self.seekbar.emit(SeekbarIn::SeekTo(0));
                        self.play_controls
                            .emit(PlayControlIn::NewState(PlayState::Play));
                        self.mpris.borrow_mut().set_position(0);
                    } else if pos_ms > self.seekbar.model().length() {
                        sender.input(AppIn::Player(Command::Next));
                    } else {
                        self.seekbar.emit(SeekbarIn::SeekTo(pos_ms));
                        self.play_controls
                            .emit(PlayControlIn::NewState(PlayState::Play));
                        self.mpris.borrow_mut().set_position(pos_ms);
                    }
                }
                Command::Volume(volume) => {
                    self.playback.borrow_mut().set_volume(volume);
                    self.volume_button
                        .emit(VolumeButtonIn::ChangeVolumeTo(volume));
                    self.mpris.borrow_mut().set_volume(volume);
                    let mut settings = Settings::get().lock().unwrap();
                    settings.volume = volume;
                    if volume > 0.01 {
                        settings.mute = false;
                    }
                    if let Err(e) = settings.save() {
                        sender.input(AppIn::DisplayToast(format!("error saving settings: {e}")));
                    }
                }
                Command::Repeat(repeat) => {
                    self.mpris.borrow_mut().set_loop_status(repeat.clone());
                }
                Command::Shuffle(shuffle) => {
                    self.mpris.borrow_mut().set_shuffle(shuffle.clone());
                }
                Command::MuteToggle => {
                    let mut settings = Settings::get().lock().unwrap();
                    settings.mute = !settings.mute;
                    let volume = if settings.mute { 0.0 } else { settings.volume };
                    self.playback.borrow_mut().set_volume(volume);
                    self.mpris.borrow_mut().set_volume(volume);
                    if let Err(e) = settings.save() {
                        sender.input(AppIn::DisplayToast(format!("error saving settings: {e}")));
                    }
                }
            },
            AppIn::FavoriteAlbumClicked(id, state) => {
                let client = Client::get().unwrap();
                let empty: Vec<String> = vec![];
                let result = match state {
                    true => client.star(empty.clone(), vec![&id], empty).await,
                    false => client.unstar(empty.clone(), vec![&id], empty).await,
                };
                match result {
                    Err(e) => {
                        sender.input(AppIn::DisplayToast(format!("could not star album: {e:?}")));
                    }
                    Ok(_info) => {
                        // change subsonic
                        self.subsonic.borrow_mut().favorite_album(id.clone(), state);

                        //update view
                        self.browser.emit(BrowserIn::UpdateFavoriteAlbum(id, state));
                    }
                }
            }
            AppIn::FavoriteArtistClicked(id, state) => {
                let client = Client::get().unwrap();
                let empty: Vec<String> = vec![];
                let result = match state {
                    true => client.star(empty.clone(), empty, vec![&id]).await,
                    false => client.unstar(empty.clone(), empty, vec![&id]).await,
                };
                match result {
                    Err(e) => {
                        sender.input(AppIn::DisplayToast(format!("could not star artist: {e:?}")));
                    }
                    Ok(_info) => {
                        // change subsonic
                        self.subsonic
                            .borrow_mut()
                            .favorite_artist(id.clone(), state);

                        //update view
                        self.browser
                            .emit(BrowserIn::UpdateFavoriteArtist(id, state));
                    }
                }
            }
            AppIn::FavoriteSongClicked(id, state) => {
                // change on server
                let client = Client::get().unwrap();
                let empty: Vec<String> = vec![];
                let result = match state {
                    true => client.star(vec![&id], empty.clone(), empty).await,
                    false => client.unstar(vec![&id], empty.clone(), empty).await,
                };
                match result {
                    Err(e) => {
                        sender.input(AppIn::DisplayToast(format!("could not star song: {e:?}")));
                    }
                    Ok(_info) => {
                        // change subsonic
                        self.subsonic.borrow_mut().favorite_song(id.clone(), state);

                        //update views
                        self.queue
                            .emit(QueueIn::UpdateFavoriteSong(id.clone(), state));
                        self.browser.emit(BrowserIn::UpdateFavoriteSong(id, state));
                    }
                }
            }
            AppIn::SearchActivate(true) => {
                Settings::get().lock().unwrap().search_active = true;
                widgets.search_bar.set_reveal_child(true);
                widgets.search_btn.set_active(true);
                self.browser
                    .emit(BrowserIn::SearchChanged(widgets.search.text().to_string()));
                widgets.search.grab_focus();
            }
            AppIn::SearchActivate(false) => {
                Settings::get().lock().unwrap().search_active = false;
                widgets.search_bar.set_reveal_child(false);
                widgets.search_btn.set_active(false);
                self.browser.emit(BrowserIn::SearchChanged(String::new()));
            }
            AppIn::SearchChanged => {
                Settings::get().lock().unwrap().search_text = widgets.search.text().to_string();
                if Settings::get().lock().unwrap().search_active {
                    self.browser
                        .emit(BrowserIn::SearchChanged(widgets.search.text().to_string()));
                }
            }
            AppIn::Download(drop) => Download::download(&self.subsonic, sender.clone(), drop),
            AppIn::ClickedNavigationBtn(view) => {
                reset_navigation_btns(widgets);

                match view {
                    ClickableViews::Dashboard => {
                        self.browser.emit(BrowserIn::ShowDashboard);
                        widgets.dashboard_rvl.set_reveal_child(true);
                        widgets.dashboard_btn.set_active(true);
                    }
                    ClickableViews::Artists => {
                        self.browser.emit(BrowserIn::ShowArtists);
                        widgets.artists_rvl.set_reveal_child(true);
                        widgets.artists_btn.set_active(true);
                    }
                    ClickableViews::Albums => {
                        self.browser.emit(BrowserIn::ShowAlbums);
                        widgets.albums_rvl.set_reveal_child(true);
                        widgets.albums_btn.set_active(true);
                    }
                    ClickableViews::Tracks => {
                        self.browser.emit(BrowserIn::ShowTracks);
                        widgets.tracks_rvl.set_reveal_child(true);
                        widgets.tracks_btn.set_active(true);
                    }
                    ClickableViews::Playlists => {
                        self.browser.emit(BrowserIn::ShowPlaylists);
                        widgets.playlists_rvl.set_reveal_child(true);
                        widgets.playlists_btn.set_active(true);
                    }
                }
            }
            AppIn::DisableBigCoverOverlay => {
                widgets.big_cover_overlay.set_visible(false);
                widgets.big_cover_overlay.set_reveal_child(false);
            }
            AppIn::LoadBigCoverPicture(cover_id) => {
                // get url
                let client = Client::get().unwrap();
                let buffer = match client.get_cover_art(&cover_id, None).await {
                    Ok(buffer) => buffer,
                    Err(e) => {
                        sender.input(AppIn::DisplayToast(format!(
                            "error fetching cover {cover_id}: {e}"
                        )));
                        return;
                    }
                };
                let bytes = gtk::glib::Bytes::from(&buffer);
                let texture = match gtk::gdk::Texture::from_bytes(&bytes) {
                    Ok(texture) => Some(texture),
                    Err(e) => {
                        // could not convert to image
                        tracing::warn!("converting buffer to Pixbuf: {e} for {cover_id}");
                        None
                    }
                };
                widgets.big_cover_picture.set_paintable(texture.as_ref());
                widgets.big_cover_overlay.set_visible(true);
                widgets.big_cover_overlay.set_reveal_child(true);
            }
            AppIn::UpdateCanPlayNextOrPrev => {
                let can_prev = self.queue.model().can_play_previous();
                self.play_controls
                    .emit(PlayControlIn::DisablePrevious(can_prev));
                self.mpris.borrow_mut().can_play_previous(can_prev);
                let can_next = self.queue.model().can_play_next();
                self.play_controls
                    .emit(PlayControlIn::DisableNext(can_next));
                self.mpris.borrow_mut().can_play_next(can_next);
            }
            AppIn::OpenSettings => self.settings_window.emit(SettingsWindowIn::Show),
            AppIn::BackPressed => self.browser.emit(BrowserIn::GoBack),
            AppIn::SettingsWindow(msg) => match msg {
                SettingsWindowOut::ClearCache => sender.input(AppIn::ClearCache),
                SettingsWindowOut::Logout => sender.input(AppIn::Logout),
            },
            AppIn::VolumeButton(msg) => match msg {
                VolumeButtonOut::Player(cmd) => sender.input(AppIn::Player(cmd)),
                VolumeButtonOut::ButtonStateChangedTo(state) => {
                    widgets.volume_btn.set_icon_name(state.as_ref());
                }
            },
        }
    }

    fn shutdown(&mut self, widgets: &mut Self::Widgets, _sender: relm4::Sender<Self::Output>) {
        tracing::info!("shutdown app");

        //save queue
        if let Err(e) = self.queue.model().save() {
            tracing::error!("could not save queue: {e}");
        }

        //save window state to settings
        let mut settings = Settings::get().lock().unwrap();
        settings.paned_position = widgets.paned.position();

        // save settings
        if let Err(e) = settings.save() {
            tracing::error!("error saving settings: {e}");
        }
    }
}

async fn show_desktop_notification(
    subsonic: &Rc<RefCell<Subsonic>>,
    child: submarine::data::Child,
    sender: relm4::component::AsyncComponentSender<App>,
) {
    // check if option to send is enabled
    if !Settings::get().lock().unwrap().send_notifications {
        return;
    }
    // check if app is in background
    let windows = relm4::main_application().windows();
    if windows.iter().any(|w| w.is_active()) {
        return;
    }

    // get cover image
    let image: Option<notify_rust::Image> = {
        let client = Client::get().unwrap();
        let Some(cover_art) = &child.cover_art else {
            return;
        };

        // check cached covers
        let cached_buffer = subsonic.borrow().cover_raw(cover_art);
        let image_buffer = if let Some(raw) = cached_buffer {
            match image::load_from_memory(&raw) {
                Ok(image) => image.to_rgb8(),
                Err(e) => {
                    sender.input(AppIn::DisplayToast(format!(
                        "error loading image from memory: {e}"
                    )));
                    return;
                }
            }
        } else {
            // cover not locally found, try server
            if let Ok(raw) = client.get_cover_art(cover_art, Some(200)).await {
                // update cache
                subsonic
                    .borrow_mut()
                    .cover_update(cover_art, Some(raw.clone()));

                // convert raw buffer to format notify_rust can use
                match image::load_from_memory(&raw) {
                    Ok(image) => image.to_rgb8(),
                    Err(e) => {
                        sender.input(AppIn::DisplayToast(format!(
                            "error loading image from memory: {e}"
                        )));
                        return;
                    }
                }
            } else {
                tracing::warn!("there is no cover for {cover_art} locally or on server");
                return;
            }
        };
        notify_rust::Image::from_rgb(
            image_buffer.width() as i32,
            image_buffer.height() as i32,
            image_buffer.to_vec(),
        )
        .ok()
    };

    // send notification
    let mut notify = notify_rust::Notification::new();
    notify.summary(config::APP_NAME);
    notify.icon(config::APP_ID);
    notify.body(&format!(
        "{}\n{}",
        child.title,
        child.artist.unwrap_or(gettext("Unkonwn Artist"))
    ));
    if let Some(image) = image {
        notify.image_data(image);
    }
    if let Err(e) = notify.show() {
        sender.input(AppIn::DisplayToast(format!(
            "Could not send desktop notification: {e:?}"
        )));
    }
}

fn reset_navigation_btns(widgets: &mut <App as relm4::component::AsyncComponent>::Widgets) {
    widgets.dashboard_rvl.set_reveal_child(false);
    widgets.artists_rvl.set_reveal_child(false);
    widgets.artist_rvl.set_reveal_child(false);
    widgets.albums_rvl.set_reveal_child(false);
    widgets.album_rvl.set_reveal_child(false);
    widgets.tracks_rvl.set_reveal_child(false);
    widgets.playlists_rvl.set_reveal_child(false);

    widgets.dashboard_btn.set_active(false);
    widgets.artists_btn.set_active(false);
    widgets.artist_btn.set_active(false);
    widgets.albums_btn.set_active(false);
    widgets.album_btn.set_active(false);
    widgets.tracks_btn.set_active(false);
    widgets.playlists_btn.set_active(false);
}
