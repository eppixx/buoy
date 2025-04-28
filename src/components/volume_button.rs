use gettextrs::gettext;
use relm4::{
    gtk::{
        self,
        prelude::{BoxExt, ButtonExt, OrientableExt, RangeExt, ScaleExt, WidgetExt},
    },
    RelmWidgetExt,
};

use crate::{player::Command, settings::Settings};

const STEP: f64 = 0.05;

#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub enum ButtonState {
    Mute(String),
    Low(String),
    Medium(String),
    High(String),
}

impl ButtonState {
    fn from_volume(volume: f64) -> Self {
        match volume {
            f64::MIN..0.01 => ButtonState::Mute(String::from("audio-volume-muted-symbolic")),
            0.01..0.33 => ButtonState::Low(String::from("audio-volume-low-symbolic")),
            0.33..0.66 => ButtonState::Mute(String::from("audio-volume-medium-symbolic")),
            0.66..f64::MAX => ButtonState::Mute(String::from("audio-volume-high-symbolic")),
            _ => unreachable!("NaN should never be possible"),
        }
    }
}

impl AsRef<str> for ButtonState {
    fn as_ref(&self) -> &str {
        match self {
            Self::Mute(s) | Self::Low(s) | Self::Medium(s) | Self::High(s) => s,
        }
    }
}

#[derive(Debug)]
pub struct VolumeButton {
    button_state: ButtonState,
}

#[derive(Debug)]
pub enum VolumeButtonIn {
    CheckButtonState,
    Increase,
    Decrease,
    VolumeChanged,
    ChangeVolumeTo(f64),
    MuteToggle,
}

#[derive(Debug)]
pub enum VolumeButtonOut {
    Player(Command),
    ButtonStateChangedTo(ButtonState),
}

#[relm4::component(pub)]
impl relm4::component::Component for VolumeButton {
    type Init = ();
    type Input = VolumeButtonIn;
    type Output = VolumeButtonOut;
    type CommandOutput = ();

    fn init(
        _: Self::Init,
        root: Self::Root,
        sender: relm4::ComponentSender<Self>,
    ) -> relm4::ComponentParts<Self> {
        let settings = Settings::get().lock().unwrap();
        let volume_with_mute = if settings.mute { 0.0 } else { settings.volume };
        let model = Self {
            button_state: ButtonState::from_volume(volume_with_mute),
        };

        let widgets = view_output!();

        // init widget
        sender
            .output(VolumeButtonOut::ButtonStateChangedTo(
                model.button_state.clone(),
            ))
            .unwrap();

        relm4::ComponentParts { model, widgets }
    }

    view! {
        gtk::Box {
            set_widget_name: "volume-button",
            set_orientation: gtk::Orientation::Vertical,
            set_spacing: 20,

            gtk::Button {
                set_icon_name: "list-add-symbolic",
                set_tooltip: &gettext("Increase volume"),

                connect_clicked => VolumeButtonIn::Increase,
            },

            append: volume = &gtk::Scale {
                set_vexpand: true,
                set_orientation: gtk::Orientation::Vertical,

                // set range and step
                set_inverted: true,
                set_range: (0.0, 1.0),
                set_increments: (STEP, STEP),

                // set volume, order is important
                set_value: volume_with_mute,

                add_mark: (1.0, gtk::PositionType::Left, Some("100%")),
                add_mark: (0.75, gtk::PositionType::Left, None),
                add_mark: (0.50, gtk::PositionType::Left, Some("50%")),
                add_mark: (0.25, gtk::PositionType::Left, None),
                add_mark: (0.0, gtk::PositionType::Left, Some("0%")),

                // connect_value_changed => VolumeButtonIn::VolumeChanged,
                connect_change_value[sender] => move |_range, _, _| {
                    sender.input(VolumeButtonIn::VolumeChanged);
                    gtk::glib::signal::Propagation::Proceed
                }
            },

            gtk::Button {
                set_icon_name: "list-remove-symbolic",
                set_tooltip: &gettext("Decrease volume"),

                connect_clicked => VolumeButtonIn::Decrease,
            }
        }
    }

    fn update_with_view(
        &mut self,
        widgets: &mut Self::Widgets,
        msg: Self::Input,
        sender: relm4::ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match msg {
            VolumeButtonIn::CheckButtonState => {
                let tmp_state = ButtonState::from_volume(widgets.volume.value());
                if tmp_state != self.button_state {
                    self.button_state = tmp_state;
                    sender
                        .output(VolumeButtonOut::ButtonStateChangedTo(
                            self.button_state.clone(),
                        ))
                        .unwrap();
                }
            }
            VolumeButtonIn::Increase => {
                // update scale
                widgets.volume.set_value(widgets.volume.value() + STEP);

                //notifiy rest of app
                let cmd = Command::Volume(widgets.volume.value());
                sender.output(VolumeButtonOut::Player(cmd)).unwrap();

                // update other widgets
                widgets.volume.set_tooltip(&format_volume(&widgets.volume));
                sender.input(VolumeButtonIn::CheckButtonState);
            }
            VolumeButtonIn::Decrease => {
                // update scale
                widgets.volume.set_value(widgets.volume.value() - STEP);

                //notifiy rest of app
                let cmd = Command::Volume(widgets.volume.value());
                sender.output(VolumeButtonOut::Player(cmd)).unwrap();

                // update other widgets
                widgets.volume.set_tooltip(&format_volume(&widgets.volume));
                sender.input(VolumeButtonIn::CheckButtonState);
            }
            VolumeButtonIn::VolumeChanged => {
                //notifiy rest of app
                let cmd = Command::Volume(widgets.volume.value());
                sender.output(VolumeButtonOut::Player(cmd)).unwrap();

                // update widgets
                widgets.volume.set_tooltip(&format_volume(&widgets.volume));
                sender.input(VolumeButtonIn::CheckButtonState);
            }
            VolumeButtonIn::ChangeVolumeTo(volume) => {
                widgets.volume.set_value(volume);
                widgets.volume.set_tooltip(&format_volume(&widgets.volume));
                sender.input(VolumeButtonIn::CheckButtonState);
            }
            VolumeButtonIn::MuteToggle => {
                let settings = Settings::get().lock().unwrap();
                if !settings.mute {
                    // update scale
                    widgets.volume.set_value(0.0);
                } else {
                    // update scale
                    widgets.volume.set_value(settings.volume);
                    // update widgets
                }

                // update widgets
                widgets.volume.set_tooltip(&format_volume(&widgets.volume));
                sender.input(VolumeButtonIn::CheckButtonState);
                //notifiy rest of app
                sender
                    .output(VolumeButtonOut::Player(Command::MuteToggle))
                    .unwrap();
            }
        }
    }
}

fn format_volume(scale: &gtk::Scale) -> String {
    format!("{:.0}%", scale.value() * 100.0)
}
