use gettextrs::gettext;
use relm4::gtk::{
    self,
    prelude::{BoxExt, ButtonExt, OrientableExt, RangeExt, ScaleExt, WidgetExt},
};

use crate::settings::Settings;

#[derive(Debug, Default)]
pub struct Equalizer {}

impl Equalizer {
    fn get_bands(widgets: &<Equalizer as relm4::Component>::Widgets) -> [&gtk::Scale; 10] {
        [
            &widgets.band0,
            &widgets.band1,
            &widgets.band2,
            &widgets.band3,
            &widgets.band4,
            &widgets.band5,
            &widgets.band6,
            &widgets.band7,
            &widgets.band8,
            &widgets.band9,
        ]
    }
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
    DisplayToast(String),
}

#[relm4::component(pub)]
impl relm4::Component for Equalizer {
    type Init = ();
    type Input = EqualizerIn;
    type Output = EqualizerOut;
    type CommandOutput = ();

    fn init(
        _init: Self::Init,
        root: Self::Root,
        sender: relm4::ComponentSender<Self>,
    ) -> relm4::ComponentParts<Self> {
        let model = Equalizer::default();
        let widgets = view_output!();

        let bands = Self::get_bands(&widgets);
        {
            let settings = Settings::get().lock().unwrap();
            widgets.reset_btn.set_sensitive(settings.equalizer_enabled);
            widgets.enabled.set_active(settings.equalizer_enabled);
            for (i, band) in bands.iter().enumerate() {
                // set properties for all bands
                band.set_vexpand(true);
                band.set_orientation(gtk::Orientation::Vertical);
                band.set_inverted(true);
                band.set_range(-10.0, 10.0);
                band.set_increments(0.1, 0.1);

                // set value from settings
                band.set_value(settings.equalizer_bands[i]);
                band.set_sensitive(settings.equalizer_enabled);
            }
        }
        // set marker for the first band
        for band in bands.iter().take(1) {
            band.add_mark(10.0, gtk::PositionType::Left, Some("10"));
            band.add_mark(5.0, gtk::PositionType::Left, Some("5"));
            band.add_mark(0.0, gtk::PositionType::Left, Some("0"));
            band.add_mark(-5.0, gtk::PositionType::Left, Some("-5"));
            band.add_mark(-10.0, gtk::PositionType::Left, Some("-10"));
        }

        // and then stops the rest
        for band in bands.iter().skip(1) {
            band.add_mark(10.0, gtk::PositionType::Left, None);
            band.add_mark(5.0, gtk::PositionType::Left, None);
            band.add_mark(0.0, gtk::PositionType::Left, None);
            band.add_mark(-5.0, gtk::PositionType::Left, None);
            band.add_mark(-10.0, gtk::PositionType::Left, None);
        }

        // set uniform size
        let group = gtk::SizeGroup::new(gtk::SizeGroupMode::Horizontal);
        group.add_widget(&widgets.box0);
        group.add_widget(&widgets.box1);
        group.add_widget(&widgets.box2);
        group.add_widget(&widgets.box3);
        group.add_widget(&widgets.box4);
        group.add_widget(&widgets.box5);
        group.add_widget(&widgets.box6);
        group.add_widget(&widgets.box7);
        group.add_widget(&widgets.box8);
        group.add_widget(&widgets.box9);

        relm4::ComponentParts { model, widgets }
    }

    view! {
        gtk::Box {
            set_widget_name: "equalizer",
            set_orientation: gtk::Orientation::Vertical,
            set_spacing: 30,

            gtk::Label {
                add_css_class: granite::STYLE_CLASS_H3_LABEL,
                set_label: &gettext("Equalizer"),
            },

            gtk::CenterBox {
                #[wrap(Some)]
                set_start_widget: reset_btn = &gtk::Button {
                    set_widget_name: "destructive-action",
                    set_label: &gettext("Reset bands"),
                    connect_clicked => EqualizerIn::Reset,
                },

                #[wrap(Some)]
                set_end_widget: enabled = &gtk::Switch {
                    connect_state_set[sender] => move |_swtich, state| {
                        sender.input(EqualizerIn::Enabled(state));
                        gtk::glib::Propagation::Proceed
                    }
                }
            },

            gtk::Box {
                set_vexpand: true,
                set_spacing: 7,

                #[name = "box0"]
                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_spacing: 5,
                    append: band0 = &gtk::Scale {
                        connect_value_changed => EqualizerIn::StateChanged,
                    },
                    gtk::Label {
                        set_label: "29Hz",
                    }
                },
                #[name = "box1"]
                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_spacing: 5,
                    append: band1 = &gtk::Scale {
                        connect_value_changed => EqualizerIn::StateChanged,
                    },
                    gtk::Label {
                        set_label: "59Hz",
                    }
                },
                #[name = "box2"]
                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_spacing: 5,
                    append: band2 = &gtk::Scale {
                        connect_value_changed => EqualizerIn::StateChanged,
                    },
                    gtk::Label {
                        set_label: "119Hz",
                    }
                },
                #[name = "box3"]
                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_spacing: 5,
                    append: band3 = &gtk::Scale {
                        connect_value_changed => EqualizerIn::StateChanged,
                    },
                    gtk::Label {
                        set_label: "237Hz",
                    }
                },
                #[name = "box4"]
                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_spacing: 5,
                    append: band4 = &gtk::Scale {
                        connect_value_changed => EqualizerIn::StateChanged,
                    },
                    gtk::Label {
                        set_label: "474Hz",
                    }
                },
                #[name = "box5"]
                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_spacing: 5,
                    append: band5 = &gtk::Scale {
                        connect_value_changed => EqualizerIn::StateChanged,
                    },
                    gtk::Label {
                        set_label: "947Hz",
                    }
                },
                #[name = "box6"]
                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_spacing: 5,
                    append: band6 = &gtk::Scale {
                        connect_value_changed => EqualizerIn::StateChanged,
                    },
                    gtk::Label {
                        set_label: "1,8kHz",
                    }
                },
                #[name = "box7"]
                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_spacing: 5,
                    append: band7 = &gtk::Scale {
                        connect_value_changed => EqualizerIn::StateChanged,
                    },
                    gtk::Label {
                        set_label: "3,7kHz",
                    }
                },
                #[name = "box8"]
                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_spacing: 5,
                    append: band8 = &gtk::Scale {
                        connect_value_changed => EqualizerIn::StateChanged,
                    },
                    gtk::Label {
                        set_label: "7,5Hz",
                    }
                },
                #[name = "box9"]
                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_spacing: 5,
                    append: band9 = &gtk::Scale {
                        connect_value_changed => EqualizerIn::StateChanged,
                    },
                    gtk::Label {
                        set_label: "15kHz",
                    }
                },
            },
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
            EqualizerIn::Reset => {
                for band in Self::get_bands(widgets) {
                    band.set_value(0.0);
                }
                sender.input(EqualizerIn::StateChanged);
            }
            EqualizerIn::Enabled(state) => {
                widgets.reset_btn.set_sensitive(state);
                for band in Self::get_bands(widgets) {
                    band.set_sensitive(state);
                }
                sender.input(EqualizerIn::StateChanged);
            }
            EqualizerIn::StateChanged => {
                {
                    let mut settings = Settings::get().lock().unwrap();
                    settings.equalizer_enabled = widgets.enabled.is_active();
                    for (i, band) in Self::get_bands(widgets).iter().enumerate() {
                        settings.equalizer_bands[i] = band.value();
                    }
                    if let Err(e) = settings.save() {
                        sender
                            .output(EqualizerOut::DisplayToast(format!(
                                "error saving settings: {e}"
                            )))
                            .unwrap();
                    }
                }
                sender.output(EqualizerOut::Changed).unwrap();
            }
        }
    }
}
