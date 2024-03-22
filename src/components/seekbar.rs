use relm4::{
    component,
    gtk::{
        self,
        prelude::{BoxExt, RangeExt, WidgetExt},
    },
};

use crate::common::convert_for_label;

#[derive(Debug, Default)]
pub struct Seekbar {
    current: i64,
    scale: gtk::Scale,
    total: i64,
}

impl Seekbar {
    pub fn current(&self) -> f64 {
        self.scale.value()
    }
}

#[derive(Debug)]
pub enum SeekbarIn {
    SeekbarDragged(f64),
    NewRange(i64), // in ms
    SeekTo(i64),   // in ms
    Disable,
}

#[derive(Debug)]
pub struct SeekbarCurrent {
    pub seek_in_ms: Option<i64>,
    pub total_in_ms: i64,
}

#[derive(Debug)]
pub enum SeekbarOut {
    SeekDragged(i64),
}

impl SeekbarCurrent {
    pub fn new(total_in_ms: i64, seek_in_ms: Option<i64>) -> Self {
        Self {
            total_in_ms,
            seek_in_ms,
        }
    }
}

#[component(pub)]
impl relm4::SimpleComponent for Seekbar {
    type Init = Option<SeekbarCurrent>;
    type Input = SeekbarIn;
    type Output = SeekbarOut;

    fn init(
        init: Self::Init,
        root: Self::Root,
        sender: relm4::ComponentSender<Self>,
    ) -> relm4::ComponentParts<Self> {
        let mut model = Self::default();
        let widgets = view_output!();

        //init widgets
        if let Some(init) = init {
            model.scale.set_range(0.0, init.total_in_ms as f64);
            model.scale.set_value(init.seek_in_ms.unwrap_or(0) as f64);
            model.total = init.total_in_ms;
            widgets.total.set_label(&convert_for_label(model.total));
            if let Some(seek) = init.seek_in_ms {
                model.current = seek;
                widgets.current.set_label(&convert_for_label(model.current));
            }
        }

        relm4::ComponentParts { model, widgets }
    }

    view! {
        #[root]
        gtk::Box {
            add_css_class: "seekbar",

            #[name = "current"]
            gtk::Label {
                add_css_class: "seekbar-current",
                #[watch]
                set_label: &convert_for_label(model.current),
            },

            append = &model.scale.clone() -> gtk::Scale {
                add_css_class: "seekbar-scale",
                set_hexpand: true,
                connect_change_value[sender] => move |_scale, _, value| {
                    sender.input(SeekbarIn::SeekbarDragged(value));
										gtk::glib::Propagation::Stop
                }
            },

            #[name = "total"]
            gtk::Label {
                add_css_class: "seekbar-total",
                #[watch]
                set_label: &convert_for_label(model.total),
            },
        }
    }

    fn update(&mut self, msg: Self::Input, sender: relm4::ComponentSender<Self>) {
        match msg {
            SeekbarIn::SeekbarDragged(value) => {
                self.current = value as i64;
								self.scale.set_value(value);
                sender.output(SeekbarOut::SeekDragged(value as i64)).unwrap();
            }
            SeekbarIn::NewRange(total) => {
                self.scale.set_sensitive(true);
                self.scale.set_range(0.0, total as f64);
                self.total = total;
            }
            SeekbarIn::SeekTo(ms) => {
                self.scale.set_value(ms as f64);
                self.current = ms;
            }
            SeekbarIn::Disable => self.scale.set_sensitive(false),
        }
    }
}
