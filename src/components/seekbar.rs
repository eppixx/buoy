use std::{cell::RefCell, rc::Rc};

use relm4::{
    component,
    gtk::{
        self,
        traits::{BoxExt, RangeExt, WidgetExt},
    },
};

#[derive(Debug, Default)]
pub struct SeekbarModel {
    current: i64,
    scale: gtk::Scale,
    total: i64,
}

#[derive(Debug)]
pub enum SeekbarInput {
    SeekbarChanged,
    NewRange(i64), // in ms
}

#[derive(Debug)]
pub struct SeekbarCurrent {
    seek_in_ms: Option<i64>,
    total_in_ms: i64,
}

impl SeekbarCurrent {
    pub fn new(total_in_ms: i64, seek_in_ms: Option<i64>) -> Self {
        Self {
            total_in_ms,
            seek_in_ms,
        }
    }
}

#[derive(Debug)]
pub enum SeekbarOutput {
    SeekedTo(i64),
}

#[component(pub)]
impl relm4::SimpleComponent for SeekbarModel {
    type Input = SeekbarInput;
    type Output = SeekbarOutput;
    type Init = Option<SeekbarCurrent>;

    fn init(
        init: Self::Init,
        root: &Self::Root,
        sender: relm4::ComponentSender<Self>,
    ) -> relm4::ComponentParts<Self> {
        let mut model = SeekbarModel::default();
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
                connect_value_changed => SeekbarInput::SeekbarChanged,
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
            SeekbarInput::SeekbarChanged => {
                let value = self.scale.value() as i64;
                self.current = value;
                _ = sender.output(SeekbarOutput::SeekedTo(value));
            }
            SeekbarInput::NewRange(total) => {
                self.scale.set_range(0.0, total as f64);
            }
        }
    }
}

fn convert_for_label(time: i64) -> String {
    let time = chrono::Duration::milliseconds(time);
    let hours = time.num_hours();
    let minutes = (time - chrono::Duration::hours(hours)).num_minutes();
    let seconds =
        (time - chrono::Duration::hours(hours) - chrono::Duration::minutes(minutes)).num_seconds();

    let mut result = String::new();
    if hours > 0 {
        result.push_str(format!("{}:{:0>2}:{:0>2}", hours, minutes, seconds).as_str());
        return result;
    }
    result.push_str(format!("{}:{:0>2}", minutes, seconds).as_str());
    result
}

#[cfg(test)]
mod tests {
    use crate::components::seekbar::convert_for_label;

    #[test]
    fn convert_time() {
        let oracle = vec![
            (1000, "0:01"),
            (10000, "0:10"),
            (1000 * 60, "1:00"),
            (1000 * 60 * 10, "10:00"),
            (1000 * 60 * 60, "1:00:00"),
            (1000 * 60 * 60 * 10, "10:00:00"),
            (1000 * 60 * 60 + 1000, "1:00:01"),
        ];
        for test in oracle {
            assert_eq!(&convert_for_label(test.0), test.1);
        }
    }
}
