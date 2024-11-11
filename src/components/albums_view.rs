use std::{cell::RefCell, rc::Rc};

use fuzzy_matcher::FuzzyMatcher;
use relm4::{
    gtk::{
        self, glib,
        prelude::{BoxExt, ButtonExt, OrientableExt, PopoverExt, WidgetExt},
        FlowBoxChild,
    },
    ComponentController,
};

use crate::{
    common,
    components::{
        album_element::{AlbumElement, AlbumElementIn, AlbumElementInit, AlbumElementOut},
        filter_box::{FilterBox, FilterBoxIn, FilterBoxOut},
        filter_categories::Category,
        filter_row::Filter,
        sort_by::{self, SortBy},
    },
    settings::Settings,
    subsonic::Subsonic,
};

#[derive(Debug)]
pub struct AlbumsView {
    subsonic: Rc<RefCell<Subsonic>>,
    albums: relm4::factory::FactoryVecDeque<AlbumElement>,
    filters: relm4::Controller<FilterBox>,
}

#[derive(Debug)]
pub enum AlbumsViewOut {
    Clicked(AlbumElementInit),
    DisplayToast(String),
    FavoriteClicked(String, bool),
}

#[derive(Debug)]
pub enum AlbumsViewIn {
    AlbumElement(AlbumElementOut),
    FilterChanged,
    FilterBox(FilterBoxOut),
    ClearFilters,
    Favorited(String, bool),
    CoverSizeChanged,
    Sort(SortBy),
}

#[relm4::component(pub)]
impl relm4::component::Component for AlbumsView {
    type Init = Rc<RefCell<Subsonic>>;
    type Input = AlbumsViewIn;
    type Output = AlbumsViewOut;
    type CommandOutput = ();

    fn init(
        init: Self::Init,
        root: Self::Root,
        sender: relm4::ComponentSender<Self>,
    ) -> relm4::component::ComponentParts<Self> {
        let mut model = Self {
            subsonic: init.clone(),
            albums: relm4::factory::FactoryVecDeque::builder()
                .launch(gtk::FlowBox::default())
                .forward(sender.input_sender(), Self::Input::AlbumElement),
            filters: FilterBox::builder()
                .launch(Category::albums_view())
                .forward(sender.input_sender(), Self::Input::FilterBox),
        };
        let widgets = view_output!();

        // add albums with cover and title
        let mut guard = model.albums.guard();
        for album in init.borrow().albums() {
            guard.push_back((
                init.clone(),
                AlbumElementInit::Child(Box::new(album.clone())),
            ));
        }
        drop(guard);

        relm4::component::ComponentParts { model, widgets }
    }

    view! {
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            set_hexpand: true,

            gtk::WindowHandle {
                gtk::CenterBox {
                    #[wrap(Some)]
                    set_center_widget = &gtk::Label {
                        add_css_class: "h2",
                        set_label: "Albums",
                        set_halign: gtk::Align::Center,
                    },

                    #[wrap(Some)]
                    set_end_widget = &gtk::Box {
                        set_spacing: 10,
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
                                set_tooltip_text: Some("Toggle showing favortited artists"),
                            }
                        },

                        // create new box to disable spacing
                        gtk::Box {
                            set_spacing: 5,

                            gtk::Label {
                                set_text: "Sort by:",
                            },
                            gtk::DropDown {
                                set_model: Some(&sort_by::SortBy::albums_store()),
                                set_factory: Some(&sort_by::SortBy::factory()),
                                set_show_arrow: true,
                                connect_selected_notify[sender] => move |drop| {
                                    use glib::object::Cast;

                                    let obj = drop.selected_item().unwrap().downcast::<glib::BoxedAnyObject>().unwrap();
                                    let sort: std::cell::Ref<SortBy> = obj.borrow();
                                    sender.input(AlbumsViewIn::Sort(sort.clone()));
                                },
                            }
                        },

                        gtk::MenuButton {
                            set_label: "Filter ",
                            #[wrap(Some)]
                            set_popover = &gtk::Popover {
                                set_focus_on_click: false,
                                set_autohide: true,

                                gtk::Box {
                                    set_orientation: gtk::Orientation::Vertical,

                                    model.filters.widget(),
                                }
                            }
                        },
                        gtk::Button {
                            set_icon_name: "user-trash-symbolic",
                            set_margin_end: 10,
                            connect_clicked => Self::Input::ClearFilters,
                        }
                    }
                }
            },

            gtk::ScrolledWindow {
                set_hexpand: true,
                set_vexpand: true,

                #[wrap(Some)]
                set_child = &model.albums.widget().clone() -> gtk::FlowBox {
                    set_valign: gtk::Align::Start,
                    set_row_spacing: 20,
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
            AlbumsViewIn::AlbumElement(msg) => match msg {
                AlbumElementOut::Clicked(clicked) => {
                    sender.output(AlbumsViewOut::Clicked(clicked)).unwrap();
                }
                AlbumElementOut::DisplayToast(title) => {
                    sender.output(AlbumsViewOut::DisplayToast(title)).unwrap()
                }
                AlbumElementOut::FavoriteClicked(id, state) => sender
                    .output(AlbumsViewOut::FavoriteClicked(id, state))
                    .unwrap(),
            },
            AlbumsViewIn::FilterChanged => {
                let subsonic = self.subsonic.clone();
                let favorite = widgets.favorite.clone();
                self.albums.widget().set_filter_func(move |element| {
                    let mut search = Settings::get().lock().unwrap().search_text.clone();
                    let (title, artist) = get_info_of_flowboxchild(element);
                    let mut title_artist = format!("{} {}", title.text(), artist.text());

                    // respect favorite filter pressed
                    if favorite.is_active() {
                        let album = subsonic
                            .borrow()
                            .albums()
                            .iter()
                            .find(|album| {
                                album.title == title.text()
                                    && album.artist.as_deref() == Some(&artist.text())
                            })
                            .unwrap()
                            .clone();
                        if album.starred.is_none() {
                            return false;
                        }
                    }

                    // when search bar is hidden every element will be shown
                    if !Settings::get().lock().unwrap().search_active {
                        return true;
                    }

                    //check for case sensitivity
                    if !Settings::get().lock().unwrap().case_sensitive {
                        title_artist = title_artist.to_lowercase();
                        search = search.to_lowercase();
                    }

                    //actual matching
                    let fuzzy_search = Settings::get().lock().unwrap().fuzzy_search;
                    if fuzzy_search {
                        let matcher = fuzzy_matcher::skim::SkimMatcherV2::default();
                        let score = matcher.fuzzy_match(&title_artist, &search);
                        score.is_some()
                    } else {
                        title_artist.contains(&search)
                    }
                });
            }
            AlbumsViewIn::FilterBox(FilterBoxOut::FiltersChanged) => {
                let filters = self.filters.model().get_filters();
                //TODO fix hacky way of figuring out what element we are iterating over
                let albums: Vec<AlbumElementInit> =
                    self.albums.iter().map(|a| a.info().clone()).collect();
                self.albums.widget().set_filter_func(move |element| {
                    let (title, artist) = get_info_of_flowboxchild(element);

                    let mut visible = true;
                    for album in &albums {
                        match album {
                            AlbumElementInit::Child(child) => {
                                if child.title == title.text()
                                    && child.artist == Some(artist.text().into())
                                {
                                    for filter in &filters {
                                        match filter {
                                            Filter::Favorite(value)
                                                if *value != child.starred.is_some() =>
                                            {
                                                visible = false
                                            }
                                            //TODO Favorite false, true and NONE set
                                            //TODO add matching for regular expressions
                                            Filter::Album(value) if value != &title.text() => {
                                                visible = false
                                            }
                                            Filter::Artist(value) if value != &artist.text() => {
                                                visible = false
                                            }
                                            Filter::Year(order, value) => {
                                                if let Some(year) = &child.year {
                                                    if year.cmp(value) != *order {
                                                        visible = false;
                                                    }
                                                } else {
                                                    visible = false;
                                                }
                                            }
                                            Filter::Cd(order, value) => {
                                                if let Some(disc) = &child.disc_number {
                                                    if disc.cmp(value) != *order {
                                                        visible = false;
                                                    }
                                                } else {
                                                    visible = false;
                                                }
                                            }
                                            Filter::Genre(value) if value != &artist.text() => {
                                                // TODO fix artist.text()
                                                visible = false
                                            }
                                            Filter::Duration(order, value) => {
                                                if let Some(duration) = &child.duration {
                                                    if duration.cmp(value) != *order {
                                                        visible = false;
                                                    }
                                                } else {
                                                    visible = false;
                                                }
                                            }
                                            _ => unreachable!("there are filters that shouldnt be"),
                                        }
                                    }
                                }
                            }
                            AlbumElementInit::AlbumId3(_album) => {
                                unreachable!("albums view is never initialized from albumId3");
                            }
                        }
                    }
                    visible
                });
            }
            AlbumsViewIn::ClearFilters => self.filters.emit(FilterBoxIn::ClearFilters),
            AlbumsViewIn::Favorited(id, state) => {
                self.albums.broadcast(AlbumElementIn::Favorited(id, state));
            }
            AlbumsViewIn::CoverSizeChanged => {
                let size = Settings::get().lock().unwrap().cover_size;
                self.albums.iter().for_each(|a| a.change_size(size));
            }
            AlbumsViewIn::Sort(category) => {
                //TODO fix hacky way of figuring out what element we are iterating over
                let albums: Vec<AlbumElementInit> =
                    self.albums.iter().map(|a| a.info().clone()).collect();
                self.albums.widget().set_sort_func(move |a, b| {
                    let match_fn = |init: &AlbumElementInit,
                                    title: &gtk::Label,
                                    artist: &gtk::Label|
                     -> bool {
                        match &init {
                            AlbumElementInit::Child(c) => {
                                c.title == title.text()
                                    && c.artist.as_deref() == Some(&artist.text())
                            }
                            AlbumElementInit::AlbumId3(c) => {
                                c.name == title.text()
                                    && c.artist.as_deref() == Some(&artist.text())
                            }
                        }
                    };

                    let (title, artist) = get_info_of_flowboxchild(a);
                    let a = albums
                        .iter()
                        .find(|a| match_fn(a, &title, &artist))
                        .expect("album should be in there");
                    let (title, artist) = get_info_of_flowboxchild(b);
                    let b = albums
                        .iter()
                        .find(|a| match_fn(a, &title, &artist))
                        .expect("album should be in there");

                    match (a, b) {
                        (AlbumElementInit::Child(a), AlbumElementInit::Child(b)) => {
                            match category {
                                SortBy::Alphabetical => common::sort_fn(&a.title, &b.title),
                                SortBy::AlphabeticalRev => common::sort_fn(&b.title, &a.title),
                                SortBy::RecentlyAdded => common::sort_fn(&b.created, &a.created),
                                SortBy::RecentlyAddedRev => common::sort_fn(&a.created, &b.created),
                                SortBy::Release => common::sort_fn(&a.year, &b.year),
                                SortBy::ReleaseRev => common::sort_fn(&b.year, &a.year),
                                _ => unimplemented!("category not implemented"),
                            }
                        }
                        (AlbumElementInit::AlbumId3(a), AlbumElementInit::AlbumId3(b)) => {
                            match category {
                                SortBy::Alphabetical => common::sort_fn(&a.name, &b.name),
                                SortBy::AlphabeticalRev => common::sort_fn(&b.name, &a.name),
                                SortBy::RecentlyAdded => common::sort_fn(&b.created, &a.created),
                                SortBy::RecentlyAddedRev => common::sort_fn(&a.created, &b.created),
                                SortBy::Release => common::sort_fn(&a.year, &b.year),
                                SortBy::ReleaseRev => common::sort_fn(&b.year, &a.year),
                                _ => unimplemented!("category not implemented"),
                            }
                        }
                        (_, _) => unreachable!(),
                    }
                });
            }
        }
    }
}

pub fn get_info_of_flowboxchild(element: &FlowBoxChild) -> (gtk::Label, gtk::Label) {
    use glib::object::Cast;
    let overlay = element.first_child().unwrap();
    let button = overlay.first_child().unwrap();
    let overlay = button.first_child().unwrap();
    let bo = overlay.first_child().unwrap();
    let cover = bo.first_child().unwrap();
    let title = cover.next_sibling().unwrap();
    let title = title.downcast::<gtk::Label>().expect("unepected element");
    let artist = title.next_sibling().unwrap();
    let artist = artist.downcast::<gtk::Label>().expect("unexpected element");

    (title, artist)
}
