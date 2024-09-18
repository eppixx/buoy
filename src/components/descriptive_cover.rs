use std::{cell::RefCell, rc::Rc};

use relm4::{
    gtk::{
        self, pango,
        prelude::{BoxExt, OrientableExt, WidgetExt},
    },
    Component, ComponentController,
};

use crate::components::cover::{Cover, CoverOut};
use crate::subsonic::Subsonic;

#[derive(Debug, Default, Clone)]
pub struct DescriptiveCoverInit {
    cover_id: Option<String>,
    title: String,
    subtitle: Option<String>,
}

impl DescriptiveCoverInit {
    pub fn new(
        title: impl Into<String>,
        cover_id: Option<String>,
        subtitle: Option<impl Into<String>>,
    ) -> Self {
        Self {
            cover_id,
            title: title.into(),
            subtitle: subtitle.map(|s| s.into()),
        }
    }
}

#[derive(Debug)]
pub enum DescriptiveCoverIn {
    SetTitle(Option<String>),
    SetSubtitle(Option<String>),
    Cover(CoverOut),
}

#[derive(Debug)]
pub struct DescriptiveCover {
    cover: relm4::Controller<Cover>,
    title: String,
    title_label: gtk::Label,
    subtitle: Option<String>,
    subtitle_label: gtk::Label,
}

impl DescriptiveCover {
    pub fn change_size(&self, size: i32) {
        self.cover.model().change_size(size);
        self.title_label.set_width_request(size);
        self.subtitle_label.set_width_request(size);
    }
}

#[derive(Debug, Clone)]
pub enum DescriptiveCoverOut {
    DisplayToast(String),
}

#[relm4::component(pub)]
impl relm4::SimpleComponent for DescriptiveCover {
    type Init = (Rc<RefCell<Subsonic>>, DescriptiveCoverInit);
    type Input = DescriptiveCoverIn;
    type Output = DescriptiveCoverOut;
    type Widgets = CoverWidgets;

    fn init(
        (subsonic, init): Self::Init,
        root: Self::Root,
        sender: relm4::ComponentSender<Self>,
    ) -> relm4::ComponentParts<Self> {
        let model = Self {
            cover: Cover::builder()
                .launch((subsonic, init.cover_id))
                .forward(sender.input_sender(), DescriptiveCoverIn::Cover),
            title: init.title,
            title_label: gtk::Label::default(),
            subtitle: init.subtitle,
            subtitle_label: gtk::Label::default(),
        };

        let widgets = view_output!();

        if model.subtitle.is_none() {
            model.subtitle_label.set_visible(false);
        }
        model.change_size(150);

        relm4::ComponentParts { model, widgets }
    }

    view! {
        gtk::Box {
            add_css_class: "descriptive-cover",
            set_orientation: gtk::Orientation::Vertical,
            set_spacing: 5,

            gtk::Box {
                set_halign: gtk::Align::Center,

                model.cover.widget().clone(),
            },

            model.title_label.clone() -> gtk::Label {
                set_halign: gtk::Align::Center,
                set_ellipsize: pango::EllipsizeMode::End,
                set_max_width_chars: 15,
                set_size_request: (150, -1),

                set_label: &model.title,
            },

            model.subtitle_label.clone() -> gtk::Label {
                set_halign: gtk::Align::Center,
                set_ellipsize: pango::EllipsizeMode::End,
                set_max_width_chars: 15,
                set_size_request: (150, -1),

                set_label: &model.subtitle.clone().unwrap_or_default(),
            }
        }
    }

    fn update(&mut self, msg: Self::Input, sender: relm4::ComponentSender<Self>) {
        match msg {
            DescriptiveCoverIn::SetTitle(title) => {
                self.title = title.unwrap_or_default();
            }
            DescriptiveCoverIn::SetSubtitle(subtitle) => {
                if subtitle.is_none() {
                    self.subtitle_label.set_visible(false);
                }
                self.subtitle = subtitle;
            }
            DescriptiveCoverIn::Cover(msg) => match msg {
                CoverOut::DisplayToast(title) => sender
                    .output(DescriptiveCoverOut::DisplayToast(title))
                    .unwrap(),
            },
        }
    }
}
