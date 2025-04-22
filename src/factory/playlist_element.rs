use std::{cell::RefCell, rc::Rc};

use gettextrs::gettext;
use relm4::{
    gtk::{
        self, gdk, glib, pango,
        prelude::{BoxExt, ButtonExt, EditableExt, OrientableExt, ToValue, WidgetExt},
    },
    RelmWidgetExt,
};

use crate::{
    css,
    factory::{playlist_row::PlaylistUids, queue_song_row::QueueUids},
    gtk_helper::stack::StackExt,
    settings::Settings,
    subsonic::Subsonic,
    types::Droppable,
};

#[derive(Debug, PartialEq, Eq, Clone)]
enum DragState {
    Ready,
    Entered,
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
    NotActive,
    Active,
}

impl std::fmt::Display for EditState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotActive => write!(f, "NotActive"),
            Self::Active => write!(f, "Active"),
        }
    }
}

impl TryFrom<String> for EditState {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.as_ref() {
            "NotActive" => Ok(Self::NotActive),
            "Active" => Ok(Self::Active),
            e => Err(format!("\"{e}\" is not a EditState")),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, glib::Boxed)]
#[boxed_type(name = "PlaylistElementDragged")]
pub struct PlaylistElementDragged(pub Box<submarine::data::PlaylistWithSongs>);

#[derive(Debug)]
pub struct PlaylistElement {
    subsonic: Rc<RefCell<Subsonic>>,
    playlist: submarine::data::PlaylistWithSongs,
    index: relm4::factory::DynamicIndex,
    list_box_row: gtk::ListBoxRow,
    drag_src: gtk::DragSource,
    drag_target_copy: gtk::DropTarget,
    main_stack: gtk::Stack,
    edit_area: gtk::Stack,
    drag_state: Rc<RefCell<DragState>>,
    write_protected: bool,
}

impl PlaylistElement {
    pub fn change_state(&self, state: &State) {
        self.main_stack.set_visible_child_enum(state);
    }

    pub fn info(&self) -> &submarine::data::PlaylistWithSongs {
        &self.playlist
    }

    pub fn set_edit_area(&self, status: EditState) {
        if self.write_protected {
            return;
        }
        self.edit_area.set_visible_child_enum(&status);
    }

    pub fn index(&self) -> &relm4::factory::DynamicIndex {
        &self.index
    }

    pub fn write_protected(&self) -> bool {
        self.write_protected
    }
}

#[derive(Debug, Clone)]
pub enum PlaylistElementIn {
    DeletePressed,
    ChangeState(State),
    ConfirmRename,
    UpdatePlaylistName(submarine::data::Playlist),
    UpdatePlaylistSongs(submarine::data::PlaylistWithSongs),
    Clicked,
    DragEnter,
    DragLeave,
    DropAppend(Droppable),
    ShowIndicatorUpdate(f64),
    ShowIndicatorReset,
    MoveDropped(PlaylistElementDragged, f64),
}

#[derive(Debug)]
pub enum PlaylistElementOut {
    Delete(relm4::factory::DynamicIndex),
    DisplayToast(String),
    RenamePlaylist(submarine::data::Playlist),
    Clicked(relm4::factory::DynamicIndex),
    DropAppend(Droppable, submarine::data::PlaylistWithSongs),
    MoveDropAbove(PlaylistElementDragged, relm4::factory::DynamicIndex),
    MoveDropBelow(PlaylistElementDragged, relm4::factory::DynamicIndex),
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
        sender: relm4::FactorySender<Self>,
    ) -> Self {
        let model = Self {
            write_protected: Subsonic::is_smart_playlist(&playlist.base),
            subsonic,
            playlist,
            index: index.clone(),
            list_box_row: gtk::ListBoxRow::default(),
            drag_src: gtk::DragSource::default(),
            drag_target_copy: gtk::DropTarget::default(),
            main_stack: gtk::Stack::default(),
            edit_area: gtk::Stack::default(),
            drag_state: Rc::new(RefCell::new(DragState::Ready)),
        };

        //setup content for DropSource
        let drop = PlaylistElementDragged(Box::new(model.playlist.clone()));
        let content = gtk::gdk::ContentProvider::for_value(&drop.to_value());
        model.drag_src.set_content(Some(&content));
        model
            .drag_src
            .set_actions(gtk::gdk::DragAction::COPY | gtk::gdk::DragAction::MOVE);
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

        // only activate dropping target when not write protected
        if !model.write_protected {
            model.drag_target_copy.set_types(&[
                <Droppable as gtk::prelude::StaticType>::static_type(),
                <PlaylistUids as gtk::prelude::StaticType>::static_type(),
                <QueueUids as gtk::prelude::StaticType>::static_type(),
            ]);
        }

        sender.input(PlaylistElementIn::ShowIndicatorReset);
        model
    }

    view! {
        self.list_box_row.clone() -> gtk::ListBoxRow {
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

                        append: list_name = &gtk::Label {
                            add_css_class: granite::STYLE_CLASS_H3_LABEL,
                            set_halign: gtk::Align::Start,
                            set_text: &self.playlist.base.name,
                        },
                        append: song_number = &gtk::Label {
                            set_halign: gtk::Align::Start,
                            set_text: &format!("{} {}", self.playlist.base.song_count, gettext("songs")),
                        }
                    },

                    gtk::Box {
                        self.edit_area.clone() -> gtk::Stack {
                            add_enumed[EditState::NotActive] = &gtk::Box {
                                set_halign: gtk::Align::End,

                                if self.write_protected {
                                    gtk::Image {
                                        set_icon_name: Some("view-pin"),
                                        set_tooltip: &gettext("Playlist is unchangeable"),
                                    }
                                } else {
                                    gtk::Box {}
                                }
                            },
                            add_enumed[EditState::Active] = &gtk::Box {
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
            },

            add_controller = gtk::DropTarget {
                set_actions: gdk::DragAction::MOVE,
                set_types: &[<PlaylistElementDragged as gtk::prelude::StaticType>::static_type()],

                connect_motion[sender] => move |_controller, _x, y| {
                    sender.input(PlaylistElementIn::ShowIndicatorUpdate(y));
                    gdk::DragAction::MOVE
                },

                connect_leave[sender] => move |_controller| {
                    sender.input(PlaylistElementIn::ShowIndicatorReset);
                },

                connect_drop[sender] => move |_controller, value, _x, y| {
                    sender.input(PlaylistElementIn::ShowIndicatorReset);

                    if let Ok(drop) = value.get::<PlaylistElementDragged>() {
                        sender.input(PlaylistElementIn::MoveDropped(drop, y));
                        true
                    } else {
                        false
                    }
                }
            },

            add_controller = self.drag_target_copy.clone() -> gtk::DropTarget {
                set_actions: gdk::DragAction::COPY,

                connect_enter[sender] => move |_controller, _x, _y| {
                    sender.input(PlaylistElementIn::DragEnter);
                    gdk::DragAction::COPY
                },

                connect_leave[sender] => move |_controller| {
                    sender.input(PlaylistElementIn::DragLeave);
                },

                connect_drop[sender] => move |_controller, value, _x, _y| {
                    sender.input(PlaylistElementIn::DragLeave);

                    if let Ok(drop) = value.get::<QueueUids>() {
                        let drop = Droppable::QueueSongs(drop.0);
                        sender.input(PlaylistElementIn::DropAppend(drop));
                        true
                    } else if let Ok(drop) = value.get::<PlaylistUids>() {
                        let drop = Droppable::PlaylistItems(drop.0);
                        sender.input(PlaylistElementIn::DropAppend(drop));
                        true
                    } else if let Ok(drop) = value.get::<Droppable>() {
                        sender.input(PlaylistElementIn::DropAppend(drop));
                        true
                    } else {
                        false
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
                if self.write_protected {
                    return;
                }
                sender
                    .output(PlaylistElementOut::Delete(self.index.clone()))
                    .unwrap();
            }
            PlaylistElementIn::ChangeState(state) => self.main_stack.set_visible_child_enum(&state),
            PlaylistElementIn::ConfirmRename => {
                sender.input(PlaylistElementIn::ChangeState(State::Normal));
                let text = widgets.edit_entry.text();
                let mut list = self.playlist.base.clone();
                list.name = text.to_string();
                // inform other widgets
                sender
                    .output(PlaylistElementOut::RenamePlaylist(list))
                    .unwrap();
            }
            PlaylistElementIn::UpdatePlaylistName(list) => {
                if self.playlist.base.id == list.id {
                    widgets.list_name.set_text(&list.name);
                    widgets.edit_entry.set_text(&list.name);
                    self.playlist.base.name = list.name;
                }
            }
            PlaylistElementIn::UpdatePlaylistSongs(list) => {
                if self.playlist.base.id == list.base.id {
                    self.playlist = list;
                    widgets.song_number.set_text(&format!(
                        "{} {}",
                        self.playlist.base.song_count,
                        gettext("songs")
                    ));
                }
            }
            PlaylistElementIn::Clicked => {
                sender
                    .output(PlaylistElementOut::Clicked(self.index.clone()))
                    .unwrap();
            }
            PlaylistElementIn::DragEnter => {
                self.drag_state.replace(DragState::Entered);
                let state = self.drag_state.clone();
                let sender = sender.clone();
                gtk::glib::spawn_future_local(async move {
                    let timeout = Settings::get().lock().unwrap().drag_time_timeout_ms;
                    tokio::time::sleep(std::time::Duration::from_millis(timeout)).await;
                    if *state.borrow() == DragState::Entered {
                        sender.input(PlaylistElementIn::Clicked);
                    }
                });
            }
            PlaylistElementIn::DragLeave => {
                self.drag_state.replace(DragState::Ready);
            }
            PlaylistElementIn::DropAppend(drop) => {
                if self.write_protected {
                    return;
                }

                sender
                    .output(PlaylistElementOut::DropAppend(drop, self.playlist.clone()))
                    .unwrap();
            }
            PlaylistElementIn::ShowIndicatorReset => {
                css::DragState::reset(&self.list_box_row);
            }
            PlaylistElementIn::ShowIndicatorUpdate(y) => {
                let height = self.list_box_row.height();
                if y < height as f64 * 0.5 {
                    css::DragState::drop_shadow_top(&self.list_box_row);
                } else {
                    css::DragState::drop_shadow_bottom(&self.list_box_row);
                }
            }
            PlaylistElementIn::MoveDropped(drop, y) => {
                let height = self.list_box_row.height();
                if y < height as f64 * 0.5 {
                    sender
                        .output(PlaylistElementOut::MoveDropAbove(drop, self.index.clone()))
                        .unwrap();
                } else {
                    sender
                        .output(PlaylistElementOut::MoveDropBelow(drop, self.index.clone()))
                        .unwrap();
                }
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
        test_self(EditState::Active);
        test_self(EditState::NotActive);
    }
}
