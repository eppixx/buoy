use relm4::{
    component,
    gtk::{
        self,
        prelude::{BoxExt, ButtonExt, WidgetExt},
    },
    ComponentParts, ComponentSender, SimpleComponent,
};

use crate::play_state::PlayState;

#[derive(Debug, Default)]
pub struct PlayControl {
    prev_btn: gtk::Button,
    play_btn: gtk::Button,
    next_btn: gtk::Button,
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
    Previous,
    Next,
    Play,
    Pause,
}

#[component(pub)]
impl SimpleComponent for PlayControl {
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

        ComponentParts { model, widgets }
    }

    view! {
        #[root]
        gtk::Box {
            add_css_class: "play-control",
            set_halign: gtk::Align::Center,
            set_spacing: 20,

            append = &model.prev_btn.clone() {
                add_css_class: "play-control-previous",
                add_css_class: "circular",
                set_icon_name: "media-skip-backward-symbolic",
                set_focus_on_click: false,
                connect_clicked[sender] => move |_| {
                    _ = sender.output(PlayControlOut::Previous);
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
                            sender.output(PlayControlOut::Play).expect("sending failed");
                        }
                        "media-playback-pause-symbolic" => {
                            sender.output(PlayControlOut::Pause).expect("sending failed");
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
                    _ = sender.output(PlayControlOut::Next);
                },
            },
        }
    }

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>) {
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
