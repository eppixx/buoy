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

use crate::{
    play_state::PlayState,
    playback::{Playback, PlaybackOutput},
    settings::Settings,
};
use components::{
    browser::Browser,
    equalizer::{Equalizer, EqualizerOut},
    login_form::LoginForm,
    login_form::LoginFormOutput,
    play_controls::{PlayControlModel, PlayControlOutput},
    play_info::PlayInfoModel,
    queue::{QueueInput, QueueModel},
    seekbar::{SeekbarCurrent, SeekbarInput, SeekbarModel},
};

mod components;
pub mod css;
mod factory;
mod play_state;
mod playback;
pub mod settings;
pub mod types;

struct AppModel {
    login_form: AsyncController<LoginForm>,
    queue: Controller<QueueModel>,
    play_controls: Controller<PlayControlModel>,
    seekbar: Controller<SeekbarModel>,
    play_info: Controller<PlayInfoModel>,
    browser: Controller<Browser>,
    equalizer: Controller<Equalizer>,
    volume_button: gtk::VolumeButton,
    playback: Playback,
}

#[derive(Debug)]
enum AppMsg {
    PlayControlOutput(PlayControlOutput),
    Seekbar(i64),
    VolumeChange(f64),
    Playback(PlaybackOutput),
    LoginForm(LoginFormOutput),
    Equalizer(EqualizerOut),
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
        let (playback_sender, receiver) = glib::MainContext::channel(glib::Priority::DEFAULT);
        let login_form: AsyncController<LoginForm> = LoginForm::builder()
            .launch(())
            .forward(sender.input_sender(), AppMsg::LoginForm);
        let queue: Controller<QueueModel> = QueueModel::builder()
            .launch(())
            .forward(sender.input_sender(), |_msg| todo!());
        let play_controls = PlayControlModel::builder()
            .launch(PlayState::Pause) // TODO change to previous state
            .forward(sender.input_sender(), AppMsg::PlayControlOutput);
        let seekbar = SeekbarModel::builder()
            .launch(Some(SeekbarCurrent::new(1000 * 60, None))) // TODO change to previous state
            .forward(sender.input_sender(), AppMsg::Seekbar);
        let play_info = PlayInfoModel::builder()
            .launch(None) // TODO change to previous state
            .detach();
        let browser = Browser::builder().launch(()).detach();
        let equalizer = Equalizer::builder()
            .launch(())
            .forward(sender.input_sender(), AppMsg::Equalizer);
        let playback = Playback::new(&playback_sender).unwrap();

        let model = AppModel {
            login_form,
            queue,
            play_controls,
            seekbar,
            play_info,
            browser,
            equalizer,
            volume_button: gtk::VolumeButton::default(),
            playback,
        };

        // Insert the macro code generation here
        let widgets = view_output!();

        //init widgets
        {
            let settings = Settings::get().lock().unwrap();
            model.volume_button.set_value(settings.volume);
        }

        receiver.attach(None, move |msg| {
            sender.input(AppMsg::Playback(msg));
            glib::ControlFlow::Continue
        });

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>) {
        match msg {
            AppMsg::PlayControlOutput(PlayControlOutput::Next) => {
                _ = self.queue.sender().send(QueueInput::PlayNext);
            }
            AppMsg::PlayControlOutput(PlayControlOutput::Previous) => {
                _ = self.queue.sender().send(QueueInput::PlayPrevious);
            }
            AppMsg::PlayControlOutput(PlayControlOutput::Status(status)) => {
                match status {
                    PlayState::Pause => self.playback.pause().unwrap(),
                    PlayState::Play => self.playback.play().unwrap(),
                    PlayState::Stop => self.playback.stop().unwrap(),
                };
                _ = self.queue.sender().send(QueueInput::NewState(status));
            }
            AppMsg::Seekbar(seek_in_ms) => self.playback.set_position(seek_in_ms),
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
                        self.seekbar.sender().emit(SeekbarInput::SeekTo(ms));
                    }
                }
            }
            AppMsg::LoginForm(msg) => {
                // tracing::error!("msg from LoginForm: {msg:?}");
                //TODO login
            }
            AppMsg::Equalizer(changed) => {
                //TODO react to equalizer changed
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

                    gtk::Label {
                        set_markup: "<span weight=\"bold\">Bouy</span>",
                        set_hexpand: true,
                    },

                    gtk::MenuButton {
                        set_icon_name: "media-eq-symbolic",
                        set_focus_on_click: false,
                        #[wrap(Some)]
                        set_popover: equalizer_popover = &gtk::Popover {
                            model.equalizer.widget(),
                        },
                    },

                    append = &model.volume_button.clone() -> gtk::VolumeButton {
                        set_focus_on_click: false,
                        connect_value_changed[sender] => move |_scale, value| {
                            sender.input(AppMsg::VolumeChange(value));
                        }
                    },

                    gtk::MenuButton {
                        set_icon_name: "open-menu-symbolic",
                        set_focus_on_click: false,

                        #[wrap(Some)]
                        set_popover: popover = &gtk::Popover {
                            set_position: gtk::PositionType::Right,

                            gtk::Box {
                                set_orientation: gtk::Orientation::Vertical,

                                gtk::Button {
                                    add_css_class: "destructive-action",
                                    set_label: "Logout from Server",
                                },

                                model.login_form.widget(),
                            },
                        },
                    },

                    gtk::WindowControls {
                        set_side: gtk::PackType::End,
                    },
                },
            },

            gtk::Box {
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
            }
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
