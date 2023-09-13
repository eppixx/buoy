use relm4::{
    component::AsyncComponentController,
    gtk::{
        self,
        gdk_pixbuf::Pixbuf,
        traits::{BoxExt, OrientableExt, WidgetExt},
    },
};

use relm4::{
    component::{AsyncComponent, AsyncComponentParts, AsyncComponentSender},
    loading_widgets::LoadingWidgets,
    view, RelmApp, RelmWidgetExt,
};

use crate::{client::Client, types::Id};

#[derive(Debug, Default)]
pub struct Artists {
    flowbox: gtk::FlowBox,
}

#[derive(Debug)]
pub enum ArtistsOut {
    ChangeTo(Id),
}

#[derive(Debug)]
pub enum ArtistsIn {
    Clicked(Id),
}

#[relm4::component(async pub)]
impl relm4::component::AsyncComponent for Artists {
    type Input = ArtistsIn;
    type Output = ArtistsOut;
    type Init = ();
    type CommandOutput = ();

    async fn init(
        _init: Self::Init,
        root: Self::Root,
        sender: relm4::AsyncComponentSender<Self>,
    ) -> relm4::component::AsyncComponentParts<Self> {
        let model = Artists::default();
        let widgets = view_output!();

        let artists: Vec<submarine::data::ArtistId3> = {
            let client = Client::get().lock().unwrap().inner.clone().unwrap();
            let indexes: Vec<submarine::data::IndexId3> = client.get_artists(None).await.unwrap();
            indexes.into_iter().flat_map(|i| i.artist).collect()
        };

        for artist in artists.into_iter().rev() {
            println!("{artist:?}");
            let artist: relm4::component::AsyncController<ArtistElement> = ArtistElement::builder()
                .launch(artist)
                .forward(sender.input_sender(), ArtistsIn::Clicked);
            model.flowbox.insert(artist.widget(), 0);
        }

        relm4::component::AsyncComponentParts { model, widgets }
    }

    fn init_loading_widgets(root: &mut Self::Root) -> Option<LoadingWidgets> {
        view! {
            append = root.clone() -> gtk::Box{
                #[name(loading_box)]
                gtk::Box {
                    gtk::Label {
                        add_css_class: "h2",
                        set_label: "fetching artists",
                    },

                    gtk::Spinner {
                        start: (),
                        set_halign: gtk::Align::Center,
                    }
                }
            }
        }

        // removes widget loading_box when function init finishes
        Some(LoadingWidgets::new(root, loading_box))
    }

    view! {
        gtk::Box {
            add_css_class: "artists-view",
            set_orientation: gtk::Orientation::Vertical,
            set_spacing: 10,

            gtk::Label {
                add_css_class: "h2",
                set_halign: gtk::Align::Center,
                set_label: "Artists",
            },

            append = &model.flowbox.clone() -> gtk::FlowBox {
                // append = ArtistElement {
                // }
            },
            gtk::Label {
                set_label: "sdfs",
            }
        }
    }
}

#[derive(Debug)]
struct ArtistElement {
    info: submarine::data::ArtistId3,
    image: gtk::Image,
}

#[relm4::component(async)]
impl relm4::component::AsyncComponent for ArtistElement {
    type Input = ();
    type Output = Id;
    type Init = submarine::data::ArtistId3;
    type CommandOutput = ();

    async fn init(
        init: Self::Init,
        root: Self::Root,
        sender: relm4::AsyncComponentSender<Self>,
    ) -> relm4::component::AsyncComponentParts<Self> {
        let model = ArtistElement {
            info: init,
            image: gtk::Image::default(),
        };
        let widgets = view_output!();

        // TODO fetch image
        {
            let client = Client::get().lock().unwrap().inner.clone().unwrap();
            if let Some(id) = &model.info.cover_art {
                let buffer: Vec<u8> = client.get_cover_art(id, Some(300)).await.unwrap();
                let bytes = gtk::glib::Bytes::from(&buffer);
                let stream = gtk::gio::MemoryInputStream::from_bytes(&bytes);
                match Pixbuf::from_stream(&stream, gtk::gio::Cancellable::NONE) {
                    Ok(pixbuf) => model.image.set_from_pixbuf(Some(&pixbuf)),
                    _ => {} // TODO replace with stock image
                            // TODO remove crates
                }
            }
        }

        relm4::component::AsyncComponentParts { model, widgets }
    }

    fn init_loading_widgets(root: &mut Self::Root) -> Option<LoadingWidgets> {
        view! {
            #[local_ref]
            root {
                add_css_class: "artist-cover",

                // This will be removed automatically by
                // LoadingWidgets when the full view has loaded
                #[name(spinner)]
                gtk::Spinner {
                    set_vexpand: true,
                    set_hexpand: true,

                    start: (),
                    set_halign: gtk::Align::Center,
                }
            }
        }
        Some(LoadingWidgets::new(root, spinner))
    }

    view! {
        gtk::Box {
            add_css_class: "artist-cover",
            set_orientation: gtk::Orientation::Vertical,
            set_spacing: 5,

            append = &model.image.clone() -> gtk::Image {
                set_vexpand: true,
                set_hexpand: true,

                add_css_class: "card",
                set_icon_name: Some("go-home"),
            },
            gtk::Label {
                set_text: &model.info.name,
            }
        }
    }
}
