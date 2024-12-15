use std::{cell::RefCell, rc::Rc};

use granite::prelude::ToastExt;
use gtk::prelude::{BoxExt, ButtonExt, CheckButtonExt, OrientableExt, ScaleButtonExt};
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
    download::Download,
    mpris::{Mpris, MprisOut},
    play_state::PlayState,
    playback::{Playback, PlaybackOut},
    player::Command,
    settings::Settings,
    subsonic::Subsonic,
    types::Droppable,
};
use crate::{
    components::{
        browser::{Browser, BrowserIn, BrowserOut},
        equalizer::{Equalizer, EqualizerOut},
        play_controls::{PlayControl, PlayControlIn, PlayControlOut},
        play_info::{PlayInfo, PlayInfoIn, PlayInfoOut},
        queue::{Queue, QueueIn, QueueOut},
        seekbar::{Seekbar, SeekbarCurrent, SeekbarIn, SeekbarOut},
    },
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
}

impl App {
    fn recalculate_mpris_next_prev(&mut self) {
        let can_prev = self.queue.model().can_play_previous();
        self.play_controls
            .emit(PlayControlIn::DisablePrevious(can_prev));
        self.mpris.borrow_mut().can_play_previous(can_prev);
        let can_next = self.queue.model().can_play_next();
        self.play_controls
            .emit(PlayControlIn::DisableNext(can_next));
        self.mpris.borrow_mut().can_play_next(can_next);
    }
}

#[derive(Debug)]
pub enum AppIn {
    ResetLogin,
    DeleteCache,
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
}

#[derive(Debug)]
pub enum AppOut {
    Logout,
    Reload,
}

relm4::new_action_group!(AppActionGroup, "win");
relm4::new_stateless_action!(ActivateSearchAction, AppActionGroup, "activate-search");

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
        (args, mpris, playback): Self::Init,
        root: Self::Root,
        sender: relm4::AsyncComponentSender<Self>,
    ) -> relm4::component::AsyncComponentParts<Self> {
        let time_startup = std::time::Instant::now();

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
            playback.borrow_mut().pause().unwrap();
        }

        tracing::info!("start loading subsonic information");
        let subsonic = Subsonic::load_or_create().await.unwrap_or_default();
        let subsonic = std::rc::Rc::new(std::cell::RefCell::new(subsonic));
        tracing::info!("finished loaded subsonic information");

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

        let model = App {
            playback,
            subsonic,
            mpris,

            queue,
            play_controls,
            seekbar,
            play_info,
            browser,
            equalizer,
        };

        let browser_sender = model.browser.sender().clone();
        let widgets = view_output!();
        tracing::info!("loaded main window");

        // set application shortcuts
        // quit
        let application = relm4::main_application();
        application.set_accelerators_for_action::<ActivateSearchAction>(&["<Primary>F"]);
        let search_btn = widgets.search_btn.clone();
        let senderc = sender.clone();
        let activate_search_action: relm4::actions::RelmAction<ActivateSearchAction> =
            relm4::actions::RelmAction::new_stateless(move |_| {
                tracing::info!("activate search called");
                search_btn.set_active(true);
                senderc.input(AppIn::SearchActivate(true));
            });

        let mut group = relm4::actions::RelmActionGroup::<AppActionGroup>::new();
        group.add_action(activate_search_action);
        group.register_for_widget(&widgets.main_window);

        //init widgets
        {
            let settings = Settings::get().lock().unwrap();
            widgets.volume_btn.set_value(settings.volume);
            model.mpris.borrow_mut().set_volume(settings.volume);

            //seekbar
            model
                .seekbar
                .emit(SeekbarIn::SeekTo(settings.queue_seek as i64));
            if model.playback.borrow().is_track_set() {
                if let Err(e) = model.playback.borrow().set_position(settings.queue_seek as i64) {
                    sender.input(AppIn::DisplayToast(format!("playback set position error: {e}")));
                }
            }

            // playcontrol
            if model.queue.model().songs().is_empty() {
                model.play_controls.emit(PlayControlIn::Disable);
            }
        }

        //regularly save
        let library = model.subsonic.clone();
        gtk::glib::spawn_future_local(async move {
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(120)).await;
                tracing::info!("periodic save");
                library.borrow().save().unwrap();
            }
        });

        if args.borrow().time_startup {
            let shutdown = std::time::Instant::now();
            let duration = shutdown - time_startup;
            tracing::info!("startup time was {duration:?}");
            application.quit();
        }

        relm4::component::AsyncComponentParts { model, widgets }
    }

    view! {
        #[root]
        main_window = gtk::Box {
            add_css_class: "main-box",
            set_orientation: gtk::Orientation::Vertical,

            append: paned = &gtk::Paned {
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
                            append: back_btn = &gtk::Button {
                                set_icon_name: "go-previous-symbolic",
                                add_css_class: "destructive-button-spacer",
                                set_tooltip: "Go back to previous page",

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

                            append: search_btn = &gtk::ToggleButton {
                                add_css_class: "browser-navigation-button",
                                set_icon_name: "system-search-symbolic",
                                set_tooltip: "Open search bar",

                                connect_toggled[sender] => move |button| {
                                    match button.is_active() {
                                        true => sender.input(AppIn::SearchActivate(true)),
                                        false => sender.input(AppIn::SearchActivate(false)),
                                    }
                                }
                            },

                            gtk::Box {
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
                                    set_tooltip: "Show artists",
                                    connect_clicked[browser_sender] => move |_| {
                                        browser_sender.emit(BrowserIn::ArtistsClicked);
                                    }
                                },
                                gtk::Button {
                                    add_css_class: "browser-navigation-button",
                                    set_icon_name: "media-optical-cd-audio-symbolic",
                                    set_tooltip: "Show albums",
                                    connect_clicked[browser_sender] => move |_| {
                                        browser_sender.emit(BrowserIn::AlbumsClicked);
                                    }
                                },
                                gtk::Button {
                                    add_css_class: "browser-navigation-button",
                                    set_icon_name: "audio-x-generic-symbolic",
                                    set_tooltip: "Show tracks",
                                    connect_clicked[browser_sender] => move |_| {
                                        browser_sender.emit(BrowserIn::TracksClicked);
                                    }
                                },
                                gtk::Button {
                                    add_css_class: "browser-navigation-button",
                                    set_icon_name: "playlist-symbolic",
                                    set_tooltip: "Show playlists",
                                    connect_clicked[browser_sender] => move |_| {
                                        browser_sender.emit(BrowserIn::PlaylistsClicked);
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
                                set_tooltip: "Open Equalizer",
                                #[wrap(Some)]
                                set_popover = &gtk::Popover {
                                    model.equalizer.widget(),
                                },
                            },

                            append: volume_btn = &gtk::VolumeButton {
                                set_focus_on_click: false,
                                connect_value_changed[sender] => move |_scale, value| {
                                    sender.input(AppIn::Player(Command::Volume(value)));
                                }
                            },

                            gtk::MenuButton {
                                set_icon_name: "open-menu-symbolic",
                                set_focus_on_click: false,
                                set_tooltip: "Open settings",

                                #[wrap(Some)]
                                set_popover = &gtk::Popover {
                                    set_position: gtk::PositionType::Right,

                                    gtk::Box {
                                        add_css_class: "config-menu",
                                        set_orientation: gtk::Orientation::Vertical,
                                        set_spacing: 15,

                                        gtk::CenterBox {
                                            #[wrap(Some)]
                                            set_start_widget = &gtk::Label {
                                                set_text: "Send desktop notifications",
                                            },
                                            #[wrap(Some)]
                                            set_end_widget = &gtk::Switch {
                                                set_tooltip: "Wether or not send desktop notifications",
                                                set_state: Settings::get().lock().unwrap().send_notifications,
                                                connect_state_set => move |_switch, value| {
                                                    Settings::get().lock().unwrap().send_notifications = value;
                                                    gtk::glib::signal::Propagation::Proceed
                                                }
                                            },
                                        },
                                        gtk::CenterBox {
                                            #[wrap(Some)]
                                            set_start_widget = &gtk::Label {
                                                set_text: "Scrobble to server",
                                            },
                                            #[wrap(Some)]
                                            set_end_widget = &gtk::Switch {
                                                set_state: Settings::get().lock().unwrap().scrobble,
                                                set_tooltip: "Updates play count, played timestamp on server and the now playing page in the web app",

                                                connect_state_set => move |_switch, value| {
                                                    Settings::get().lock().unwrap().scrobble = value;
                                                    gtk::glib::signal::Propagation::Proceed
                                                }
                                            },
                                        },
                                        gtk::Separator {},
                                        gtk::Box {
                                            set_halign: gtk::Align::End,
                                            gtk::Button {
                                                add_css_class: "destructive-action",
                                                set_label: "Delete cache",
                                                set_tooltip: "Deletes the local cache of Covers and Metadata of music. They will be redownloaded from the server on the next start",
                                                connect_clicked => AppIn::DeleteCache,
                                            }
                                        },
                                        gtk::Box {
                                            set_halign: gtk::Align::End,
                                            gtk::Button {
                                                add_css_class: "destructive-action",
                                                set_label: "Logout from Server",
                                                set_tooltip: "Logging out will delete the cache and also require to login again to listen to music",
                                                connect_clicked => AppIn::ResetLogin,
                                            },

                                        },
                                    },
                                },
                            },

                            gtk::WindowControls {
                                set_side: gtk::PackType::End,
                            }
                        },
                    },

                    gtk::Overlay {
                        #[wrap(Some)]
                        set_child = &gtk::Box {
                            set_orientation: gtk::Orientation::Vertical,

                            append: search_bar = &gtk::Revealer {
                                set_transition_duration: 200,
                                set_transition_type: gtk::RevealerTransitionType::SlideUp,

                                gtk::Box {
                                    set_spacing: 10,
                                    set_halign: gtk::Align::Center,
                                    set_margin_vertical: 2,

                                    append: search = &gtk::SearchEntry {
                                        set_placeholder_text: Some("Search..."),
                                        set_text: &Settings::get().lock().unwrap().search_text,
                                        set_tooltip: "Enter your search here",
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
                                        set_label: Some("Use fuzzy search"),
                                        set_tooltip: "Shows close and similar search results if activated",
                                        set_active: Settings::get().lock().unwrap().fuzzy_search,

                                        connect_toggled[sender] => move |btn| {
                                            Settings::get().lock().unwrap().fuzzy_search = btn.is_active();
                                            sender.input(AppIn::SearchChanged);
                                        }
                                    },
                                    gtk::CheckButton {
                                        set_label: Some("Use case sensitivity"),
                                        set_tooltip: "Ignores case sensitivity in search term and results",
                                        set_active: Settings::get().lock().unwrap().case_sensitive,

                                        connect_toggled[sender] => move |btn| {
                                            Settings::get().lock().unwrap().case_sensitive = btn.is_active();
                                            sender.input(AppIn::SearchChanged);

                                        }
                                    }
                                }
                            },

                            model.browser.widget(),
                        },
                        add_overlay: toasts = &granite::Toast,
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
            },
            AppIn::Equalizer(_changed) => {
                self.playback.borrow_mut().sync_equalizer();
            }
            AppIn::ResetLogin => {
                let mut settings = Settings::get().lock().unwrap();
                settings.reset_login();
                sender.output(AppOut::Logout).unwrap();
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
                            "could set playback to play: {e:?}"
                        )));
                    }

                    sender.input(AppIn::DesktopNotification);

                    //scrobble
                    let scrobble = Settings::get().lock().unwrap().scrobble;
                    if scrobble {
                        if let Err(e) = client.scrobble(vec![(&child.id, None)], Some(true)).await {
                            sender.input(AppIn::DisplayToast(format!(
                                "could not find song streaming url: {e:?}"
                            )));
                            return;
                        }
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
                    self.mpris.borrow_mut().set_song(Some(*child)).await;
                    self.recalculate_mpris_next_prev();
                    self.mpris.borrow_mut().set_state(PlayState::Play);
                }
                QueueOut::QueueEmpty => {
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
                        String::from("Playlist from Queue"),
                        self.queue.model().songs(),
                    ));
                }
                QueueOut::DisplayToast(title) => sender.input(AppIn::DisplayToast(title)),
                QueueOut::DesktopNotification(child) => {
                    if Settings::get().lock().unwrap().send_notifications {
                        let image: Option<notify_rust::Image> = {
                            let client = Client::get().unwrap();
                            if let Ok(raw) = client
                                .get_cover_art(child.cover_art.unwrap(), Some(100))
                                .await
                            {
                                let image_buffer = image::load_from_memory(&raw).unwrap().to_rgb8();
                                let buffer: Vec<u8> = image_buffer.to_vec();
                                match notify_rust::Image::from_rgb(
                                    image_buffer.width() as i32,
                                    image_buffer.height() as i32,
                                    buffer,
                                ) {
                                    Ok(image) => Some(image),
                                    Err(_) => None,
                                }
                            } else {
                                None
                            }
                        };

                        let mut notify = notify_rust::Notification::new();
                        notify.summary("buoy");
                        notify.body(&format!(
                            "{}\n{}",
                            child.title,
                            child.artist.unwrap_or(String::from("Unkonwn Artist"))
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
                }
                QueueOut::FavoriteClicked(id, state) => {
                    sender.input(AppIn::FavoriteSongClicked(id, state));
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
            },
            AppIn::PlayInfo(msg) => match msg {
                PlayInfoOut::DisplayToast(title) => sender.input(AppIn::DisplayToast(title)),
            },
            AppIn::DisplayToast(title) => {
                tracing::error!(title);
                widgets.toasts.set_title(&title);
                widgets.toasts.send_notification();
            }
            AppIn::DesktopNotification => {
                if !Settings::get().lock().unwrap().send_notifications {
                    return;
                }

                // take current song, then its album, then the album art
                let song = self.queue.model().current_song();
                if let Some(song) = song {
                    if let Some(album) = song.album_id {
                        let album = self.subsonic.borrow().find_album(album);
                        if let Some(album) = album {
                            let image: Option<notify_rust::Image> = {
                                if let Some(cover_art) = album.cover_art {
                                    if let Some(buffer) =
                                        self.subsonic.borrow().cover_raw(&cover_art)
                                    {
                                        match image::load_from_memory(&buffer) {
                                            Err(e) => {
                                                tracing::error!("converting cached buffer: {e:?}");
                                                None
                                            }
                                            Ok(image_buffer) => {
                                                let image_buffer = image_buffer.to_rgb8();
                                                let buffer: Vec<u8> = image_buffer.to_vec();
                                                match notify_rust::Image::from_rgb(
                                                    image_buffer.width() as i32,
                                                    image_buffer.height() as i32,
                                                    buffer,
                                                ) {
                                                    Ok(image) => Some(image),
                                                    Err(_) => None,
                                                }
                                            }
                                        }
                                    } else {
                                        None
                                    }
                                } else {
                                    None
                                }
                            };

                            let mut notify = notify_rust::Notification::new();
                            notify.summary("buoy");
                            notify.body(&format!(
                                "{}\n{}",
                                song.title,
                                song.artist.unwrap_or(String::from("Unkonwn Artist"))
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
                    }
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
                    self.recalculate_mpris_next_prev();
                    self.mpris.borrow_mut().set_state(PlayState::Play);
                }
                Command::Previous => {
                    if !self.queue.model().can_play_previous() {
                        return;
                    }
                    self.queue.emit(QueueIn::PlayPrevious);
                    self.recalculate_mpris_next_prev();
                    self.mpris.borrow_mut().set_state(PlayState::Play);
                }
                Command::Play => {
                    if !self.queue.model().can_play() {
                        return;
                    }

                    if let Err(e) = self.playback.borrow_mut().play() {
                        sender.input(AppIn::DisplayToast(format!(
                            "could not play playback: {e:?}"
                        )));
                    }
                    self.recalculate_mpris_next_prev();
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
                }
                Command::SetSongPosition(pos_ms) => {
                    self.seekbar.emit(SeekbarIn::SeekTo(pos_ms));
                    self.play_controls
                        .emit(PlayControlIn::NewState(PlayState::Play));
                    self.mpris.borrow_mut().set_position(pos_ms);
                }
                Command::Volume(volume) => {
                    self.playback.borrow_mut().set_volume(volume);
                    widgets.volume_btn.set_value(volume);
                    self.mpris.borrow_mut().set_volume(volume);
                    let mut settings = Settings::get().lock().unwrap();
                    settings.volume = volume;
                    settings.save();
                }
                Command::Repeat(repeat) => {
                    self.mpris.borrow_mut().set_loop_status(repeat.clone());
                }
                Command::Shuffle(shuffle) => {
                    self.mpris.borrow_mut().set_shuffle(shuffle.clone());
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
                        self.browser.emit(BrowserIn::FavoriteAlbum(id, state));
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
                        self.browser.emit(BrowserIn::FavoriteArtist(id, state));
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
                        self.queue.emit(QueueIn::Favorite(id.clone(), state));
                        self.browser.emit(BrowserIn::FavoriteSong(id, state));
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
            AppIn::Download(drop) => {
                Download::download(sender.clone(), drop);
            }
        }
    }

    fn shutdown(&mut self, widgets: &mut Self::Widgets, _sender: relm4::Sender<Self::Output>) {
        tracing::info!("shutdown app");

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
        settings.paned_position = widgets.paned.position();
        settings.save();
    }
}
