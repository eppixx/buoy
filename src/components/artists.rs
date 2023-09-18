use relm4::{
    gtk::{
        self,
        traits::{BoxExt, OrientableExt, WidgetExt},
    },
    loading_widgets::LoadingWidgets,
    view, Component, ComponentController,
};

use crate::{client::Client, types::Id};

use super::cover::{Cover, CoverIn};

#[derive(Debug, Default)]
pub struct ArtistsView {
    artists: gtk::FlowBox,
}

#[derive(Debug)]
pub enum ArtistsViewOut {
    ClickedArtist(Id),
}

#[derive(Debug)]
pub enum ArtistsViewIn {
    ClickedArtist(Id),
    AddArtist(submarine::data::ArtistId3),
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

        let artists: Vec<submarine::data::ArtistId3> = {
            let client = Client::get().lock().unwrap().inner.clone().unwrap();
            let indexes: Vec<submarine::data::IndexId3> = client.get_artists(None).await.unwrap();
            indexes.into_iter().flat_map(|i| i.artist).collect()
        };

        for artist in artists.into_iter().take(1).rev() {
            let cover: relm4::Controller<ArtistElement> =
                ArtistElement::builder().launch(artist).detach();
            model.artists.insert(cover.widget(), 0);
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
        _sender: relm4::AsyncComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match msg {
            ArtistsViewIn::ClickedArtist(_) => todo!(),
            ArtistsViewIn::AddArtist(_) => todo!(),
        }
    }
}

#[derive(Debug)]
pub struct ArtistElement {
    cover: relm4::Controller<Cover>,
}

#[derive(Debug)]
pub enum ArtistElementOut {
    Clicked(Id),
}

#[relm4::component(pub)]
impl relm4::SimpleComponent for ArtistElement {
    type Input = ();
    type Output = ArtistElementOut;
    type Init = submarine::data::ArtistId3;

    fn init(
        id: Self::Init,
        root: &Self::Root,
        sender: relm4::ComponentSender<Self>,
    ) -> relm4::ComponentParts<Self> {
        let cover: relm4::Controller<Cover> = Cover::builder().launch(()).detach();
        let model = Self { cover };
        let widgets = view_output!();

        //init widgets
        model.cover.emit(CoverIn::SetTitle(Some(id.name)));
        if let Some(id) = id.cover_art {
            // model.cover.emit(CoverIn::LoadImage(Some(id)));
        }

        relm4::ComponentParts { model, widgets }
    }

    view! {
        gtk::Box {
            //TODO add button
            append: &model.cover.widget().clone(),
        }
    }
}
