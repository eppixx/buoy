use std::{cell::RefCell, collections::HashSet, rc::Rc};

use fuzzy_matcher::FuzzyMatcher;
use gettextrs::gettext;
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
        AlbumRow, ArtistColumn, CoverColumn, FavColumn, GenreColumn, LengthColumn, PlayCountColumn,
        TitleColumn, YearColumn,
    },
    settings::Settings,
    subsonic::Subsonic,
    types::{Droppable, Id},
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
                .set_tooltip(&gettext("There are too many albums to add to queue"));
            widgets.append_to_queue.set_sensitive(false);
            widgets
                .append_to_queue
                .set_tooltip(&gettext("There are too many albums to append to queue"));
            widgets.replace_queue.set_sensitive(false);
            widgets
                .replace_queue
                .set_tooltip(&gettext("There are too many albums to replace queue"));
        } else {
            widgets.add_to_queue.set_sensitive(true);
            widgets
                .add_to_queue
                .set_tooltip(&gettext("Append shown albums to end of queue"));
            widgets.append_to_queue.set_sensitive(true);
            widgets.append_to_queue.set_tooltip(&gettext(
                "Insert shown albums after currently played or paused item",
            ));
            widgets.replace_queue.set_sensitive(true);
            widgets
                .replace_queue
                .set_tooltip(&gettext("Replaces current queue with shown albums"));
        }
    }
}

#[derive(Debug)]
pub enum AlbumsViewIn {
    FilterChanged,
    UpdateFavoriteAlbum(String, bool),
    UpdatePlayCountAlbum(String, Option<i64>),
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
pub enum AlbumsViewOut {
    ClickedAlbum(Id),
    ClickedArtist(Id),
    DisplayToast(String),
    FavoriteClicked(String, bool),
    AddToQueue(Droppable),
    AppendToQueue(Droppable),
    ReplaceQueue(Droppable),
}

#[relm4::component(pub)]
impl relm4::component::Component for AlbumsView {
    type Init = Rc<RefCell<Subsonic>>;
    type Input = AlbumsViewIn;
    type Output = AlbumsViewOut;
    type CommandOutput = ();

    fn init(
        subsonic: Self::Init,
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
        // entries.append_column::<CdColumn>();
        entries.append_column::<PlayCountColumn>();
        entries.append_column::<FavColumn>();

        let columns = entries.get_columns();
        columns
            .get("Cover")
            .unwrap()
            .set_title(Some(&gettext("Cover")));
        columns
            .get("Album")
            .unwrap()
            .set_title(Some(&gettext("Album")));
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
            .get("Year")
            .unwrap()
            .set_title(Some(&gettext("Year")));
        columns
            .get("Favorite")
            .unwrap()
            .set_title(Some(&gettext("Favorite")));

        let mut model = Self {
            subsonic,
            entries,
            filters: relm4::factory::FactoryVecDeque::builder()
                .launch(gtk::ListBox::default())
                .forward(sender.input_sender(), Self::Input::FilterRow),
            shown_artists: Rc::new(RefCell::new(HashSet::new())),
            shown_albums: Rc::new(RefCell::new(HashSet::new())),
        };

        //add some albums
        let list = model.subsonic.borrow().albums().to_vec();
        for album in list.iter() {
            model.shown_albums.borrow_mut().insert(album.album.clone());
            model
                .shown_artists
                .borrow_mut()
                .insert(album.artist.clone());
            let album = AlbumRow::new(&model.subsonic, album.clone(), sender.clone());
            model.entries.append(album);
        }

        model.filters.guard().push_back(Category::Favorite);
        let widgets = view_output!();

        //update labels and buttons
        update_labels(
            &widgets.shown_albums,
            &model.shown_albums,
            &widgets.shown_artists,
            &model.shown_artists,
        );
        model.calc_sensitivity_of_buttons(&widgets);
        relm4::component::ComponentParts { model, widgets }
    }

    view! {
        gtk::Box {
            gtk::Box {
                add_css_class: "albums-view",
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

                                //tracks info
                                gtk::Box {
                                    set_orientation: gtk::Orientation::Vertical,
                                    set_spacing: 8,

                                    append: shown_albums = &gtk::Label {
                                        set_halign: gtk::Align::Start,
                                        set_text: &format!("{}: {}", gettext("Shown albums"), model.shown_albums.borrow().len()),
                                    },
                                    append: shown_artists = &gtk::Label {
                                        set_halign: gtk::Align::Start,
                                        set_text: &format!("{}: {}", gettext("Shown artists"), model.shown_artists.borrow().len()),
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
                                            connect_clicked => AlbumsViewIn::AppendToQueue,
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
                                            connect_clicked => AlbumsViewIn::AddToQueue,
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
                                            connect_clicked => AlbumsViewIn::ReplaceQueue,
                                        }
                                    }
                                }
                            },
                            #[wrap(Some)]
                            set_end_widget = &gtk::Box {
                                set_spacing: 10,
                                set_margin_end: 10,

                                gtk::Label {
                                    set_text: &gettext("Filters:"),
                                },
                                gtk::Switch {
                                    set_valign: gtk::Align::Center,
                                    connect_active_notify => AlbumsViewIn::ToggleFilters,
                                }
                            }
                        }
                    },
                    gtk::ScrolledWindow {
                        set_hexpand: true,
                        set_vexpand: true,

                        model.entries.view.clone() {
                            set_widget_name: "albums-view-tracks",
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
                let shown_albums = self.shown_albums.clone();
                let shown_artists = self.shown_artists.clone();
                let shown_artists_widget = widgets.shown_artists.clone();
                let shown_albums_widget = widgets.shown_albums.clone();
                update_labels(
                    &shown_albums_widget,
                    &shown_albums,
                    &shown_artists_widget,
                    &shown_artists,
                );

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
                    update_labels(
                        &shown_albums_widget,
                        &shown_albums,
                        &shown_artists_widget,
                        &shown_artists,
                    );
                    return;
                }

                self.shown_artists.borrow_mut().clear();
                self.shown_albums.borrow_mut().clear();

                self.entries.add_filter(move |track| {
                    let mut search = Settings::get().lock().unwrap().search_text.clone();
                    let title = track.item().title.clone();

                    for filter in &filters {
                        match filter {
                            //TODO add matching for regular expressions
                            Filter::Favorite(None) => {}
                            Filter::Favorite(Some(state)) => {
                                if *state != track.item().starred.is_some() {
                                    return false;
                                }
                            }
                            Filter::Album(_, value) if value.is_empty() => {} // filter matches
                            Filter::Album(relation, value) => match relation {
                                TextRelation::ExactNot
                                    if Some(value) == track.item().album.as_ref() =>
                                {
                                    return false
                                }
                                TextRelation::Exact
                                    if Some(value) != track.item().album.as_ref() =>
                                {
                                    return false
                                }
                                TextRelation::ContainsNot => {
                                    if let Some(album) = &track.item().album {
                                        if album.contains(value) {
                                            return false;
                                        }
                                    }
                                }
                                TextRelation::Contains => {
                                    if let Some(album) = &track.item().album {
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
                                    if Some(value) == track.item().artist.as_ref() =>
                                {
                                    return false
                                }
                                TextRelation::Exact
                                    if Some(value) != track.item().artist.as_ref() =>
                                {
                                    return false
                                }
                                TextRelation::ContainsNot => {
                                    if let Some(artist) = &track.item().artist {
                                        if artist.contains(value) {
                                            return false;
                                        }
                                    }
                                }
                                TextRelation::Contains => {
                                    if let Some(artist) = &track.item().artist {
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
                                if let Some(year) = &track.item().year {
                                    if year.cmp(value) != *order {
                                        return false;
                                    }
                                } else {
                                    return false;
                                }
                            }
                            Filter::Cd(_, 0) => {
                                if track.item().disc_number.is_some() {
                                    return false;
                                }
                            }
                            Filter::Cd(order, value) => {
                                if let Some(disc) = &track.item().disc_number {
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
                                    if Some(value) == track.item().genre.as_ref() =>
                                {
                                    return false
                                }
                                TextRelation::Exact
                                    if Some(value) != track.item().genre.as_ref() =>
                                {
                                    return false
                                }
                                TextRelation::ContainsNot => {
                                    if let Some(genre) = &track.item().genre {
                                        if genre.contains(value) {
                                            return false;
                                        }
                                    }
                                }
                                TextRelation::Contains => {
                                    if let Some(genre) = &track.item().genre {
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
                                if let Some(duration) = &track.item().duration {
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
                        shown_artists
                            .borrow_mut()
                            .insert(track.item().artist.clone());
                        shown_albums.borrow_mut().insert(track.item().album.clone());
                        update_labels(
                            &shown_albums_widget,
                            &shown_albums,
                            &shown_artists_widget,
                            &shown_artists,
                        );
                        return true;
                    }

                    let mut title_artist_album = format!(
                        "{title} {} {}",
                        track.item().artist.clone().unwrap_or_default(),
                        track.item().album.clone().unwrap_or_default()
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
                            shown_artists
                                .borrow_mut()
                                .insert(track.item().artist.clone());
                            shown_albums.borrow_mut().insert(track.item().album.clone());
                            update_labels(
                                &shown_albums_widget,
                                &shown_albums,
                                &shown_artists_widget,
                                &shown_artists,
                            );
                            true
                        } else {
                            false
                        }
                    } else if title_artist_album.contains(&search) {
                        shown_artists
                            .borrow_mut()
                            .insert(track.item().artist.clone());
                        shown_albums.borrow_mut().insert(track.item().album.clone());
                        update_labels(
                            &shown_albums_widget,
                            &shown_albums,
                            &shown_artists_widget,
                            &shown_artists,
                        );
                        true
                    } else {
                        false
                    }
                });
            }
            AlbumsViewIn::UpdateFavoriteAlbum(id, state) => {
                (0..self.entries.len())
                    .filter_map(|i| self.entries.get(i))
                    .filter(|a| a.borrow().item().id == id)
                    .for_each(|album| match state {
                        true => {
                            if let Some(fav) = &album.borrow().fav_btn() {
                                fav.set_icon_name("starred-symbolic");
                            }
                            album.borrow_mut().item_mut().starred =
                                Some(chrono::offset::Local::now().into());
                        }
                        false => {
                            if let Some(fav) = &album.borrow().fav_btn() {
                                fav.set_icon_name("non-starred-symbolic");
                            }
                            album.borrow_mut().item_mut().starred = None;
                        }
                    });
            }
            AlbumsViewIn::UpdatePlayCountAlbum(id, play_count) => {
                (0..self.entries.len())
                    .filter_map(|i| self.entries.get(i))
                    .filter(|a| a.borrow().item().id == id)
                    .for_each(|album| album.borrow_mut().item_mut().play_count = play_count);
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
                        .map(|i| i.borrow().item().clone())
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
                        .map(|i| i.borrow().item().clone())
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
                        .map(|i| i.borrow().item().clone())
                        .collect();
                for album in albums {
                    let drop = Droppable::Child(Box::new(album));
                    sender.output(AlbumsViewOut::ReplaceQueue(drop)).unwrap();
                }
            }
            AlbumsViewIn::ClickedAlbum(index) => {
                if let Some(clicked_album) = self.entries.get_visible(index) {
                    let id = Id::album(&clicked_album.borrow().item().id);
                    sender.output(AlbumsViewOut::ClickedAlbum(id)).unwrap();
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
}

fn update_labels(
    album_label: &gtk::Label,
    albums: &Rc<RefCell<HashSet<Option<String>>>>,
    artist_label: &gtk::Label,
    artists: &Rc<RefCell<HashSet<Option<String>>>>,
) {
    album_label.set_text(&format!(
        "{}: {}",
        gettext("Shown albums"),
        albums.borrow().len()
    ));
    artist_label.set_text(&format!(
        "{}: {}",
        gettext("Shown artists"),
        artists.borrow().len()
    ));
}
