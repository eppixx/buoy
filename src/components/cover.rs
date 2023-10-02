use relm4::{gtk, gtk::traits::WidgetExt};

use crate::cache::Cache;

#[derive(Debug)]
pub struct Cover {
    image: gtk::Image,
    loading: bool,
}

impl Cover {
    pub fn add_css_class_image(&self, class: &str) {
        self.image.add_css_class(class);
    }
}

#[derive(Debug)]
pub enum CoverIn {
    LoadImage(Option<String>),
}

// use tuple struct to keep the logging small
pub struct Image(Vec<u8>);

impl std::fmt::Debug for Image {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Image buffer")
            .field(&format!("size: {}", self.0.len()))
            .finish()
    }
}

#[derive(Debug)]
pub enum CoverOut {}

#[derive(Debug)]
pub enum CoverCmd {
    LoadedImage(Option<Image>),
}

#[relm4::component(pub)]
impl relm4::Component for Cover {
    type Init = ();
    type Input = CoverIn;
    type Output = CoverOut;
    type Widgets = CoverWidgets;
    type CommandOutput = CoverCmd;

    fn init(
        _init: Self::Init,
        root: &Self::Root,
        _sender: relm4::ComponentSender<Self>,
    ) -> relm4::ComponentParts<Self> {
        let model = Self {
            loading: false,
            image: gtk::Image::default(),
        };
        let widgets = view_output!();

        relm4::ComponentParts { model, widgets }
    }

    view! {
        gtk::Box {
            #[transition = "Crossfade"]
            if model.loading {
                gtk::Box {
                    add_css_class: "card",

                    gtk::Spinner {
                        add_css_class: "size32",
                        set_halign: gtk::Align::Center,
                        set_valign: gtk::Align::Center,
                        start: (),
                    }
                }
            } else {
                model.image.clone() -> gtk::Image {
                    add_css_class: "card",
                    add_css_class: "cover",
                }
            },
        }
    }

    fn update(
        &mut self,
        msg: Self::Input,
        sender: relm4::ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match msg {
            CoverIn::LoadImage(id) => match id {
                None => self.image.set_from_pixbuf(None),
                Some(id) => {
                    self.loading = true;
                    sender.oneshot_command(async move {
                        let mut cache = Cache::get().lock().await;
                        match cache.cover(&id).await {
                            None => CoverCmd::LoadedImage(None),
                            Some(buffer) => CoverCmd::LoadedImage(Some(Image(buffer))),
                        }
                    });
                }
            },
        }
    }

    fn update_cmd(
        &mut self,
        message: Self::CommandOutput,
        _sender: relm4::ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match message {
            CoverCmd::LoadedImage(None) => {
                self.loading = false;
                self.image.set_from_pixbuf(None);
                self.image.add_css_class("cover");
            }
            CoverCmd::LoadedImage(Some(buffer)) => {
                let bytes = gtk::glib::Bytes::from(&buffer.0);
                let stream = gtk::gio::MemoryInputStream::from_bytes(&bytes);
                match gtk::gdk_pixbuf::Pixbuf::from_stream(&stream, gtk::gio::Cancellable::NONE) {
                    Ok(pixbuf) => self.image.set_from_pixbuf(Some(&pixbuf)),
                    _ => self.image.set_from_pixbuf(None),
                }
                self.image.remove_css_class("cover");
                self.loading = false;
            }
        }
    }
}
