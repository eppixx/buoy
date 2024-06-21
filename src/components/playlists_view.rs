use crate::{components::playlist_element::PlaylistElement, subsonic::Subsonic};
use relm4::{
    gtk::{
        self,
        prelude::{BoxExt, ButtonExt, ListBoxRowExt, OrientableExt, WidgetExt},
    },
    Component, ComponentController,
};

use std::{cell::RefCell, rc::Rc};

use super::playlist_element::PlaylistElementOut;

#[derive(Debug, Default)]
pub struct PlaylistsView {
    playlists: gtk::ListBox,
    playlist_items: Vec<gtk::ListBoxRow>,
}

#[derive(Debug)]
pub enum PlaylistsViewOut {
    ReplaceQueue(Vec<submarine::data::Child>),
    AppendToQueue(Vec<submarine::data::Child>),
    DisplayToast(String),
}

#[derive(Debug)]
pub enum PlaylistsViewIn {
    SearchChanged(String),
    NewPlaylist(Vec<submarine::data::Child>),
    PlaylistElement(PlaylistElementOut),
}

#[relm4::component(pub)]
impl relm4::SimpleComponent for PlaylistsView {
    type Init = Rc<RefCell<Subsonic>>;
    type Input = PlaylistsViewIn;
    type Output = PlaylistsViewOut;

    fn init(
        init: Self::Init,
        root: Self::Root,
        sender: relm4::ComponentSender<Self>,
    ) -> relm4::ComponentParts<Self> {
        let model = PlaylistsView::default();
        let widgets = view_output!();

        // add playlists to list
        for (i, playlist) in init.borrow().playlists().iter().enumerate() {
            let row = PlaylistElement::builder()
                .launch((init.clone(), playlist.clone()))
                .forward(sender.input_sender(), PlaylistsViewIn::PlaylistElement);
            // let item = gtk::ListBoxRow::new();
            // item.set_child(Some(&gtk::Label::builder().label(&playlist.base.name).build()));
            model.playlists.insert(row.widget(), i as i32);
        }

        relm4::ComponentParts { model, widgets }
    }

    view! {
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            set_spacing: 5,

            gtk::WindowHandle {
                gtk::Label {
                    add_css_class: granite::STYLE_CLASS_H2_LABEL,
                    set_halign: gtk::Align::Center,
                    set_text: "Playlists",
                },
            },

            gtk::Paned {
                set_position: 250,
                set_shrink_start_child: false,
                set_resize_start_child: false,
                set_shrink_end_child: false,

                #[wrap(Some)]
                set_start_child = &model.playlists.clone() -> gtk::ListBox {
                    add_css_class: "playlist-view-playlist-list",
                    set_vexpand: true,

                    gtk::ListBoxRow {
                        add_css_class: "playlist-view-add-playlist",

                        gtk::Button {
                            gtk::Box {
                                set_halign: gtk::Align::Center,

                                gtk::Image {
                                    set_icon_name: Some("list-add-symbolic"),
                                },
                                gtk::Label {
                                    set_text: "New Playlist",
                                }
                            },

                            connect_clicked => PlaylistsViewIn::NewPlaylist(vec![]),
                        }
                    }
                },

                #[wrap(Some)]
                set_end_child = &gtk::ListBox {
                    set_hexpand: true,
                    set_vexpand: true,

                    gtk::ListBoxRow {
                        gtk::Box {
                            set_vexpand: true,

                            gtk::Label {
                                set_halign: gtk::Align::Center,
                                set_valign: gtk::Align::Center,

                                set_text: "Select a playlist to list its contents",
                            },
                        }
                    }
                }
            },
        }
    }

    fn update(&mut self, msg: Self::Input, sender: relm4::ComponentSender<Self>) {
        match msg {
            PlaylistsViewIn::SearchChanged(_search) => {
                // unimplemented!("search in dashboard"); //TODO implement
            }
            PlaylistsViewIn::NewPlaylist(list) => {
                sender
                    .output(PlaylistsViewOut::DisplayToast(String::from(
                        "new playlist clicked",
                    )))
                    .expect("sending failed");
            }
            PlaylistsViewIn::PlaylistElement(msg) => match msg {
                _ => {}
            },
        }
    }
}
