use std::{cell::RefCell, rc::Rc};

use fuzzy_matcher::FuzzyMatcher;
use relm4::{
    gtk::{
        self, glib,
        prelude::{BoxExt, ButtonExt, OrientableExt, PopoverExt, ToggleButtonExt, WidgetExt},
        FlowBoxChild,
    },
    ComponentController,
};

use crate::{
    components::{
        album_element::{AlbumElement, AlbumElementIn, AlbumElementInit, AlbumElementOut},
        filter_box::{FilterBox, FilterBoxIn, FilterBoxOut},
        filter_row::{Category, Filter},
    }, settings::Settings, subsonic::Subsonic
};

#[derive(Debug)]
pub struct AlbumsView {
    albums: gtk::FlowBox,
    album_list: Vec<relm4::Controller<AlbumElement>>,
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
    SearchChanged(String),
    FilterBox(FilterBoxOut),
    ClearFilters,
    ShowStarred(bool),
    Favorited(String, bool),
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
            albums: gtk::FlowBox::default(),
            album_list: vec![],
            filters: FilterBox::builder()
                .launch(Category::albums_view())
                .forward(sender.input_sender(), Self::Input::FilterBox),
        };
        let widgets = view_output!();

        // add albums with cover and title
        for album in init.borrow().albums() {
            let cover: relm4::Controller<AlbumElement> = AlbumElement::builder()
                .launch((
                    init.clone(),
                    AlbumElementInit::Child(Box::new(album.clone())),
                ))
                .forward(sender.input_sender(), AlbumsViewIn::AlbumElement);
            model.albums.append(cover.widget());
            model.album_list.push(cover);
        }

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

                        gtk::ToggleButton {
                            set_icon_name: "non-starred-symbolic",
                            set_width_request: 50,
                            connect_clicked[sender] => move |btn| {
                                if btn.is_active() {
                                    btn.set_icon_name("starred-symbolic");
                                } else {
                                    btn.set_icon_name("non-starred-symbolic");
                                }
                                sender.input(Self::Input::ShowStarred(btn.is_active()));
                            },
                            set_tooltip_text: Some("Toggle showing favortited albums"),
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
                            connect_clicked => Self::Input::ClearFilters,
                        }
                    }
                }
            },

            gtk::ScrolledWindow {
                set_hexpand: true,
                set_vexpand: true,

                #[wrap(Some)]
                set_child = &model.albums.clone() -> gtk::FlowBox {
                    set_valign: gtk::Align::Start,
                    set_row_spacing: 20,
                }
            }
        }
    }

    fn update(
        &mut self,
        msg: Self::Input,
        sender: relm4::ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match msg {
            AlbumsViewIn::AlbumElement(msg) => match msg {
                AlbumElementOut::Clicked(clicked) => {
                    sender.output(AlbumsViewOut::Clicked(clicked)).unwrap();
                }
                AlbumElementOut::DisplayToast(title) => sender
                    .output(AlbumsViewOut::DisplayToast(title))
                    .expect("sending failed"),
                AlbumElementOut::FavoriteClicked(id, state) => sender
                    .output(AlbumsViewOut::FavoriteClicked(id, state))
                    .expect("sending failed"),
            },
            AlbumsViewIn::SearchChanged(search) => {
                self.albums.set_filter_func(move |element| {
                    let mut search = search.clone();
                    let (title, artist) = get_info_of_flowboxchild(element);
                    let mut title_artist = format!("{} {}", title.text(), artist.text());

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
            AlbumsViewIn::ShowStarred(false) => {
                self.albums.set_filter_func(move |_element| true);
            }
            AlbumsViewIn::ShowStarred(true) => {
                let albums: Vec<_> = self
                    .album_list
                    .iter()
                    .map(|controller| controller.model().info().clone())
                    .collect();
                self.albums.set_filter_func(move |element| {
                    let (title, artist) = get_info_of_flowboxchild(element);

                    for album in &albums {
                        match album {
                            AlbumElementInit::Child(child) => {
                                if child.title == title.text()
                                    && child.artist == Some(artist.text().into())
                                {
                                    return child.starred.is_some();
                                }
                            }
                            AlbumElementInit::AlbumId3(album) => {
                                if album.name == title.text()
                                    && album.artist == Some(artist.text().into())
                                {
                                    return album.starred.is_some();
                                }
                            }
                        }
                    }
                    true
                });
            }
            AlbumsViewIn::FilterBox(FilterBoxOut::FiltersChanged) => {
                let filters = self.filters.model().get_filters();
                //TODO fix hacky way of figuring out what element we are iterating over
                let albums: Vec<_> = self
                    .album_list
                    .iter()
                    .map(|controller| controller.model().info().clone())
                    .collect();
                self.albums.set_filter_func(move |element| {
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
                            AlbumElementInit::AlbumId3(album) => {}
                        }
                    }
                    visible
                });
            }
            AlbumsViewIn::ClearFilters => self.filters.emit(FilterBoxIn::ClearFilters),
            AlbumsViewIn::Favorited(id, state) => {
                for album in &self.album_list {
                    album.emit(AlbumElementIn::Favorited(id.clone(), state));
                }
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
