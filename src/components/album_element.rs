use std::{cell::RefCell, rc::Rc};

use relm4::{
    gtk::{
        self,
        prelude::{ButtonExt, ToValue, WidgetExt},
    },
    Component, ComponentController,
};

use super::descriptive_cover::DescriptiveCoverOut;
use crate::{
    common::convert_for_label,
    components::descriptive_cover::{DescriptiveCover, DescriptiveCoverInit},
    subsonic::Subsonic,
    types::{Droppable, Id},
};

#[derive(Debug)]
pub struct AlbumElement {
    cover: relm4::Controller<DescriptiveCover>,
}

#[derive(Debug)]
pub enum AlbumElementIn {
    DescriptiveCover(DescriptiveCoverOut),
}

#[derive(Debug)]
pub enum AlbumElementOut {
    Clicked(AlbumElementInit),
    DisplayToast(String),
}

#[derive(Debug, Clone)]
pub enum AlbumElementInit {
    Child(Box<submarine::data::Child>),
    AlbumId3(Box<submarine::data::AlbumId3>),
}

#[relm4::component(pub)]
impl relm4::SimpleComponent for AlbumElement {
    type Init = (Rc<RefCell<Subsonic>>, AlbumElementInit);
    type Input = AlbumElementIn;
    type Output = AlbumElementOut;

    fn init(
        (subsonic, init): Self::Init,
        root: Self::Root,
        sender: relm4::ComponentSender<Self>,
    ) -> relm4::ComponentParts<Self> {
        // init cover
        // let mut builder = DescriptiveCoverInit::default();
        let (builder, drop) = match &init {
            AlbumElementInit::AlbumId3(id3) => {
                let builder = DescriptiveCoverInit::new(
                    id3.name.clone(),
                    id3.cover_art.as_ref().map(|s| Id::album(s)).clone(),
                    id3.artist.clone(),
                );
                (builder, Droppable::Album(id3.clone()))
            }
            AlbumElementInit::Child(child) => {
                let builder = DescriptiveCoverInit::new(
                    child.title.clone(),
                    child.cover_art.as_ref().map(|s| Id::song(s)).clone(),
                    child.artist.clone(),
                );
                (builder, Droppable::AlbumChild(child.clone()))
            }
        };

        let cover: relm4::Controller<DescriptiveCover> = DescriptiveCover::builder()
            .launch((subsonic, builder))
            .forward(sender.input_sender(), AlbumElementIn::DescriptiveCover);
        let model = Self { cover };

        // tooltip string
        let tooltip = match &init {
            AlbumElementInit::AlbumId3(album) => {
                let year = match album.year {
                    Some(year) => format!("Year: {year} • "),
                    None => String::new(),
                };
                format!(
                    "{year}{} songs • Length: {}",
                    album.song_count,
                    convert_for_label(i64::from(album.duration) * 1000)
                )
            }
            AlbumElementInit::Child(child) => {
                let year = match child.year {
                    Some(year) => format!("Year: {year} • "),
                    None => String::new(),
                };
                let duration = match child.duration {
                    Some(duration) => {
                        format!("Length: {}", convert_for_label(i64::from(duration) * 1000))
                    }
                    None => String::new(),
                };
                format!("{year}{duration}")
            }
        };

        let widgets = view_output!();

        //setup DropSource
        let content = gtk::gdk::ContentProvider::for_value(&drop.to_value());
        let drag_src = gtk::DragSource::new();
        drag_src.set_actions(gtk::gdk::DragAction::MOVE);
        drag_src.set_content(Some(&content));
        model.cover.widget().add_controller(drag_src);

        relm4::ComponentParts { model, widgets }
    }

    view! {
        gtk::FlowBoxChild {
            add_css_class: "album-element",

            gtk::Button {
                add_css_class: "flat",
                set_halign: gtk::Align::Center,

                connect_clicked[sender, init] => move |_btn| {
                    sender.output(AlbumElementOut::Clicked(init.clone())).unwrap();
                },

                #[wrap(Some)]
                set_child = &model.cover.widget().clone() {
                    set_tooltip_text: Some(&tooltip),
                }
                //TODO add tooltip with info about album
            }
        }
    }

    fn update(&mut self, msg: Self::Input, sender: relm4::ComponentSender<Self>) {
        match msg {
            AlbumElementIn::DescriptiveCover(msg) => match msg {
                DescriptiveCoverOut::DisplayToast(title) => sender
                    .output(AlbumElementOut::DisplayToast(title))
                    .expect("sending failed"),
            },
        }
    }
}
