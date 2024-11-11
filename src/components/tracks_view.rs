use std::{cell::RefCell, rc::Rc};

use fuzzy_matcher::FuzzyMatcher;
use relm4::{
    gtk::{
        self, glib,
        prelude::{BoxExt, ButtonExt, OrientableExt, WidgetExt},
    },
    RelmWidgetExt,
};

use crate::{
    components::filter_row::{Filter, FilterRow, FilterRowOut, TextRelation},
    factory::playlist_tracks_row::GenreColumn,
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

#[derive(Debug)]
pub struct TracksView {
    subsonic: Rc<RefCell<Subsonic>>,
    tracks: relm4::typed_view::column::TypedColumnView<PlaylistTracksRow, gtk::SingleSelection>,
    filters: relm4::factory::FactoryVecDeque<FilterRow>,
}

#[derive(Debug)]
pub enum TracksViewIn {
    FilterChanged,
    Favorited(String, bool),
    FilterAdd,
    FilterRow(FilterRowOut),
}

#[derive(Debug)]
pub enum TracksViewOut {}

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
        tracks.append_column::<FavColumn>();

        for track in subsonic.borrow().tracks() {
            tracks.append(PlaylistTracksRow::new(&subsonic, track.clone()));
        }

        let model = Self {
            subsonic,
            tracks,
            filters: relm4::factory::FactoryVecDeque::builder()
                .launch(gtk::ListBox::default())
                .forward(sender.input_sender(), Self::Input::FilterRow),
        };

        let widgets = view_output!();
        relm4::ComponentParts { model, widgets }
    }

    view! {
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

            gtk::Box {
                append: sidebar = &gtk::Box {
                    gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,

                        gtk::Label {
                            add_css_class: granite::STYLE_CLASS_H2_LABEL,
                            set_text: "Active Filters",
                        },

                        model.filters.widget().clone() -> gtk::ListBox {},

                        gtk::ListBox {
                            add_css_class: "tracks-view-filter-list",
                            add_css_class: granite::STYLE_CLASS_FRAME,
                            add_css_class: granite::STYLE_CLASS_RICH_LIST,
                            set_vexpand: true,
                            set_selection_mode: gtk::SelectionMode::None,

                            gtk::ListBoxRow {
                                gtk::Box {
                                    set_orientation: gtk::Orientation::Vertical,
                                    set_spacing: 8,

                                    gtk::Separator {},

                                    gtk::Box {
                                        set_spacing: 15,
                                        set_halign: gtk::Align::Center,

                                        gtk::Separator {},

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
                                }
                            }
                        },
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
                let model = self.tracks.selection_model.model().unwrap();
                // println!("{}", model.);
            }
            TracksViewIn::FilterChanged => {
                self.tracks.pop_filter();
                let filters: Vec<Filter> = self
                    .filters
                    .iter()
                    .filter_map(|row| row.filter().as_ref())
                    .cloned()
                    .collect();

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
                            _ => unreachable!("there are filters that shouldnt be"),
                        }
                    }

                    // respect favorite filter pressed
                    if favorite.is_active() && track.item.starred.is_none() {
                        return false;
                    }

                    // when search bar is hidden every element will be shown
                    if !Settings::get().lock().unwrap().search_active {
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
                        score.is_some()
                    } else {
                        title_artist_album.contains(&search)
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
        }
    }
}
