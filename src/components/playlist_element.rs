use relm4::gtk::{
    self,
    prelude::{BoxExt, ButtonExt, EditableExt, OrientableExt, ToValue, WidgetExt},
};

use crate::types::Droppable;

#[derive(Debug)]
pub struct PlaylistElement {
    playlist: submarine::data::PlaylistWithSongs,
    index: relm4::factory::DynamicIndex,
    drag_src: gtk::DragSource,

    main_stack: gtk::Stack,

    edit_area: gtk::Stack,
    edit_entry: gtk::Entry,
    edit: gtk::Button,
    delete: gtk::Button,
}

impl PlaylistElement {
    pub fn change_state(&self, state: State) {
        self.main_stack.set_visible_child_name(state.to_str());
    }
}

#[derive(Debug)]
pub enum State {
    DeleteInProgress,
    Edit,
    Normal,
}

impl State {
    fn to_str(&self) -> &str {
        match self {
            Self::DeleteInProgress => "delete",
            Self::Edit => "edit",
            Self::Normal => "normal",
        }
    }
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
    ChangeState(State),
    ConfirmRename,
}

#[derive(Debug)]
pub enum PlaylistElementOut {
    Clicked(
        relm4::factory::DynamicIndex,
        submarine::data::PlaylistWithSongs,
    ),
    Delete(relm4::factory::DynamicIndex),
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

            main_stack: gtk::Stack::default(),

            edit_area: gtk::Stack::default(),
            edit_entry: gtk::Entry::default(),
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

            self.main_stack.clone() -> gtk::Stack {
                add_named[Some(State::Normal.to_str())] = &gtk::Box {
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
                            #[watch]
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
                                        connect_clicked => PlaylistElementIn::ChangeState(State::Edit),
                                    },
                                    self.delete.clone() -> gtk::Button {
                                        add_css_class: granite::STYLE_CLASS_DESTRUCTIVE_ACTION,
                                        set_icon_name: "edit-delete-symbolic",
                                        set_tooltip_text: Some("Delete Playlist"),

                                        connect_clicked => PlaylistElementIn::ChangeState(State::DeleteInProgress),
                                    }
                                }
                            }
                        }
                    }
                },
                add_named[Some(State::Edit.to_str())] = &gtk::Box {
                    set_spacing: 10,

                    gtk::Box {
                        set_valign: gtk::Align::Center,

                        self.edit_entry.clone() -> gtk::Entry {
                            set_hexpand: true,
                            set_halign: gtk::Align::Fill,

                            #[watch]
                            set_text: &self.playlist.base.name,
                            set_tooltip_text: Some("Renamed title of playlist"),
                        }
                    },

                    gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_valign: gtk::Align::Center,

                        gtk::Box {
                            set_spacing: 10,

                            gtk::Button {
                                set_icon_name: "process-completed-symbolic",
                                connect_clicked => PlaylistElementIn::ConfirmRename,
                                set_tooltip_text: Some("Confirm renaming of playlist"),
                            },
                            gtk::Button {
                                set_icon_name: "process-stop",
                                connect_clicked => PlaylistElementIn::ChangeState(State::Normal),
                                set_tooltip_text: Some("Don't change playlist name"),
                            }
                        }
                    }
                },
                add_named[Some(State::DeleteInProgress.to_str())] = &gtk::Box {
                    set_spacing: 10,

                    gtk::Label {
                        set_hexpand: true,
                        set_label: &format!("Really delete \"{}\"?", self.playlist.base.name),
                        set_halign: gtk::Align::Start,
                    },

                    gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_valign: gtk::Align::Center,

                        gtk::Box {
                            set_spacing: 10,

                            gtk::Button {
                                set_icon_name: "process-completed-symbolic",
                                connect_clicked => PlaylistElementIn::DeletePressed,
                                set_tooltip_text: Some("Confirm deletion of playlist"),
                            },
                            gtk::Button {
                                set_icon_name: "process-stop",
                                connect_clicked => PlaylistElementIn::ChangeState(State::Normal),
                                set_tooltip_text: Some("Don't delete playlist"),
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
                sender
                    .output(PlaylistElementOut::Delete(self.index.clone()))
                    .expect("sending failed");
            }
            PlaylistElementIn::ChangeState(state) => {
                self.main_stack.set_visible_child_name(state.to_str())
            }
            PlaylistElementIn::ConfirmRename => {
                let text = self.edit_entry.text();
                self.playlist.base.name = String::from(text);
                sender.input(PlaylistElementIn::ChangeState(State::Normal));
                //TODO rename playlist on server
            }
        }
    }
}
