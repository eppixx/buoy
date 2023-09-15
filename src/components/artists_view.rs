use relm4::{
    component::AsyncComponentController,
    gtk::{
        self, gdk,
        gdk_pixbuf::Pixbuf,
        pango,
        prelude::ToValue,
        traits::{BoxExt, ButtonExt, OrientableExt, WidgetExt},
    },
};

use relm4::{loading_widgets::LoadingWidgets, view};

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
        let artists: Vec<submarine::data::ArtistId3> = {
            let client = Client::get().lock().unwrap().inner.clone().unwrap();
            let indexes: Vec<submarine::data::IndexId3> = client.get_artists(None).await.unwrap();
            indexes.into_iter().flat_map(|i| i.artist).collect()
        };

        let model = Artists::default();
        let widgets = view_output!();

        for artist in artists.into_iter().rev() {
            let id = artist.id.clone();
            let artist_element: relm4::component::AsyncController<ArtistElement> =
                ArtistElement::builder()
                    .launch(artist)
                    .forward(sender.input_sender(), ArtistsIn::Clicked);
            let btn = gtk::Button::default();
            btn.add_css_class("flat");
            let sender = sender.clone();
            btn.connect_clicked(move |_btn| {
                sender
                    .output(ArtistsOut::ChangeTo(Id::artist(&id)))
                    .unwrap();
            });
            btn.set_child(Some(artist_element.widget()));
            model.flowbox.insert(&btn, 0);
        }

        widgets.reveal_after_init.set_visible(true);

        relm4::component::AsyncComponentParts { model, widgets }
    }

    fn init_loading_widgets(root: &mut Self::Root) -> Option<LoadingWidgets> {
        view! {
            append = root.clone() -> gtk::Box {
                add_css_class: "artists-view",

                #[name(loading_box)]
                gtk::Box {
                    set_spacing: 30,
                    set_halign: gtk::Align::Center,
                    set_orientation: gtk::Orientation::Vertical,

                    gtk::Label {
                        add_css_class: "h2",
                        set_label: "Loading artists",
                    },

                    gtk::Spinner {
                        add_css_class: "size100",
                        set_halign: gtk::Align::Center,
                        start: (),
                    }
                }
            }
        }

        // removes widget loading_box when function init finishes
        Some(LoadingWidgets::new(root, loading_box))
    }

    view! {
        gtk::Box {
            set_hexpand: true,
            set_orientation: gtk::Orientation::Vertical,

            #[name = "reveal_after_init"]
            gtk::Label {
                set_visible: false,
                add_css_class: "h2",
                set_halign: gtk::Align::Center,
                set_label: "Artists",
            },

            gtk::ScrolledWindow {
                set_vexpand: true,
                set_hexpand: true,

                #[wrap(Some)]
                set_child = &model.flowbox.clone() -> gtk::FlowBox {
                },
            }
        }
    }
}

#[derive(Debug)]
struct ArtistElement {
    info: submarine::data::ArtistId3,
    image: gtk::Image,
    drag_src: gtk::DragSource,
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
        let id = Id::artist(&init.id);
        let model = ArtistElement {
            info: init,
            image: gtk::Image::default(),
            drag_src: gtk::DragSource::default(),
        };

        // set drag source
        let content = gdk::ContentProvider::for_value(&id.to_value());
        model.drag_src.set_content(Some(&content));
        model.drag_src.set_actions(gdk::DragAction::MOVE);

        // loading artist images
        {
            let client = Client::get().lock().unwrap().inner.clone().unwrap();
            if let Some(id) = &model.info.cover_art {
                let buffer: Vec<u8> = client.get_cover_art(id, Some(300)).await.unwrap();
                let bytes = gtk::glib::Bytes::from(&buffer);
                let stream = gtk::gio::MemoryInputStream::from_bytes(&bytes);
                match Pixbuf::from_stream(&stream, gtk::gio::Cancellable::NONE) {
                    Ok(pixbuf) => model.image.set_from_pixbuf(Some(&pixbuf)),
                    _ => {} // TODO replace with stock image
                }
            }
        }

        let widgets = view_output!();

        widgets.actual_cover.set_visible(true);

        relm4::component::AsyncComponentParts { model, widgets }
    }

    fn init_loading_widgets(root: &mut Self::Root) -> Option<LoadingWidgets> {
        view! {
            #[local_ref]
            root {
                add_css_class: "artists-element",

                #[name(spinner)]
                gtk::Box {
                    add_css_class: "card",
                    add_css_class: "size150",
                    set_halign: gtk::Align::Center,

                    gtk::Spinner {
                        add_css_class: "size50",
                        set_hexpand: true,
                        set_halign: gtk::Align::Center,
                        start: (),
                    }
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
            add_controller: &model.drag_src.clone(),

            #[name = "actual_cover"]
            gtk::Box {
                set_visible: false,
                set_halign: gtk::Align::Center,

                append = &model.image.clone() -> gtk::Image {
                    add_css_class: "size150",
                    add_css_class: "card",
                },
            },
            gtk::Label {
                set_text: &model.info.name,
                set_ellipsize: pango::EllipsizeMode::End,
                set_max_width_chars: 15,
                set_size_request: (150, -1),
            }
        }
    }
}
