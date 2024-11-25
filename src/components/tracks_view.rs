use std::{cell::RefCell, collections::HashSet, rc::Rc};

use fuzzy_matcher::FuzzyMatcher;
use itertools::Itertools;
use relm4::{
    gtk::{
        self, glib,
        prelude::{BoxExt, ButtonExt, OrientableExt, WidgetExt},
    },
    ComponentController, RelmWidgetExt,
};

use crate::{
    components::{
        cover::{Cover, CoverIn},
        filter_row::{Filter, FilterRow, FilterRowOut, TextRelation},
    },
    factory::track_row::{BitRateColumn, GenreColumn},
    types::Droppable,
};
use crate::{
    components::{filter_categories::Category, filter_row::FilterRowIn},
    factory::track_row::{
        AlbumColumn, ArtistColumn, FavColumn, LengthColumn, PositionColumn, TitleColumn, TrackRow,
    },
    settings::Settings,
    subsonic::Subsonic,
};

use super::cover::CoverOut;

#[derive(Debug)]
pub struct TracksView {
    subsonic: Rc<RefCell<Subsonic>>,
    tracks: relm4::typed_view::column::TypedColumnView<TrackRow, gtk::SingleSelection>,
    filters: relm4::factory::FactoryVecDeque<FilterRow>,

    info_cover: relm4::Controller<Cover>,
    shown_tracks: Rc<RefCell<Vec<submarine::data::Child>>>,
    shown_artists: Rc<RefCell<HashSet<Option<String>>>>,
    shown_albums: Rc<RefCell<HashSet<Option<String>>>>,
}

impl TracksView {
    fn active_filters(&self) -> bool {
        self.filters.iter().any(|f| f.active())
    }

    fn calc_sensitivity_of_buttons(&self, widgets: &<TracksView as relm4::Component>::Widgets) {
        let allowed_queue_modifier_len = 1000;

        if (!self.active_filters() && self.tracks.len() >= allowed_queue_modifier_len)
            || (self.active_filters()
                && self.shown_tracks.borrow().len() >= allowed_queue_modifier_len as usize)
        {
            widgets.add_to_queue.set_sensitive(false);
            widgets
                .add_to_queue
                .set_tooltip("There are too many tracks to add to queue");
            widgets.append_to_queue.set_sensitive(false);
            widgets
                .append_to_queue
                .set_tooltip("There are too many tracks to append to queue");
            widgets.replace_queue.set_sensitive(false);
            widgets
                .replace_queue
                .set_tooltip("There are too many tracks to replace queue");
        } else {
            widgets.add_to_queue.set_sensitive(true);
            widgets
                .add_to_queue
                .set_tooltip("Append shown tracks to end of queue");
            widgets.append_to_queue.set_sensitive(true);
            widgets
                .append_to_queue
                .set_tooltip("Insert shown after currently played or paused item");
            widgets.replace_queue.set_sensitive(true);
            widgets
                .replace_queue
                .set_tooltip("Replaces current queue with shown tracks");
        }
    }
}

#[derive(Debug)]
pub enum TracksViewIn {
    FilterChanged,
    Favorited(String, bool),
    FilterAdd,
    FilterRow(FilterRowOut),
    Cover(CoverOut),
    AppendToQueue,
    AddToQueue,
    ReplaceQueue,
    DownloadClicked,
    ToggleFilters,
    TrackClicked(u32),
}

#[derive(Debug)]
pub enum TracksViewOut {
    DisplayToast(String),
    AddToQueue(Droppable),
    AppendToQueue(Droppable),
    ReplaceQueue(Droppable),
    Download(Droppable),
    FavoriteClicked(String, bool),
    ClickedArtist(String),
    ClickedAlbum(String),
}

#[derive(Debug)]
pub enum TracksViewCmd {
    AddTracks(Vec<submarine::data::Child>),
    LoadingTracksFinished,
}

#[relm4::component(pub)]
impl relm4::Component for TracksView {
    type Init = Rc<RefCell<Subsonic>>;
    type Input = TracksViewIn;
    type Output = TracksViewOut;
    type CommandOutput = TracksViewCmd;

    fn init(
        subsonic: Self::Init,
        root: Self::Root,
        sender: relm4::ComponentSender<Self>,
    ) -> relm4::ComponentParts<Self> {
        let mut tracks =
            relm4::typed_view::column::TypedColumnView::<TrackRow, gtk::SingleSelection>::new();
        tracks.append_column::<PositionColumn>();
        tracks.append_column::<TitleColumn>();
        tracks.append_column::<ArtistColumn>();
        tracks.append_column::<AlbumColumn>();
        tracks.append_column::<GenreColumn>();
        tracks.append_column::<LengthColumn>();
        tracks.append_column::<BitRateColumn>();
        tracks.append_column::<FavColumn>();

        // add tracks in chunks to not overwhelm the app
        const CHUNK_SIZE: usize = 20;
        const WAIT: u64 = 20;
        let mut countdown = 0;
        for chunk in &subsonic
            .borrow()
            .tracks()
            .iter()
            .cloned()
            .chunks(CHUNK_SIZE)
        {
            let chunk: Vec<submarine::data::Child> = chunk.into_iter().collect();
            sender.oneshot_command(async move {
                tokio::time::sleep(std::time::Duration::from_millis(countdown)).await;
                TracksViewCmd::AddTracks(chunk)
            });
            countdown += WAIT;
        }
        sender.oneshot_command(async move {
            tokio::time::sleep(std::time::Duration::from_millis(countdown)).await;
            TracksViewCmd::LoadingTracksFinished
        });
        tracing::info!("loading tracks should be finished in {countdown}ms");

        let mut model = Self {
            subsonic: subsonic.clone(),
            tracks,
            filters: relm4::factory::FactoryVecDeque::builder()
                .launch(gtk::ListBox::default())
                .forward(sender.input_sender(), Self::Input::FilterRow),
            info_cover: Cover::builder()
                .launch((subsonic, None))
                .forward(sender.input_sender(), TracksViewIn::Cover),
            shown_tracks: Rc::new(RefCell::new(vec![])),
            shown_artists: Rc::new(RefCell::new(HashSet::new())),
            shown_albums: Rc::new(RefCell::new(HashSet::new())),
        };
        model.info_cover.model().add_css_class_image("size100");

        let widgets = view_output!();
        model.filters.guard().push_back(Category::Favorite);
        model.calc_sensitivity_of_buttons(&widgets);
        relm4::ComponentParts { model, widgets }
    }

    view! {
        gtk::Box {
            // tracks
            gtk::Box {
                add_css_class: "tracks-view",
                set_orientation: gtk::Orientation::Vertical,

                gtk::WindowHandle {
                    gtk::CenterBox {
                        #[wrap(Some)]
                        set_center_widget = &gtk::Box {
                            set_spacing: 5,
                            gtk::Label {
                                add_css_class: "h2",
                                set_label: "Tracks",
                                set_halign: gtk::Align::Center,
                            },
                            append: spinner = &gtk::Spinner {
                                set_spinning: true,
                                start: (),
                            }
                        },

                        #[wrap(Some)]
                        set_end_widget = &gtk::Box {
                            set_spacing: 10,
                            set_margin_end: 10,

                            gtk::Label {
                                set_text: "Filters:",
                            },
                            gtk::Switch {
                                set_valign: gtk::Align::Center,
                                connect_active_notify => TracksViewIn::ToggleFilters,
                            }
                        }
                    }
                },

                // info
                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_spacing: 8,

                    gtk::WindowHandle {
                        gtk::Box {
                            set_spacing: 15,

                            model.info_cover.widget().clone() -> gtk::Box {},

                            //tracks info
                            gtk::Box {
                                set_orientation: gtk::Orientation::Vertical,
                                set_spacing: 8,

                                append: shown_tracks = &gtk::Label {
                                    set_halign: gtk::Align::Start,
                                    set_text: &format!("Shown tracks: {}", model.tracks.len()),
                                },
                                append: shown_artists = &gtk::Label {
                                    set_halign: gtk::Align::Start,
                                    set_text: &format!("Shown artists: {}", model.subsonic.borrow().artists().len()),
                                },
                                append: shown_albums = &gtk::Label {
                                    set_halign: gtk::Align::Start,
                                    set_text: &format!("Shown albums: {}", model.subsonic.borrow().albums().len()),
                                },

                                gtk::Box {
                                    set_spacing: 15,

                                    #[name = "append_to_queue"]
                                    gtk::Button {
                                        gtk::Box {
                                            gtk::Image {
                                                set_icon_name: Some("list-add-symbolic"),
                                            },
                                            gtk::Label {
                                                set_label: "Append",
                                            }
                                        },
                                        connect_clicked => TracksViewIn::AppendToQueue,
                                    },
                                    #[name = "add_to_queue"]
                                    gtk::Button {
                                        gtk::Box {
                                            gtk::Image {
                                                set_icon_name: Some("list-add-symbolic"),
                                            },
                                            gtk::Label {
                                                set_label: "Play next"
                                            }
                                        },
                                        connect_clicked => TracksViewIn::AddToQueue,
                                    },
                                    #[name = "replace_queue"]
                                    gtk::Button {
                                        gtk::Box {
                                            gtk::Image {
                                                set_icon_name: Some("emblem-symbolic-link-symbolic"),
                                            },
                                            gtk::Label {
                                                set_label: "Replace queue",
                                            }
                                        },
                                        connect_clicked => TracksViewIn::ReplaceQueue,
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
                                        set_tooltip: "Click to select a folder to download shown tracks to",
                                        connect_clicked => TracksViewIn::DownloadClicked,
                                    }
                                }
                            }
                        }
                    },
                    gtk::ScrolledWindow {
                        set_hexpand: true,
                        set_vexpand: true,

                        model.tracks.view.clone() {
                            add_css_class: "tracks-view-tracks-row",
                            set_vexpand: true,
                            set_single_click_activate: true,

                            connect_activate[sender] => move |_column_view, index| {
                                sender.input(TracksViewIn::TrackClicked(index));
                            }
                        }
                    }
                },
            },

            // filters
            append: filters = &gtk::Revealer {
                set_transition_duration: 200,
                set_transition_type: gtk::RevealerTransitionType::SlideLeft,

                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_size_request: (400, -1),

                    gtk::WindowHandle {
                        gtk::Label {
                            add_css_class: granite::STYLE_CLASS_H2_LABEL,
                            set_text: "Active Filters",
                        }
                    },

                    model.filters.widget().clone() -> gtk::ListBox {
                        set_margin_all: 5,
                        add_css_class: granite::STYLE_CLASS_FRAME,
                        add_css_class: granite::STYLE_CLASS_RICH_LIST,
                        set_vexpand: true,
                        set_selection_mode: gtk::SelectionMode::None,

                        // display new filter button
                        gtk::ListBoxRow {
                            set_focusable: false,
                            set_valign: gtk::Align::Center,


                            gtk::Box {
                                set_spacing: 15,
                                set_halign: gtk::Align::Center,

                                gtk::Label {
                                    set_text: "New filter:",
                                },

                                #[name = "new_filter"]
                                gtk::DropDown {
                                    set_model: Some(&Category::tracks()),
                                    set_factory: Some(&Category::factory()),
                                },

                                gtk::Button {
                                    set_icon_name: "list-add-symbolic",
                                    connect_clicked => Self::Input::FilterAdd,
                                }
                            }
                        },
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
            TracksViewIn::Favorited(id, state) => {
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
            TracksViewIn::FilterChanged => {
                self.calc_sensitivity_of_buttons(widgets);

                self.shown_tracks.borrow_mut().clear();
                self.shown_artists.borrow_mut().clear();
                self.shown_albums.borrow_mut().clear();
                let shown_tracks = self.shown_tracks.clone();
                let shown_albums = self.shown_albums.clone();
                let shown_artists = self.shown_artists.clone();
                let shown_tracks_widget = widgets.shown_tracks.clone();
                let shown_artists_widget = widgets.shown_artists.clone();
                let shown_albums_widget = widgets.shown_albums.clone();
                shown_tracks_widget
                    .set_text(&format!("Shown tracks: {}", shown_tracks.borrow().len()));
                shown_artists_widget
                    .set_text(&format!("Shown artists: {}", shown_artists.borrow().len()));
                shown_albums_widget
                    .set_text(&format!("Shown albums: {}", shown_albums.borrow().len()));

                self.tracks.pop_filter();
                let filters: Vec<Filter> = self
                    .filters
                    .iter()
                    .filter_map(|row| row.filter().as_ref())
                    .cloned()
                    .collect();
                if (filters.is_empty() || !widgets.filters.reveals_child())
                    && !Settings::get().lock().unwrap().search_active
                {
                    shown_tracks_widget.set_text(&format!("Shown tracks: {}", self.tracks.len()));
                    shown_artists_widget.set_text(&format!(
                        "Shown artists: {}",
                        self.subsonic.borrow().artists().len()
                    ));
                    shown_albums_widget.set_text(&format!(
                        "Shown albums: {}",
                        self.subsonic.borrow().albums().len()
                    ));
                    return;
                }

                self.tracks.add_filter(move |track| {
                    let mut search = Settings::get().lock().unwrap().search_text.clone();
                    let title = track.item.title.clone();

                    for filter in &filters {
                        match filter {
                            //TODO add matching for regular expressions
                            Filter::Favorite(None) => {}
                            Filter::Favorite(Some(state)) => {
                                if *state != track.item.starred.is_some() {
                                    return false;
                                }
                            }
                            Filter::Title(_, value) if value.is_empty() => {}
                            Filter::Title(relation, value) => match relation {
                                TextRelation::ExactNot if value == &track.item.title => {
                                    return false
                                }
                                TextRelation::Exact if value != &track.item.title => return false,
                                TextRelation::ContainsNot if track.item.title.contains(value) => {
                                    return false
                                }
                                TextRelation::Contains if !track.item.title.contains(value) => {
                                    return false
                                }
                                _ => {}
                            },
                            Filter::Album(_, value) if value.is_empty() => {}
                            Filter::Album(relation, value) => match relation {
                                TextRelation::ExactNot
                                    if Some(value) == track.item.album.as_ref() =>
                                {
                                    return false
                                }
                                TextRelation::Exact if Some(value) != track.item.album.as_ref() => {
                                    return false
                                }
                                TextRelation::ContainsNot => {
                                    if let Some(album) = &track.item.album {
                                        if album.contains(value) {
                                            return false;
                                        }
                                    }
                                }
                                TextRelation::Contains => {
                                    if let Some(album) = &track.item.album {
                                        if !album.contains(value) {
                                            return false;
                                        }
                                    } else {
                                        return false;
                                    }
                                }
                                _ => {}
                            },
                            Filter::Artist(_, value) if value.is_empty() => {}
                            Filter::Artist(relation, value) => match relation {
                                TextRelation::ExactNot
                                    if Some(value) == track.item.artist.as_ref() =>
                                {
                                    return false
                                }
                                TextRelation::Exact
                                    if Some(value) != track.item.artist.as_ref() =>
                                {
                                    return false
                                }
                                TextRelation::ContainsNot => {
                                    if let Some(artist) = &track.item.artist {
                                        if artist.contains(value) {
                                            return false;
                                        }
                                    }
                                }
                                TextRelation::Contains => {
                                    if let Some(artist) = &track.item.artist {
                                        if !artist.contains(value) {
                                            return false;
                                        }
                                    } else {
                                        return false;
                                    }
                                }
                                _ => {}
                            },
                            Filter::Year(order, value) => {
                                if let Some(year) = &track.item.year {
                                    if year.cmp(value) != *order {
                                        return false;
                                    }
                                } else {
                                    return false;
                                }
                            }
                            Filter::Cd(order, value) => {
                                if let Some(disc) = &track.item.disc_number {
                                    if disc.cmp(value) != *order {
                                        return false;
                                    }
                                } else {
                                    return false;
                                }
                            }
                            Filter::Genre(_, value) if value.is_empty() => {}
                            Filter::Genre(relation, value) => match relation {
                                TextRelation::ExactNot
                                    if Some(value) == track.item.genre.as_ref() =>
                                {
                                    return false
                                }
                                TextRelation::Exact if Some(value) != track.item.genre.as_ref() => {
                                    return false
                                }
                                TextRelation::ContainsNot => {
                                    if let Some(genre) = &track.item.genre {
                                        if genre.contains(value) {
                                            return false;
                                        }
                                    }
                                }
                                TextRelation::Contains => {
                                    if let Some(genre) = &track.item.genre {
                                        if !genre.contains(value) {
                                            return false;
                                        }
                                    } else {
                                        return false;
                                    }
                                }
                                _ => {}
                            },
                            Filter::Duration(order, value) => {
                                if let Some(duration) = &track.item.duration {
                                    if duration.cmp(value) != *order {
                                        return false;
                                    }
                                } else {
                                    return false;
                                }
                            }
                            Filter::BitRate(order, value) => {
                                if let Some(bitrate) = &track.item.bit_rate {
                                    if bitrate.cmp(&(*value as i32)) != *order {
                                        return false;
                                    }
                                } else {
                                    return false;
                                }
                            }
                            _ => unreachable!("there are filters that shouldnt be"),
                        }
                    }

                    // when search bar is hidden every element will be shown
                    if !Settings::get().lock().unwrap().search_active {
                        shown_tracks.borrow_mut().push(track.item.clone());
                        shown_artists.borrow_mut().insert(track.item.artist.clone());
                        shown_albums.borrow_mut().insert(track.item.album.clone());
                        shown_tracks_widget
                            .set_text(&format!("Shown tracks: {}", shown_tracks.borrow().len()));
                        shown_artists_widget
                            .set_text(&format!("Shown artists: {}", shown_artists.borrow().len()));
                        shown_albums_widget
                            .set_text(&format!("Shown albums: {}", shown_albums.borrow().len()));
                        return true;
                    }

                    let mut title_artist_album = format!(
                        "{title} {} {}",
                        track.item.artist.clone().unwrap_or_default(),
                        track.item.album.clone().unwrap_or_default()
                    );
                    //check for case sensitivity
                    if !Settings::get().lock().unwrap().case_sensitive {
                        title_artist_album = title_artist_album.to_lowercase();
                        search = search.to_lowercase();
                    }

                    //actual matching
                    let fuzzy_search = Settings::get().lock().unwrap().fuzzy_search;
                    if fuzzy_search {
                        let matcher = fuzzy_matcher::skim::SkimMatcherV2::default();
                        let score = matcher.fuzzy_match(&title_artist_album, &search);
                        if score.is_some() {
                            shown_tracks.borrow_mut().push(track.item.clone());
                            shown_artists.borrow_mut().insert(track.item.artist.clone());
                            shown_albums.borrow_mut().insert(track.item.album.clone());
                            shown_tracks_widget.set_text(&format!(
                                "Shown tracks: {}",
                                shown_tracks.borrow().len()
                            ));
                            shown_artists_widget.set_text(&format!(
                                "Shown artists: {}",
                                shown_artists.borrow().len()
                            ));
                            shown_albums_widget.set_text(&format!(
                                "Shown albums: {}",
                                shown_albums.borrow().len()
                            ));
                            true
                        } else {
                            false
                        }
                    } else if title_artist_album.contains(&search) {
                        shown_tracks.borrow_mut().push(track.item.clone());
                        shown_artists.borrow_mut().insert(track.item.artist.clone());
                        shown_albums.borrow_mut().insert(track.item.album.clone());
                        shown_tracks_widget
                            .set_text(&format!("Shown tracks: {}", shown_tracks.borrow().len()));
                        shown_artists_widget
                            .set_text(&format!("Shown artists: {}", shown_artists.borrow().len()));
                        shown_albums_widget
                            .set_text(&format!("Shown albums: {}", shown_albums.borrow().len()));
                        true
                    } else {
                        false
                    }
                });
            }
            TracksViewIn::FilterAdd => {
                use glib::object::Cast;

                let list_item = widgets.new_filter.selected_item().unwrap();
                let boxed = list_item
                    .downcast_ref::<glib::BoxedAnyObject>()
                    .expect("is not a BoxedAnyObject");
                let category: std::cell::Ref<Category> = boxed.borrow();

                let index = self.filters.guard().push_back(category.clone());
                self.filters
                    .send(index.current_index(), FilterRowIn::SetTo(category.clone()));
                sender.input(TracksViewIn::FilterChanged);
            }
            TracksViewIn::FilterRow(msg) => match msg {
                FilterRowOut::RemoveFilter(index) => {
                    self.filters.guard().remove(index.current_index());
                    sender.input(TracksViewIn::FilterChanged);
                }
                FilterRowOut::ParameterChanged => sender.input(TracksViewIn::FilterChanged),
            },
            TracksViewIn::Cover(msg) => match msg {
                CoverOut::DisplayToast(msg) => {
                    sender.output(TracksViewOut::DisplayToast(msg)).unwrap();
                }
            },
            TracksViewIn::AddToQueue => {
                if self.active_filters() {
                    if self.shown_tracks.borrow().is_empty() {
                        return;
                    }
                    let drop = Droppable::Queue(self.shown_tracks.borrow().clone());
                    sender.output(TracksViewOut::AddToQueue(drop)).unwrap();
                } else {
                    let tracks = self.subsonic.borrow().tracks().clone();
                    let drop = Droppable::Queue(tracks);
                    sender.output(TracksViewOut::AddToQueue(drop)).unwrap();
                }
            }
            TracksViewIn::AppendToQueue => {
                if self.active_filters() {
                    if self.shown_tracks.borrow().is_empty() {
                        return;
                    }
                    let drop = Droppable::Queue(self.shown_tracks.borrow().clone());
                    sender.output(TracksViewOut::AppendToQueue(drop)).unwrap();
                } else {
                    let tracks = self.subsonic.borrow().tracks().clone();
                    let drop = Droppable::Queue(tracks);
                    sender.output(TracksViewOut::AddToQueue(drop)).unwrap();
                }
            }
            TracksViewIn::ReplaceQueue => {
                if self.active_filters() {
                    if self.shown_tracks.borrow().is_empty() {
                        return;
                    }
                    let drop = Droppable::Queue(self.shown_tracks.borrow().clone());
                    sender.output(TracksViewOut::ReplaceQueue(drop)).unwrap();
                } else {
                    let tracks = self.subsonic.borrow().tracks().clone();
                    let drop = Droppable::Queue(tracks);
                    sender.output(TracksViewOut::AddToQueue(drop)).unwrap();
                }
            }
            TracksViewIn::DownloadClicked => {
                if self.shown_tracks.borrow().is_empty() {
                    return;
                }
                let drop = Droppable::Queue(self.shown_tracks.borrow().clone());
                sender.output(TracksViewOut::Download(drop)).unwrap();
            }
            TracksViewIn::ToggleFilters => {
                sender.input(TracksViewIn::FilterChanged);
                widgets
                    .filters
                    .set_reveal_child(!widgets.filters.reveals_child());
            }
            TracksViewIn::TrackClicked(index) => {
                if let Some(track) = self.tracks.get(index) {
                    self.info_cover
                        .emit(CoverIn::LoadSong(Box::new(track.borrow().item.clone())));
                }
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
            TracksViewCmd::AddTracks(tracks) => {
                for track in tracks {
                    let track = TrackRow::new_track(&self.subsonic, track, sender.clone());
                    self.tracks.append(track);
                }
            }
            TracksViewCmd::LoadingTracksFinished => {
                widgets.spinner.set_visible(false);
            }
        }
    }
}
