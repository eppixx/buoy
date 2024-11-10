use std::{cell::RefCell, rc::Rc};

use fuzzy_matcher::FuzzyMatcher;
use relm4::{
    gtk::{
        self,
        prelude::{BoxExt, ButtonExt, OrientableExt, WidgetExt},
    },
    RelmWidgetExt,
};

use crate::{
    components::filter_categories::Category,
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
    filter_add: gtk::DropDown,
}

#[derive(Debug)]
pub enum TracksViewIn {
    FilterChanged,
    FilterSidebar,
    Favorited(String, bool),
    FilterAdd,
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
        tracks.append_column::<LengthColumn>();
        tracks.append_column::<FavColumn>();

        for track in subsonic.borrow().tracks() {
            tracks.append(PlaylistTracksRow::new(&subsonic, track.clone()));
        }

        let model = Self {
            subsonic,
            tracks,
            filter_add: gtk::DropDown::default(),
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
                    set_start_widget = &gtk::Box {
                        set_spacing: 10,
                        set_margin_start: 10,
                        //prevent cutoff of "glow" when widget has focus
                        set_margin_top: 2,
                        set_margin_bottom: 2,

                        gtk::Box {
                            set_spacing: 5,

                            gtk::Label {
                                set_label: "Filter",
                            },
                            append: filter = &gtk::Switch {
                                set_active: false,
                                connect_state_notify => Self::Input::FilterSidebar,
                                set_tooltip: "Activate the sidebar to set filters",
                            }
                        }
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
                append: sidebar = &gtk::Revealer {
                    set_transition_type: gtk::RevealerTransitionType::SlideRight,

                    gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,

                        gtk::Label {
                            add_css_class: granite::STYLE_CLASS_H2_LABEL,
                            set_text: "Active Filters",
                        },

                        gtk::ListBox {
                            add_css_class: "tracks-view-filter-list",
                            add_css_class: granite::STYLE_CLASS_FRAME,
                            add_css_class: granite::STYLE_CLASS_RICH_LIST,
                            set_vexpand: true,
                            set_selection_mode: gtk::SelectionMode::None,

                            gtk::ListBoxRow {
                                add_css_class: "tracks.view-filter-add",
                                gtk::Box {
                                    set_spacing: 15,

                                    gtk::Label {
                                        set_text: "Field Album",
                                    },

                                    gtk::Button {
                                        set_valign: gtk::Align::Center,
                                        set_label: "==",
                                    },

                                    gtk::Entry {
                                    },

                                    gtk::Button {
                                        set_icon_name: "user-trash-symbolic",
                                    }
                                }
                            },

                            gtk::ListBoxRow {
                                gtk::Box {
                                    set_orientation: gtk::Orientation::Vertical,
                                    set_spacing: 8,

                                    gtk::Separator {},

                                    gtk::Box {
                                        set_spacing: 15,
                                        set_halign: gtk::Align::Center,

                                        gtk::Separator {},

                                        model.filter_add.clone() {
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
                todo!();
            }
            TracksViewIn::FilterChanged => {
                self.tracks.pop_filter();

                let favorite = widgets.favorite.clone();
                self.tracks.add_filter(move |track| {
                    let mut search = Settings::get().lock().unwrap().search_text.clone();
                    let title = track.item.title.clone();

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
            TracksViewIn::FilterSidebar => {
                if widgets.sidebar.reveals_child() {
                    widgets.sidebar.set_reveal_child(false);
                    sender.input(TracksViewIn::FilterChanged);
                } else {
                    widgets.sidebar.set_reveal_child(true);
                    sender.input(TracksViewIn::FilterChanged);
                }
            }
            TracksViewIn::FilterAdd => {}
        }
    }
}
