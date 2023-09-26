use relm4::{
    gtk::{
        self, glib, pango,
        traits::{BoxExt, OrientableExt, WidgetExt},
    },
    Component, ComponentController, RelmWidgetExt,
};

use super::cover::{Cover, CoverIn};

#[derive(Debug, Default, Clone)]
pub struct DescriptiveCoverBuilder {
    image: Option<String>,
    title: Option<String>,
    subtitle: Option<String>,
}

impl DescriptiveCoverBuilder {
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
    LoadImage(Option<String>),
    SetTitle(Option<String>),
    SetSubtitle(Option<String>),
}

#[derive(Debug)]
pub struct DescriptiveCover {
    cover: relm4::Controller<Cover>,
    title: Option<String>,
    subtitle: Option<String>,
}

#[relm4::component(pub)]
impl relm4::SimpleComponent for DescriptiveCover {
    type Init = DescriptiveCoverBuilder;
    type Input = DescriptiveCoverIn;
    type Output = ();
    type Widgets = CoverWidgets;

    fn init(
        init: Self::Init,
        root: &Self::Root,
        sender: relm4::ComponentSender<Self>,
    ) -> relm4::ComponentParts<Self> {
        let model = Self {
            cover: Cover::builder().launch(()).detach(),
            title: init.title,
            subtitle: init.subtitle,
        };
        let widgets = view_output!();

        if let Some(id) = init.image {
            sender.input(DescriptiveCoverIn::LoadImage(Some(id)));
        }
        widgets.invisible.set_visible(false);
        widgets.invisible2.set_visible(false);
        model.cover.model().add_css_class_image("size100");

        relm4::ComponentParts { model, widgets }
    }

    view! {
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            set_margin_all: 12,
            set_spacing: 5,

            gtk::Box {
                // set_hexpand: true,
                set_halign: gtk::Align::Center,

                model.cover.widget().clone() {
                    add_css_class: "play-info-cover",
                }
            },

            if model.title.is_some() {
                gtk::Label {
                    set_halign: gtk::Align::Center,
                    set_ellipsize: pango::EllipsizeMode::End,
                    set_max_width_chars: 15,
                    set_size_request: (150, -1),
                    #[watch]
                    set_label: model.title.as_ref().unwrap(),
                }
            } else {
                #[name = "invisible"]
                gtk::Label {
                    set_visible: false,
                    set_label: "invisible",
                }
            },

            if let Some(subtitle) = &model.subtitle {
                gtk::Label {
                    set_halign: gtk::Align::Center,
                    set_ellipsize: pango::EllipsizeMode::End,
                    set_max_width_chars: 15,
                    set_size_request: (150, -1),
                    #[watch]
                    set_markup: &format!("<span style=\"italic\">{}</span>", glib::markup_escape_text(subtitle)),
                }
            } else {
                #[name = "invisible2"]
                gtk::Label {
                    set_visible: false,
                    set_label: "invisible",
                }
            }
        }
    }

    fn update(&mut self, msg: Self::Input, _sender: relm4::ComponentSender<Self>) {
        match msg {
            DescriptiveCoverIn::LoadImage(id) => self.cover.emit(CoverIn::LoadImage(id)),
            DescriptiveCoverIn::SetTitle(title) => self.title = title,
            DescriptiveCoverIn::SetSubtitle(subtitle) => self.subtitle = subtitle,
        }
    }
}
