use std::str::FromStr;

use relm4::{
    component,
    gtk::{
        self,
        prelude::{BoxExt, ButtonExt, WidgetExt},
    },
};

use crate::{
    components::{
        sequence_button::Sequence,
        sequence_button_impl::{repeat::Repeat, shuffle::Shuffle},
    },
    play_state::PlayState,
    player::Command,
    settings::Settings,
};

#[derive(Debug, Default)]
pub struct PlayControl {
    prev_btn: gtk::Button,
    play_btn: gtk::Button,
    next_btn: gtk::Button,
    random_btn: gtk::Button,
    repeat_btn: gtk::Button,
}

#[derive(Debug)]
pub enum PlayControlIn {
    NewState(PlayState),
    Disable,
    Enable,
    DisableNext(bool),
    DisablePrevious(bool),
}

#[derive(Debug)]
pub enum PlayControlOut {
    Player(Command),
}

#[component(pub)]
impl relm4::SimpleComponent for PlayControl {
    type Init = PlayState;
    type Input = PlayControlIn;
    type Output = PlayControlOut;

    fn init(
        state: Self::Init,
        root: Self::Root,
        sender: relm4::ComponentSender<Self>,
    ) -> relm4::ComponentParts<Self> {
        let model = PlayControl::default();
        let widgets = view_output!();

        //init buttons
        sender.input(PlayControlIn::NewState(state));

        // random and shuffle buttons
        let settings = Settings::get().lock().unwrap();
        model.random_btn.set_icon_name(settings.shuffle.current());
        model.repeat_btn.set_icon_name(settings.repeat.current());
        drop(settings);

        relm4::ComponentParts { model, widgets }
    }

    view! {
        #[root]
        gtk::Box {
            add_css_class: "play-control",
            set_halign: gtk::Align::Center,
            set_spacing: 20,

            append = &gtk::Box {
                set_valign: gtk::Align::End,

                model.random_btn.clone() -> gtk::Button {
                    set_focus_on_click: false,
                    set_margin_end: 10,

                    connect_clicked[sender] => move |btn| {
                        let mut shuffle = Shuffle::from_str(&btn.icon_name().unwrap()).unwrap();
                        shuffle.next();
                        btn.set_icon_name(shuffle.current());
                        let mut settings = Settings::get().lock().unwrap();
                        settings.shuffle = shuffle.clone();
                        drop(settings);
                        sender.output(PlayControlOut::Player(Command::Shuffle(shuffle))).unwrap();
                    }
                }
            },

            append = &model.prev_btn.clone() {
                add_css_class: "play-control-previous",
                add_css_class: "circular",
                set_icon_name: "media-skip-backward-symbolic",
                set_focus_on_click: false,
                connect_clicked[sender] => move |_| {
                    sender.output(PlayControlOut::Player(Command::Previous)).unwrap();
                },
            },

            append = &model.play_btn.clone() {
                add_css_class: "play-control-play-pause",
                add_css_class: "circular",
                set_icon_name: "media-playback-stop-symbolic",
                set_focus_on_click: false,
                connect_clicked[sender] => move |btn| {
                    match btn.icon_name().unwrap().as_str() {
                        "media-playback-start-symbolic" => {
                            sender.output(PlayControlOut::Player(Command::Play)).unwrap();
                        }
                        "media-playback-pause-symbolic" => {
                            sender.output(PlayControlOut::Player(Command::Pause)).unwrap();
                        }
                        _ => unreachable!("unkonwn icon name"),
                    }
                },
            },

            append = &model.next_btn.clone() {
                add_css_class: "play-control-next",
                add_css_class: "circular",
                set_icon_name: "media-skip-forward-symbolic",
                set_focus_on_click: false,
                connect_clicked[sender] => move |_| {
                    _ = sender.output(PlayControlOut::Player(Command::Next));
                },
            },

            append = &gtk::Box {
                set_valign: gtk::Align::End,
                set_margin_start: 10,

                model.repeat_btn.clone() {
                    set_focus_on_click: false,

                    connect_clicked[sender] => move |btn| {
                        let mut repeat = Repeat::from_str(&btn.icon_name().unwrap()).unwrap();
                        repeat.next();
                        btn.set_icon_name(repeat.current());
                        let mut settings = Settings::get().lock().unwrap();
                        settings.repeat = repeat.clone();
                        sender.output(PlayControlOut::Player(Command::Repeat(repeat))).unwrap();
                    }
                }
            },
        }
    }

    fn update(&mut self, msg: Self::Input, _sender: relm4::ComponentSender<Self>) {
        match msg {
            PlayControlIn::NewState(PlayState::Play) => {
                self.play_btn.set_icon_name("media-playback-pause-symbolic");
            }
            PlayControlIn::NewState(PlayState::Pause | PlayState::Stop) => {
                self.play_btn.set_icon_name("media-playback-start-symbolic");
            }
            PlayControlIn::Disable => {
                self.prev_btn.set_sensitive(false);
                self.play_btn.set_sensitive(false);
                self.next_btn.set_sensitive(false);
            }
            PlayControlIn::Enable => {
                self.prev_btn.set_sensitive(true);
                self.play_btn.set_sensitive(true);
                self.next_btn.set_sensitive(true);
            }
            PlayControlIn::DisableNext(state) => self.next_btn.set_sensitive(state),
            PlayControlIn::DisablePrevious(state) => self.prev_btn.set_sensitive(state),
        }
    }
}
