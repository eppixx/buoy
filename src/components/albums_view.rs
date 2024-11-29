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
    components::{
        filter_categories::Category,
        filter_row::{Filter, FilterRowIn},
    },
    factory::album_row::{
        AlbumRow, ArtistColumn, CdColumn, CoverColumn, FavColumn, GenreColumn, LengthColumn,
        TitleColumn, YearColumn,
    },
    settings::Settings,
    subsonic::Subsonic,
    types::Droppable,
};

use super::{
    cover::CoverOut,
    filter_row::{FilterRow, FilterRowOut, TextRelation},
};

#[derive(Debug)]
pub struct AlbumsView {
    subsonic: Rc<RefCell<Subsonic>>,
    entries: relm4::typed_view::column::TypedColumnView<AlbumRow, gtk::SingleSelection>,
    filters: relm4::factory::FactoryVecDeque<FilterRow>,
    shown_artists: Rc<RefCell<HashSet<Option<String>>>>,
    shown_albums: Rc<RefCell<HashSet<Option<String>>>>,
}

impl AlbumsView {
    fn active_filters(&self) -> bool {
        self.filters.iter().any(|f| f.active())
    }

    fn calc_sensitivity_of_buttons(&self, widgets: &<AlbumsView as relm4::Component>::Widgets) {
        let allowed_queue_modifier_len = 10;

        if (!self.active_filters() && self.entries.len() >= allowed_queue_modifier_len)
            || (self.active_filters()
                && self.shown_artists.borrow().len() >= allowed_queue_modifier_len as usize)
        {
            widgets.add_to_queue.set_sensitive(false);
            widgets
                .add_to_queue
                .set_tooltip("There are too many albums to add to queue");
            widgets.append_to_queue.set_sensitive(false);
            widgets
                .append_to_queue
                .set_tooltip("There are too many albums to append to queue");
            widgets.replace_queue.set_sensitive(false);
            widgets
                .replace_queue
                .set_tooltip("There are too many albums to replace queue");
        } else {
            widgets.add_to_queue.set_sensitive(true);
            widgets
                .add_to_queue
                .set_tooltip("Append shown albums to end of queue");
            widgets.append_to_queue.set_sensitive(true);
            widgets
                .append_to_queue
                .set_tooltip("Insert shown albums after currently played or paused item");
            widgets.replace_queue.set_sensitive(true);
            widgets
                .replace_queue
                .set_tooltip("Replaces current queue with shown albums");
        }
    }
}

#[derive(Debug)]
pub enum AlbumsViewOut {
    ClickedAlbum(Box<submarine::data::Child>),
    DisplayToast(String),
    FavoriteClicked(String, bool),
    AddToQueue(Droppable),
    AppendToQueue(Droppable),
    ReplaceQueue(Droppable),
    ClickedArtist(String),
}

#[derive(Debug)]
pub enum AlbumsViewIn {
    FilterChanged,
    Favorited(String, bool),
    Cover(CoverOut),
    FilterRow(FilterRowOut),
    FilterAdd,
    AppendToQueue,
    AddToQueue,
    ReplaceQueue,
    ClickedAlbum(u32),
    ToggleFilters,
}

#[derive(Debug)]
pub enum AlbumsViewCmd {
    AddAlbums(Vec<submarine::data::Child>, usize),
}

#[relm4::component(pub)]
impl relm4::component::Component for AlbumsView {
    type Init = Rc<RefCell<Subsonic>>;
    type Input = AlbumsViewIn;
    type Output = AlbumsViewOut;
    type CommandOutput = AlbumsViewCmd;

    fn init(
        init: Self::Init,
        root: Self::Root,
        sender: relm4::ComponentSender<Self>,
    ) -> relm4::component::ComponentParts<Self> {
        let mut entries =
            relm4::typed_view::column::TypedColumnView::<AlbumRow, gtk::SingleSelection>::new();
        entries.append_column::<CoverColumn>();
        entries.append_column::<TitleColumn>();
        entries.append_column::<ArtistColumn>();
        entries.append_column::<GenreColumn>();
        entries.append_column::<LengthColumn>();
        entries.append_column::<YearColumn>();
        entries.append_column::<CdColumn>();
        entries.append_column::<FavColumn>();

        let mut model = Self {
            subsonic: init.clone(),
            entries,
            filters: relm4::factory::FactoryVecDeque::builder()
                .launch(gtk::ListBox::default())
                .forward(sender.input_sender(), Self::Input::FilterRow),
            shown_artists: Rc::new(RefCell::new(HashSet::new())),
            shown_albums: Rc::new(RefCell::new(HashSet::new())),
        };

        // add albums in chunks to not overwhelm the app
        let list = model.subsonic.borrow().albums().to_vec();
        sender.oneshot_command(async move { AlbumsViewCmd::AddAlbums(list, 0) });

        model.filters.guard().push_back(Category::Favorite);
        let widgets = view_output!();

        model.calc_sensitivity_of_buttons(&widgets);
        relm4::component::ComponentParts { model, widgets }
    }

    view! {
        gtk::Box {
            gtk::Box {
                add_css_class: "albums-view",
                set_orientation: gtk::Orientation::Vertical,

                gtk::WindowHandle {
                    gtk::CenterBox {
                        #[wrap(Some)]
                        set_center_widget = &gtk::Box {
                            set_spacing: 5,

                            gtk::Label {
                                add_css_class: "h2",
                                set_label: "Albums",
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
                                connect_active_notify => AlbumsViewIn::ToggleFilters,
                            }
                        }
                    }
                },

                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_spacing: 8,

                    // info
                    gtk::WindowHandle {
                        gtk::Box {
                            set_spacing: 15,
                            set_margin_horizontal: 7,

                            //tracks info
                            gtk::Box {
                                set_orientation: gtk::Orientation::Vertical,
                                set_spacing: 8,

                                append: shown_albums = &gtk::Label {
                                    set_halign: gtk::Align::Start,
                                    set_text: &format!("Shown albums: {}", model.shown_albums.borrow().len()),
                                },
                                append: shown_artists = &gtk::Label {
                                    set_halign: gtk::Align::Start,
                                    set_text: &format!("Shown artists: {}", model.shown_artists.borrow().len()),
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
                                        connect_clicked => AlbumsViewIn::AppendToQueue,
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
                                        connect_clicked => AlbumsViewIn::AddToQueue,
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
                                        connect_clicked => AlbumsViewIn::ReplaceQueue,
                                    }
                                }
                            }
                        }
                    },
                    gtk::ScrolledWindow {
                        set_hexpand: true,
                        set_vexpand: true,

                        model.entries.view.clone() {
                            add_css_class: "albums-view-tracks-row",
                            set_vexpand: true,
                            set_single_click_activate: true,

                            connect_activate[sender] => move |_column_view, index| {
                                sender.input(AlbumsViewIn::ClickedAlbum(index));
                            },
                        }
                    }
                }
            },

            //filters
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

                                append: new_filter = &gtk::DropDown {
                                    set_model: Some(&Category::albums()),
                                    set_factory: Some(&Category::factory()),
                                },

                                gtk::Button {
                                    set_icon_name: "list-add-symbolic",
                                    connect_clicked => AlbumsViewIn::FilterAdd,
                                }
                            }
                        },
                    }
                }
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
            AlbumsViewIn::FilterChanged => {
                self.calc_sensitivity_of_buttons(widgets);

                let update_label =
                    |label: &gtk::Label, name: &str, counter: &Rc<RefCell<HashSet<_>>>| {
                        label.set_text(&format!("Shown {name}: {}", counter.borrow().len()));
                    };

                let shown_albums = self.shown_albums.clone();
                let shown_artists = self.shown_artists.clone();
                let shown_artists_widget = widgets.shown_artists.clone();
                let shown_albums_widget = widgets.shown_albums.clone();
                update_label(&shown_artists_widget, "artists", &shown_artists);
                update_label(&shown_albums_widget, "albums", &shown_albums);

                self.entries.pop_filter();
                let filters: Vec<Filter> = self
                    .filters
                    .iter()
                    .filter_map(|row| row.filter().as_ref())
                    .cloned()
                    .collect();
                if (filters.is_empty() || !widgets.filters.reveals_child())
                    && !Settings::get().lock().unwrap().search_active
                {
                    update_label(&shown_artists_widget, "artists", &shown_artists);
                    update_label(&shown_albums_widget, "albums", &shown_albums);
                    return;
                }

                self.shown_artists.borrow_mut().clear();
                self.shown_albums.borrow_mut().clear();

                self.entries.add_filter(move |track| {
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
                            Filter::Cd(_, 0) => {
                                if track.item.disc_number.is_some() {
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
                                let value = value * 60;
                                if let Some(duration) = &track.item.duration {
                                    if duration.cmp(&value) != *order {
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
                        shown_artists.borrow_mut().insert(track.item.artist.clone());
                        shown_albums.borrow_mut().insert(track.item.album.clone());
                        update_label(&shown_artists_widget, "artists", &shown_artists);
                        update_label(&shown_albums_widget, "albums", &shown_albums);
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
                            shown_artists.borrow_mut().insert(track.item.artist.clone());
                            shown_albums.borrow_mut().insert(track.item.album.clone());
                            update_label(&shown_artists_widget, "artists", &shown_artists);
                            update_label(&shown_albums_widget, "albums", &shown_albums);
                            true
                        } else {
                            false
                        }
                    } else if title_artist_album.contains(&search) {
                        shown_artists.borrow_mut().insert(track.item.artist.clone());
                        shown_albums.borrow_mut().insert(track.item.album.clone());
                        update_label(&shown_artists_widget, "artists", &shown_artists);
                        update_label(&shown_albums_widget, "albums", &shown_albums);
                        true
                    } else {
                        false
                    }
                });
            }
            AlbumsViewIn::Favorited(id, state) => {
                (0..self.entries.len())
                    .filter_map(|i| self.entries.get(i))
                    .filter(|a| a.borrow().item.id == id)
                    .for_each(|album| match state {
                        true => {
                            album
                                .borrow_mut()
                                .fav
                                .set_value(String::from("starred-symbolic"));
                            album.borrow_mut().item.starred =
                                Some(chrono::offset::Local::now().into());
                        }
                        false => {
                            album
                                .borrow_mut()
                                .fav
                                .set_value(String::from("non-starred-symbolic"));
                            album.borrow_mut().item.starred = None;
                        }
                    });
            }
            AlbumsViewIn::Cover(msg) => match msg {
                CoverOut::DisplayToast(msg) => {
                    sender.output(AlbumsViewOut::DisplayToast(msg)).unwrap();
                }
            },
            AlbumsViewIn::FilterAdd => {
                use glib::object::Cast;

                let list_item = widgets.new_filter.selected_item().unwrap();
                let boxed = list_item
                    .downcast_ref::<glib::BoxedAnyObject>()
                    .expect("is not a BoxedAnyObject");
                let category: std::cell::Ref<Category> = boxed.borrow();

                let index = self.filters.guard().push_back(category.clone());
                self.filters
                    .send(index.current_index(), FilterRowIn::SetTo(category.clone()));
                sender.input(AlbumsViewIn::FilterChanged);
            }
            AlbumsViewIn::FilterRow(msg) => match msg {
                FilterRowOut::RemoveFilter(index) => {
                    self.filters.guard().remove(index.current_index());
                    sender.input(AlbumsViewIn::FilterChanged);
                }
                FilterRowOut::ParameterChanged => sender.input(AlbumsViewIn::FilterChanged),
            },
            AlbumsViewIn::AddToQueue => {
                if self.shown_albums.borrow().is_empty() {
                    return;
                }
                let albums: Vec<submarine::data::Child> =
                    (0..self.entries.selection_model.n_items())
                        .filter_map(|i| self.entries.get_visible(i))
                        .map(|i| i.borrow().item.clone())
                        .collect();
                for album in albums {
                    let drop = Droppable::Child(Box::new(album));
                    sender.output(AlbumsViewOut::AddToQueue(drop)).unwrap();
                }
            }
            AlbumsViewIn::AppendToQueue => {
                if self.shown_albums.borrow().is_empty() {
                    return;
                }
                let albums: Vec<submarine::data::Child> =
                    (0..self.entries.selection_model.n_items())
                        .filter_map(|i| self.entries.get_visible(i))
                        .map(|i| i.borrow().item.clone())
                        .collect();
                for album in albums {
                    let drop = Droppable::Child(Box::new(album));
                    sender.output(AlbumsViewOut::AppendToQueue(drop)).unwrap();
                }
            }
            AlbumsViewIn::ReplaceQueue => {
                if self.shown_albums.borrow().is_empty() {
                    return;
                }
                let albums: Vec<submarine::data::Child> =
                    (0..self.entries.selection_model.n_items())
                        .filter_map(|i| self.entries.get_visible(i))
                        .map(|i| i.borrow().item.clone())
                        .collect();
                for album in albums {
                    let drop = Droppable::Child(Box::new(album));
                    sender.output(AlbumsViewOut::ReplaceQueue(drop)).unwrap();
                }
            }
            AlbumsViewIn::ClickedAlbum(index) => {
                if let Some(clicked_album) = self.entries.get_visible(index) {
                    sender
                        .output(AlbumsViewOut::ClickedAlbum(Box::new(
                            clicked_album.borrow().item.clone(),
                        )))
                        .unwrap();
                }
            }
            AlbumsViewIn::ToggleFilters => {
                sender.input(AlbumsViewIn::FilterChanged);
                widgets
                    .filters
                    .set_reveal_child(!widgets.filters.reveals_child());
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
            AlbumsViewCmd::AddAlbums(candidates, processed) => {
                const CHUNK: usize = 20;
                const TIMEOUT: u64 = 2;

                //add some albums
                for album in candidates.iter().skip(processed).take(CHUNK) {
                    self.shown_albums.borrow_mut().insert(album.album.clone());
                    self.shown_artists.borrow_mut().insert(album.artist.clone());
                    let album = AlbumRow::new(&self.subsonic, album.clone(), sender.clone());
                    self.entries.append(album);
                }

                //update labels and buttons
                widgets.shown_albums.set_label(&format!(
                    "Shown albums: {}",
                    self.shown_albums.borrow().len()
                ));
                widgets.shown_artists.set_label(&format!(
                    "Shown artists: {}",
                    self.shown_artists.borrow().len()
                ));
                self.calc_sensitivity_of_buttons(widgets);

                // recursion anchor
                if processed >= candidates.len() {
                    widgets.spinner.set_visible(false);
                    return;
                }

                //recursion the rest of the list
                sender.oneshot_command(async move {
                    tokio::time::sleep(std::time::Duration::from_millis(TIMEOUT)).await;
                    AlbumsViewCmd::AddAlbums(candidates, processed + CHUNK)
                });
            }
        }
    }
}
