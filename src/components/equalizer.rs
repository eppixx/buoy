use relm4::gtk::{
    self,
    traits::{BoxExt, ButtonExt, OrientableExt, RangeExt, WidgetExt},
};

use crate::settings::Settings;

#[derive(Debug, Default)]
pub struct Equalizer {
    reset_btn: gtk::Button,
    enabled: gtk::Switch,
    band0: gtk::Scale,
    band1: gtk::Scale,
    band2: gtk::Scale,
    band3: gtk::Scale,
    band4: gtk::Scale,
    band5: gtk::Scale,
    band6: gtk::Scale,
    band7: gtk::Scale,
    band8: gtk::Scale,
    band9: gtk::Scale,
    root: gtk::Box,
}

#[derive(Debug)]
pub enum EqualizerIn {
    Reset,
    Enabled(bool),
    StateChanged,
}

#[derive(Debug)]
pub enum EqualizerOut {
    Changed,
}

#[relm4::component(pub)]
impl relm4::SimpleComponent for Equalizer {
    type Input = EqualizerIn;
    type Output = EqualizerOut;
    type Init = ();

    fn init(
        _init: Self::Init,
        root: &Self::Root,
        sender: relm4::ComponentSender<Self>,
    ) -> relm4::ComponentParts<Self> {
        let model = Equalizer::default();
        let widgets = view_output!();

        //TODO init state
        {
            let settings = Settings::get().lock().unwrap();
            model.reset_btn.set_sensitive(settings.equalizer_enabled);
            model.enabled.set_active(settings.equalizer_enabled);
            let bands = [
                &model.band0,
                &model.band1,
                &model.band2,
                &model.band3,
                &model.band4,
                &model.band5,
                &model.band6,
                &model.band7,
                &model.band8,
                &model.band9,
            ];
            for (i, band) in bands.iter().enumerate() {
                band.set_value(settings.equalizer_bands[i]);
                band.set_sensitive(settings.equalizer_enabled);
            }
        }

        relm4::ComponentParts { model, widgets }
    }

    view! {
        gtk::Box {
            add_css_class: "equalizer",
            set_orientation: gtk::Orientation::Vertical,
            set_spacing: 30,

            gtk::Label {
                add_css_class: "h3",
                set_label: "Equalizer",
            },

            gtk::CenterBox {
                #[wrap(Some)]
                set_start_widget = &model.reset_btn.clone() -> gtk::Button {
                    add_css_class: "destructive-action",
                    set_label: "Reset bands",
                    connect_clicked => EqualizerIn::Reset,
                },

                #[wrap(Some)]
                set_end_widget = &model.enabled.clone() -> gtk::Switch {
                    connect_state_set[sender] => move |_swtich, state| {
                        sender.input(EqualizerIn::Enabled(state));
                        gtk::Inhibit(true)
                    }
                }
            },

            gtk::Box {
                add_css_class: "equalizer-bands",
                set_vexpand: true,
                set_spacing: 7,

                gtk::Box {
                    set_hexpand: true,
                    set_orientation: gtk::Orientation::Vertical,
                    set_spacing: 5,
                    append = &model.band0.clone() -> gtk::Scale {
                        set_vexpand: true,
                        set_orientation: gtk::Orientation::Vertical,
                        set_range: (-1.0, 1.0),
                        set_increments: (0.1, 0.1),
                        connect_value_changed => EqualizerIn::StateChanged,
                    },
                    gtk::Label {
                        set_label: "29Hz",
                    }
                },
                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_spacing: 5,
                    append = &model.band1.clone() -> gtk::Scale {
                        set_orientation: gtk::Orientation::Vertical,
                        set_vexpand: true,
                        set_range: (-1.0, 1.0),
                        set_increments: (0.1, 0.1),
                        connect_value_changed => EqualizerIn::StateChanged,
                    },
                    gtk::Label {
                        set_label: "59Hz",
                    }
                },
                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_spacing: 5,
                    append = &model.band2.clone() -> gtk::Scale {
                        set_vexpand: true,
                        set_orientation: gtk::Orientation::Vertical,
                        set_range: (-1.0, 1.0),
                        set_increments: (0.1, 0.1),
                        connect_value_changed => EqualizerIn::StateChanged,
                    },
                    gtk::Label {
                        set_label: "119Hz",
                    }
                },
                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_spacing: 5,
                    append = &model.band3.clone() -> gtk::Scale {
                        set_vexpand: true,
                        set_orientation: gtk::Orientation::Vertical,
                        set_range: (-1.0, 1.0),
                        set_increments: (0.1, 0.1),
                        connect_value_changed => EqualizerIn::StateChanged,
                    },
                    gtk::Label {
                        set_label: "237Hz",
                    }
                },
                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_spacing: 5,
                    append = &model.band4.clone() -> gtk::Scale {
                        set_vexpand: true,
                        set_orientation: gtk::Orientation::Vertical,
                        set_range: (-1.0, 1.0),
                        set_increments: (0.1, 0.1),
                        connect_value_changed => EqualizerIn::StateChanged,
                    },
                    gtk::Label {
                        set_label: "474Hz",
                    }
                },
                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_spacing: 5,
                    append = &model.band5.clone() -> gtk::Scale {
                        set_vexpand: true,
                        set_orientation: gtk::Orientation::Vertical,
                        set_range: (-1.0, 1.0),
                        set_increments: (0.1, 0.1),
                        connect_value_changed => EqualizerIn::StateChanged,
                    },
                    gtk::Label {
                        set_label: "947Hz",
                    }
                },
                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_spacing: 5,
                    append = &model.band6.clone() -> gtk::Scale {
                        set_vexpand: true,
                        set_orientation: gtk::Orientation::Vertical,
                        set_range: (-1.0, 1.0),
                        set_increments: (0.1, 0.1),
                        set_show_fill_level: false,
                        connect_value_changed => EqualizerIn::StateChanged,
                    },
                    gtk::Label {
                        set_label: "1889Hz",
                    }
                },
                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_spacing: 5,
                    append = &model.band7.clone() -> gtk::Scale {
                        set_vexpand: true,
                        set_orientation: gtk::Orientation::Vertical,
                        set_range: (-1.0, 1.0),
                        set_increments: (0.1, 0.1),
                        connect_value_changed => EqualizerIn::StateChanged,
                    },
                    gtk::Label {
                        set_label: "3770Hz",
                    }
                },
                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_spacing: 5,
                    append = &model.band8.clone() -> gtk::Scale {
                        set_vexpand: true,
                        set_orientation: gtk::Orientation::Vertical,
                        set_range: (-1.0, 1.0),
                        set_increments: (0.1, 0.1),
                        connect_value_changed => EqualizerIn::StateChanged,
                    },
                    gtk::Label {
                        set_label: "7523Hz",
                    }
                },
                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_spacing: 5,
                    append = &model.band9.clone() -> gtk::Scale {
                        set_vexpand: true,
                        set_orientation: gtk::Orientation::Vertical,
                        set_range: (-1.0, 1.0),
                        set_increments: (0.1, 0.1),
                        connect_value_changed => EqualizerIn::StateChanged,
                    },
                    gtk::Label {
                        set_label: "15011Hz",
                    }
                },
            },
        }
    }

    fn update(&mut self, msg: Self::Input, sender: relm4::ComponentSender<Self>) {
        let bands = [
            &self.band0,
            &self.band1,
            &self.band2,
            &self.band3,
            &self.band4,
            &self.band5,
            &self.band6,
            &self.band7,
            &self.band8,
            &self.band9,
        ];

        match msg {
            EqualizerIn::Reset => {
                for band in bands {
                    band.set_value(0.0);
                }
                sender.input(EqualizerIn::StateChanged);
            }
            EqualizerIn::Enabled(state) => {
                self.reset_btn.set_sensitive(state);
                for band in bands {
                    band.set_sensitive(state);
                }
                sender.input(EqualizerIn::StateChanged);
            }
            EqualizerIn::StateChanged => {
                {
                    let mut settings = Settings::get().lock().unwrap();
                    settings.equalizer_enabled = self.enabled.is_active();
                    for (i, band) in bands.iter().enumerate() {
                        settings.equalizer_bands[i] = band.value();
                    }
                    settings.save();
                }
                sender.output(EqualizerOut::Changed).unwrap();
            }
        }
    }
}
