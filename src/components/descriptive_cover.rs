use relm4::{
    gtk::{
        self, glib, pango,
        traits::{BoxExt, OrientableExt, WidgetExt},
    },
    RelmWidgetExt,
};

use crate::client::Client;

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
    loading: bool,
    image: gtk::Image,
    title: Option<String>,
    subtitle: Option<String>,
}

#[derive(Debug)]
pub enum DescriptiveCoverCmd {
    LoadedImage(Option<Vec<u8>>),
}

#[relm4::component(pub)]
impl relm4::Component for DescriptiveCover {
    type Init = DescriptiveCoverBuilder;
    type Input = DescriptiveCoverIn;
    type Output = ();
    type Widgets = CoverWidgets;
    type CommandOutput = DescriptiveCoverCmd;

    fn init(
        init: Self::Init,
        root: &Self::Root,
        sender: relm4::ComponentSender<Self>,
    ) -> relm4::ComponentParts<Self> {
        let model = Self {
            loading: false,
            image: gtk::Image::default(),
            title: init.title,
            subtitle: init.subtitle,
        };
        let widgets = view_output!();

        if let Some(id) = init.image {
            sender.input(DescriptiveCoverIn::LoadImage(Some(id)));
        }
        widgets.invisible.set_visible(false);
        widgets.invisible2.set_visible(false);

        relm4::ComponentParts { model, widgets }
    }

    view! {
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            set_margin_all: 12,
            set_spacing: 5,

            //test button
            // gtk::Button {
            //     set_label: "Start search",
            //     connect_clicked => CoverIn::LoadImage(Some(String::from("al-e0efdaf212ce4152eab39fac22c5c9e7_6115467e"))),
            //     #[watch]
            //     set_sensitive: !model.loading,
            // },

            gtk::Box {
                // set_hexpand: true,
                set_halign: gtk::Align::Center,

                #[transition = "Crossfade"]
                if model.loading {
                    gtk::Box {
                        add_css_class: "card",
                        add_css_class: "size100",

                        gtk::Spinner {
                            add_css_class: "size50",
                            set_hexpand: true,
                            set_halign: gtk::Align::Center,
                            set_valign: gtk::Align::Center,
                            start: (),
                        }
                    }
                } else {
                    model.image.clone() -> gtk::Image {
                        add_css_class: "card",
                        add_css_class: "play-info-cover",
                    }
                },
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

    fn update(
        &mut self,
        msg: Self::Input,
        sender: relm4::ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match msg {
            DescriptiveCoverIn::LoadImage(id) => match id {
                None => self.image.set_from_pixbuf(None),
                Some(id) => {
                    self.loading = true;
                    sender.oneshot_command(async move {
                        let client = Client::get().lock().unwrap().inner.clone().unwrap();
                        match client.get_cover_art(&id, Some(200)).await {
                            Ok(buffer) => DescriptiveCoverCmd::LoadedImage(Some(buffer)),
                            Err(_) => DescriptiveCoverCmd::LoadedImage(None),
                        }
                    });
                }
            },
            DescriptiveCoverIn::SetTitle(title) => self.title = title,
            DescriptiveCoverIn::SetSubtitle(subtitle) => self.subtitle = subtitle,
        }
    }

    fn update_cmd(
        &mut self,
        message: Self::CommandOutput,
        _sender: relm4::ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match message {
            DescriptiveCoverCmd::LoadedImage(buffer) => {
                let buffer = match buffer {
                    None => {
                        self.loading = false;
                        return;
                    }
                    Some(buffer) => buffer,
                };
                let bytes = gtk::glib::Bytes::from(&buffer);
                let stream = gtk::gio::MemoryInputStream::from_bytes(&bytes);
                match gtk::gdk_pixbuf::Pixbuf::from_stream(&stream, gtk::gio::Cancellable::NONE) {
                    Ok(pixbuf) => self.image.set_from_pixbuf(Some(&pixbuf)),
                    _ => self.image.set_from_pixbuf(None),
                }
                println!("loaded ");
                self.loading = false;
            }
        }
    }
}
