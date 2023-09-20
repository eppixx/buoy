use relm4::{
    gtk::{
        self,
        traits::{BoxExt, ButtonExt, OrientableExt, WidgetExt},
    },
    loading_widgets::LoadingWidgets,
    view, Component, ComponentController,
};

use crate::components::descriptive_cover::{DescriptiveCover, DescriptiveCoverBuilder};
use crate::{client::Client, types::Id};

#[derive(Debug, Default)]
pub struct AlbumsView {
    albums: gtk::FlowBox,
    album_list: Vec<relm4::Controller<AlbumElement>>,
}

#[derive(Debug)]
pub enum AlbumsViewOut {
    ClickedAlbum(Id),
}

#[derive(Debug)]
pub enum AlbumsViewIn {
    AlbumElement(AlbumElementOut),
}

#[relm4::component(async, pub)]
impl relm4::component::AsyncComponent for AlbumsView {
    type Input = AlbumsViewIn;
    type Output = AlbumsViewOut;
    type Init = ();
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
        for album in albums.into_iter().rev() {
            let cover: relm4::Controller<AlbumElement> = AlbumElement::builder()
                .launch(album)
                .forward(sender.input_sender(), AlbumsViewIn::AlbumElement);
            model.albums.insert(cover.widget(), 0);
            model.album_list.insert(0, cover);
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
                AlbumElementOut::Clicked(id) => {
                    sender.output(AlbumsViewOut::ClickedAlbum(id)).unwrap()
                }
            },
        }
    }
}

#[derive(Debug)]
pub struct AlbumElement {
    cover: relm4::Controller<DescriptiveCover>,
}

#[derive(Debug)]
pub enum AlbumElementOut {
    Clicked(Id),
}

#[relm4::component(pub)]
impl relm4::SimpleComponent for AlbumElement {
    type Input = ();
    type Output = AlbumElementOut;
    type Init = submarine::data::Child;

    fn init(
        id: Self::Init,
        root: &Self::Root,
        sender: relm4::ComponentSender<Self>,
    ) -> relm4::ComponentParts<Self> {
        // init cover
        let mut builder = DescriptiveCoverBuilder::default().title(&id.title);
        if let Some(id) = &id.cover_art {
            // builder = builder.image(id);
        }
        if let Some(artist) = &id.artist {
            builder = builder.subtitle(artist);
        }

        let cover: relm4::Controller<DescriptiveCover> =
            DescriptiveCover::builder().launch(builder).detach();
        let model = Self { cover };

        let widgets = view_output!();

        relm4::ComponentParts { model, widgets }
    }

    view! {
        gtk::Box {
            add_css_class: "albums-view-album",

            gtk::Button {
                add_css_class: "flat",
                connect_clicked[sender, id] => move |_btn| {
                    sender.output(AlbumElementOut::Clicked(Id::album(&id.id))).unwrap();
                },

                #[wrap(Some)]
                set_child = &model.cover.widget().clone(),
            }
        }
    }
}
