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
    types::{Droppable},
};

#[derive(Debug)]
pub struct PlaylistElement {
    playlist: submarine::data::PlaylistWithSongs,
    cover: relm4::Controller<Cover>,
    // index: relm4::factory::DynamicIndex,
}

#[derive(Debug)]
pub enum PlaylistElementIn {
    Cover(CoverOut),
}

#[derive(Debug)]
pub enum PlaylistElementOut {
    Clicked(submarine::data::PlaylistWithSongs),
    // Clicked(relm4::factory::DynamicIndex, submarine::data::PlaylistWithSongs),
    DisplayToast(String),
}

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
        //init cover
        let cover: relm4::Controller<Cover> = Cover::builder()
            .launch((subsonic, Some(init.base.id.clone())))
            .forward(sender.input_sender(), PlaylistElementIn::Cover);
        cover.model().add_css_class_image("size50");
        let model = Self {
            playlist: init.clone(),
            cover,
        };

        //setup content for DropSource
        let drop = Droppable::Playlist(Box::new(init.clone()));
        let content = gtk::gdk::ContentProvider::for_value(&drop.to_value());

        let widgets = view_output!();

        relm4::ComponentParts { model, widgets }
    }

    view! {
        gtk::ListBoxRow {
            add_css_class: "playlist-element",

            gtk::Button {
                add_css_class: "flat",
                set_halign: gtk::Align::Fill,

                add_controller = gtk::DragSource {
                    set_actions: gtk::gdk::DragAction::MOVE,
                    set_content: Some(&content),
                },

                connect_clicked[sender] => move |_btn| {
                    sender.output(PlaylistElementOut::Clicked(init.clone())).unwrap();
                },

                gtk::Box {
                    set_spacing: 5,

                    model.cover.widget().clone(),

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
                                               , model.playlist.base.song_count
                                               , convert_for_label(i64::from(model.playlist.base.duration) * 1000)),
                        }
                    },

                    gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_homogeneous: true,

                        gtk::Button {
                            set_icon_name: "edit-symbolic",
                            set_tooltip_text: Some("Rename Playlist"),
                        },
                        gtk::Button {
                            set_icon_name: "edit-delete-symbolic",
                            set_tooltip_text: Some("Delete Playlist"),
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
