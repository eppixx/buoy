use std::{cell::RefCell, rc::Rc};

use fuzzy_matcher::FuzzyMatcher;
use gettextrs::gettext;
use relm4::gtk::glib::prelude::ToValue;
use relm4::RelmWidgetExt;
use relm4::{
    gtk::{
        self,
        prelude::{
            BoxExt, ButtonExt, ListBoxRowExt, ListModelExt, OrientableExt, SelectionModelExt,
            WidgetExt,
        },
    },
    ComponentController,
};

use crate::factory::playlist_row::{
    AlbumColumn, ArtistColumn, FavColumn, LengthColumn, PlaylistRow, TitleColumn,
};
use crate::settings::Settings;
use crate::{
    common::convert_for_label,
    components::{
        cover::{Cover, CoverIn, CoverOut},
        playlist_element::{PlaylistElement, PlaylistElementOut, State},
    },
    subsonic::Subsonic,
    types::Droppable,
};

#[derive(Debug, Clone, PartialEq, Eq)]
enum TracksState {
    Tracks,
    Stock,
}

impl std::fmt::Display for TracksState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Tracks => write!(f, "Tracks"),
            Self::Stock => write!(f, "Stock"),
        }
    }
}

impl TryFrom<String> for TracksState {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.as_ref() {
            "Tracks" => Ok(Self::Tracks),
            "Stock" => Ok(Self::Stock),
            e => Err(format!("\"{e}\" is not a State")),
        }
    }
}

#[derive(Debug)]
pub struct PlaylistsView {
    subsonic: Rc<RefCell<Subsonic>>,
    playlists: relm4::factory::FactoryVecDeque<PlaylistElement>,

    tracks: relm4::typed_view::column::TypedColumnView<PlaylistRow, gtk::MultiSelection>,
    info_cover: relm4::Controller<Cover>,
    info_cover_controller: gtk::DragSource,
}

#[derive(Debug)]
pub enum PlaylistsViewOut {
    ReplaceQueue(submarine::data::PlaylistWithSongs),
    AddToQueue(submarine::data::PlaylistWithSongs),
    AppendToQueue(submarine::data::PlaylistWithSongs),
    DeletePlaylist(
        relm4::factory::DynamicIndex,
        submarine::data::PlaylistWithSongs,
    ),
    CreatePlaylist,
    RenamePlaylist(submarine::data::Playlist),
    DisplayToast(String),
    Download(Droppable),
    FavoriteClicked(String, bool),
    ClickedArtist(String),
    ClickedAlbum(String),
}

#[derive(Debug)]
pub enum PlaylistsViewIn {
    SearchChanged(String),
    ReplaceQueue,
    AddToQueue,
    AppendToQueue,
    PlaylistElement(PlaylistElementOut),
    Cover(CoverOut),
    NewPlaylist(submarine::data::PlaylistWithSongs),
    DeletePlaylist(relm4::factory::DynamicIndex),
    Favorited(String, bool),
    DownloadClicked,
    Selected(i32),
    RecalcDragSource,
}

#[relm4::component(pub)]
impl relm4::Component for PlaylistsView {
    type Init = Rc<RefCell<Subsonic>>;
    type Input = PlaylistsViewIn;
    type Output = PlaylistsViewOut;
    type CommandOutput = ();

    fn init(
        subsonic: Self::Init,
        root: Self::Root,
        sender: relm4::ComponentSender<Self>,
    ) -> relm4::ComponentParts<Self> {
        let mut tracks =
            relm4::typed_view::column::TypedColumnView::<PlaylistRow, gtk::MultiSelection>::new();
        tracks.append_column::<TitleColumn>();
        tracks.append_column::<ArtistColumn>();
        tracks.append_column::<AlbumColumn>();
        tracks.append_column::<LengthColumn>();
        tracks.append_column::<FavColumn>();

        let mut model = PlaylistsView {
            subsonic: subsonic.clone(),
            playlists: relm4::factory::FactoryVecDeque::builder()
                .launch(gtk::ListBox::default())
                .forward(sender.input_sender(), PlaylistsViewIn::PlaylistElement),

            tracks,
            info_cover: Cover::builder()
                .launch((subsonic, None))
                .forward(sender.input_sender(), PlaylistsViewIn::Cover),
            info_cover_controller: gtk::DragSource::default(),
        };

        let widgets = view_output!();
        model.info_cover.model().add_css_class_image("size100");

        model
            .info_cover
            .widget()
            .add_controller(model.info_cover_controller.clone());

        // add playlists to list
        let mut guard = model.playlists.guard();
        for playlist in model.subsonic.borrow().playlists() {
            guard.push_back((model.subsonic.clone(), playlist.clone()));
        }
        drop(guard);

        // send signal on selection change
        model
            .tracks
            .selection_model
            .connect_selection_changed(move |_selection_model, _x, _y| {
                sender.input(PlaylistsViewIn::RecalcDragSource);
            });

        relm4::ComponentParts { model, widgets }
    }

    view! {
        gtk::Box {
            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                add_css_class: "playlist-view-info",
                set_spacing: 7,

                gtk::WindowHandle {
                    gtk::Label {
                        add_css_class: granite::STYLE_CLASS_H2_LABEL,
                        set_label: &gettext("Playlists"),
                    }
                },

                model.playlists.widget().clone() -> gtk::ListBox {
                    add_css_class: "playlist-view-playlist-list",
                    add_css_class: granite::STYLE_CLASS_FRAME,
                    add_css_class: granite::STYLE_CLASS_RICH_LIST,
                    set_vexpand: true,

                    connect_row_selected[sender] => move |_listbox, row| {
                        if let Some(row) = row {
                            sender.input(PlaylistsViewIn::Selected(row.index()));
                        }
                    },

                    gtk::ListBoxRow {
                        add_css_class: "playlist-view-add-playlist",

                        gtk::Button {
                            gtk::Box {
                                set_halign: gtk::Align::Center,

                                gtk::Image {
                                    set_icon_name: Some("list-add-symbolic"),
                                },
                                gtk::Label {
                                    set_text: &gettext("New Playlist"),
                                }
                            },

                            connect_clicked[sender] => move |_btn| {
                                sender.output(PlaylistsViewOut::CreatePlaylist).unwrap();
                            }
                        }
                    }
                }
            },

            gtk::Box {
                append: track_stack = &gtk::Stack {
                    add_named[Some("tracks-stock")] = &gtk::Box {
                        gtk::Label {
                            add_css_class: granite::STYLE_CLASS_H2_LABEL,
                            set_hexpand: true,

                            set_label: &gettext("Select a playlist to show its songs"),
                        }
                    },
                    add_named[Some("tracks")] = &gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_spacing: 8,

                        // top
                        gtk::Box {
                            add_css_class: "playlist-view-info",
                            set_spacing: 15,

                            model.info_cover.widget().clone() -> gtk::Box {},

                            // playlist info
                            gtk::WindowHandle {
                                set_hexpand: true,

                                gtk::Box {
                                    set_orientation: gtk::Orientation::Vertical,
                                    set_spacing: 8,

                                    append: info_title = &gtk::Label {
                                        add_css_class: granite::STYLE_CLASS_H2_LABEL,
                                        set_label: "title", // overwritten later
                                        set_halign: gtk::Align::Start,
                                    },

                                    append: info_details = &gtk::Label {
                                        set_label: "more info", // overwritten later
                                        set_halign: gtk::Align::Start,
                                    },

                                    gtk::Box {
                                        set_spacing: 15,
                                        gtk::Button {
                                            gtk::Box {
                                                gtk::Image {
                                                    set_icon_name: Some("list-add-symbolic"),
                                                },
                                                gtk::Label {
                                                    set_label: &gettext("Append"),
                                                }
                                            },
                                            set_tooltip: &gettext("Append playlist to end of queue"),
                                            connect_clicked => PlaylistsViewIn::AppendToQueue,
                                        },
                                        gtk::Button {
                                            gtk::Box {
                                                gtk::Image {
                                                    set_icon_name: Some("list-add-symbolic"),
                                                },
                                                gtk::Label {
                                                    set_label: &gettext("Play next"),
                                                }
                                            },
                                            set_tooltip: &gettext("Insert playlist after currently played or paused item"),
                                            connect_clicked => PlaylistsViewIn::AddToQueue,
                                        },
                                        gtk::Button {
                                            gtk::Box {
                                                gtk::Image {
                                                    set_icon_name: Some("emblem-symbolic-link-symbolic"),
                                                },
                                                gtk::Label {
                                                    set_label: &gettext("Replace queue"),
                                                }
                                            },
                                            set_tooltip: &gettext("Replaces current queue with this playlist"),
                                            connect_clicked => PlaylistsViewIn::ReplaceQueue,
                                        },
                                        gtk::Button {
                                            gtk::Box {
                                                gtk::Image {
                                                    set_icon_name: Some("browser-download-symbolic"),
                                                },
                                                gtk::Label {
                                                    set_label: &gettext("Download Playlist"),
                                                }
                                            },
                                            set_tooltip: &gettext("Click to select a folder to download this album to"),
                                            connect_clicked => PlaylistsViewIn::DownloadClicked,
                                        }
                                    }
                                }
                            }
                        },

                        //bottom
                        gtk::ScrolledWindow {
                            set_hexpand: true,
                            set_vexpand: true,

                            model.tracks.view.clone() -> gtk::ColumnView {
                                add_css_class: "playlist-view-tracks-row",

                                add_controller = gtk::DragSource {
                                    connect_prepare[sender] => move |_drag_src, _x, _y| {
                                        sender.input(PlaylistsViewIn::RecalcDragSource);
                                        None
                                    }
                                }
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
        sender: relm4::ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match msg {
            PlaylistsViewIn::SearchChanged(search) => {
                self.tracks.clear_filters();
                self.tracks.add_filter(move |row| {
                    let mut search = search.clone();
                    let mut test = format!(
                        "{} {} {}",
                        row.item.title,
                        row.item.artist.as_deref().unwrap_or_default(),
                        row.item.album.as_deref().unwrap_or_default()
                    );

                    //check for case sensitivity
                    if !Settings::get().lock().unwrap().case_sensitive {
                        test = test.to_lowercase();
                        search = search.to_lowercase();
                    }

                    //actual matching
                    let fuzzy_search = Settings::get().lock().unwrap().fuzzy_search;
                    if fuzzy_search {
                        let matcher = fuzzy_matcher::skim::SkimMatcherV2::default();
                        let score = matcher.fuzzy_match(&test, &search);
                        score.is_some()
                    } else {
                        test.contains(&search)
                    }
                });
            }
            PlaylistsViewIn::PlaylistElement(msg) => match msg {
                PlaylistElementOut::DisplayToast(msg) => {
                    sender.output(PlaylistsViewOut::DisplayToast(msg)).unwrap();
                }
                PlaylistElementOut::Delete(index) => {
                    let list = match self.playlists.get(index.current_index()) {
                        None => {
                            sender
                                .output(PlaylistsViewOut::DisplayToast(String::from(
                                    "index does not point to a playlist",
                                )))
                                .unwrap();
                            return;
                        }
                        Some(list) => list,
                    };
                    sender
                        .output(PlaylistsViewOut::DeletePlaylist(index, list.info().clone()))
                        .unwrap();
                }
                PlaylistElementOut::RenamePlaylist(list) => {
                    widgets.info_title.set_label(&list.name);
                    sender
                        .output(PlaylistsViewOut::RenamePlaylist(list))
                        .unwrap();
                }
            },
            PlaylistsViewIn::Cover(msg) => match msg {
                CoverOut::DisplayToast(title) => sender
                    .output(PlaylistsViewOut::DisplayToast(title))
                    .unwrap(),
            },
            PlaylistsViewIn::ReplaceQueue => {
                let Some(row) = self.playlists.widget().selected_row() else {
                    unreachable!("replace should not be possible when no playlists selected");
                };
                if let Some(element) = self.playlists.get(row.index() as usize) {
                    sender
                        .output(PlaylistsViewOut::ReplaceQueue(element.info().clone()))
                        .unwrap();
                }
            }
            PlaylistsViewIn::AddToQueue => {
                let Some(row) = self.playlists.widget().selected_row() else {
                    unreachable!("add to queue should not be possible when no playlists selected");
                };
                if let Some(element) = self.playlists.get(row.index() as usize) {
                    sender
                        .output(PlaylistsViewOut::AddToQueue(element.info().clone()))
                        .unwrap();
                }
            }
            PlaylistsViewIn::AppendToQueue => {
                let Some(row) = self.playlists.widget().selected_row() else {
                    unreachable!("add to queue should not be possible when no playlists selected");
                };
                if let Some(element) = self.playlists.get(row.index() as usize) {
                    sender
                        .output(PlaylistsViewOut::AppendToQueue(element.info().clone()))
                        .unwrap();
                }
            }
            PlaylistsViewIn::NewPlaylist(list) => {
                //show new playlist
                self.playlists
                    .guard()
                    .push_back((self.subsonic.clone(), list));
            }
            PlaylistsViewIn::DeletePlaylist(index) => {
                widgets.track_stack.set_visible_child_name("tracks-stock");
                self.playlists.guard().remove(index.current_index());
            }
            PlaylistsViewIn::Favorited(id, state) => {
                (0..self.tracks.len())
                    .filter_map(|i| self.tracks.get(i))
                    .filter(|t| t.borrow().item.id == id)
                    .for_each(|track| match state {
                        true => {
                            track.borrow_mut().item.starred =
                                Some(chrono::offset::Local::now().into());
                        }
                        false => track.borrow_mut().item.starred = None,
                    });
            }
            PlaylistsViewIn::DownloadClicked => {
                let Some(row) = self.playlists.widget().selected_row() else {
                    unreachable!("add to queue should not be possible when no playlists selected");
                };
                if let Some(element) = self.playlists.get(row.index() as usize) {
                    let drop = Droppable::Playlist(Box::new(element.info().clone()));
                    sender.output(PlaylistsViewOut::Download(drop)).unwrap();
                }
            }
            PlaylistsViewIn::Selected(index) => {
                // set every state in PlaylistElement to normal
                for list in self.playlists.guard().iter() {
                    list.change_state(&State::Normal);
                    list.set_edit_area(false);
                }

                widgets.track_stack.set_visible_child_name("tracks");

                let guard = self.playlists.guard();
                let Some(list) = guard.get(index as usize) else {
                    tracing::error!("index has no playlist");
                    return;
                };
                list.set_edit_area(true);
                let list = list.info();

                // set info
                self.info_cover
                    .emit(CoverIn::LoadPlaylist(Box::new(list.clone())));
                widgets.info_title.set_text(&list.base.name);
                widgets.info_details.set_text(&build_info_string(list));

                //update drag controller for cover
                let drop = Droppable::Playlist(Box::new(list.clone()));
                let content = gtk::gdk::ContentProvider::for_value(&drop.to_value());
                self.info_cover_controller.set_content(Some(&content));
                self.info_cover_controller
                    .set_actions(gtk::gdk::DragAction::COPY);
                let playlist_id = list.base.cover_art.clone();
                let subsonic = self.subsonic.clone();
                self.info_cover_controller
                    .connect_drag_begin(move |src, _drag| {
                        if let Some(playlist_id) = &playlist_id {
                            let cover = subsonic.borrow().cover_icon(playlist_id);
                            if let Some(tex) = cover {
                                src.set_icon(Some(&tex), 0, 0);
                            }
                        }
                    });

                //set tracks
                self.tracks.clear();
                for track in &list.entry {
                    self.tracks.append(PlaylistRow::new(
                        &self.subsonic,
                        track.clone(),
                        sender.clone(),
                    ));
                }
            }
            PlaylistsViewIn::RecalcDragSource => {
                let len = self.tracks.selection_model.n_items();
                let selected_rows: Vec<u32> = (0..len)
                    .filter(|i| self.tracks.view.model().unwrap().is_selected(*i))
                    .collect();

                // remove DragSource of not selected items
                (0..len)
                    .filter(|i| !selected_rows.contains(i))
                    .filter_map(|i| self.tracks.get(i))
                    .for_each(|row| row.borrow_mut().remove_drag_src());

                // get selected children
                let children: Vec<submarine::data::Child> = selected_rows
                    .iter()
                    .filter_map(|i| self.tracks.get(*i))
                    .map(|row| row.borrow().item.clone())
                    .collect();

                // set children as content for DragSource
                let drop = Droppable::Queue(children);
                selected_rows
                    .iter()
                    .filter_map(|i| self.tracks.get(*i))
                    .for_each(|row| row.borrow_mut().set_drag_src(drop.clone()));
            }
        }
    }
}

fn build_info_string(list: &submarine::data::PlaylistWithSongs) -> String {
    let created = list
        .base
        .created
        .format(&gettext("Created at: %d.%m.%Y, %H:%M"))
        .to_string();
    format!(
        "{}: {} • {}: {} • {}",
        gettext("Songs"),
        list.base.song_count,
        gettext("Length"),
        convert_for_label(i64::from(list.base.duration) * 1000),
        created
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gtk_helper::stack::test_self;

    #[test]
    fn track_state_conversion() {
        test_self(TracksState::Tracks);
        test_self(TracksState::Stock);
    }
}
