use std::{cell::RefCell, rc::Rc};

use gettextrs::gettext;
use relm4::{
    gtk::{
        self, gdk,
        glib::prelude::ToValue,
        prelude::{BoxExt, ButtonExt, ListBoxRowExt, OrientableExt, SelectionModelExt, WidgetExt},
    },
    Component, ComponentController, RelmWidgetExt,
};

use crate::{
    client::Client,
    common::{self, convert_for_label},
    components::cover::{Cover, CoverIn, CoverOut},
    factory::{
        playlist_element::{
            EditState, PlaylistElement, PlaylistElementDragged, PlaylistElementIn,
            PlaylistElementOut, State,
        },
        playlist_row::{
            AlbumColumn, ArtistColumn, FavColumn, LengthColumn, PlayCountColumn, PlaylistRow,
            PlaylistUid, PlaylistUids, TitleColumn,
        },
        queue_song_row::QueueUids,
        DragIndicatable,
    },
    settings::Settings,
    subsonic::Subsonic,
    types::{Droppable, Id},
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

    selected_playlist: Option<submarine::data::PlaylistWithSongs>,
    tracks: relm4::typed_view::column::TypedColumnView<PlaylistRow, gtk::MultiSelection>,
    info_cover: relm4::Controller<Cover>,
    info_cover_controller: gtk::DragSource,
    drop_target_move: gtk::DropTarget,
    drop_target_copy: gtk::DropTarget,
}

impl PlaylistsView {
    async fn sync_current_playlist(&mut self, sender: &relm4::AsyncComponentSender<Self>) {
        let Some(list) = &self.selected_playlist else {
            return;
        };

        // subsonic does not allow moving songs, so we need to remove songs
        // and then readd them

        //TODO improve efficiency by removing the parts that need removing instead of all

        let client = Client::get().unwrap();
        // fetch playlist and see its length so we can delete every index
        // the fetch is needed when removing, because we dont know how big the list was
        match client.get_playlist(&list.base.id).await {
            Err(e) => {
                sender
                    .output(PlaylistsViewOut::DisplayToast(format!(
                        "fetching playlist failed: {e}",
                    )))
                    .unwrap();
                return;
            }
            Ok(list) => {
                let temp_delete_indices: Vec<i64> = (0..list.entry.len() as i64).collect();
                if let Err(e) = client
                    .update_playlist(
                        &list.base.id,        // id of playlist
                        None::<String>,       // don't change name
                        None::<String>,       // don't change comment
                        None,                 // don't change public/private
                        Vec::<String>::new(), // no ids to append
                        temp_delete_indices,  // removed indices
                    )
                    .await
                {
                    sender
                        .output(PlaylistsViewOut::DisplayToast(format!(
                            "deleting rows from playlist failed: {e}",
                        )))
                        .unwrap();
                    return;
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
                &list.base.id,  // id of playlist
                None::<String>, // don't chang name
                None::<String>, // don't change comment
                None,           // don't change public/private
                ids,            // ids to append
                vec![],         // nothing to remove
            )
            .await
        {
            sender
                .output(PlaylistsViewOut::DisplayToast(format!(
                    "readding ids to playlist failed: {e}",
                )))
                .unwrap();
            return;
        }
        let updated_list = match client.get_playlist(&list.base.id).await {
            Ok(list) => list,
            Err(e) => {
                tracing::error!("updated list not found on server: {e}");
                return;
            }
        };

        // update cache
        self.subsonic.borrow_mut().replace_playlist(&updated_list);

        //sync local cache playlist content
        self.playlists
            .broadcast(PlaylistElementIn::UpdatePlaylist(updated_list));
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
pub enum PlaylistsViewIn {
    SearchChanged,
    ReplaceQueue,
    AddToQueue,
    AppendToQueue,
    PlaylistElement(PlaylistElementOut),
    Cover(CoverOut),
    NewPlaylist(submarine::data::PlaylistWithSongs),
    DeletePlaylist(relm4::factory::DynamicIndex),
    UpdateFavoriteSong(String, bool),
    UpdatePlayCountSong(String, Option<i64>),
    DownloadClicked,
    Selected(i32),
    DropHover(f64),
    DropMotionLeave,
    DropMove(Droppable, f64),
    DropInsert(Droppable, f64),
    DragCssReset,
    RemovePlaylistRow,
    InsertSongsTo(u32, Vec<submarine::data::Child>),
    SelectionChanged,
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
    CreateEmptyPlaylist,
    CreatePlaylist(Droppable),
    RenamePlaylist(submarine::data::Playlist),
    DisplayToast(String),
    Download(Droppable),
    FavoriteClicked(String, bool),
    ClickedArtist(Id),
    ClickedAlbum(Id),
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
        tracks.append_column::<PlayCountColumn>();
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
            drop_target_move: gtk::DropTarget::default(),
            drop_target_copy: gtk::DropTarget::default(),
        };

        let widgets = view_output!();

        // set some things for InfoCover
        model.info_cover.model().add_css_class_image("size100");
        model
            .info_cover
            .widget()
            .add_controller(model.info_cover_controller.clone());

        // connect signal SelectionChanged
        let send = sender.clone();
        model
            .tracks
            .view
            .model()
            .unwrap()
            .connect_selection_changed(move |_model, _, _| {
                send.input(PlaylistsViewIn::SelectionChanged);
            });

        // add playlists to list
        let mut guard = model.playlists.guard();
        for playlist in model.subsonic.borrow().playlists() {
            guard.push_back((model.subsonic.clone(), playlist.clone()));
        }
        drop(guard);

        // add search filter
        model.tracks.add_filter(move |track| {
            let search = Settings::get().lock().unwrap().search_text.clone();
            let title_artist_album = format!(
                "{} {} {}",
                track.item().title.clone(),
                track.item().artist.clone().unwrap_or_default(),
                track.item().album.clone().unwrap_or_default()
            );
            common::search_matching(title_artist_album, search)
        });

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
                                sender.output(PlaylistsViewOut::CreateEmptyPlaylist).unwrap();
                            },

                            add_controller = gtk::DropTarget {
                                set_actions: gdk::DragAction::COPY,
                                set_types: &[<Droppable as gtk::prelude::StaticType>::static_type()
                                             , <QueueUids as gtk::prelude::StaticType>::static_type()
                                             , <PlaylistElementDragged as gtk::prelude::StaticType>::static_type(),
                                ],

                                connect_motion[sender] => move |_controller, _x, y| {
                                    sender.input(PlaylistsViewIn::DropHover(y));
                                    gdk::DragAction::COPY
                                },

                                connect_leave[sender] => move |_controller| {
                                    sender.input(PlaylistsViewIn::DropMotionLeave)
                                },

                                connect_drop[sender] => move |_controller, value, _x, _y| {
                                    sender.input(PlaylistsViewIn::DropMotionLeave);

                                    let drop = if let Ok(drop) = value.get::<QueueUids>() {
                                        Droppable::QueueSongs(drop.0)
                                    } else if let Ok(drop) = value.get::<PlaylistElementDragged>() {
                                        Droppable::Playlist(drop.0)
                                    } else if let Ok(drop) = value.get::<Droppable>() {
                                        drop
                                    } else {
                                        return false;
                                    };

                                    sender.output(PlaylistsViewOut::CreatePlaylist(drop)).unwrap();

                                    true
                                }
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
                                                    set_icon_name: Some("queue-append-symbolic"),
                                                    set_pixel_size: 20,
                                                },
                                            },
                                            set_tooltip: &gettext("Append playlist to end of queue"),
                                            connect_clicked => PlaylistsViewIn::AppendToQueue,
                                        },
                                        gtk::Button {
                                            gtk::Box {
                                                gtk::Image {
                                                    set_icon_name: Some("queue-insert-symbolic"),
                                                    set_pixel_size: 20,
                                                },
                                            },
                                            set_tooltip: &gettext("Insert playlist after currently played or paused item"),
                                            connect_clicked => PlaylistsViewIn::AddToQueue,
                                        },
                                        gtk::Button {
                                            gtk::Box {
                                                gtk::Image {
                                                    set_icon_name: Some("queue-replace-symbolic"),
                                                    set_pixel_size: 20,
                                                },
                                            },
                                            set_tooltip: &gettext("Replaces current queue with this playlist"),
                                            connect_clicked => PlaylistsViewIn::ReplaceQueue,
                                        },
                                        gtk::Button {
                                            gtk::Box {
                                                gtk::Image {
                                                    set_icon_name: Some("browser-download-symbolic"),
                                                    set_pixel_size: 20,
                                                },
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

                                // moving a playlist item
                                add_controller = model.drop_target_move.clone() -> gtk::DropTarget {
                                    set_actions: gdk::DragAction::MOVE,
                                    set_types: &[<PlaylistUids as gtk::prelude::StaticType>::static_type()],

                                    connect_motion[sender] => move |_controller, _x, y| {
                                        sender.input(PlaylistsViewIn::DropHover(y));
                                        gdk::DragAction::MOVE
                                    },

                                    connect_leave[sender] => move |_controller| {
                                        sender.input(PlaylistsViewIn::DropMotionLeave)
                                    },

                                    connect_drop[sender] => move |_controller, value, _x, y| {
                                        sender.input(PlaylistsViewIn::DropMotionLeave);

                                        if let Ok(drop) = value.get::<PlaylistUids>() {
                                            let drop = Droppable::PlaylistItems(drop.0);
                                            sender.input(PlaylistsViewIn::DropMove(drop, y));
                                            true
                                        } else {
                                            false
                                        }
                                    }
                                },

                                // adding new songs
                                add_controller = model.drop_target_copy.clone() -> gtk::DropTarget {
                                    set_actions: gdk::DragAction::COPY,
                                    set_types: &[<Droppable as gtk::prelude::StaticType>::static_type()
                                                 , <QueueUids as gtk::prelude::StaticType>::static_type()
                                                 , <PlaylistElementDragged as gtk::prelude::StaticType>::static_type(),
                                    ],

                                    connect_motion[sender] => move |_controller, _x, y| {
                                        sender.input(PlaylistsViewIn::DropHover(y));
                                        gdk::DragAction::COPY
                                    },

                                    connect_leave[sender] => move |_controller| {
                                        sender.input(PlaylistsViewIn::DropMotionLeave)
                                    },

                                    connect_drop[sender] => move |_controller, value, _x, y| {
                                        sender.input(PlaylistsViewIn::DropMotionLeave);

                                        if let Ok(drop) = value.get::<QueueUids>() {
                                            let drop = Droppable::QueueSongs(drop.0);
                                            sender.input(PlaylistsViewIn::DropInsert(drop, y));
                                            true
                                        } else if let Ok(drop) = value.get::<PlaylistElementDragged>() {
                                            let drop = Droppable::Playlist(drop.0);
                                            sender.input(PlaylistsViewIn::DropInsert(drop, y));
                                            true
                                        } else if let Ok(drop) = value.get::<Droppable>() {
                                            match &drop {
                                                Droppable::PlaylistItems(_) => sender.input(PlaylistsViewIn::DropMove(drop, y)),
                                                _ => sender.input(PlaylistsViewIn::DropInsert(drop, y)),
                                            }
                                            true
                                        } else {
                                            false
                                        }
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
            PlaylistsViewIn::SearchChanged => _ = self.tracks.notify_filter_changed(0),
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
                PlaylistElementOut::Clicked(index) => {
                    sender.input(PlaylistsViewIn::Selected(index.current_index() as i32));
                }
                PlaylistElementOut::DropAppend(drop, list) => {
                    sender.input(PlaylistsViewIn::DragCssReset);

                    let client = Client::get().unwrap();
                    let songs = drop.get_songs(&self.subsonic);
                    let ids = songs.iter().map(|s| s.id.clone()).collect();
                    // send song to server
                    if let Err(e) = client
                        .update_playlist(
                            &list.base.id,  // id of playlist
                            None::<String>, // don't change name
                            None::<String>, // don't change comment
                            None,           // don't change public/private
                            ids,            // ids to append
                            vec![],         // nothing to remove
                        )
                        .await
                    {
                        sender
                            .output(PlaylistsViewOut::DisplayToast(format!(
                                "appending ids to playlist failed: {e}",
                            )))
                            .unwrap();
                        return;
                    }

                    // get updated info
                    let updated_list = match client.get_playlist(&list.base.id).await {
                        Ok(list) => list,
                        Err(e) => {
                            sender
                                .output(PlaylistsViewOut::DisplayToast(format!(
                                    "updated list not found on server: {e}",
                                )))
                                .unwrap();
                            return;
                        }
                    };

                    // update local cache
                    self.subsonic.borrow_mut().replace_playlist(&updated_list);

                    // update widget info
                    self.playlists
                        .broadcast(PlaylistElementIn::UpdatePlaylist(updated_list.clone()));
                    if let Some(current_list) = &self.selected_playlist {
                        widgets
                            .info_details
                            .set_text(&build_info_string(current_list));

                        // append on shown playlist
                        if current_list.base.id == updated_list.base.id {
                            for song in songs.iter() {
                                let row =
                                    PlaylistRow::new(&self.subsonic, song.clone(), sender.clone());
                                self.tracks.append(row);
                            }
                        }
                    }
                }
                PlaylistElementOut::MoveDropAbove(drop, target_index) => {
                    let mut guard = self.playlists.guard();
                    let Some(src_element) =
                        guard.iter().find(|e| e.info().base.id == drop.0.base.id)
                    else {
                        sender
                            .output(PlaylistsViewOut::DisplayToast(
                                "dragged PlaylistElement not foundin elements".to_string(),
                            ))
                            .unwrap();
                        return;
                    };
                    let src_index = src_element.index();

                    // update cache
                    self.subsonic
                        .borrow_mut()
                        .move_playlist(src_index.current_index(), target_index.current_index());

                    // update widgets
                    guard.move_to(src_index.current_index(), target_index.current_index());
                }
                PlaylistElementOut::MoveDropBelow(drop, target_index) => {
                    let mut guard = self.playlists.guard();
                    let Some(src_element) =
                        guard.iter().find(|e| e.info().base.id == drop.0.base.id)
                    else {
                        sender
                            .output(PlaylistsViewOut::DisplayToast(
                                "dragged PlaylistElement not foundin elements".to_string(),
                            ))
                            .unwrap();
                        return;
                    };
                    let src_index = src_element.index();

                    // update cache
                    self.subsonic
                        .borrow_mut()
                        .move_playlist(src_index.current_index(), target_index.current_index() + 1);

                    // update widgets
                    guard.move_to(src_index.current_index(), target_index.current_index() + 1);
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
                //check for write protection
                let Some(list) = self.playlists.get(index.current_index()) else {
                    sender
                        .output(PlaylistsViewOut::DisplayToast(
                            "trying to delete a playlist index that doesn't exists".to_string(),
                        ))
                        .unwrap();
                    return;
                };
                if list.write_protected() {
                    return;
                }

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
            PlaylistsViewIn::UpdatePlayCountSong(id, play_count) => (0..self.tracks.len())
                .filter_map(|i| self.tracks.get(i))
                .filter(|t| t.borrow().item().id == id)
                .for_each(|track| track.borrow_mut().set_play_count(play_count)),
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
                let mut guard = self.playlists.guard();
                let Some(element) = guard.get_mut(index as usize) else {
                    // do not output a error message, because it also triggers when deleting the last playlist
                    return;
                };
                let selected_list = element.info().clone();
                let is_write_protected = element.write_protected();
                drop(guard);

                // check for smart playlist
                if is_write_protected {
                    // update list
                    let client = Client::get().unwrap();
                    let list = match client.get_playlist(&selected_list.base.id).await {
                        Err(e) => {
                            sender
                                .output(PlaylistsViewOut::DisplayToast(format!(
                                    "could not update smart playlist: {e}"
                                )))
                                .unwrap();
                            return;
                        }
                        Ok(list) => list,
                    };

                    // update cache
                    self.subsonic.borrow_mut().replace_playlist(&list);
                    // update widgets
                    self.playlists.send(
                        index as usize,
                        PlaylistElementIn::UpdatePlaylist(list.clone()),
                    );
                    tracing::info!("updated smart playlist {}", list.base.name);

                    self.drop_target_copy.set_types(&[]);
                    self.drop_target_move.set_types(&[]);
                } else {
                    self.drop_target_move
                        .set_types(&[<PlaylistUids as gtk::prelude::StaticType>::static_type()]);
                    self.drop_target_copy.set_types(&[
                        <Droppable as gtk::prelude::StaticType>::static_type(),
                        <QueueUids as gtk::prelude::StaticType>::static_type(),
                        <PlaylistElementDragged as gtk::prelude::StaticType>::static_type(),
                    ]);
                }

                // return when list already active
                if let Some(current) = &self.selected_playlist {
                    if selected_list.base.id == current.base.id {
                        return;
                    }
                }

                let guard = self.playlists.guard();
                let Some(element) = guard.get(index as usize) else {
                    // do not output a error message, because it also triggers when deleting the last playlist
                    return;
                };

                // set every state in PlaylistElement to normal
                for list in guard.iter() {
                    list.change_state(&State::Normal);
                    list.set_edit_area(EditState::NotActive);
                }

                widgets.track_stack.set_visible_child_name("tracks");

                element.set_edit_area(EditState::Active);
                self.selected_playlist = Some(element.info().clone());
                let list = element.info();

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
                let cover_art = list.base.cover_art.clone();
                let subsonic = self.subsonic.clone();
                self.info_cover_controller
                    .connect_drag_begin(move |src, _drag| {
                        if let Some(art) = &cover_art {
                            let cover = subsonic.borrow().cover_icon(art);
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
            PlaylistsViewIn::DropHover(y) => {
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
            PlaylistsViewIn::DropMove(drop, y) => {
                sender.input(PlaylistsViewIn::DragCssReset);

                //finding the index which is the closest to mouse pointer
                let Some((diff, i)) = self.find_nearest_widget(y) else {
                    sender
                        .output(PlaylistsViewOut::DisplayToast(String::from(
                            "could not find widget to drop to",
                        )))
                        .unwrap();
                    return;
                };

                // return when playlist is write protected
                let guard = self.playlists.guard();
                if let Some(current_list) = &self.selected_playlist {
                    if let Some(list) = guard
                        .iter()
                        .find(|e| e.info().base.id == current_list.base.id)
                    {
                        if list.write_protected() {
                            return;
                        }
                    }
                }
                std::mem::drop(guard);

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

                // update cache
                self.sync_current_playlist(&sender).await;
            }
            PlaylistsViewIn::DropInsert(drop, y) => {
                sender.input(PlaylistsViewIn::DragCssReset);

                // return when playlist is write protected
                let guard = self.playlists.guard();
                if let Some(current_list) = &self.selected_playlist {
                    if let Some(list) = guard
                        .iter()
                        .find(|e| e.info().base.id == current_list.base.id)
                    {
                        if list.write_protected() {
                            return;
                        }
                    }
                }
                std::mem::drop(guard);

                //finding the index which is the closest
                let Some((diff, i)) = self.find_nearest_widget(y) else {
                    sender
                        .output(PlaylistsViewOut::DisplayToast(String::from(
                            "could not find widget to drop to",
                        )))
                        .unwrap();
                    return;
                };

                // return when write protected
                let Some(list) = self.playlists.get(i as usize) else {
                    sender
                        .output(PlaylistsViewOut::DisplayToast(
                            "trying to delete an index that doesn't exist".to_string(),
                        ))
                        .unwrap();
                    return;
                };
                if list.write_protected() {
                    return;
                }

                let i = if diff < 0.0 { i } else { i + 1 };
                let songs = drop.get_songs(&self.subsonic);
                //insert songs
                for song in songs.iter().rev() {
                    let row = PlaylistRow::new(&self.subsonic, song.clone(), sender.clone());
                    self.tracks.insert(i, row);
                }

                // update cache
                self.sync_current_playlist(&sender).await;

                // update widgets
                if let Some(current_list) = &self.selected_playlist {
                    widgets
                        .info_details
                        .set_text(&build_info_string(current_list));
                }
            }
            PlaylistsViewIn::DragCssReset => {
                (0..self.tracks.len())
                    .filter_map(|i| self.tracks.get(i))
                    .for_each(|track| track.borrow().reset_drag_indicators());
            }
            PlaylistsViewIn::RemovePlaylistRow => {
                let Some(current_list) = &self.selected_playlist else {
                    return;
                };

                // return when write protected
                let guard = self.playlists.guard();
                if let Some(playlist) = guard
                    .iter()
                    .find(|e| e.info().base.id == current_list.base.id)
                {
                    let Some(list) = guard.get(playlist.index().current_index()) else {
                        sender
                            .output(PlaylistsViewOut::DisplayToast(
                                "trying to delete an index that doesn't exist".to_string(),
                            ))
                            .unwrap();
                        return;
                    };
                    if list.write_protected() {
                        return;
                    }
                };
                drop(guard);

                // find all selected rows
                let selected_rows: Vec<u32> = (0..self.tracks.len())
                    .filter(|i| self.tracks.view.model().unwrap().is_selected(*i))
                    .collect();

                // remove rows from server
                let client = Client::get().unwrap();
                if let Err(e) = client
                    .update_playlist(
                        &current_list.base.id,                                     // id of playlist
                        None::<String>,       // don't change name
                        None::<String>,       // don't change comment
                        None,                 // don't change public/private
                        Vec::<String>::new(), // no ids to append
                        selected_rows.iter().cloned().map(|i| i as i64).collect(), // ids to remove
                    )
                    .await
                {
                    sender
                        .output(PlaylistsViewOut::DisplayToast(format!(
                            "removing from playlist on server failed: {e}",
                        )))
                        .unwrap();
                    return;
                }

                // removing rows in widgets
                selected_rows
                    .iter()
                    .rev()
                    .for_each(|i| self.tracks.remove(*i));

                // update cache
                self.sync_current_playlist(&sender).await;

                // update widgets
                if let Some(current_list) = &self.selected_playlist {
                    widgets
                        .info_details
                        .set_text(&build_info_string(current_list));
                }
            }
            PlaylistsViewIn::InsertSongsTo(index, songs) => {
                sender.input(PlaylistsViewIn::DragCssReset);

                let Some(list) = self.playlists.get(index as usize) else {
                    sender
                        .output(PlaylistsViewOut::DisplayToast(
                            "trying to delete an index that doesn't exist".to_string(),
                        ))
                        .unwrap();
                    return;
                };
                if list.write_protected() {
                    return;
                }

                // insert songs
                for song in songs.iter().rev() {
                    let row = PlaylistRow::new(&self.subsonic, song.clone(), sender.clone());
                    self.tracks.insert(index, row);
                }

                // update cache
                self.sync_current_playlist(&sender).await;

                // update widgets
                if let Some(current_list) = &self.selected_playlist {
                    widgets
                        .info_details
                        .set_text(&build_info_string(current_list));
                }
            }
            PlaylistsViewIn::SelectionChanged => {
                // update content for drag and drop
                let uids: Vec<_> = (0..self.tracks.len())
                    .filter(|i| self.tracks.view.model().unwrap().is_selected(*i))
                    .filter_map(|i| self.tracks.get(i))
                    .map(|row| PlaylistUid {
                        uid: *row.borrow().uid(),
                        child: row.borrow().item().clone(),
                    })
                    .collect();

                //reset multiple selection
                (0..self.tracks.len())
                    .filter_map(|i| self.tracks.get(i))
                    .for_each(|track| track.borrow_mut().set_multiple_selection(vec![]));

                // set multiple selection for selected items
                (0..self.tracks.len())
                    .filter(|i| self.tracks.view.model().unwrap().is_selected(*i))
                    .filter_map(|i| self.tracks.get(i))
                    .for_each(|track| track.borrow_mut().set_multiple_selection(uids.clone()));
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
