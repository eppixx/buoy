use relm4::gtk::{
    self,
    traits::{OrientableExt, WidgetExt},
};

#[derive(Debug, Default)]
pub struct Dashboard {}

#[derive(Debug)]
pub enum DashboardOutput {
    Entered,
}

#[relm4::component(pub)]
impl relm4::SimpleComponent for Dashboard {
    type Input = ();
    type Output = DashboardOutput;
    type Init = ();

    fn init(
        _init: Self::Init,
        root: &Self::Root,
        sender: relm4::ComponentSender<Self>,
    ) -> relm4::ComponentParts<Self> {
        let model = Dashboard::default();
        let widgets = view_output!();

        _ = sender.output(DashboardOutput::Entered);

        relm4::ComponentParts { model, widgets }
    }

    view! {
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,

            gtk::Label {
                add_css_class: "h2",
                set_halign: gtk::Align::Center,
                set_text: "Dashboard",
            },

            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_valign: gtk::Align::Start,

                gtk::Label {
                    add_css_class: "h3",
                    set_text: "Newly added",
                },
                gtk::FlowBox {
                    //TODO add cover here
                }
            },

            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,

                gtk::Label {
                    add_css_class: "h3",
                    set_text: "Recently Played",
                },
                gtk::FlowBox {
                    //TODO add cover here
                }
            },

            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,

                gtk::Label {
                    add_css_class: "h3",
                    set_text: "Random"
                },
                gtk::ScrolledWindow {
                    gtk::Box {
                        // TODO add cover here
                    }
                }
            },

            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,

                gtk::Label {
                    add_css_class: "h3",
                    set_text: "Often Played",
                },
                gtk::FlowBox {
                    //TODO add cover here
                }
            }
        }
    }
}
