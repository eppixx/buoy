use std::{cell::RefCell, collections::HashSet, rc::Rc};

use fuzzy_matcher::FuzzyMatcher;
use relm4::{
    gtk::{
        self, glib,
        prelude::{BoxExt, ButtonExt, ListModelExt, OrientableExt, WidgetExt},
    },
    RelmWidgetExt,
};

use crate::{
    components::{filter_categories::Category, filter_row::FilterRowIn},
    factory::artist_row::{AlbumCountColumn, ArtistRow, CoverColumn, FavColumn, TitleColumn},
    settings::Settings,
    subsonic::Subsonic,
};

use super::{
    cover::CoverOut,
    filter_row::{Filter, FilterRow, FilterRowOut, TextRelation},
};

#[derive(Debug)]
pub struct ArtistsView {
    subsonic: Rc<RefCell<Subsonic>>,
    filters: relm4::factory::FactoryVecDeque<FilterRow>,
    entries: relm4::typed_view::column::TypedColumnView<ArtistRow, gtk::SingleSelection>,
    shown_artists: Rc<RefCell<HashSet<String>>>,
}

impl ArtistsView {
    fn active_filters(&self) -> bool {
        self.filters.iter().any(|f| f.active())
    }

    fn calc_sensitivity_of_buttons(&self, widgets: &<ArtistsView as relm4::Component>::Widgets) {
        let allowed_queue_modifier_len = 1000;

        if (!self.active_filters() && self.entries.len() >= allowed_queue_modifier_len)
            || (self.active_filters()
                && self.shown_artists.borrow().len() >= allowed_queue_modifier_len as usize)
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
pub enum ArtistsViewOut {
    ClickedArtist(submarine::data::ArtistId3),
    DisplayToast(String),
    FavoriteClicked(String, bool),
}

#[derive(Debug)]
pub enum ArtistsViewIn {
    FilterChanged,
    Favorited(String, bool),
    Cover(CoverOut),
    FilterRow(FilterRowOut),
    FilterAdd,
    AppendToQueue,
    AddToQueue,
    ReplaceQueue,
    ArtistClicked(u32),
}

#[relm4::component(pub)]
impl relm4::component::Component for ArtistsView {
    type Init = Rc<RefCell<Subsonic>>;
    type Input = ArtistsViewIn;
    type Output = ArtistsViewOut;
    type CommandOutput = ();

    fn init(
        init: Self::Init,
        root: Self::Root,
        sender: relm4::ComponentSender<Self>,
    ) -> relm4::component::ComponentParts<Self> {
        let mut entries =
            relm4::typed_view::column::TypedColumnView::<ArtistRow, gtk::SingleSelection>::new();
        entries.append_column::<CoverColumn>();
        entries.append_column::<TitleColumn>();
        entries.append_column::<AlbumCountColumn>();
        entries.append_column::<FavColumn>();

        let mut model = Self {
            subsonic: init.clone(),
            entries,
            filters: relm4::factory::FactoryVecDeque::builder()
                .launch(gtk::ListBox::default())
                .forward(sender.input_sender(), Self::Input::FilterRow),
            shown_artists: Rc::new(RefCell::new(HashSet::new())),
        };

        // add artists with cover and title
        for artist in init.borrow().artists().iter() {
            model
                .entries
                .append(ArtistRow::new(&init, artist.clone(), sender.clone()));
        }

        model.filters.guard().push_back(Category::Favorite);
        let widgets = view_output!();
        relm4::component::ComponentParts { model, widgets }
    }

    view! {
        gtk::Box {
            //filters
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
                            set_valign: gtk::Align::Center,


                            gtk::Box {
                                set_spacing: 15,
                                set_halign: gtk::Align::Center,

                                gtk::Label {
                                    set_text: "New filter:",
                                },

                                #[name = "new_filter"]
                                gtk::DropDown {
                                    set_model: Some(&Category::artists()),
                                    set_factory: Some(&Category::factory()),
                                },

                                gtk::Button {
                                    set_icon_name: "list-add-symbolic",
                                    connect_clicked => ArtistsViewIn::FilterAdd,
                                }
                            }
                        },
                    }
                }
            },
            gtk::Box {
                add_css_class: "tracks-view",
                set_orientation: gtk::Orientation::Vertical,

                gtk::WindowHandle {
                    gtk::Label {
                        add_css_class: "h2",
                        set_label: "Artists",
                        set_halign: gtk::Align::Center,
                    }
                },

                // info
                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_spacing: 8,

                    gtk::WindowHandle {
                        gtk::Box {
                            set_spacing: 15,

                            //tracks info
                            gtk::Box {
                                set_orientation: gtk::Orientation::Vertical,
                                set_spacing: 8,

                                append: shown_artists = &gtk::Label {
                                    set_halign: gtk::Align::Start,
                                    set_text: &format!("Shown artists: {}", model.entries.len()),
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
                                        connect_clicked => ArtistsViewIn::AppendToQueue,
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
                                        connect_clicked => ArtistsViewIn::AddToQueue,
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
                                        connect_clicked => ArtistsViewIn::ReplaceQueue,
                                    }
                                }
                            }
                        }
                    },
                    gtk::ScrolledWindow {
                        set_hexpand: true,
                        set_vexpand: true,

                        model.entries.view.clone() {
                            add_css_class: "album-view-tracks-row",
                            set_vexpand: true,
                            set_single_click_activate: true,

                            connect_activate[sender] => move |_column_view, index| {
                                sender.input(ArtistsViewIn::ArtistClicked(index))
                            },
                        }
                    }
                },
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
            ArtistsViewIn::FilterChanged => {
                self.calc_sensitivity_of_buttons(widgets);

                let update_label = |label: &gtk::Label, counter: &Rc<RefCell<HashSet<String>>>| {
                    label.set_text(&format!("Shown artists: {}", counter.borrow().len()));
                };

                self.shown_artists.borrow_mut().clear();
                let shown_artists = self.shown_artists.clone();
                let shown_artists_widget = widgets.shown_artists.clone();
                update_label(&shown_artists_widget, &shown_artists);

                self.entries.pop_filter();
                let filters: Vec<Filter> = self
                    .filters
                    .iter()
                    .filter_map(|row| row.filter().as_ref())
                    .cloned()
                    .collect();
                if filters.is_empty() && !Settings::get().lock().unwrap().search_active {
                    update_label(&shown_artists_widget, &shown_artists);
                    return;
                }

                self.entries.add_filter(move |track| {
                    let mut search = Settings::get().lock().unwrap().search_text.clone();
                    for filter in &filters {
                        match filter {
                            //TODO add matching for regular expressions
                            Filter::Favorite(None) => {}
                            Filter::Favorite(Some(state)) => {
                                if *state != track.item.starred.is_some() {
                                    return false;
                                }
                            }
                            Filter::Artist(_, value) if value.is_empty() => {}
                            Filter::Artist(relation, value) => match relation {
                                TextRelation::ExactNot if value == &track.item.name => {
                                    return false
                                }
                                TextRelation::Exact if value != &track.item.name => return false,
                                TextRelation::ContainsNot if track.item.name.contains(value) => {
                                    return false
                                }
                                TextRelation::Contains if !track.item.name.contains(value) => {
                                    return false
                                }
                                _ => {}
                            },
                            Filter::AlbumCount(order, value) => {
                                if track.item.album_count.cmp(value) != *order {
                                    return false;
                                }
                            }
                            _ => unreachable!("there are filters that shouldnt be"),
                        }
                    }

                    // when search bar is hidden every element will be shown
                    if !Settings::get().lock().unwrap().search_active {
                        shown_artists.borrow_mut().insert(track.item.name.clone());
                        update_label(&shown_artists_widget, &shown_artists);
                        return true;
                    }

                    let mut artist = track.item.name.clone();
                    //check for case sensitivity
                    if !Settings::get().lock().unwrap().case_sensitive {
                        artist = artist.to_lowercase();
                        search = search.to_lowercase();
                    }

                    //actual matching
                    let fuzzy_search = Settings::get().lock().unwrap().fuzzy_search;
                    if fuzzy_search {
                        let matcher = fuzzy_matcher::skim::SkimMatcherV2::default();
                        let score = matcher.fuzzy_match(&artist, &search);
                        if score.is_some() {
                            shown_artists.borrow_mut().insert(track.item.name.clone());
                            update_label(&shown_artists_widget, &shown_artists);
                            true
                        } else {
                            false
                        }
                    } else if artist.contains(&search) {
                        shown_artists.borrow_mut().insert(track.item.name.clone());
                        update_label(&shown_artists_widget, &shown_artists);
                        true
                    } else {
                        false
                    }
                });
            }
            ArtistsViewIn::Favorited(id, state) => {
                let len = self.entries.view.columns().n_items();
                (0..len)
                    .filter_map(|i| self.entries.get(i))
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
            ArtistsViewIn::Cover(msg) => match msg {
                CoverOut::DisplayToast(msg) => {
                    sender.output(ArtistsViewOut::DisplayToast(msg)).unwrap()
                }
            },
            ArtistsViewIn::FilterAdd => {
                use glib::object::Cast;

                let list_item = widgets.new_filter.selected_item().unwrap();
                let boxed = list_item
                    .downcast_ref::<glib::BoxedAnyObject>()
                    .expect("is not a BoxedAnyObject");
                let category: std::cell::Ref<Category> = boxed.borrow();

                let index = self.filters.guard().push_back(category.clone());
                self.filters
                    .send(index.current_index(), FilterRowIn::SetTo(category.clone()));
                sender.input(ArtistsViewIn::FilterChanged);
            }
            ArtistsViewIn::FilterRow(msg) => match msg {
                FilterRowOut::RemoveFilter(index) => {
                    self.filters.guard().remove(index.current_index());
                    sender.input(ArtistsViewIn::FilterChanged);
                }
                FilterRowOut::ParameterChanged => sender.input(ArtistsViewIn::FilterChanged),
            },
            ArtistsViewIn::AddToQueue => {}
            ArtistsViewIn::AppendToQueue => {}
            ArtistsViewIn::ReplaceQueue => {}
            ArtistsViewIn::ArtistClicked(index) => {
                if let Some(clicked_artist) = self.entries.get(index) {
                    sender
                        .output(ArtistsViewOut::ClickedArtist(
                            clicked_artist.borrow().item.clone(),
                        ))
                        .unwrap();
                }
            }
        }
    }
}
