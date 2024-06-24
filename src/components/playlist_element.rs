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
    pub fn get_list(&self) -> &submarine::data::PlaylistWithSongs {
        &self.playlist
    }

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
    DeletePressed,
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

            gtk::Box {
                add_css_class: "flat",
                set_spacing: 5,
                set_halign: gtk::Align::Fill,

                add_controller = self.drag_src.clone(),

                add_controller = gtk::GestureClick {
                    connect_released[sender] => move |_w, _, _, _| {
                        sender.input(PlaylistElementIn::Clicked);
                    }
                },

                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_hexpand: true,
                    set_homogeneous: true,

                    gtk::Label {
                        add_css_class: granite::STYLE_CLASS_H3_LABEL,
                        set_halign: gtk::Align::Start,
                        set_text: &self.playlist.base.name,
                    },
                    gtk::Label {
                        set_halign: gtk::Align::Start,
                        set_text: &format!("{} songs", self.playlist.base.song_count),
                    }
                },

                gtk::Box {
                    self.edit_area.clone() -> gtk::Stack {
                        set_transition_duration: 250,
                        set_transition_type: gtk::StackTransitionType::Crossfade,

                        add_named[Some("clean")] = &gtk::Box {},
                        add_named[Some("edit")] = &gtk::Box {
                            set_orientation: gtk::Orientation::Vertical,
                            set_valign: gtk::Align::Center,

                            gtk::Box {
                                set_spacing: 10,

                                self.edit.clone() -> gtk::Button {
                                    set_icon_name: "edit-symbolic",
                                    set_tooltip_text: Some("Rename Playlist"),
                                    connect_clicked[sender] => move |_widget| {
                                        sender.output(PlaylistElementOut::DisplayToast(String::from("edit list"))).expect("sending failed");
                                    },
                                },
                                self.delete.clone() -> gtk::Button {
                                    add_css_class: granite::STYLE_CLASS_DESTRUCTIVE_ACTION,
                                    set_icon_name: "edit-delete-symbolic",
                                    set_tooltip_text: Some("Delete Playlist"),

                                    connect_clicked => PlaylistElementIn::DeletePressed,
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
            PlaylistElementIn::DeletePressed => {
                //TODO
            }
        }
    }
}
