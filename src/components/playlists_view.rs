use std::{cell::RefCell, rc::Rc};

use fuzzy_matcher::FuzzyMatcher;
use relm4::gtk::glib::prelude::ToValue;
use relm4::{
    gtk::{
        self,
        prelude::{BoxExt, ButtonExt, ListModelExt, OrientableExt, WidgetExt},
    },
    Component, ComponentController,
};

use crate::factory::track_row::{
    AlbumColumn, ArtistColumn, FavColumn, LengthColumn, TitleColumn, TrackRow,
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
    index_shown: Option<relm4::factory::DynamicIndex>,

    track_stack: gtk::Stack,
    tracks: relm4::typed_view::column::TypedColumnView<TrackRow, gtk::SingleSelection>,
    info_cover: relm4::Controller<Cover>,
    info_cover_controller: gtk::DragSource,
    info_title: gtk::Label,
    info_details: gtk::Label,
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
    DisplayToast(String),
    Download(Droppable),
    FavoriteClicked(String, bool),
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
}

#[relm4::component(pub)]
impl relm4::SimpleComponent for PlaylistsView {
    type Init = Rc<RefCell<Subsonic>>;
    type Input = PlaylistsViewIn;
    type Output = PlaylistsViewOut;

    fn init(
        subsonic: Self::Init,
        root: Self::Root,
        sender: relm4::ComponentSender<Self>,
    ) -> relm4::ComponentParts<Self> {
        let mut tracks =
            relm4::typed_view::column::TypedColumnView::<TrackRow, gtk::SingleSelection>::new();
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
            index_shown: None,

            track_stack: gtk::Stack::default(),
            tracks,
            info_cover: Cover::builder()
                .launch((subsonic, None))
                .forward(sender.input_sender(), PlaylistsViewIn::Cover),
            info_cover_controller: gtk::DragSource::default(),
            info_title: gtk::Label::default(),
            info_details: gtk::Label::default(),
        };

        let track_stack = &model.track_stack.clone();
        let column = &model.tracks.view;
        let info_cover = model.info_cover.widget().clone();
        let info_title = model.info_title.clone();
        let info_details = model.info_details.clone();
        let widgets = view_output!();
        model.info_cover.model().add_css_class_image("size100");

        model
            .info_cover
            .widget()
            .add_controller(model.info_cover_controller.clone());

        // add playlists to list
        for playlist in model.subsonic.borrow().playlists() {
            model.playlists.guard().push_back(playlist.clone());
        }

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
                        set_label: "Playlists",
                    }
                },

                model.playlists.widget().clone() -> gtk::ListBox {
                    add_css_class: "playlist-view-playlist-list",
                    add_css_class: granite::STYLE_CLASS_FRAME,
                    add_css_class: granite::STYLE_CLASS_RICH_LIST,
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

                            connect_clicked[sender] => move |_btn| {
                                sender.output(PlaylistsViewOut::CreatePlaylist).unwrap();
                            }
                        }
                    }
                }
            },

            gtk::Box {
                #[local_ref]
                track_stack -> gtk::Stack {
                    add_named[Some("tracks-stock")] = &gtk::Box {
                        gtk::Label {
                            add_css_class: granite::STYLE_CLASS_H2_LABEL,
                            set_hexpand: true,

                            set_label: "Select a playlist to show its songs",
                        }
                    },
                    add_named[Some("tracks")] = &gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_spacing: 8,

                        gtk::Box {
                            add_css_class: "playlist-view-info",
                            set_spacing: 15,

                            #[local_ref]
                            info_cover -> gtk::Box {},

                            // playlist info
                            gtk::WindowHandle {
                                set_hexpand: true,

                                gtk::Box {
                                    set_orientation: gtk::Orientation::Vertical,
                                    set_spacing: 8,

                                    #[local_ref]
                                    info_title -> gtk::Label {
                                        add_css_class: granite::STYLE_CLASS_H2_LABEL,
                                        set_label: "title",
                                        set_halign: gtk::Align::Start,
                                    },

                                    #[local_ref]
                                    info_details -> gtk::Label {
                                        set_label: "more info",
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
                                                    set_label: "Append",
                                                }
                                            },
                                            set_tooltip_text: Some("Append Album to end of queue"),
                                            connect_clicked => PlaylistsViewIn::AppendToQueue,
                                        },
                                        gtk::Button {
                                            gtk::Box {
                                                gtk::Image {
                                                    set_icon_name: Some("list-add-symbolic"),
                                                },
                                                gtk::Label {
                                                    set_label: "Play next"
                                                }
                                            },
                                            set_tooltip_text: Some("Insert Album after currently played or paused item"),
                                            connect_clicked => PlaylistsViewIn::AddToQueue,
                                        },
                                        gtk::Button {
                                            gtk::Box {
                                                gtk::Image {
                                                    set_icon_name: Some("emblem-symbolic-link-symbolic"),
                                                },
                                                gtk::Label {
                                                    set_label: "Replace queue",
                                                }
                                            },
                                            set_tooltip_text: Some("Replaces current queue with this playlist"),
                                            connect_clicked => PlaylistsViewIn::ReplaceQueue,
                                        },
                                        gtk::Button {
                                            gtk::Box {
                                                gtk::Image {
                                                    set_icon_name: Some("browser-download-symbolic"),
                                                },
                                                gtk::Label {
                                                    set_label: "Download Playlist",
                                                }
                                            },
                                            set_tooltip_text: Some("Click to select a folder to download this album to"),
                                            connect_clicked => PlaylistsViewIn::DownloadClicked,
                                        }
                                    }
                                }
                            }
                        },

                        gtk::ScrolledWindow {
                            set_hexpand: true,
                            set_vexpand: true,

                            #[local_ref]
                            column -> gtk::ColumnView {
                                add_css_class: "playlist-view-tracks-row",
                            }
                        }
                    }
                }
            }
        }
    }

    fn update(&mut self, msg: Self::Input, sender: relm4::ComponentSender<Self>) {
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
                PlaylistElementOut::Clicked(index, list) => {
                    // set every state in PlaylistElement to normal
                    for list in self.playlists.guard().iter() {
                        list.change_state(&State::Normal);
                    }

                    self.track_stack.set_visible_child_name("tracks");
                    if self.index_shown == Some(index.clone()) {
                        return;
                    }

                    if let Some(i) = &self.index_shown {
                        self.playlists
                            .guard()
                            .get(i.current_index())
                            .unwrap()
                            .set_edit_area(false);
                    }
                    self.playlists
                        .guard()
                        .get(index.current_index())
                        .unwrap()
                        .set_edit_area(true);

                    // set info
                    self.info_cover
                        .emit(CoverIn::LoadPlaylist(Box::new(list.clone())));
                    self.info_title.set_text(&list.base.name);
                    self.info_details.set_text(&build_info_string(&list));

                    //set drag controller
                    let drop = Droppable::Playlist(Box::new(list.clone()));
                    let content = gtk::gdk::ContentProvider::for_value(&drop.to_value());
                    self.info_cover_controller.set_content(Some(&content));
                    self.info_cover_controller
                        .set_actions(gtk::gdk::DragAction::MOVE);

                    //set tracks
                    self.tracks.clear();
                    for track in list.entry {
                        self.tracks.append(TrackRow::new_playlist_track(
                            &self.subsonic,
                            track,
                            sender.clone(),
                        ));
                    }
                    self.index_shown = Some(index);
                }
                PlaylistElementOut::DisplayToast(msg) => {
                    sender.output(PlaylistsViewOut::DisplayToast(msg)).unwrap()
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
            },
            PlaylistsViewIn::Cover(msg) => match msg {
                CoverOut::DisplayToast(title) => sender
                    .output(PlaylistsViewOut::DisplayToast(title))
                    .unwrap(),
            },
            PlaylistsViewIn::ReplaceQueue => {
                if let Some(index) = &self.index_shown {
                    let list = self.playlists.guard()[index.current_index()].info().clone();
                    sender.output(PlaylistsViewOut::ReplaceQueue(list)).unwrap();
                }
            }
            PlaylistsViewIn::AddToQueue => {
                if let Some(index) = &self.index_shown {
                    let list = self.playlists.guard()[index.current_index()].info().clone();
                    sender.output(PlaylistsViewOut::AddToQueue(list)).unwrap();
                }
            }
            PlaylistsViewIn::AppendToQueue => {
                if let Some(index) = &self.index_shown {
                    let list = self.playlists.guard()[index.current_index()].info().clone();
                    sender
                        .output(PlaylistsViewOut::AppendToQueue(list))
                        .unwrap();
                }
            }
            PlaylistsViewIn::NewPlaylist(list) => {
                //show new playlist
                self.playlists.guard().push_back(list);
            }
            PlaylistsViewIn::DeletePlaylist(index) => {
                self.track_stack.set_visible_child_name("tracks-stock");
                self.playlists.guard().remove(index.current_index());
            }
            PlaylistsViewIn::Favorited(id, state) => {
                use relm4::typed_view::TypedListItem;

                let len = self.tracks.len();
                let tracks: Vec<TypedListItem<TrackRow>> =
                    (0..len).filter_map(|i| self.tracks.get(i)).collect();
                for track in tracks {
                    let track_id = track.borrow().item.id.clone();
                    if track_id == id {
                        match state {
                            true => {
                                track
                                    .borrow_mut()
                                    .fav
                                    .set_value(String::from("starred-symbolic"));
                                track.borrow_mut().item.starred =
                                    Some(chrono::offset::Local::now().into());
                            }
                            false => {
                                track
                                    .borrow_mut()
                                    .fav
                                    .set_value(String::from("non-starred-symbolic"));
                                track.borrow_mut().item.starred = None;
                            }
                        }
                    }
                }
            }
            PlaylistsViewIn::DownloadClicked => {
                if let Some(index) = &self.index_shown {
                    let element = self.playlists.get(index.current_index()).unwrap();
                    let drop = Droppable::Playlist(Box::new(element.info().clone()));
                    sender.output(PlaylistsViewOut::Download(drop)).unwrap();
                }
            }
        }
    }
}

fn build_info_string(list: &submarine::data::PlaylistWithSongs) -> String {
    let created = list
        .base
        .created
        .format("Created at: %d.%m.%Y, %H:%M")
        .to_string();
    format!(
        "Songs: {} • Length: {} • {}",
        list.base.song_count,
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
