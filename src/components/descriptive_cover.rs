use std::{cell::RefCell, rc::Rc};

use relm4::{
    gtk::{
        self, pango,
        prelude::{BoxExt, OrientableExt, WidgetExt},
    },
    Component, ComponentController,
};

use crate::{subsonic::Subsonic, types::Id};

use super::cover::{Cover, CoverIn, CoverOut};

#[derive(Debug, Default, Clone)]
pub struct DescriptiveCoverBuilder {
    id: Option<Id>,
    image: Option<String>,
    title: Option<String>,
    subtitle: Option<String>,
}

impl DescriptiveCoverBuilder {
    pub fn id(mut self, id: Id) -> Self {
        self.id = Some(id);
        self
    }

    pub fn image(mut self, image: impl Into<String>) -> Self {
        self.image = Some(image.into());
        self
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn subtitle(mut self, subtitle: impl Into<String>) -> Self {
        self.subtitle = Some(subtitle.into());
        self
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
    title: gtk::Viewport,
    subtitle: gtk::Viewport,
}

#[derive(Debug)]
pub enum DescriptiveCoverOut {}

#[relm4::component(pub)]
impl relm4::SimpleComponent for DescriptiveCover {
    type Init = (Rc<RefCell<Subsonic>>, DescriptiveCoverBuilder);
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
                .launch((subsonic, None))
                .forward(sender.input_sender(), DescriptiveCoverIn::Cover),
            title: gtk::Viewport::default(),
            subtitle: gtk::Viewport::default(),
        };

        let widgets = view_output!();

        sender.input(DescriptiveCoverIn::SetTitle(init.title));
        sender.input(DescriptiveCoverIn::SetSubtitle(init.subtitle));
        model.cover.emit(CoverIn::LoadId(init.id));
        model.cover.model().add_css_class_image("size150");

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

            model.title.clone() -> gtk::Viewport {
                set_halign: gtk::Align::Center,
            },

            model.subtitle.clone() -> gtk::Viewport {
                set_halign: gtk::Align::Center,
            }
        }
    }

    fn update(&mut self, msg: Self::Input, _sender: relm4::ComponentSender<Self>) {
        match msg {
            DescriptiveCoverIn::SetTitle(title) => {
                if let Some(title) = title {
                    let label = gtk::Label::new(Some(&title));
                    label.set_halign(gtk::Align::Center);
                    label.set_ellipsize(pango::EllipsizeMode::End);
                    label.set_max_width_chars(15);
                    label.set_size_request(150, -1);
                    self.title.set_child(Some(&label));
                } else {
                    self.title.set_child(None::<gtk::Label>.as_ref());
                }
            }
            DescriptiveCoverIn::SetSubtitle(subtitle) => {
                if let Some(subtitle) = subtitle {
                    let label = gtk::Label::new(Some(&subtitle));
                    label.set_halign(gtk::Align::Center);
                    label.set_ellipsize(pango::EllipsizeMode::End);
                    label.set_max_width_chars(15);
                    label.set_size_request(150, -1);
                    self.subtitle.set_child(Some(&label));
                } else {
                    self.subtitle.set_child(None::<gtk::Label>.as_ref());
                }
            }
            DescriptiveCoverIn::Cover(msg) => match msg {},
        }
    }
}
