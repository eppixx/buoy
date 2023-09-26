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
            builder = builder.image(id);
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
        gtk::FlowBoxChild {
            add_css_class: "albums-view-album",

            gtk::Button {
                add_css_class: "flat",
                connect_clicked[sender, id] => move |_btn| {
                    sender.output(AlbumElementOut::Clicked(Id::album(&id.id))).unwrap();
                },

                #[wrap(Some)]
                set_child = &model.cover.widget().clone(),
                //TODO add tooltip with info about album
            }
        }
    }
}
