use relm4::{
    gtk::{
        self,
        traits::{BoxExt, OrientableExt, WidgetExt},
    },
    loading_widgets::LoadingWidgets,
    view, Component, ComponentController,
};

use super::artist_element::ArtistElementOut;
use crate::{client::Client, components::artist_element::ArtistElement};

#[derive(Debug, Default)]
pub struct ArtistsView {
    artists: gtk::FlowBox,
    artist_list: Vec<relm4::Controller<ArtistElement>>,
}

#[derive(Debug)]
pub enum ArtistsViewOut {
    ClickedArtist(submarine::data::ArtistId3),
}

#[derive(Debug)]
pub enum ArtistsViewIn {
    ArtistElement(ArtistElementOut),
}

#[relm4::component(async, pub)]
impl relm4::component::AsyncComponent for ArtistsView {
    type Input = ArtistsViewIn;
    type Output = ArtistsViewOut;
    type Init = ();
    type CommandOutput = ();

    fn init_loading_widgets(root: &mut Self::Root) -> Option<LoadingWidgets> {
        view! {
            append = root.clone() -> gtk::Box {
                add_css_class: "artists-view",

                #[name(loading_box)]
                gtk::Box {
                    set_hexpand: true,
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

    async fn init(
        _init: Self::Init,
        root: Self::Root,
        sender: relm4::AsyncComponentSender<Self>,
    ) -> relm4::component::AsyncComponentParts<Self> {
        let mut model = Self::default();
        let widgets = view_output!();

        // get artists
        let artists: Vec<submarine::data::ArtistId3> = {
            let client = Client::get().lock().unwrap().inner.clone().unwrap();
            let indexes: Vec<submarine::data::IndexId3> = client.get_artists(None).await.unwrap();
            indexes.into_iter().flat_map(|i| i.artist).collect()
        };

        // add artists with cover and title
        for (i, artist) in artists.into_iter().enumerate() {
            let cover: relm4::Controller<ArtistElement> = ArtistElement::builder()
                .launch(artist)
                .forward(sender.input_sender(), ArtistsViewIn::ArtistElement);
            model.artists.insert(cover.widget(), i as i32);
            model.artist_list.insert(i, cover);
        }

        relm4::component::AsyncComponentParts { model, widgets }
    }

    view! {
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            set_hexpand: true,

            gtk::Label {
                add_css_class: "h2",
                set_label: "Artists",
                set_halign: gtk::Align::Center,
            },

            gtk::ScrolledWindow {
                set_hexpand: true,
                set_vexpand: true,

                #[wrap(Some)]
                set_child = &model.artists.clone() -> gtk::FlowBox {
                }
            }
        }
    }

    async fn update(
        &mut self,
        msg: Self::Input,
        sender: relm4::AsyncComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match msg {
            ArtistsViewIn::ArtistElement(msg) => match msg {
                ArtistElementOut::Clicked(id) => {
                    sender.output(ArtistsViewOut::ClickedArtist(id)).unwrap()
                }
            },
        }
    }
}
