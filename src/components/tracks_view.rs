use std::{cell::RefCell, collections::HashSet, rc::Rc};

use fuzzy_matcher::FuzzyMatcher;
use relm4::{
    gtk::{
        self, glib,
        prelude::{BoxExt, ButtonExt, ListModelExt, OrientableExt, WidgetExt},
    },
    ComponentController, RelmWidgetExt,
};

use crate::{
    components::{
        cover::Cover,
        filter_row::{Filter, FilterRow, FilterRowOut, TextRelation},
    },
    factory::playlist_tracks_row::{BitRateColumn, GenreColumn},
    types::Droppable,
};
use crate::{
    components::{filter_categories::Category, filter_row::FilterRowIn},
    factory::playlist_tracks_row::{
        AlbumColumn, ArtistColumn, FavColumn, LengthColumn, PlaylistTracksRow, PositionColumn,
        TitleColumn,
    },
    settings::Settings,
    subsonic::Subsonic,
};

use super::cover::CoverOut;

#[derive(Debug)]
pub struct TracksView {
    subsonic: Rc<RefCell<Subsonic>>,
    tracks: relm4::typed_view::column::TypedColumnView<PlaylistTracksRow, gtk::SingleSelection>,
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
}

#[derive(Debug)]
pub enum TracksViewOut {
    DisplayToast(String),
    AddToQueue(Droppable),
    AppendToQueue(Droppable),
    ReplaceQueue(Droppable),
    Download(Droppable),
}

#[relm4::component(pub)]
impl relm4::Component for TracksView {
    type Init = Rc<RefCell<Subsonic>>;
    type Input = TracksViewIn;
    type Output = TracksViewOut;
    type CommandOutput = ();

    fn init(
        subsonic: Self::Init,
        root: Self::Root,
        sender: relm4::ComponentSender<Self>,
    ) -> relm4::ComponentParts<Self> {
        let mut tracks = relm4::typed_view::column::TypedColumnView::<
            PlaylistTracksRow,
            gtk::SingleSelection,
        >::new();
        tracks.append_column::<PositionColumn>();
        tracks.append_column::<TitleColumn>();
        tracks.append_column::<ArtistColumn>();
        tracks.append_column::<AlbumColumn>();
        tracks.append_column::<GenreColumn>();
        tracks.append_column::<LengthColumn>();
        tracks.append_column::<BitRateColumn>();
        tracks.append_column::<FavColumn>();

        for track in subsonic.borrow().tracks() {
            tracks.append(PlaylistTracksRow::new(&subsonic, track.clone()));
        }

        let model = Self {
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
        model.calc_sensitivity_of_buttons(&widgets);
        relm4::ComponentParts { model, widgets }
    }

    view! {
        gtk::Box {
            // filters
            gtk::Box {
                append: sidebar = &gtk::Box {
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

                                gtk::Box {
                                    set_spacing: 15,
                                    set_halign: gtk::Align::Center,

                                    gtk::Label {
                                        set_text: "New filter:",
                                    },

                                    #[name = "new_filter"]
                                    gtk::DropDown {
                                        set_model: Some(&Category::all()),
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
            },

            // tracks
            gtk::Box {
                add_css_class: "tracks-view",
                set_orientation: gtk::Orientation::Vertical,

                gtk::WindowHandle {
                    gtk::CenterBox {
                        #[wrap(Some)]
                        set_center_widget = &gtk::Label {
                            add_css_class: "h2",
                            set_label: "Tracks",
                            set_halign: gtk::Align::Center,
                        },

                        #[wrap(Some)]
                        set_end_widget = &gtk::Box {
                            set_spacing: 10,
                            set_margin_end: 10,
                            //prevent cutoff of "glow" when widget has focus
                            set_margin_top: 2,
                            set_margin_bottom: 2,

                            gtk::Box {
                                set_spacing: 5,

                                gtk::Label {
                                    set_text: "Show only favorites:",
                                },
                                append: favorite = &gtk::Switch {
                                    set_active: false,
                                    connect_state_notify => Self::Input::FilterChanged,
                                    set_tooltip: "Toggle showing favortited artists",
                                }
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
                                    set_text: &format!("Shown Artists: {}", model.subsonic.borrow().artists().len()),
                                },
                                append: shown_albums = &gtk::Label {
                                    set_halign: gtk::Align::Start,
                                    set_text: &format!("Shown Albums: {}", model.subsonic.borrow().albums().len()),
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
                            add_css_class: "album-view-tracks-row",
                            set_vexpand: true,
                        }
                    }
                },
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
                let len = self.tracks.view.columns().n_items();
                (0..len)
                    .filter_map(|i| self.tracks.get(i))
                    .filter(|t| t.borrow().item.id == id)
                    .for_each(|track| match state {
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
                if filters.is_empty() {
                    shown_tracks_widget.set_text(&format!("Shown tracks: {}", self.tracks.len()));
                    shown_artists_widget.set_text(&format!(
                        "Shown tracks: {}",
                        self.subsonic.borrow().artists().len()
                    ));
                    shown_albums_widget.set_text(&format!(
                        "Shown albums: {}",
                        self.subsonic.borrow().albums().len()
                    ));
                    return;
                }

                let favorite = widgets.favorite.clone();
                self.tracks.add_filter(move |track| {
                    let mut search = Settings::get().lock().unwrap().search_text.clone();
                    let title = track.item.title.clone();

                    for filter in &filters {
                        match filter {
                            //TODO add matching for regular expressions
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

                    // respect favorite filter pressed
                    if favorite.is_active() && track.item.starred.is_none() {
                        return false;
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
                    sender.output(TracksViewOut::DisplayToast(msg)).unwrap()
                }
            },
            TracksViewIn::AddToQueue => {
                if !self.active_filters() {
                    let tracks = self.subsonic.borrow().tracks().clone();
                    let drop = Droppable::Queue(tracks);
                    sender.output(TracksViewOut::AddToQueue(drop)).unwrap();
                } else {
                    if self.shown_tracks.borrow().is_empty() {
                        return;
                    }
                    let drop = Droppable::Queue(self.shown_tracks.borrow().clone());
                    sender.output(TracksViewOut::AddToQueue(drop)).unwrap();
                }
            }
            TracksViewIn::AppendToQueue => {
                if !self.active_filters() {
                    let tracks = self.subsonic.borrow().tracks().clone();
                    let drop = Droppable::Queue(tracks);
                    sender.output(TracksViewOut::AddToQueue(drop)).unwrap();
                } else {
                    if self.shown_tracks.borrow().is_empty() {
                        return;
                    }
                    let drop = Droppable::Queue(self.shown_tracks.borrow().clone());
                    sender.output(TracksViewOut::AppendToQueue(drop)).unwrap();
                }
            }
            TracksViewIn::ReplaceQueue => {
                if !self.active_filters() {
                    let tracks = self.subsonic.borrow().tracks().clone();
                    let drop = Droppable::Queue(tracks);
                    sender.output(TracksViewOut::AddToQueue(drop)).unwrap();
                } else {
                    if self.shown_tracks.borrow().is_empty() {
                        return;
                    }
                    let drop = Droppable::Queue(self.shown_tracks.borrow().clone());
                    sender.output(TracksViewOut::ReplaceQueue(drop)).unwrap();
                }
            }
            TracksViewIn::DownloadClicked => {
                //TODO deactivate download button when shown tracks too much
                if self.shown_tracks.borrow().is_empty() {
                    return;
                }
                let drop = Droppable::Queue(self.shown_tracks.borrow().clone());
                sender.output(TracksViewOut::Download(drop)).unwrap();
            }
        }
    }
}
