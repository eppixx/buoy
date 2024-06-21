use std::{cell::RefCell, rc::Rc};

use relm4::{
    gtk::{
        self,
        prelude::{BoxExt, ButtonExt, OrientableExt, ToValue, WidgetExt},
    },
    Component, ComponentController,
};

use super::cover::CoverOut;
use crate::{
    common::convert_for_label,
    components::cover::Cover,
    subsonic::Subsonic,
    types::{Droppable, Id},
};

#[derive(Debug)]
pub struct PlaylistElement {
    playlist: submarine::data::PlaylistWithSongs,
    cover: relm4::Controller<Cover>,
}

#[derive(Debug)]
pub enum PlaylistElementIn {
    Cover(CoverOut),
}

#[derive(Debug)]
pub enum PlaylistElementOut {
    // Clicked(AlbumElementInit),
    DisplayToast(String),
}

// #[derive(Debug, Clone)]
// pub enum AlbumElementInit {
//     Child(Box<submarine::data::Child>),
//     AlbumId3(Box<submarine::data::AlbumId3>),
// }

#[relm4::component(pub)]
impl relm4::SimpleComponent for PlaylistElement {
    type Init = (Rc<RefCell<Subsonic>>, submarine::data::PlaylistWithSongs);
    type Input = PlaylistElementIn;
    type Output = PlaylistElementOut;

    fn init(
        (subsonic, init): Self::Init,
        root: Self::Root,
        sender: relm4::ComponentSender<Self>,
    ) -> relm4::ComponentParts<Self> {
        // init cover
        // let mut builder = DescriptiveCoverBuilder::default();
        // let drop = match &init {
        //     AlbumElementInit::Child(child) => {
        //         builder = builder.title(child.title.clone());
        //         if let Some(id) = &child.cover_art {
        //             builder = builder.id(Id::song(id));
        //         }
        //         if let Some(artist) = &child.artist {
        //             builder = builder.subtitle(artist);
        //         }
        //         Droppable::AlbumChild(child.clone())
        //     }
        // };

        let cover: relm4::Controller<Cover> = Cover::builder()
            .launch((subsonic, init.base.cover_art.clone()))
            .forward(sender.input_sender(), PlaylistElementIn::Cover);
        cover.model().add_css_class_image("size50");
        let model = Self {
            playlist: init,
            cover,
        };

        // tooltip string
        // let tooltip = match &init {
        //     AlbumElementInit::Child(child) => {
        //         let year = match child.year {
        //             Some(year) => format!("Year: {year} • "),
        //             None => String::new(),
        //         };
        //         let duration = match child.duration {
        //             Some(duration) => {
        //                 format!("Length: {}", convert_for_label(i64::from(duration) * 1000))
        //             }
        //             None => String::new(),
        //         };
        //         format!("{year}{duration}")
        //     }
        // };

        let widgets = view_output!();

        //setup DropSource
        // let content = gtk::gdk::ContentProvider::for_value(&drop.to_value());
        // let drag_src = gtk::DragSource::new();
        // drag_src.set_actions(gtk::gdk::DragAction::MOVE);
        // drag_src.set_content(Some(&content));
        // model.cover.widget().add_controller(drag_src);

        relm4::ComponentParts { model, widgets }
    }

    view! {
        gtk::ListBoxRow {
            add_css_class: "playlist-element",

            gtk::Button {
                add_css_class: "flat",
                set_halign: gtk::Align::Fill,

                // connect_clicked[sender, init] => move |_btn| {
                //     sender.output(PlaylistElementOut::Clicked(init.clone())).unwrap();
                // },

                gtk::Box {
                    set_spacing: 5,

                    model.cover.widget().clone() {
                        // set_tooltip_text: Some(&tooltip),
                    },
                    gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_hexpand: true,
                        set_homogeneous: true,

                        gtk::Label {
                            set_halign: gtk::Align::Start,
                            set_text: &model.playlist.base.name,
                        },
                        gtk::Label {
                            set_halign: gtk::Align::Start,
                            set_text: &format!("{} songs • {}"
                                               , model.playlist.entry.len()
                                               , convert_for_label(i64::from(model.playlist.base.duration) * 1000)),
                        }
                    },

                    gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_homogeneous: true,

                        gtk::Button {
                            set_icon_name: "edit-symbolic",
                        },
                        gtk::Button {
                            set_icon_name: "edit-delete-symbolic",
                        }
                    }
                }
            }
        }
    }

    fn update(&mut self, msg: Self::Input, sender: relm4::ComponentSender<Self>) {
        match msg {
            PlaylistElementIn::Cover(msg) => match msg {
                CoverOut::DisplayToast(title) => sender
                    .output(PlaylistElementOut::DisplayToast(title))
                    .expect("sending failed"),
            },
        }
    }
}
