use std::{cell::RefCell, collections::HashSet, rc::Rc};

use fuzzy_matcher::FuzzyMatcher;
use gettextrs::gettext;
use relm4::{
    gtk::{
        self, glib,
        prelude::{BoxExt, ButtonExt, ListModelExt, OrientableExt, SelectionModelExt, WidgetExt},
    },
    ComponentController, RelmWidgetExt,
};

use crate::{
    components::{
        cover::{Cover, CoverIn},
        filter_row::{Filter, FilterRow, FilterRowOut, TextRelation},
    },
    factory::track_row::{BitRateColumn, GenreColumn},
    types::{Droppable, Id},
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
    tracks: relm4::typed_view::column::TypedColumnView<TrackRow, gtk::MultiSelection>,
    filters: relm4::factory::FactoryVecDeque<FilterRow>,

    info_cover: relm4::Controller<Cover>,
    shown_tracks: Rc<RefCell<Vec<String>>>,
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
                .set_tooltip(&gettext("There are too many tracks to add to queue"));
            widgets.append_to_queue.set_sensitive(false);
            widgets
                .append_to_queue
                .set_tooltip(&gettext("There are too many tracks to append to queue"));
            widgets.replace_queue.set_sensitive(false);
            widgets
                .replace_queue
                .set_tooltip(&gettext("There are too many tracks to replace queue"));
        } else {
            widgets.add_to_queue.set_sensitive(true);
            widgets
                .add_to_queue
                .set_tooltip(&gettext("Append shown tracks to end of queue"));
            widgets.append_to_queue.set_sensitive(true);
            widgets.append_to_queue.set_tooltip(&gettext(
                "Insert shown after currently played or paused item",
            ));
            widgets.replace_queue.set_sensitive(true);
            widgets
                .replace_queue
                .set_tooltip(&gettext("Replaces current queue with shown tracks"));
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
    RecalcDragSource,
}

#[derive(Debug)]
pub enum TracksViewOut {
    DisplayToast(String),
    AddToQueue(Droppable),
    AppendToQueue(Droppable),
    ReplaceQueue(Droppable),
    Download(Droppable),
    FavoriteClicked(String, bool),
    ClickedArtist(Id),
    ClickedAlbum(Id),
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
        let mut tracks =
            relm4::typed_view::column::TypedColumnView::<TrackRow, gtk::MultiSelection>::new();
        tracks.append_column::<PositionColumn>();
        tracks.append_column::<TitleColumn>();
        tracks.append_column::<ArtistColumn>();
        tracks.append_column::<AlbumColumn>();
        tracks.append_column::<GenreColumn>();
        tracks.append_column::<LengthColumn>();
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
            .get("Album")
            .unwrap()
            .set_title(Some(&gettext("Album")));
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

        let mut model = Self {
            subsonic: subsonic.clone(),
            tracks,
            filters: relm4::factory::FactoryVecDeque::builder()
                .launch(gtk::ListBox::default())
                .forward(sender.input_sender(), Self::Input::FilterRow),
            info_cover: Cover::builder()
                .launch((subsonic.clone(), None))
                .forward(sender.input_sender(), TracksViewIn::Cover),
            shown_tracks: Rc::new(RefCell::new(Vec::with_capacity(
                subsonic.borrow().tracks().len(),
            ))),
            shown_artists: Rc::new(RefCell::new(HashSet::new())),
            shown_albums: Rc::new(RefCell::new(HashSet::new())),
        };
        model.info_cover.model().add_css_class_image("size100");

        // add tracks
        let list = subsonic.borrow().tracks().to_vec();
        let tracks: Vec<TrackRow> = list
            .iter()
            .map(|track| {
                model.shown_tracks.borrow_mut().push(track.id.clone());
                model.shown_albums.borrow_mut().insert(track.album.clone());
                model.shown_artists.borrow_mut().insert(track.artist.clone());
                TrackRow::new(&model.subsonic, track.clone(), &sender)
            })
            .collect();
        model.tracks.extend_from_iter(tracks);

        let widgets = view_output!();

        //update labels and buttons
        set_count_labels(
            &model.shown_tracks,
            &widgets.shown_tracks,
            &model.shown_albums,
            &widgets.shown_albums,
            &model.shown_artists,
            &widgets.shown_artists,
        );

        model.filters.guard().push_back(Category::Favorite);
        model.calc_sensitivity_of_buttons(&widgets);

        // send signal on selection change
        model
            .tracks
            .selection_model
            .connect_selection_changed(move |_selection_model, _x, _y| {
                sender.input(TracksViewIn::RecalcDragSource);
            });

        relm4::ComponentParts { model, widgets }
    }

    view! {
        gtk::Box {
            // tracks
            gtk::Box {
                add_css_class: "tracks-view",
                set_orientation: gtk::Orientation::Vertical,

                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_spacing: 8,

                    // info
                    gtk::WindowHandle {
                        gtk::CenterBox {
                            #[wrap(Some)]
                            set_start_widget = &gtk::Box {
                                set_spacing: 15,
                                set_margin_horizontal: 7,

                                model.info_cover.widget().clone() -> gtk::Box {},

                                //tracks info
                                gtk::Box {
                                    set_orientation: gtk::Orientation::Vertical,
                                    set_spacing: 8,

                                    append: shown_tracks = &gtk::Label {
                                        set_halign: gtk::Align::Start,
                                        set_text: &format!("{}: {}", gettext("Shown tracks"), model.shown_tracks.borrow().len()),
                                    },
                                    append: shown_artists = &gtk::Label {
                                        set_halign: gtk::Align::Start,
                                        set_text: &format!("{}: {}", gettext("Shown artists"), model.shown_artists.borrow().len()),
                                    },
                                    append: shown_albums = &gtk::Label {
                                        set_halign: gtk::Align::Start,
                                        set_text: &format!("{}: {}", gettext("Shown albums"), model.shown_albums.borrow().len()),
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
                                                    set_label: &gettext("Append"),
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
                                                    set_label: &gettext("Play next"),
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
                                                    set_label: &gettext("Replace queue"),
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
                                                    set_label: &gettext("Download Tracks"),
                                                }
                                            },
                                            set_tooltip: &gettext("Click to select a folder to download shown tracks to"),
                                            connect_clicked => TracksViewIn::DownloadClicked,
                                        }
                                    }
                                }
                            },
                            #[wrap(Some)]
                            set_end_widget = &gtk::Box {
                                set_spacing: 10,
                                set_margin_end: 10,

                                append: spinner = &gtk::Spinner {
                                    set_spinning: true,
                                    start: (),
                                },
                                gtk::Label {
                                    set_text: &gettext("Filters:"),
                                },
                                gtk::Switch {
                                    set_valign: gtk::Align::Center,
                                    connect_active_notify => TracksViewIn::ToggleFilters,
                                }
                            }
                        }
                    },
                    gtk::ScrolledWindow {
                        set_hexpand: true,
                        set_vexpand: true,

                        model.tracks.view.clone() {
                            set_widget_name: "tracks-view-tracks",
                            set_vexpand: true,

                            connect_activate[sender] => move |_column_view, index| {
                                sender.input(TracksViewIn::TrackClicked(index));
                            },

                            add_controller = gtk::DragSource {
                                connect_prepare[sender] => move |_drag_src, _x, _y| {
                                    sender.input(TracksViewIn::RecalcDragSource);
                                    None
                                }
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
                            set_text: &gettext("Active Filters"),
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
                                    set_text: &gettext("New filter:"),
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
                            if let Some(fav) = &track.borrow().fav_btn {
                                fav.set_icon_name("starred-symbolic");
                            }
                        }
                        false => {
                            track.borrow_mut().item.starred = None;
                            if let Some(fav) = &track.borrow().fav_btn {
                                fav.set_icon_name("non-starred-symbolic");
                            }
                        }
                    });
            }
            TracksViewIn::FilterChanged => {
                self.calc_sensitivity_of_buttons(widgets);

                let shown_tracks = self.shown_tracks.clone();
                let shown_albums = self.shown_albums.clone();
                let shown_artists = self.shown_artists.clone();
                let shown_tracks_widget = widgets.shown_tracks.clone();
                let shown_artists_widget = widgets.shown_artists.clone();
                let shown_albums_widget = widgets.shown_albums.clone();
                set_count_labels(
                    &shown_tracks,
                    &shown_tracks_widget,
                    &shown_albums,
                    &shown_albums_widget,
                    &shown_artists,
                    &shown_artists_widget,
                );

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
                    set_count_labels(
                        &shown_tracks,
                        &shown_tracks_widget,
                        &shown_albums,
                        &shown_albums_widget,
                        &shown_artists,
                        &shown_artists_widget,
                    );
                    return;
                }

                self.shown_tracks.borrow_mut().clear();
                self.shown_artists.borrow_mut().clear();
                self.shown_albums.borrow_mut().clear();

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
                            Filter::Title(_, value) if value.is_empty() => {} // filter matches
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
                                _ => {} // filter matches
                            },
                            Filter::Album(_, value) if value.is_empty() => {} // filter matches
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
                                _ => {} // filter matches
                            },
                            Filter::Artist(_, value) if value.is_empty() => {} // filter matches
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
                                _ => {} // filter matches
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
                                _ => {} // filter matches
                            },
                            Filter::DurationMin(order, value) => {
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
                        shown_tracks.borrow_mut().push(track.item.id.clone());
                        shown_artists.borrow_mut().insert(track.item.artist.clone());
                        shown_albums.borrow_mut().insert(track.item.album.clone());
                        set_count_labels(
                            &shown_tracks,
                            &shown_tracks_widget,
                            &shown_albums,
                            &shown_albums_widget,
                            &shown_artists,
                            &shown_artists_widget,
                        );
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
                            shown_tracks.borrow_mut().push(track.item.id.clone());
                            shown_artists.borrow_mut().insert(track.item.artist.clone());
                            shown_albums.borrow_mut().insert(track.item.album.clone());
                            set_count_labels(
                                &shown_tracks,
                                &shown_tracks_widget,
                                &shown_albums,
                                &shown_albums_widget,
                                &shown_artists,
                                &shown_artists_widget,
                            );
                            true
                        } else {
                            false
                        }
                    } else if title_artist_album.contains(&search) {
                        shown_tracks.borrow_mut().push(track.item.id.clone());
                        shown_artists.borrow_mut().insert(track.item.artist.clone());
                        shown_albums.borrow_mut().insert(track.item.album.clone());
                        set_count_labels(
                            &shown_tracks,
                            &shown_tracks_widget,
                            &shown_albums,
                            &shown_albums_widget,
                            &shown_artists,
                            &shown_artists_widget,
                        );
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
                    let tracks = self
                        .shown_tracks
                        .borrow()
                        .iter()
                        .filter_map(|id| self.subsonic.borrow().find_track(id))
                        .collect();
                    let drop = Droppable::Queue(tracks);
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
                    let tracks = self
                        .shown_tracks
                        .borrow()
                        .iter()
                        .filter_map(|id| self.subsonic.borrow().find_track(id))
                        .collect();
                    let drop = Droppable::Queue(tracks);
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
                    let tracks = self
                        .shown_tracks
                        .borrow()
                        .iter()
                        .filter_map(|id| self.subsonic.borrow().find_track(id))
                        .collect();
                    let drop = Droppable::Queue(tracks);
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
                let tracks = self
                    .shown_tracks
                    .borrow()
                    .iter()
                    .filter_map(|id| self.subsonic.borrow().find_track(id))
                    .collect();
                let drop = Droppable::Queue(tracks);
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
            TracksViewIn::RecalcDragSource => {
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

fn set_count_labels(
    tracks: &Rc<RefCell<Vec<String>>>,
    track_label: &gtk::Label,
    albums: &Rc<RefCell<HashSet<Option<String>>>>,
    album_label: &gtk::Label,
    artists: &Rc<RefCell<HashSet<Option<String>>>>,
    artist_label: &gtk::Label,
) {
    track_label.set_text(&format!(
        "{}: {}",
        gettext("Shown tracks"),
        tracks.borrow().len()
    ));
    artist_label.set_text(&format!(
        "{}: {}",
        gettext("Shown artists"),
        artists.borrow().len()
    ));
    album_label.set_text(&format!(
        "{}: {}",
        gettext("Shown albums"),
        albums.borrow().len()
    ));
}
