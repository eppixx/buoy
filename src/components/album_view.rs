use std::{cell::RefCell, rc::Rc};

use fuzzy_matcher::FuzzyMatcher;
use gettextrs::gettext;
use relm4::{
    gtk::{
        self, glib,
        prelude::{
            BoxExt, ButtonExt, ListModelExt, OrientableExt, SelectionModelExt, ToValue, WidgetExt,
        },
    },
    ComponentController, RelmWidgetExt,
};

use crate::{
    client::Client,
    common::convert_for_label,
    components::cover::{Cover, CoverIn, CoverOut},
    factory::album_track_row::{
        AlbumTrackRow, ArtistColumn, BitRateColumn, FavColumn, GenreColumn, LengthColumn,
        PlayCountColumn, PositionColumn, TitleColumn,
    },
    gtk_helper::{loading_widget::LoadingWidgetState, stack::StackExt},
    settings::Settings,
    subsonic::Subsonic,
    types::{Droppable, Id},
};

#[derive(Debug)]
pub struct AlbumView {
    subsonic: Rc<RefCell<Subsonic>>,
    id: Id,
    cover: relm4::Controller<Cover>,
    tracks: relm4::typed_view::column::TypedColumnView<AlbumTrackRow, gtk::MultiSelection>,
}

#[derive(Debug)]
pub enum AlbumViewIn {
    AlbumTracks,
    Cover(CoverOut),
    UpdateFavoriteAlbum(String, bool),
    UpdateFavoriteSong(String, bool),
    UpdatePlayCountSong(String, Option<i64>),
    FilterChanged(String),
    HoverCover(bool),
    RecalcDragSource,
}

#[derive(Debug)]
pub enum AlbumViewOut {
    AppendAlbum(Droppable),
    InsertAfterCurrentPlayed(Droppable),
    ReplaceQueue(Droppable),
    FavoriteAlbumClicked(String, bool),
    FavoriteSongClicked(String, bool),
    DisplayToast(String),
    Download(Droppable),
    ArtistClicked(Id),
}

#[derive(Debug)]
pub enum AlbumViewCmd {
    LoadedAlbum(Result<submarine::data::AlbumWithSongsId3, submarine::SubsonicError>),
}

#[relm4::component(pub)]
impl relm4::Component for AlbumView {
    type Init = (Rc<RefCell<Subsonic>>, Id);
    type Input = AlbumViewIn;
    type Output = AlbumViewOut;
    type Widgets = AlbumViewWidgets;
    type CommandOutput = AlbumViewCmd;

    fn init(
        (subsonic, id): Self::Init,
        root: Self::Root,
        sender: relm4::ComponentSender<Self>,
    ) -> relm4::ComponentParts<Self> {
        //check id
        let Id::Album(_) = &id else {
            panic!("given id: '{id}' is not an album");
        };
        let album = subsonic.borrow().find_album(id.as_ref()).unwrap();

        let mut tracks =
            relm4::typed_view::column::TypedColumnView::<AlbumTrackRow, gtk::MultiSelection>::new();
        tracks.append_column::<PositionColumn>();
        tracks.append_column::<TitleColumn>();
        tracks.append_column::<ArtistColumn>();
        tracks.append_column::<GenreColumn>();
        tracks.append_column::<LengthColumn>();
        tracks.append_column::<PlayCountColumn>();
        tracks.append_column::<BitRateColumn>();
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
            .get("Genre")
            .unwrap()
            .set_title(Some(&gettext("Genre")));
        columns
            .get("Length")
            .unwrap()
            .set_title(Some(&gettext("Length")));
        columns
            .get("Bitrate")
            .unwrap()
            .set_title(Some(&gettext("Bitrate")));
        columns
            .get("Favorite")
            .unwrap()
            .set_title(Some(&gettext("Favorite")));

        let model = Self {
            subsonic: subsonic.clone(),
            id: id.clone(),
            cover: Cover::builder()
                .launch((subsonic.clone(), album.cover_art.clone()))
                .forward(sender.input_sender(), AlbumViewIn::Cover),
            tracks,
        };

        let widgets = view_output!();
        model.cover.model().add_css_class_image("size150");

        //load album
        let id = model.id.clone();
        sender.oneshot_command(async move {
            let client = Client::get().unwrap();
            AlbumViewCmd::LoadedAlbum(client.get_album(id.as_ref()).await)
        });

        // send signal on selection change
        model
            .tracks
            .selection_model
            .connect_selection_changed(move |_selection_model, _x, _y| {
                sender.input(AlbumViewIn::RecalcDragSource);
            });

        relm4::ComponentParts { model, widgets }
    }

    view! {
        gtk::Box {
            set_widget_name: "album-view",
            set_orientation: gtk::Orientation::Vertical,

            gtk::Box {
                set_spacing: 15,

                gtk::Overlay {
                    #[wrap(Some)]
                    set_child = &model.cover.widget().clone() -> gtk::Box {},

                    add_overlay: favorite = &gtk::Button {
                        set_halign: gtk::Align::End,
                        set_valign: gtk::Align::End,
                        set_width_request: 24,
                        set_height_request: 24,
                        set_icon_name: "non-starred-symbolic",

                        connect_clicked[sender, id] => move |btn| {
                            let state = match btn.icon_name().as_deref() {
                                Some("starred-symbolic") => false,
                                Some("non-starred-symbolic") => true,
                                name => unreachable!("unkonwn icon name: {name:?}"),
                            };
                            sender.output(AlbumViewOut::FavoriteSongClicked(String::from(id.inner()), state)).unwrap();
                        }
                    },

                    add_controller = gtk::EventControllerMotion {
                        connect_enter[sender] => move |_event, _x, _y| {
                            sender.input(AlbumViewIn::HoverCover(true));
                        },
                        connect_leave => AlbumViewIn::HoverCover(false),
                    },
                },

                gtk::WindowHandle {
                    set_hexpand: true,

                    gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_spacing: 8,

                        append: album_title = &gtk::Label {
                            add_css_class: granite::STYLE_CLASS_H2_LABEL,
                            set_label: &gettext("Loading Album"),
                            set_halign: gtk::Align::Start,
                        },
                        append: album_artist = &gtk::Label {
                            add_css_class: granite::STYLE_CLASS_H3_LABEL,
                            set_halign: gtk::Align::Start,

                            connect_activate_link[sender] => move |_label, text| {
                                sender.output(AlbumViewOut::ArtistClicked(Id::artist(text.to_string()))).unwrap();
                                gtk::glib::signal::Propagation::Stop
                            }
                        },
                        append: album_info = &gtk::Label {
                            set_halign: gtk::Align::Start,
                        },
                        append: album_genres = &gtk::Box {
                            set_spacing: 10,
                            gtk::Label {
                                set_label: &gettext("Genre:"),
                            }
                        },
                        gtk::Box {
                            set_spacing: 15,
                            append: append_append = &gtk::Button {
                                set_sensitive: false,

                                gtk::Box {
                                    gtk::Image {
                                        set_icon_name: Some("list-add-symbolic"),
                                    },
                                    gtk::Label {
                                        set_label: &gettext("Append"),
                                    }
                                },
                                set_tooltip: &gettext("Append Album to end of queue"),
                                connect_clicked[sender, album] => move |_btn| {
                                    let drop = Droppable::AlbumChild(Box::new(album.clone()));
                                    sender.output(AlbumViewOut::AppendAlbum(drop)).unwrap();
                                }
                            },
                            append: insert_album = &gtk::Button {
                                set_sensitive: false,

                                gtk::Box {
                                    gtk::Image {
                                        set_icon_name: Some("list-add-symbolic"),
                                    },
                                    gtk::Label {
                                        set_label: &gettext("Play next")
                                    }
                                },
                                set_tooltip: &gettext("Insert Album after currently played or paused item"),
                                connect_clicked[sender, album] => move |_btn| {
                                    let drop = Droppable::AlbumChild(Box::new(album.clone()));
                                    sender.output(AlbumViewOut::InsertAfterCurrentPlayed(drop)).unwrap();
                                }
                            },
                            append: replace_queue = &gtk::Button {
                                set_sensitive: false,

                                gtk::Box {
                                    gtk::Image {
                                        set_icon_name: Some("emblem-symbolic-link-symbolic"),
                                    },
                                    gtk::Label {
                                        set_label: &gettext("Replace queue"),
                                    }
                                },
                                set_tooltip: &gettext("Replaces current queue with this album"),
                                connect_clicked[sender, album] => move |_btn| {
                                    let drop = Droppable::AlbumChild(Box::new(album.clone()));
                                    sender.output(AlbumViewOut::ReplaceQueue(drop)).unwrap();
                                }
                            },
                            append: download_album = &gtk::Button {
                                set_sensitive: false,

                                gtk::Box {
                                    gtk::Image {
                                        set_icon_name: Some("browser-download-symbolic"),
                                    },
                                    gtk::Label {
                                        set_label: &gettext("Download Album"),
                                    }
                                },
                                set_tooltip: &gettext("Click to select a folder to download this album to"),
                                connect_clicked[sender, album] => move |_btn| {
                                    let drop =  Droppable::Child(Box::new(album.clone()));
                                    sender.output(AlbumViewOut::Download(drop)).unwrap();
                                }
                            }
                        }
                    }
                },
            },

            // bottom
            append: tracks_stack = &gtk::Stack {
                set_transition_type: gtk::StackTransitionType::Crossfade,
                set_transition_duration: 200,

                add_enumed[LoadingWidgetState::NotEmpty] = &gtk::ScrolledWindow {
                    set_hexpand: true,
                    set_vexpand: true,

                    model.tracks.view.clone() {
                        set_widget_name: "album-view-tracks",
                        set_vexpand: true,

                        add_controller = gtk::DragSource {
                            connect_prepare[sender] => move |_drag_src, _x, _y| {
                                sender.input(AlbumViewIn::RecalcDragSource);
                                None
                            }
                        }
                    }
                },
                add_enumed[LoadingWidgetState::Loading] = &gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_valign: gtk::Align::Center,

                    gtk::Spinner {
                        add_css_class: "size32",
                        set_spinning: true,
                        start: (),
                    }
                },

                set_visible_child_enum: &LoadingWidgetState::Loading,
            },
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
            AlbumViewIn::AlbumTracks => {} //do nothing
            AlbumViewIn::Cover(msg) => match msg {
                CoverOut::DisplayToast(title) => {
                    sender.output(AlbumViewOut::DisplayToast(title)).unwrap();
                }
            },
            AlbumViewIn::FilterChanged(search) => {
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
            AlbumViewIn::UpdateFavoriteAlbum(id, state) => {
                let Some(album) = self.subsonic.borrow().find_album(self.id.as_ref()) else {
                    sender
                        .output(AlbumViewOut::DisplayToast(format!(
                            "error finding album {id}"
                        )))
                        .unwrap();
                    return;
                };

                match (state, album.id == id) {
                    (true, true) => widgets.favorite.set_icon_name("starred-symbolic"),
                    (false, true) => widgets.favorite.set_icon_name("non-starred-symbolic"),
                    (_, false) => {} // signal is not for this album
                }
            }
            AlbumViewIn::UpdateFavoriteSong(id, state) => {
                (0..self.tracks.len())
                    .filter_map(|i| self.tracks.get(i))
                    .filter(|t| t.borrow().item().id == id)
                    .for_each(|track| match state {
                        true => {
                            track.borrow_mut().item_mut().starred =
                                Some(chrono::offset::Local::now().into());
                            if let Some(fav) = &track.borrow().fav_btn() {
                                fav.set_icon_name("starred-symbolic");
                            }
                        }
                        false => {
                            track.borrow_mut().item_mut().starred = None;
                            if let Some(fav) = &track.borrow().fav_btn() {
                                fav.set_icon_name("non-starred-symbolic");
                            }
                        }
                    });
            }
            AlbumViewIn::UpdatePlayCountSong(id, play_count) => (0..self.tracks.len())
                .filter_map(|i| self.tracks.get(i))
                .filter(|t| t.borrow().item().id == id)
                .for_each(|track| track.borrow_mut().set_play_count(play_count)),
            AlbumViewIn::HoverCover(false) => {
                widgets.favorite.remove_css_class("neutral-color");
                if widgets.favorite.icon_name().as_deref() != Some("starred-symbolic") {
                    widgets.favorite.set_visible(false);
                }
            }
            AlbumViewIn::HoverCover(true) => {
                widgets.favorite.add_css_class("neutral-color");
                widgets.favorite.set_visible(true);
            }
            AlbumViewIn::RecalcDragSource => {
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
                    .map(|row| row.borrow().item().clone())
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

    fn update_cmd_with_view(
        &mut self,
        widgets: &mut Self::Widgets,
        msg: Self::CommandOutput,
        sender: relm4::ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match msg {
            AlbumViewCmd::LoadedAlbum(Err(e)) => {
                sender
                    .output(AlbumViewOut::DisplayToast(format!("could not load: {e:?}")))
                    .unwrap();
            }
            AlbumViewCmd::LoadedAlbum(Ok(album)) => {
                //load tracks
                for track in &album.song {
                    let track = AlbumTrackRow::new(&self.subsonic, track.clone(), sender.clone());
                    self.tracks.append(track);
                }
                // show tracks
                widgets
                    .tracks_stack
                    .set_visible_child_enum(&LoadingWidgetState::NotEmpty);

                // update dragSource for cover
                let drop = Droppable::AlbumWithSongs(Box::new(album.clone()));
                let content = gtk::gdk::ContentProvider::for_value(&drop.to_value());
                let drag_src = gtk::DragSource::new();
                drag_src.set_actions(gtk::gdk::DragAction::COPY);
                drag_src.set_content(Some(&content));
                let subsonic = self.subsonic.clone();
                let alb = album.clone();
                drag_src.connect_drag_begin(move |src, _drag| {
                    if let Some(cover_id) = &alb.base.cover_art {
                        let cover = subsonic.borrow().cover_icon(cover_id);
                        if let Some(tex) = cover {
                            src.set_icon(Some(&tex), 0, 0);
                        }
                    }
                });
                self.cover.widget().add_controller(drag_src);

                // update cover
                self.cover
                    .emit(CoverIn::LoadAlbumId3(Box::new(album.clone())));

                // update title
                widgets.album_title.set_label(&album.base.name);

                //update info label
                widgets.album_info.set_label(&build_info_string(&album));

                album
                    .base
                    .genres
                    .iter()
                    .flat_map(|map| {
                        map.iter()
                            .filter_map(|(key, value)| (*key == "name").then_some(value))
                    })
                    .for_each(|genre| {
                        let genre = gtk::Label::new(Some(genre));
                        genre.add_css_class("round-badge");
                        widgets.album_genres.append(&genre);
                    });

                // set artist label
                let markup = format!(
                    "by <span style=\"italic\"><a href=\"{}\">{}</a></span>",
                    album.base.artist_id.as_deref().unwrap_or(""),
                    glib::markup_escape_text(
                        album
                            .base
                            .artist
                            .as_deref()
                            .unwrap_or(&gettext("Unkown Artist"))
                    )
                );
                widgets.album_artist.set_markup(&markup);

                // update favorite
                widgets.favorite.set_visible(false);
                if album.base.starred.is_some() {
                    widgets.favorite.set_icon_name("starred-symbolic");
                    widgets.favorite.set_visible(true);
                }

                // set sensitivity for buttons
                widgets.append_append.set_sensitive(true);
                widgets.insert_album.set_sensitive(true);
                widgets.replace_queue.set_sensitive(true);
                widgets.download_album.set_sensitive(true);
            }
        }
    }
}

fn build_info_string(child: &submarine::data::AlbumWithSongsId3) -> String {
    let mut result = gettext("Songs");
    result.push_str(": ");
    result.push_str(&child.song.len().to_string());
    result.push_str(" • ");
    result.push_str(&gettext("Length"));
    result.push_str(": ");
    result.push_str(&convert_for_label(i64::from(child.base.duration) * 1000));
    if let Some(year) = child.base.year {
        result.push_str(" • ");
        result.push_str(&gettext("Release"));
        result.push_str(": ");
        result.push_str(&year.to_string());
    }
    if let Some(played) = child.base.play_count {
        result.push_str(" • ");
        result.push_str(&gettext("played"));
        result.push(' ');
        result.push_str(&played.to_string());
        result.push(' ');
        result.push_str(&gettext("times"));
    }
    result
}
