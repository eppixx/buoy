use std::{cell::RefCell, rc::Rc};

use fuzzy_matcher::FuzzyMatcher;
use gettextrs::gettext;
use relm4::gtk::gdk;
use relm4::gtk::glib::prelude::ToValue;
use relm4::{
    gtk::{
        self,
        prelude::{BoxExt, ButtonExt, ListBoxRowExt, OrientableExt, SelectionModelExt, WidgetExt},
    },
    ComponentController,
};
use relm4::{Component, RelmWidgetExt};

use crate::client::Client;
use crate::factory::playlist_row::{
    AlbumColumn, ArtistColumn, FavColumn, LengthColumn, PlaylistRow, TitleColumn,
};
use crate::settings::Settings;
use crate::types::Id;
use crate::{
    common::convert_for_label,
    components::{
        cover::{Cover, CoverIn, CoverOut},
        playlist_element::{PlaylistElement, PlaylistElementOut, State},
    },
    subsonic::Subsonic,
    types::Droppable,
};

use super::playlist_element::PlaylistElementIn;

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

    selected_playlist: Option<submarine::data::PlaylistWithSongs>,
    tracks: relm4::typed_view::column::TypedColumnView<PlaylistRow, gtk::MultiSelection>,
    info_cover: relm4::Controller<Cover>,
    info_cover_controller: gtk::DragSource,
}

impl PlaylistsView {
    async fn update_playlist(&mut self, sender: &relm4::AsyncComponentSender<Self>) {
        let Some(current_playlist) = &mut self.selected_playlist else {
            return;
        };

        // subsonic does not allow moving songs, so we need to remove songs
        // and then readd them

        //TODO improve efficiency by removing the parts that need removing instead of all

        let client = Client::get().unwrap();
        // fetch playlist and see its length so we can delete every index
        // the fetch is needed when removing, because we dont know how big the list was
        match client.get_playlist(&current_playlist.base.id).await {
            Err(e) => sender
                .output(PlaylistsViewOut::DisplayToast(format!(
                    "fetching playlist failed: {e}",
                )))
                .unwrap(),
            Ok(list) => {
                let temp_delete_indices: Vec<i64> = (0..list.entry.len() as i64).collect();
                if let Err(e) = client
                    .update_playlist(
                        &current_playlist.base.id,
                        None::<String>,
                        None::<String>,
                        None,
                        Vec::<String>::new(),
                        temp_delete_indices,
                    )
                    .await
                {
                    sender
                        .output(PlaylistsViewOut::DisplayToast(format!(
                            "moving playlist entry, removing failed: {e}",
                        )))
                        .unwrap();
                }
            }
        }

        //readd playlist content
        let ids: Vec<String> = (0..self.tracks.len())
            .filter_map(|i| self.tracks.get(i))
            .map(|track| track.borrow().item().id.clone())
            .collect();
        if let Err(e) = client
            .update_playlist(
                &current_playlist.base.id,
                None::<String>,
                None::<String>,
                None,
                ids,
                vec![],
            )
            .await
        {
            sender
                .output(PlaylistsViewOut::DisplayToast(format!(
                    "moving playlist entry, readding failed: {e}",
                )))
                .unwrap();
        }
        let updated_list = match client.get_playlist(&current_playlist.base.id).await {
            Ok(list) => list,
            Err(e) => {
                tracing::error!("updated list not found on server: {e}");
                return;
            }
        };

        // update cache
        self.subsonic.borrow_mut().replace_playlist(&updated_list);

        //sync local cache playlist content
        *current_playlist = updated_list.clone();
        self.playlists
            .broadcast(PlaylistElementIn::UpdatePlaylistSongs(
                current_playlist.base.id.clone(),
                updated_list.base,
            ));
    }

    fn find_nearest_widget(&self, y: f64) -> Option<(f64, u32)> {
        (0..self.tracks.len())
            .filter_map(|i| self.tracks.get(i).map(|t| (i, t)))
            .filter_map(|(i, track)| {
                let track = track.borrow();
                let Some(widget) = track.fav_btn() else {
                    return None;
                };
                let translated_y = widget.translate_coordinates(
                    &self.tracks.view,
                    0.0,
                    widget.height() as f64 * 0.5,
                )?;
                let y_diff = y - translated_y.1;
                Some((y_diff, i))
            })
            .min_by(|(diff, _), (diff1, _)| {
                diff.abs()
                    .partial_cmp(&diff1.abs())
                    .expect("widget has no NaN")
            })
    }

    fn index_of_uid(&self, uid: usize) -> Option<(usize, PlaylistRow)> {
        (0..self.tracks.len())
            .filter_map(|i| self.tracks.get(i).map(|t| (i, t)))
            .find(|(_i, t)| *t.borrow().uid() == uid)
            .map(|(i, track)| (i as usize, track.borrow().clone()))
    }
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
    ClickedArtist(Id),
    ClickedAlbum(Id),
}

#[derive(Debug)]
pub enum PlaylistsViewIn {
    FilterChanged(String),
    ReplaceQueue,
    AddToQueue,
    AppendToQueue,
    PlaylistElement(PlaylistElementOut),
    Cover(CoverOut),
    NewPlaylist(submarine::data::PlaylistWithSongs),
    DeletePlaylist(relm4::factory::DynamicIndex),
    UpdateFavoriteSong(String, bool),
    DownloadClicked,
    Selected(i32),
    DropHover(f64, f64),
    DropMotionLeave,
    DropMove(Droppable, f64, f64),
    DropInsert(Droppable, f64, f64),
    DragCssReset,
    RemovePlaylistRow,
}

#[relm4::component(pub, async)]
impl relm4::component::AsyncComponent for PlaylistsView {
    type Init = Rc<RefCell<Subsonic>>;
    type Input = PlaylistsViewIn;
    type Output = PlaylistsViewOut;
    type CommandOutput = ();

    async fn init(
        subsonic: Self::Init,
        root: Self::Root,
        sender: relm4::AsyncComponentSender<Self>,
    ) -> relm4::component::AsyncComponentParts<Self> {
        let mut tracks =
            relm4::typed_view::column::TypedColumnView::<PlaylistRow, gtk::MultiSelection>::new();
        tracks.append_column::<TitleColumn>();
        tracks.append_column::<ArtistColumn>();
        tracks.append_column::<AlbumColumn>();
        tracks.append_column::<LengthColumn>();
        tracks.append_column::<FavColumn>();

        let columns = tracks.get_columns();
        columns
            .get("Title")
            .unwrap()
            .set_title(Some(&gettext("Title")));
        columns
            .get("Artist")
            .unwrap()
            .set_title(Some(&gettext("Artist")));
        columns
            .get("Album")
            .unwrap()
            .set_title(Some(&gettext("Album")));
        columns
            .get("Length")
            .unwrap()
            .set_title(Some(&gettext("Length")));

        let mut model = PlaylistsView {
            subsonic: subsonic.clone(),
            playlists: relm4::factory::FactoryVecDeque::builder()
                .launch(gtk::ListBox::default())
                .forward(sender.input_sender(), PlaylistsViewIn::PlaylistElement),

            selected_playlist: None,
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

        relm4::component::AsyncComponentParts { model, widgets }
    }

    view! {
        gtk::Box {
            set_widget_name: "playlists-view",

            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_spacing: 7,

                gtk::WindowHandle {
                    gtk::Label {
                        add_css_class: granite::STYLE_CLASS_H2_LABEL,
                        set_label: &gettext("Playlists"),
                    }
                },

                model.playlists.widget().clone() -> gtk::ListBox {
                    add_css_class: granite::STYLE_CLASS_FRAME,
                    add_css_class: granite::STYLE_CLASS_RICH_LIST,
                    set_vexpand: true,

                    connect_row_selected[sender] => move |_listbox, row| {
                        if let Some(row) = row {
                            sender.input(PlaylistsViewIn::Selected(row.index()));
                        }
                    },

                    gtk::ListBoxRow {
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
                                set_widget_name: "playlist-view-tracks",

                                add_controller = gtk::EventControllerKey {
                                    connect_key_pressed[sender] => move |_widget, key, _code, _state| {
                                        if key == gtk::gdk::Key::Delete {
                                            sender.input(PlaylistsViewIn::RemovePlaylistRow);
                                        }
                                        gtk::glib::Propagation::Proceed
                                    }
                                },

                                add_controller = gtk::DropTarget {
                                    set_actions: gdk::DragAction::MOVE | gdk::DragAction::COPY,
                                    set_types: &[<Droppable as gtk::prelude::StaticType>::static_type()],

                                    connect_motion[sender] => move |_controller, x, y| {
                                        sender.input(PlaylistsViewIn::DropHover(x, y));
                                        gdk::DragAction::MOVE
                                    },

                                    connect_leave[sender] => move |_controller| {
                                        sender.input(PlaylistsViewIn::DropMotionLeave)
                                    },

                                    connect_drop[sender] => move |_controller, value, x, y| {
                                        sender.input(PlaylistsViewIn::DropMotionLeave);

                                        if let Ok(drop) = value.get::<Droppable>() {
                                            match &drop {
                                                Droppable::PlaylistItems(_) => sender.input(PlaylistsViewIn::DropMove(drop, x, y)),
                                                _ => sender.input(PlaylistsViewIn::DropInsert(drop, x, y)),
                                            }
                                        }
                                        true
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    async fn update_with_view(
        &mut self,
        widgets: &mut Self::Widgets,
        msg: Self::Input,
        sender: relm4::AsyncComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match msg {
            PlaylistsViewIn::FilterChanged(search) => {
                self.tracks.clear_filters();
                self.tracks.add_filter(move |row| {
                    let mut search = search.clone();
                    let mut test = format!(
                        "{} {} {}",
                        row.item().title,
                        row.item().artist.as_deref().unwrap_or_default(),
                        row.item().album.as_deref().unwrap_or_default()
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
                    self.playlists
                        .broadcast(PlaylistElementIn::UpdatePlaylistName(list.clone()));
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
            PlaylistsViewIn::UpdateFavoriteSong(id, state) => {
                (0..self.tracks.len())
                    .filter_map(|i| self.tracks.get(i))
                    .filter(|t| t.borrow().item().id == id)
                    .for_each(|track| match state {
                        true => {
                            if let Some(fav) = &track.borrow().fav_btn() {
                                fav.set_icon_name("starred-symbolic");
                            }
                            track.borrow_mut().item_mut().starred =
                                Some(chrono::offset::Local::now().into());
                        }
                        false => {
                            if let Some(fav) = &track.borrow().fav_btn() {
                                fav.set_icon_name("non-starred-symbolic");
                            }
                            track.borrow_mut().item_mut().starred = None;
                        }
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
                self.selected_playlist = Some(list.info().clone());
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
                (0..self.tracks.len())
                    .filter_map(|i| self.tracks.get(i))
                    .for_each(|entry| entry.borrow().reset_drag_indicators());
            }
            PlaylistsViewIn::DropHover(_x, y) => {
                //reset drag indicators
                (0..self.tracks.len())
                    .filter_map(|i| self.tracks.get(i))
                    .for_each(|track| track.borrow().reset_drag_indicators());

                //finding the index which is the closest
                if let Some((diff, i)) = self.find_nearest_widget(y) {
                    if diff < 0.0 {
                        self.tracks
                            .get(i)
                            .unwrap()
                            .borrow()
                            .add_drag_indicator_top();
                    } else {
                        self.tracks
                            .get(i)
                            .unwrap()
                            .borrow()
                            .add_drag_indicator_bottom();
                    }
                }
            }
            PlaylistsViewIn::DropMotionLeave => {
                (0..self.tracks.len())
                    .filter_map(|i| self.tracks.get(i))
                    .for_each(|track| track.borrow().reset_drag_indicators());
            }
            PlaylistsViewIn::DropMove(drop, _x, y) => {
                //finding the index which is the closest to mouse pointer
                let Some((diff, i)) = self.find_nearest_widget(y) else {
                    sender
                        .output(PlaylistsViewOut::DisplayToast(String::from(
                            "could not find widget to drop to",
                        )))
                        .unwrap();
                    return;
                };

                let dragged = match drop {
                    Droppable::PlaylistItems(songs) => songs,
                    _ => unreachable!("can only move QueueSongs"),
                };

                // find all selected rows
                let selected_idx: Vec<u32> = (0..self.tracks.len())
                    .filter(|i| self.tracks.view.model().unwrap().is_selected(*i))
                    .collect();

                // convert uid to index and track
                let Some((dragged_index, dragged_track)) = self.index_of_uid(dragged[0].uid) else {
                    return;
                };

                let mut src_index: Vec<u32> = vec![dragged_index as u32];
                let mut src_tracks: Vec<PlaylistRow> = vec![dragged_track];
                if (selected_idx).contains(&(dragged_index as u32)) {
                    (src_index, src_tracks) = selected_idx
                        .iter()
                        .filter_map(|i| self.tracks.get(*i).map(|t| (i, t)))
                        .map(|(i, track)| (i, track.borrow().clone()))
                        .collect();
                }

                // insert new tracks
                let mut inserted_uids = vec![]; // remember uids to select them later
                let i = if diff < 0.0 { i } else { i + 1 };
                tracing::info!("moving queue index {src_index:?} to {i}");
                for track in src_tracks.iter().rev() {
                    let row =
                        PlaylistRow::new(&self.subsonic, track.item().clone(), sender.clone());
                    inserted_uids.push(*row.uid());
                    self.tracks.insert(i, row);
                }

                // remove old tracks
                src_tracks.iter().for_each(|track| {
                    if let Some((i, _row)) = self.index_of_uid(*track.uid()) {
                        self.tracks.remove(i as u32);
                    }
                });

                // //unselect rows
                self.tracks.view.model().unwrap().unselect_all();
                // reselect moved rows
                (0..self.tracks.len())
                    .filter_map(|i| self.tracks.get(i).map(|t| (i, t)))
                    .filter(|(_i, track)| inserted_uids.contains(track.borrow().uid()))
                    .for_each(|(i, _track)| {
                        _ = self.tracks.view.model().unwrap().select_item(i, false)
                    });

                self.update_playlist(&sender).await;
                sender.input(PlaylistsViewIn::DragCssReset);
            }
            PlaylistsViewIn::DropInsert(drop, _x, y) => {
                //finding the index which is the closest
                if let Some((diff, i)) = self.find_nearest_widget(y) {
                    let songs = drop.get_songs(&self.subsonic);
                    //insert songs
                    let i = if diff < 0.0 { i } else { i + 1 };
                    for song in songs.iter().rev() {
                        let row = PlaylistRow::new(&self.subsonic, song.clone(), sender.clone());
                        self.tracks.insert(i, row);
                    }
                }

                self.update_playlist(&sender).await;
                let Some(current_playlist) = &mut self.selected_playlist else {
                    return;
                };
                widgets
                    .info_details
                    .set_text(&build_info_string(current_playlist));
                sender.input(PlaylistsViewIn::DragCssReset);
            }
            PlaylistsViewIn::DragCssReset => {
                (0..self.tracks.len())
                    .filter_map(|i| self.tracks.get(i))
                    .for_each(|track| track.borrow().reset_drag_indicators());
            }
            PlaylistsViewIn::RemovePlaylistRow => {
                // find all selected rows
                let selected_rows: Vec<u32> = (0..self.tracks.len())
                    .filter(|i| self.tracks.view.model().unwrap().is_selected(*i))
                    .collect();

                // removing rows
                selected_rows
                    .iter()
                    .rev()
                    .for_each(|i| self.tracks.remove(*i));

                self.update_playlist(&sender).await;

                //update playlist info
                let Some(current_playlist) = &mut self.selected_playlist else {
                    return;
                };
                widgets
                    .info_details
                    .set_text(&build_info_string(current_playlist));
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
