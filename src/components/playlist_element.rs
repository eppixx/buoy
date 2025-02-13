use std::{cell::RefCell, rc::Rc};

use gettextrs::gettext;
use relm4::{
    gtk::{
        self,
        prelude::{BoxExt, ButtonExt, EditableExt, OrientableExt, ToValue, WidgetExt},
    },
    RelmWidgetExt,
};

use crate::{gtk_helper::stack::StackExt, subsonic::Subsonic, types::Droppable};

#[derive(Debug)]
pub struct PlaylistElement {
    subsonic: Rc<RefCell<Subsonic>>,
    playlist: submarine::data::PlaylistWithSongs,
    index: relm4::factory::DynamicIndex,
    drag_src: gtk::DragSource,
    main_stack: gtk::Stack,
    edit_area: gtk::Stack,
}

impl PlaylistElement {
    pub fn change_state(&self, state: &State) {
        self.main_stack.set_visible_child_enum(state);
    }

    pub fn info(&self) -> &submarine::data::PlaylistWithSongs {
        &self.playlist
    }

    pub fn set_edit_area(&self, status: bool) {
        if status {
            self.edit_area.set_visible_child_enum(&EditState::Edit);
        } else {
            self.edit_area.set_visible_child_enum(&EditState::Clean);
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum State {
    DeleteInProgress,
    Edit,
    Normal,
}

impl std::fmt::Display for State {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DeleteInProgress => write!(f, "Delete dialog"),
            Self::Edit => write!(f, "Editing"),
            Self::Normal => write!(f, "Normal"),
        }
    }
}

impl TryFrom<String> for State {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.as_ref() {
            "Delete dialog" => Ok(Self::DeleteInProgress),
            "Editing" => Ok(Self::Edit),
            "Normal" => Ok(Self::Normal),
            e => Err(format!("\"{e}\" is not a State")),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum EditState {
    Clean,
    Edit,
}

impl std::fmt::Display for EditState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Clean => write!(f, "Clean"),
            Self::Edit => write!(f, "Edit"),
        }
    }
}

impl TryFrom<String> for EditState {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.as_ref() {
            "Clean" => Ok(Self::Clean),
            "Edit" => Ok(Self::Edit),
            e => Err(format!("\"{e}\" is not a State")),
        }
    }
}

#[derive(Debug)]
pub enum PlaylistElementIn {
    DeletePressed,
    ChangeState(State),
    ConfirmRename,
}

#[derive(Debug)]
pub enum PlaylistElementOut {
    Delete(relm4::factory::DynamicIndex),
    DisplayToast(String),
    RenamePlaylist(submarine::data::Playlist),
}

#[relm4::factory(pub)]
impl relm4::factory::FactoryComponent for PlaylistElement {
    type Init = (Rc<RefCell<Subsonic>>, submarine::data::PlaylistWithSongs);
    type Input = PlaylistElementIn;
    type Output = PlaylistElementOut;
    type ParentWidget = gtk::ListBox;
    type CommandOutput = ();

    fn init_model(
        (subsonic, playlist): Self::Init,
        index: &relm4::factory::DynamicIndex,
        _sender: relm4::FactorySender<Self>,
    ) -> Self {
        let model = Self {
            subsonic,
            playlist,
            index: index.clone(),
            drag_src: gtk::DragSource::default(),
            main_stack: gtk::Stack::default(),
            edit_area: gtk::Stack::default(),
        };

        //setup content for DropSource
        let drop = Droppable::Playlist(Box::new(model.playlist.clone()));
        let content = gtk::gdk::ContentProvider::for_value(&drop.to_value());
        model.drag_src.set_content(Some(&content));
        model.drag_src.set_actions(gtk::gdk::DragAction::COPY);
        let cover_art = model.playlist.base.cover_art.clone();
        let subsonic = model.subsonic.clone();
        model.drag_src.connect_drag_begin(move |src, _drag| {
            if let Some(art) = &cover_art {
                let cover = subsonic.borrow().cover_icon(art);
                if let Some(tex) = cover {
                    src.set_icon(Some(&tex), 0, 0);
                }
            }
        });

        model
    }

    view! {
        gtk::ListBoxRow {
            set_widget_name: "playlist-element",

            self.main_stack.clone() -> gtk::Stack {
                add_enumed[State::Normal] = &gtk::Box {
                    add_css_class: "flat",
                    set_spacing: 5,
                    set_halign: gtk::Align::Fill,

                    add_controller = self.drag_src.clone(),

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
                            set_text: &format!("{} {}", self.playlist.base.song_count, gettext("songs")),
                        }
                    },

                    gtk::Box {
                        self.edit_area.clone() -> gtk::Stack {
                            add_enumed[EditState::Clean] = &gtk::Box {},
                            add_enumed[EditState::Edit] = &gtk::Box {
                                set_orientation: gtk::Orientation::Vertical,
                                set_valign: gtk::Align::Center,

                                gtk::Box {
                                    set_spacing: 10,

                                    gtk::Button {
                                        set_icon_name: "edit-symbolic",
                                        set_tooltip: &gettext("Rename Playlist"),
                                        connect_clicked => PlaylistElementIn::ChangeState(State::Edit),
                                    },
                                    gtk::Button {
                                        add_css_class: granite::STYLE_CLASS_DESTRUCTIVE_ACTION,
                                        set_icon_name: "edit-delete-symbolic",
                                        set_tooltip: &gettext("Delete Playlist"),

                                        connect_clicked => PlaylistElementIn::ChangeState(State::DeleteInProgress),
                                    }
                                }
                            }
                        }
                    }
                },
                add_enumed[State::Edit] = &gtk::Box {
                    set_spacing: 10,

                    gtk::Box {
                        set_valign: gtk::Align::Center,

                        append: edit_entry = &gtk::Entry {
                            set_hexpand: true,
                            set_halign: gtk::Align::Fill,

                            #[watch]
                            set_text: &self.playlist.base.name,
                            set_tooltip: &gettext("Renamed title of the playlist"),
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
                                set_tooltip: &gettext("Confirm renaming the playlist"),
                            },
                            gtk::Button {
                                set_icon_name: "process-stop",
                                connect_clicked => PlaylistElementIn::ChangeState(State::Normal),
                                set_tooltip: &gettext("Don't change the playlist name"),
                            }
                        }
                    }
                },
                add_enumed[State::DeleteInProgress] = &gtk::Box {
                    set_spacing: 10,

                    gtk::Label {
                        set_hexpand: true,
                        set_label: &format!("{} \"{}\"?", gettext("Realy delete"), self.playlist.base.name),
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
                                set_tooltip: &gettext("Confirm deletion of the playlist"),
                            },
                            gtk::Button {
                                set_icon_name: "process-stop",
                                connect_clicked => PlaylistElementIn::ChangeState(State::Normal),
                                set_tooltip: &gettext("Don't delete the playlist"),
                            }
                        }
                    }
                }
            }
        }
    }

    fn update_with_view(
        &mut self,
        widgets: &mut Self::Widgets,
        msg: Self::Input,
        sender: relm4::FactorySender<Self>,
    ) {
        match msg {
            PlaylistElementIn::DeletePressed => {
                sender
                    .output(PlaylistElementOut::Delete(self.index.clone()))
                    .unwrap();
            }
            PlaylistElementIn::ChangeState(state) => self.main_stack.set_visible_child_enum(&state),
            PlaylistElementIn::ConfirmRename => {
                let text = widgets.edit_entry.text();
                self.playlist.base.name = String::from(text);
                sender.input(PlaylistElementIn::ChangeState(State::Normal));
                sender
                    .output(PlaylistElementOut::RenamePlaylist(
                        self.playlist.base.clone(),
                    ))
                    .unwrap();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gtk_helper::stack::test_self;

    #[test]
    fn test_state_conversion() {
        test_self(State::Normal);
        test_self(State::Edit);
        test_self(State::DeleteInProgress);
    }

    #[test]
    fn test_edit_state_conversion() {
        test_self(EditState::Edit);
        test_self(EditState::Clean);
    }
}
