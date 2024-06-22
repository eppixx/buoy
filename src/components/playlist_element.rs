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
    common::convert_for_label, components::cover::Cover, subsonic::Subsonic, types::Droppable,
};

#[derive(Debug)]
pub struct PlaylistElement {
    playlist: submarine::data::PlaylistWithSongs,
    cover: relm4::Controller<Cover>,
    index: relm4::factory::DynamicIndex,
    drag_src: gtk::DragSource,
}

#[derive(Debug)]
pub enum PlaylistElementIn {
    Clicked,
    Cover(CoverOut),
}

#[derive(Debug)]
pub enum PlaylistElementOut {
    Clicked(
        relm4::factory::DynamicIndex,
        submarine::data::PlaylistWithSongs,
    ),
    DisplayToast(String),
}

#[relm4::factory(pub)]
impl relm4::factory::FactoryComponent for PlaylistElement {
    type Init = (Rc<RefCell<Subsonic>>, submarine::data::PlaylistWithSongs);
    type Input = PlaylistElementIn;
    type Output = PlaylistElementOut;
    type ParentWidget = gtk::ListBox;
    type CommandOutput = ();

    fn init_model(
        (subsonic, init): Self::Init,
        index: &relm4::factory::DynamicIndex,
        sender: relm4::FactorySender<Self>,
    ) -> Self {
        //init cover
        let cover: relm4::Controller<Cover> = Cover::builder()
            .launch((subsonic, Some(init.base.id.clone())))
            .forward(sender.input_sender(), PlaylistElementIn::Cover);
        cover.model().add_css_class_image("size50");

        let model = Self {
            playlist: init.clone(),
            cover,
            index: index.clone(),
            drag_src: gtk::DragSource::default(),
        };

        //setup content for DropSource
        let drop = Droppable::Playlist(Box::new(init.clone()));
        let content = gtk::gdk::ContentProvider::for_value(&drop.to_value());
        model.drag_src.set_content(Some(&content));
        model.drag_src.set_actions(gtk::gdk::DragAction::MOVE);

        model
    }

    view! {
        gtk::ListBoxRow {
            add_css_class: "playlist-element",

            gtk::Button {
                add_css_class: "flat",
                set_halign: gtk::Align::Fill,

                add_controller = self.drag_src.clone(),

                connect_clicked => move |_btn| sender.input(PlaylistElementIn::Clicked),

                gtk::Box {
                    set_spacing: 5,

                    self.cover.widget().clone(),

                    gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_hexpand: true,
                        set_homogeneous: true,

                        gtk::Label {
                            set_halign: gtk::Align::Start,
                            set_text: &self.playlist.base.name,
                        },
                        gtk::Label {
                            set_halign: gtk::Align::Start,
                            set_text: &format!("{} songs • {}"
                                               , self.playlist.base.song_count
                                               , convert_for_label(i64::from(self.playlist.base.duration) * 1000)),
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

    fn update(&mut self, msg: Self::Input, sender: relm4::FactorySender<Self>) {
        match msg {
            PlaylistElementIn::Cover(msg) => match msg {
                CoverOut::DisplayToast(title) => sender
                    .output(PlaylistElementOut::DisplayToast(title))
                    .expect("sending failed"),
            },
            PlaylistElementIn::Clicked => {
                sender
                    .output(PlaylistElementOut::Clicked(
                        self.index.clone(),
                        self.playlist.clone(),
                    ))
                    .expect("sending failed");
            }
        }
    }
}
