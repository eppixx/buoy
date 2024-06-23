use relm4::gtk::{
    self,
    prelude::{BoxExt, ButtonExt, OrientableExt, ToValue, WidgetExt},
};

use crate::types::Droppable;

#[derive(Debug)]
pub struct PlaylistElement {
    playlist: submarine::data::PlaylistWithSongs,
    index: relm4::factory::DynamicIndex,
    drag_src: gtk::DragSource,

    edit_area: gtk::Stack,
    edit: gtk::Button,
    delete: gtk::Button,
}

impl PlaylistElement {
    pub fn set_edit_area(&self, status: bool) {
        if status {
            self.edit_area.set_visible_child_name("edit");
        } else {
            self.edit_area.set_visible_child_name("clean");
        }
    }
}

#[derive(Debug)]
pub enum PlaylistElementIn {
    Clicked,
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
    type Init = submarine::data::PlaylistWithSongs;
    type Input = PlaylistElementIn;
    type Output = PlaylistElementOut;
    type ParentWidget = gtk::ListBox;
    type CommandOutput = ();

    fn init_model(
        init: Self::Init,
        index: &relm4::factory::DynamicIndex,
        _sender: relm4::FactorySender<Self>,
    ) -> Self {
        let model = Self {
            playlist: init.clone(),
            index: index.clone(),
            drag_src: gtk::DragSource::default(),

            edit_area: gtk::Stack::default(),
            edit: gtk::Button::default(),
            delete: gtk::Button::default(),
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
                            set_text: &format!("{} songs", self.playlist.base.song_count),
                        }
                    },

                    gtk::Box {
                        // set_orientation: gtk::Orientation::Vertical,
                        // set_homogeneous: true,
                        self.edit_area.clone() -> gtk::Stack {

                            add_named[Some("clean")] = &gtk::Box {},
                            add_named[Some("edit")] = &gtk::Box {
                                set_spacing: 10,

                                self.edit.clone() -> gtk::Button {
                                    set_icon_name: "edit-symbolic",
                                    set_tooltip_text: Some("Rename Playlist"),
                                },
                                self.delete.clone() -> gtk::Button {
                                    add_css_class: granite::STYLE_CLASS_DESTRUCTIVE_ACTION,
                                    set_icon_name: "edit-delete-symbolic",
                                    set_tooltip_text: Some("Delete Playlist"),
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    fn update(&mut self, msg: Self::Input, sender: relm4::FactorySender<Self>) {
        match msg {
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
