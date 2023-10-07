use relm4::{
    gtk::{
        self,
        prelude::ToValue,
        traits::{ButtonExt, WidgetExt},
    },
    Component, ComponentController,
};

use crate::{
    components::descriptive_cover::{DescriptiveCover, DescriptiveCoverBuilder},
    types::Droppable,
};

use super::descriptive_cover::DescriptiveCoverOut;

#[derive(Debug)]
pub struct ArtistElement {
    cover: relm4::Controller<DescriptiveCover>,
}

#[derive(Debug)]
pub enum ArtistElementIn {
    DescriptiveCover(DescriptiveCoverOut),
}

#[derive(Debug)]
pub enum ArtistElementOut {
    Clicked(submarine::data::ArtistId3),
}

#[relm4::component(pub)]
impl relm4::SimpleComponent for ArtistElement {
    type Init = submarine::data::ArtistId3;
    type Input = ArtistElementIn;
    type Output = ArtistElementOut;

    fn init(
        init: Self::Init,
        root: &Self::Root,
        sender: relm4::ComponentSender<Self>,
    ) -> relm4::ComponentParts<Self> {
        // init cover
        let mut builder = DescriptiveCoverBuilder::default().title(&init.name);
        if let Some(id) = &init.cover_art {
            builder = builder.image(id);
        }
        let cover: relm4::Controller<DescriptiveCover> = DescriptiveCover::builder()
            .launch(builder)
            .forward(sender.input_sender(), ArtistElementIn::DescriptiveCover);
        let model = Self { cover };

        let widgets = view_output!();

        //setup DropSource
        let drop = Droppable::Artist(Box::new(init.clone()));
        let content = gtk::gdk::ContentProvider::for_value(&drop.to_value());
        let drag_src = gtk::DragSource::new();
        drag_src.set_actions(gtk::gdk::DragAction::MOVE);
        drag_src.set_content(Some(&content));
        model.cover.widget().add_controller(drag_src);

        relm4::ComponentParts { model, widgets }
    }

    view! {
        gtk::Box {
            add_css_class: "artist-element",

            gtk::Button {
                add_css_class: "flat",
                set_halign: gtk::Align::Center,

                connect_clicked[sender, init] => move |_btn| {
                    sender.output(ArtistElementOut::Clicked(init.clone())).unwrap();
                },

                #[wrap(Some)]
                set_child = &model.cover.widget().clone(),
            }
        }
    }
}
