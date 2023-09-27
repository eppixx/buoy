use relm4::{
    gtk::{
        self,
        traits::{ButtonExt, WidgetExt},
    },
    Component, ComponentController,
};

use crate::{
    components::descriptive_cover::{DescriptiveCover, DescriptiveCoverBuilder},
    types::Id,
};

#[derive(Debug)]
pub struct AlbumElement {
    cover: relm4::Controller<DescriptiveCover>,
}

#[derive(Debug)]
pub enum AlbumElementOut {
    Clicked(Id),
}

#[derive(Debug)]
pub enum AlbumElementInit {
    Child(submarine::data::Child),
    AlbumId3(submarine::data::AlbumId3),
}

#[relm4::component(pub)]
impl relm4::SimpleComponent for AlbumElement {
    type Init = AlbumElementInit;
    type Input = ();
    type Output = AlbumElementOut;

    fn init(
        init: Self::Init,
        root: &Self::Root,
        sender: relm4::ComponentSender<Self>,
    ) -> relm4::ComponentParts<Self> {
        // init cover
        let mut builder = DescriptiveCoverBuilder::default();
        let id = match init {
            AlbumElementInit::AlbumId3(id3) => {
                builder = builder.title(id3.name);
                if let Some(id) = &id3.cover_art {
                    builder = builder.image(id);
                }
                if let Some(artist) = &id3.artist {
                    builder = builder.subtitle(artist);
                }
                id3.id
            }
            AlbumElementInit::Child(child) => {
                builder = builder.title(child.title);
                if let Some(id) = &child.cover_art {
                    builder = builder.image(id);
                }
                if let Some(artist) = &child.artist {
                    builder = builder.subtitle(artist);
                }
                child.id
            }
        };

        let cover: relm4::Controller<DescriptiveCover> =
            DescriptiveCover::builder().launch(builder).detach();
        let model = Self { cover };

        let widgets = view_output!();

        relm4::ComponentParts { model, widgets }
    }

    view! {
        gtk::FlowBoxChild {
            add_css_class: "albums-view-album",

            gtk::Button {
                add_css_class: "flat",
                connect_clicked[sender, id] => move |_btn| {
                    sender.output(AlbumElementOut::Clicked(Id::album(&id))).unwrap();
                },

                #[wrap(Some)]
                set_child = &model.cover.widget().clone(),
                //TODO add tooltip with info about album
            }
        }
    }
}
