use std::{cell::RefCell, rc::Rc};

use fuzzy_matcher::FuzzyMatcher;
use relm4::{
    gtk::{
        self, glib,
        prelude::{BoxExt, OrientableExt, WidgetExt},
        FlowBoxChild,
    },
    loading_widgets::LoadingWidgets,
    view, Component, ComponentController,
};

use crate::{
    common,
    components::{
        artist_element::{ArtistElement, ArtistElementIn, ArtistElementOut},
        sort_by::SortBy,
    },
    settings::Settings,
    subsonic::Subsonic,
};

#[derive(Debug)]
pub struct ArtistsView {
    subsonic: Rc<RefCell<Subsonic>>,
    artists: gtk::FlowBox,
    artist_list: Vec<relm4::Controller<ArtistElement>>,
}

#[derive(Debug)]
pub enum ArtistsViewOut {
    ClickedArtist(submarine::data::ArtistId3),
    DisplayToast(String),
    FavoriteClicked(String, bool),
}

#[derive(Debug)]
pub enum ArtistsViewIn {
    ArtistElement(ArtistElementOut),
    FilterChanged,
    Favorited(String, bool),
    CoverSizeChanged,
    Sort(SortBy),
}

#[relm4::component(async, pub)]
impl relm4::component::AsyncComponent for ArtistsView {
    type Init = Rc<RefCell<Subsonic>>;
    type Input = ArtistsViewIn;
    type Output = ArtistsViewOut;
    type CommandOutput = ();

    fn init_loading_widgets(root: Self::Root) -> Option<LoadingWidgets> {
        view! {
            append = root.clone() -> gtk::Box {
                add_css_class: "artists-view",

                #[name(loading_box)]
                gtk::Box {
                    set_hexpand: true,
                    set_spacing: 30,
                    set_halign: gtk::Align::Center,
                    set_orientation: gtk::Orientation::Vertical,

                    gtk::Spinner {
                        add_css_class: "size100",
                        set_halign: gtk::Align::Center,
                        start: (),
                    }
                }
            }
        }

        // removes widget loading_box when function init finishes
        Some(LoadingWidgets::new(root, loading_box))
    }

    async fn init(
        init: Self::Init,
        root: Self::Root,
        sender: relm4::AsyncComponentSender<Self>,
    ) -> relm4::component::AsyncComponentParts<Self> {
        let mut model = Self {
            subsonic: init.clone(),
            artists: gtk::FlowBox::default(),
            artist_list: vec![],
        };
        let widgets = view_output!();

        // add artists with cover and title
        for (i, artist) in init.borrow().artists().iter().enumerate() {
            let cover: relm4::Controller<ArtistElement> = ArtistElement::builder()
                .launch((init.clone(), artist.clone()))
                .forward(sender.input_sender(), ArtistsViewIn::ArtistElement);
            model.artists.insert(cover.widget(), i as i32);
            model.artist_list.insert(i, cover);
        }

        relm4::component::AsyncComponentParts { model, widgets }
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
                        set_label: "Artists",
                        set_halign: gtk::Align::Center,
                    },

                    #[wrap(Some)]
                    set_end_widget = &gtk::Box {
                        set_spacing: 10,
                        //prevent cutoff of "glow" when widget has focus
                        set_margin_top: 2,

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

                        gtk::Box {
                            set_spacing: 5,
                            //create space to the end of the window
                            set_margin_end: 10,

                            gtk::Label {
                                set_text: "Sort by:",
                            },
                            gtk::DropDown {
                                set_model: Some(&SortBy::artists_store()),
                                set_factory: Some(&SortBy::factory()),
                                set_show_arrow: true,
                                connect_selected_notify[sender] => move |drop| {
                                    use glib::object::Cast;

                                    let obj = drop.selected_item().unwrap().downcast::<glib::BoxedAnyObject>().unwrap();
                                    let sort: std::cell::Ref<SortBy> = obj.borrow();
                                    sender.input(ArtistsViewIn::Sort(sort.clone()));
                                },
                            }
                        },
                    }
                },
            },

            gtk::ScrolledWindow {
                set_hexpand: true,
                set_vexpand: true,

                #[wrap(Some)]
                set_child = &model.artists.clone() -> gtk::FlowBox {
                    set_valign: gtk::Align::Start,
                    set_row_spacing: 20,
                }
            }
        }
    }

    async fn update_with_view(
        &mut self,
        widgets: &mut Self::Widgets,
        msg: Self::Input,
        sender: relm4::AsyncComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match msg {
            ArtistsViewIn::ArtistElement(msg) => match msg {
                ArtistElementOut::Clicked(id) => {
                    sender.output(ArtistsViewOut::ClickedArtist(id)).unwrap();
                }
                ArtistElementOut::DisplayToast(msg) => {
                    sender.output(ArtistsViewOut::DisplayToast(msg)).unwrap()
                }
                ArtistElementOut::FavoriteClicked(id, state) => sender
                    .output(ArtistsViewOut::FavoriteClicked(id, state))
                    .unwrap(),
            },
            ArtistsViewIn::FilterChanged => {
                let subsonic = self.subsonic.clone();
                let favorite = widgets.favorite.clone();
                self.artists.set_filter_func(move |element| {
                    let mut search = Settings::get().lock().unwrap().search_text.clone();
                    let mut title = get_title_of_flowboxchild(element).text().to_string();

                    // respect favorite filter pressed
                    if favorite.is_active() {
                        let artist = subsonic
                            .borrow()
                            .artists()
                            .iter()
                            .find(|artist| artist.name == title)
                            .unwrap()
                            .clone();
                        if artist.starred.is_none() {
                            return false;
                        }
                    }

                    // when search bar is hidden every element will be shown
                    if !Settings::get().lock().unwrap().search_active {
                        return true;
                    }

                    //check for case sensitivity
                    if !Settings::get().lock().unwrap().case_sensitive {
                        title = title.to_lowercase();
                        search = search.to_lowercase();
                    }

                    //actual matching
                    let fuzzy_search = Settings::get().lock().unwrap().fuzzy_search;
                    if fuzzy_search {
                        let matcher = fuzzy_matcher::skim::SkimMatcherV2::default();
                        let score = matcher.fuzzy_match(&title, &search);
                        score.is_some()
                    } else {
                        title.contains(&search)
                    }
                });
            }
            ArtistsViewIn::Favorited(id, state) => {
                for artist in &self.artist_list {
                    artist.emit(ArtistElementIn::Favorited(id.clone(), state));
                }
            }
            ArtistsViewIn::CoverSizeChanged => {
                let size = Settings::get().lock().unwrap().cover_size;
                for element in &self.artist_list {
                    element.model().change_size(size);
                }
            }
            ArtistsViewIn::Sort(category) => {
                let artists: Vec<_> = self
                    .artist_list
                    .iter()
                    .map(|controller| controller.model().info().clone())
                    .collect();
                self.artists.set_sort_func(move |a, b| {
                    let title = get_title_of_flowboxchild(a);
                    let a = artists
                        .iter()
                        .find(|a| a.name == title.text())
                        .expect("artist should be in there");
                    let title = get_title_of_flowboxchild(b);
                    let b = artists
                        .iter()
                        .find(|a| a.name == title.text())
                        .expect("artist should be in there");

                    match category {
                        SortBy::Alphabetical => common::sort_fn(&a.name, &b.name),
                        SortBy::AlphabeticalRev => common::sort_fn(&b.name, &a.name),
                        SortBy::MostAlbums => common::sort_fn(&b.album_count, &a.album_count),
                        SortBy::MostAlbumsRev => common::sort_fn(&a.album_count, &b.album_count),
                        _ => unimplemented!("category not implemented"),
                    }
                });
            }
        }
    }
}

fn get_title_of_flowboxchild(element: &FlowBoxChild) -> gtk::Label {
    use glib::object::Cast;
    let bo = element.first_child().unwrap();
    let overlay = bo.first_child().unwrap();
    let button = overlay.first_child().unwrap();
    let overlay = button.first_child().unwrap();
    let bo = overlay.first_child().unwrap();
    let bo = bo.first_child().unwrap();
    let title = bo.next_sibling().unwrap();
    title.downcast::<gtk::Label>().expect("unepected element")
}
