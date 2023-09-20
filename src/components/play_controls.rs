use relm4::{
    component,
    gtk::{
        self,
        traits::{BoxExt, ButtonExt, WidgetExt},
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
}

#[derive(Debug)]
pub enum PlayControlOut {
    Status(PlayState),
    Previous,
    Next,
}

#[component(pub)]
impl SimpleComponent for PlayControl {
    type Input = PlayControlIn;
    type Output = PlayControlOut;
    type Init = PlayState;

    fn init(
        state: Self::Init,
        root: &Self::Root,
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
                            sender.input(PlayControlIn::NewState(PlayState::Play));
                            sender.output(PlayControlOut::Status(PlayState::Play)).unwrap();
                        }
                        "media-playback-pause-symbolic" => {
                            sender.input(PlayControlIn::NewState(PlayState::Pause));
                            sender.output(PlayControlOut::Status(PlayState::Pause)).unwrap();
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
            PlayControlIn::NewState(PlayState::Pause) => {
                self.play_btn.set_icon_name("media-playback-start-symbolic");
            }
            PlayControlIn::NewState(PlayState::Stop) => {
                self.play_btn.set_icon_name("media-playback-start-symbolic");
            }
        }
    }
}
