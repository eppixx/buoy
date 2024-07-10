use std::{cell::RefCell, rc::Rc};

use relm4::{
    gtk::{
        self,
        prelude::{ButtonExt, ToValue, WidgetExt},
    },
    Component, ComponentController,
};

use crate::{
    components::descriptive_cover::{DescriptiveCover, DescriptiveCoverInit},
    subsonic::Subsonic,
    types::{Droppable, Id},
};

use super::descriptive_cover::DescriptiveCoverOut;

#[derive(Debug)]
pub struct ArtistElement {
    cover: relm4::Controller<DescriptiveCover>,
    init: submarine::data::ArtistId3,
}

impl ArtistElement {
    pub fn info(&self) -> &submarine::data::ArtistId3 {
        &self.init
    }
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
    type Init = (Rc<RefCell<Subsonic>>, submarine::data::ArtistId3);
    type Input = ArtistElementIn;
    type Output = ArtistElementOut;

    fn init(
        (subsonic, init): Self::Init,
        root: Self::Root,
        sender: relm4::ComponentSender<Self>,
    ) -> relm4::ComponentParts<Self> {
        // init cover
        let builder =
            DescriptiveCoverInit::new(init.name.clone(), init.cover_art.clone(), None::<&str>);
        let cover: relm4::Controller<DescriptiveCover> = DescriptiveCover::builder()
            .launch((subsonic, builder, true, Some(Id::artist(init.id.clone()))))
            .forward(sender.input_sender(), ArtistElementIn::DescriptiveCover);
        let model = Self {
            cover,
            init: init.clone(),
        };

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
