use relm4::{
    component,
    gtk::{
        self,
        traits::{ButtonExt, WidgetExt},
    },
    ComponentParts, ComponentSender, SimpleComponent,
};

use crate::play_state::PlayState;

#[derive(Debug, Default)]
pub struct PlayControlModel {}

#[derive(Debug)]
pub enum PlayControlOutput {
    Status(PlayState),
    Previous,
    Next,
}

#[component(pub)]
impl SimpleComponent for PlayControlModel {
    type Input = PlayState;
    type Output = PlayControlOutput;
    type Init = PlayState;

    fn init(
        state: Self::Init,
        root: &Self::Root,
        sender: relm4::ComponentSender<Self>,
    ) -> relm4::ComponentParts<Self> {
        let model = PlayControlModel {};
        let widgets = view_output!();

        //init button
        let play_pause = &widgets.play_pause;
        match state {
            PlayState::Pause => play_pause.set_icon_name("media-playback-pause-symbolic"),
            PlayState::Play => play_pause.set_icon_name("media-playback-start-symbolic"),
            PlayState::Stop => play_pause.set_icon_name("media-playback-stop-symbolic"),
        }

        ComponentParts { model, widgets }
    }

    view! {
        #[root]
        gtk::Box {
            add_css_class: "play-control",

            gtk::Button {
                add_css_class: "play-control-previous",
                add_css_class: "circular",
                set_icon_name: "media-skip-backward-symbolic",
                connect_clicked[sender] => move |_| {
                    _ = sender.output(PlayControlOutput::Previous);
                },
            },

            #[name = "play_pause"]
            gtk::Button {
                add_css_class: "play-control-play-pause",
                add_css_class: "circular",
                set_icon_name: "media-playback-stop-symbolic",
                connect_clicked[sender] => move |btn| {
                    match btn.icon_name().unwrap().as_str() {
                        "media-playback-start-symbolic" => {
                            btn.set_icon_name("media-playback-pause-symbolic");
                            sender.input(PlayState::Pause);
                        }
                        "media-playback-pause-symbolic" => {
                            btn.set_icon_name("media-playback-start-symbolic");
                            sender.input(PlayState::Play);
                        }
                        "media-playback-stop-symbolic" => {
                            btn.set_icon_name("media-playback-start-symbolic");
                            sender.input(PlayState::Play);
                        }
                        _ => {
                            btn.set_icon_name("media-playback-stop-symbolic");
                            sender.input(PlayState::Stop);
                        }
                    }
                },
            },

            gtk::Button {
                add_css_class: "play-control-next",
                add_css_class: "circular",
                set_icon_name: "media-skip-forward-symbolic",
                connect_clicked[sender] => move |_| {
                    _ = sender.output(PlayControlOutput::Next);
                },
            },
        }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        _ = sender.output(PlayControlOutput::Status(msg));
    }
}
