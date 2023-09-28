use relm4::{
    gtk::{
        self,
        traits::{BoxExt, OrientableExt, WidgetExt},
    },
    loading_widgets::LoadingWidgets,
    view, Component, ComponentController,
};

use super::album_element::AlbumElementOut;
use crate::client::Client;
use crate::components::album_element::{AlbumElement, AlbumElementInit};

#[derive(Debug, Default)]
pub struct AlbumsView {
    albums: gtk::FlowBox,
    album_list: Vec<relm4::Controller<AlbumElement>>,
}

#[derive(Debug)]
pub enum AlbumsViewOut {
    Clicked(AlbumElementInit),
}

#[derive(Debug)]
pub enum AlbumsViewIn {
    AlbumElement(AlbumElementOut),
}

#[relm4::component(async, pub)]
impl relm4::component::AsyncComponent for AlbumsView {
    type Init = ();
    type Input = AlbumsViewIn;
    type Output = AlbumsViewOut;
    type CommandOutput = ();

    fn init_loading_widgets(root: &mut Self::Root) -> Option<LoadingWidgets> {
        view! {
            append = root.clone() -> gtk::Box {
                add_css_class: "albums-view",

                #[name(loading_box)]
                gtk::Box {
                    set_hexpand: true,
                    set_spacing: 30,
                    set_halign: gtk::Align::Center,
                    set_orientation: gtk::Orientation::Vertical,

                    gtk::Label {
                        add_css_class: "h2",
                        set_label: "Loading albums",
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

        // get albums
        let albums: Vec<submarine::data::Child> = {
            let mut albums = vec![];
            let client = Client::get().lock().unwrap().inner.clone().unwrap();
            let mut offset = 0;
            loop {
                let mut part = client
                    .get_album_list2(
                        submarine::api::get_album_list::Order::AlphabeticalByName,
                        Some(500),
                        Some(offset),
                        None::<&str>,
                    )
                    .await
                    .unwrap();
                if part.len() < 500 || part.is_empty() {
                    albums.append(&mut part);
                    break;
                } else {
                    albums.append(&mut part);
                    offset += 500;
                }
            }
            albums
        };

        // add albums with cover and title
        for (i, album) in albums.into_iter().enumerate() {
            let cover: relm4::Controller<AlbumElement> = AlbumElement::builder()
                .launch(AlbumElementInit::Child(Box::new(album)))
                .forward(sender.input_sender(), AlbumsViewIn::AlbumElement);
            model.albums.insert(cover.widget(), i as i32);
            model.album_list.insert(i, cover);
        }

        relm4::component::AsyncComponentParts { model, widgets }
    }

    view! {
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            set_hexpand: true,

            gtk::Label {
                add_css_class: "h2",
                set_label: "Albums",
                set_halign: gtk::Align::Center,
            },

            gtk::ScrolledWindow {
                set_hexpand: true,
                set_vexpand: true,

                #[wrap(Some)]
                set_child = &model.albums.clone() -> gtk::FlowBox {
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
            AlbumsViewIn::AlbumElement(msg) => match msg {
                AlbumElementOut::Clicked(clicked) => {
                    sender.output(AlbumsViewOut::Clicked(clicked)).unwrap()
                }
            },
        }
    }
}
