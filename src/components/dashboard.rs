use std::cell::RefCell;
use std::rc::Rc;

use fuzzy_matcher::FuzzyMatcher;
use rand::prelude::SliceRandom;
use relm4::{
    gtk::{
        self,
        prelude::{AdjustmentExt, BoxExt, ButtonExt, OrientableExt, WidgetExt},
    },
    ComponentController, RelmRemoveAllExt, RelmWidgetExt,
};

use crate::components::album_element::{
    AlbumElement, AlbumElementIn, AlbumElementInit, AlbumElementOut,
};
use crate::{client::Client, subsonic::Subsonic};

enum Scrolling {
    RecentlyAddedLeft,
    RecentlyAddedRight,
    RecentlyPlayedLeft,
    RecentlyPlayedRight,
    RandomAlbumLeft,
    RandomAlbumRight,
    MostPlayedLeft,
    MostPlayedRight,
}

#[derive(Debug)]
pub struct Dashboard {
    subsonic: Rc<RefCell<Subsonic>>,

    recently_added: gtk::Box,
    recently_added_scroll: gtk::ScrolledWindow,
    recently_added_list: Vec<relm4::Controller<AlbumElement>>,

    recently_played: gtk::Box,
    recently_played_scroll: gtk::ScrolledWindow,
    recently_played_list: Vec<relm4::Controller<AlbumElement>>,

    random_album: gtk::Box,
    random_album_scroll: gtk::ScrolledWindow,
    random_album_list: Vec<relm4::Controller<AlbumElement>>,

    most_played: gtk::Box,
    most_played_scroll: gtk::ScrolledWindow,
    most_played_list: Vec<relm4::Controller<AlbumElement>>,
}

#[derive(Debug)]
pub enum DashboardOut {
    ClickedAlbum(AlbumElementInit),
    DisplayToast(String),
    FavoriteClicked(String, bool),
}

#[derive(Debug)]
pub enum DashboardIn {
    SearchChanged(String),
    AlbumElement(AlbumElementOut),
    ClickedRandomize,
    FavoritedAlbum(String, bool),
}

#[derive(Debug)]
pub enum DashboardCmd {
    Error(String),
    LoadedRecentlyPlayed(Result<Vec<submarine::data::Child>, submarine::SubsonicError>),
}

#[relm4::component(pub)]
impl relm4::Component for Dashboard {
    type Init = Rc<RefCell<Subsonic>>;
    type Input = DashboardIn;
    type Output = DashboardOut;
    type CommandOutput = DashboardCmd;

    fn init(
        subsonic: Self::Init,
        root: Self::Root,
        sender: relm4::ComponentSender<Self>,
    ) -> relm4::ComponentParts<Self> {
        let mut model = Self {
            subsonic: subsonic.clone(),

            recently_added: gtk::Box::default(),
            recently_added_scroll: gtk::ScrolledWindow::default(),
            recently_added_list: vec![],

            recently_played: gtk::Box::default(),
            recently_played_scroll: gtk::ScrolledWindow::default(),
            recently_played_list: vec![],

            random_album: gtk::Box::default(),
            random_album_scroll: gtk::ScrolledWindow::default(),
            random_album_list: vec![],

            most_played: gtk::Box::default(),
            most_played_scroll: gtk::ScrolledWindow::default(),
            most_played_list: vec![],
        };

        //load recently added albums
        let mut albums = subsonic.borrow().albums().clone();
        albums.sort_by(|a, b| a.created.cmp(&b.created));
        model.recently_added_list = albums
            .iter()
            .take(10)
            .map(|album| {
                AlbumElement::builder()
                    .launch((
                        subsonic.clone(),
                        AlbumElementInit::Child(Box::new(album.clone())),
                    ))
                    .forward(sender.input_sender(), DashboardIn::AlbumElement)
            })
            .collect();
        for album in &model.recently_added_list {
            model.recently_added.append(album.widget());
        }

        //load recently played albums
        sender.oneshot_command(async move {
            let client = match Client::get() {
                None => return DashboardCmd::Error(String::from("no client found")),
                Some(client) => client,
            };
            DashboardCmd::LoadedRecentlyPlayed(
                client
                    .get_album_list2(
                        submarine::api::get_album_list::Order::Recent,
                        Some(10),
                        None,
                        None::<String>,
                    )
                    .await,
            )
        });

        //load random albums
        sender.input(DashboardIn::ClickedRandomize);

        //load most played albums
        albums.sort_by(|a, b| b.play_count.cmp(&a.play_count));
        model.most_played_list = albums
            .iter()
            .take(10)
            .map(|album| {
                AlbumElement::builder()
                    .launch((
                        subsonic.clone(),
                        AlbumElementInit::Child(Box::new(album.clone())),
                    ))
                    .forward(sender.input_sender(), DashboardIn::AlbumElement)
            })
            .collect();
        for album in &model.most_played_list {
            model.most_played.append(album.widget());
        }

        let scrolling = Rc::new(RefCell::new(None));
        let widgets = view_output!();

        //update scrolling of boxes
        let recently_added_scroll = model.recently_added_scroll.clone();
        let recently_played_scroll = model.recently_played_scroll.clone();
        let random_album_scroll = model.random_album_scroll.clone();
        let most_played_scroll = model.most_played_scroll.clone();
        gtk::glib::source::timeout_add_local(core::time::Duration::from_millis(15), move || {
            const SCROLL_MOVE: f64 = 5f64;
            match *scrolling.borrow() {
                None => {}
                Some(Scrolling::RecentlyAddedLeft) => {
                    let vadj = recently_added_scroll.hadjustment();
                    vadj.set_value(vadj.value() - SCROLL_MOVE);
                    recently_added_scroll.set_hadjustment(Some(&vadj));
                }
                Some(Scrolling::RecentlyAddedRight) => {
                    let vadj = recently_added_scroll.hadjustment();
                    vadj.set_value(vadj.value() + SCROLL_MOVE);
                    recently_added_scroll.set_hadjustment(Some(&vadj));
                }
                Some(Scrolling::RecentlyPlayedLeft) => {
                    let vadj = recently_played_scroll.hadjustment();
                    vadj.set_value(vadj.value() - SCROLL_MOVE);
                    recently_played_scroll.set_hadjustment(Some(&vadj));
                }
                Some(Scrolling::RecentlyPlayedRight) => {
                    let vadj = recently_played_scroll.hadjustment();
                    vadj.set_value(vadj.value() + SCROLL_MOVE);
                    recently_played_scroll.set_hadjustment(Some(&vadj));
                }
                Some(Scrolling::RandomAlbumLeft) => {
                    let vadj = random_album_scroll.hadjustment();
                    vadj.set_value(vadj.value() - SCROLL_MOVE);
                    random_album_scroll.set_hadjustment(Some(&vadj));
                }
                Some(Scrolling::RandomAlbumRight) => {
                    let vadj = random_album_scroll.hadjustment();
                    vadj.set_value(vadj.value() + SCROLL_MOVE);
                    random_album_scroll.set_hadjustment(Some(&vadj));
                }
                Some(Scrolling::MostPlayedLeft) => {
                    let vadj = most_played_scroll.hadjustment();
                    vadj.set_value(vadj.value() - SCROLL_MOVE);
                    most_played_scroll.set_hadjustment(Some(&vadj));
                }
                Some(Scrolling::MostPlayedRight) => {
                    let vadj = most_played_scroll.hadjustment();
                    vadj.set_value(vadj.value() + SCROLL_MOVE);
                    most_played_scroll.set_hadjustment(Some(&vadj));
                }
            }
            gtk::glib::ControlFlow::Continue
        });

        relm4::ComponentParts { model, widgets }
    }

    view! {
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            add_css_class: "dashboard-view",
            set_margin_horizontal: 7,

            gtk::WindowHandle {
                gtk::Label {
                    add_css_class: granite::STYLE_CLASS_H2_LABEL,
                    set_halign: gtk::Align::Center,
                    set_text: "Dashboard",
                }
            },

            gtk::ScrolledWindow {
                set_hexpand: true,
                set_vexpand: true,
                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_halign: gtk::Align::Fill,
                    set_spacing: 50,

                    gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,

                        gtk::Label {
                            add_css_class: granite::STYLE_CLASS_H2_LABEL,
                            set_halign: gtk::Align::Start,
                            set_text: "Newly added",
                        },
                        gtk::CenterBox {
                            set_hexpand: true,

                            #[wrap(Some)]
                            set_start_widget = &gtk::Box {
                                gtk::Image {
                                    set_icon_name: Some("go-previous-symbolic"),
                                    set_size_request: (50, 50),

                                    add_controller = gtk::EventControllerMotion {
                                        connect_enter[scrolling] => move |_controller, _x, _y| {
                                            scrolling.replace(Some(Scrolling::RecentlyAddedLeft));
                                        },
                                        connect_leave[scrolling] => move |_controller| {
                                            scrolling.replace(None);
                                        }
                                    }
                                },
                            },

                            #[wrap(Some)]
                            set_center_widget = &model.recently_added_scroll.clone() -> gtk::ScrolledWindow {
                                set_vscrollbar_policy: gtk::PolicyType::Never,
                                set_hscrollbar_policy: gtk::PolicyType::External,
                                set_hexpand: true,

                                #[wrap(Some)]
                                set_child = &model.recently_added.clone() {
                                    set_vexpand: true,
                                    set_hexpand: true,
                                },
                            },

                            #[wrap(Some)]
                            set_end_widget = &gtk::Box {
                                gtk::Image {
                                    set_icon_name: Some("go-next-symbolic"),
                                    set_size_request: (50, 50),
                                },
                                add_controller = gtk::EventControllerMotion {
                                    connect_enter[scrolling] => move |_controller, _x, _y| {
                                        scrolling.replace(Some(Scrolling::RecentlyAddedRight));
                                    },
                                    connect_leave[scrolling] => move |_controller| {
                                        scrolling.replace(None);
                                    }
                                }
                            }
                        },
                    },

                    gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,

                        gtk::Label {
                            add_css_class: granite::STYLE_CLASS_H2_LABEL,
                            set_halign: gtk::Align::Start,
                            set_text: "Recently Played",
                        },
                        gtk::CenterBox {
                            #[wrap(Some)]
                            set_start_widget = &gtk::Box {
                                gtk::Image {
                                    set_icon_name: Some("go-previous-symbolic"),
                                    set_size_request: (50, 50),

                                    add_controller = gtk::EventControllerMotion {
                                        connect_enter[scrolling] => move |_controller, _x, _y| {
                                            scrolling.replace(Some(Scrolling::RecentlyPlayedLeft));
                                        },
                                        connect_leave[scrolling] => move |_controller| {
                                            scrolling.replace(None);
                                        }
                                    }
                                },
                            },

                            #[wrap(Some)]
                            set_center_widget = &model.recently_played_scroll.clone() -> gtk::ScrolledWindow {
                                set_vscrollbar_policy: gtk::PolicyType::Never,
                                set_hscrollbar_policy: gtk::PolicyType::External,
                                set_hexpand: true,

                                model.recently_played.clone() {
                                    set_halign: gtk::Align::Start,
                                    set_vexpand: true,
                                }
                            },

                            #[wrap(Some)]
                            set_end_widget = &gtk::Box {
                                gtk::Image {
                                    set_icon_name: Some("go-next-symbolic"),
                                    set_size_request: (50, 50),
                                },
                                add_controller = gtk::EventControllerMotion {
                                    connect_enter[scrolling] => move |_controller, _x, _y| {
                                        scrolling.replace(Some(Scrolling::RecentlyPlayedRight));
                                    },
                                    connect_leave[scrolling] => move |_controller| {
                                        scrolling.replace(None);
                                    }
                                }
                            }
                        },
                    },

                    gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        gtk::Box {
                            set_spacing: 7,

                            gtk::Label {
                                add_css_class: granite::STYLE_CLASS_H2_LABEL,
                                set_halign: gtk::Align::Start,
                                set_text: "Random"
                            },
                            gtk::Button {
                                set_icon_name: "media-playlist-shuffle-symbolic",
                                set_tooltip: "Rerandomize albums",
                                connect_clicked => DashboardIn::ClickedRandomize,
                            }
                        },
                        gtk::CenterBox {
                            #[wrap(Some)]
                            set_start_widget = &gtk::Box {
                                gtk::Image {
                                    set_icon_name: Some("go-previous-symbolic"),
                                    set_size_request: (50, 50),

                                    add_controller = gtk::EventControllerMotion {
                                        connect_enter[scrolling] => move |_controller, _x, _y| {
                                            scrolling.replace(Some(Scrolling::RandomAlbumLeft));
                                        },
                                        connect_leave[scrolling] => move |_controller| {
                                            scrolling.replace(None);
                                        }
                                    }
                                },
                            },

                            #[wrap(Some)]
                            set_center_widget = &model.random_album_scroll.clone() -> gtk::ScrolledWindow {
                                set_vscrollbar_policy: gtk::PolicyType::Never,
                                set_hscrollbar_policy: gtk::PolicyType::External,
                                set_hexpand: true,

                                gtk::Box {
                                    set_halign: gtk::Align::Start,

                                    model.random_album.clone() {
                                        set_halign: gtk::Align::Start,
                                        set_vexpand: true,
                                    },
                                }
                            },

                            #[wrap(Some)]
                            set_end_widget = &gtk::Box {
                                gtk::Image {
                                    set_icon_name: Some("go-next-symbolic"),
                                    set_size_request: (50, 50),
                                },
                                add_controller = gtk::EventControllerMotion {
                                    connect_enter[scrolling] => move |_controller, _x, _y| {
                                        scrolling.replace(Some(Scrolling::RandomAlbumRight));
                                    },
                                    connect_leave[scrolling] => move |_controller| {
                                        scrolling.replace(None);
                                    }
                                }
                            }
                        },
                    },

                    gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,

                        gtk::Label {
                            set_halign: gtk::Align::Start,
                            add_css_class: granite::STYLE_CLASS_H2_LABEL,
                            set_text: "Most Played",
                        },
                        gtk::CenterBox {
                            #[wrap(Some)]
                            set_start_widget = &gtk::Box {
                                gtk::Image {
                                    set_icon_name: Some("go-previous-symbolic"),
                                    set_size_request: (50, 50),

                                    add_controller = gtk::EventControllerMotion {
                                        connect_enter[scrolling] => move |_controller, _x, _y| {
                                            scrolling.replace(Some(Scrolling::MostPlayedLeft));
                                        },
                                        connect_leave[scrolling] => move |_controller| {
                                            scrolling.replace(None);
                                        }
                                    }
                                },
                            },

                            #[wrap(Some)]
                            set_center_widget = &model.most_played_scroll.clone() -> gtk::ScrolledWindow {
                                set_vscrollbar_policy: gtk::PolicyType::Never,
                                set_hscrollbar_policy: gtk::PolicyType::External,
                                set_hexpand: true,

                                gtk::Box {
                                    set_halign: gtk::Align::Start,

                                    model.most_played.clone() {
                                        set_halign: gtk::Align::Start,
                                        set_vexpand: true,
                                    }
                                }
                            },

                            #[wrap(Some)]
                            set_end_widget = &gtk::Box {
                                gtk::Image {
                                    set_icon_name: Some("go-next-symbolic"),
                                    set_size_request: (50, 50),
                                },
                                add_controller = gtk::EventControllerMotion {
                                    connect_enter[scrolling] => move |_controller, _x, _y| {
                                        scrolling.replace(Some(Scrolling::MostPlayedRight));
                                    },
                                    connect_leave[scrolling] => move |_controller| {
                                        scrolling.replace(None);
                                    }
                                }
                            }
                        },
                    }
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
            DashboardIn::SearchChanged(search) => {
                for album in self
                    .recently_added_list
                    .iter()
                    .chain(self.recently_played_list.iter())
                    .chain(self.random_album_list.iter())
                    .chain(self.most_played_list.iter())
                {
                    use gtk::glib::object::Cast;

                    // get the Label of the AlbumElement
                    let overlay = album.widget().first_child().unwrap();
                    let button = overlay.first_child().unwrap();
                    let bo = button.first_child().unwrap();
                    let cover = bo.first_child().unwrap();
                    let title = cover.next_sibling().unwrap();
                    let title = title.downcast::<gtk::Label>().expect("unepected element");

                    let artist = title.next_sibling().unwrap();
                    let artist = artist.downcast::<gtk::Label>().expect("unexpected element");
                    let title_artist = format!("{} {}", title.text(), artist.text());

                    //actual matching
                    let matcher = fuzzy_matcher::skim::SkimMatcherV2::default();
                    let score = matcher.fuzzy_match(&title_artist, &search);
                    album.widget().set_visible(score.is_some())
                }
            }
            DashboardIn::AlbumElement(msg) => match msg {
                AlbumElementOut::Clicked(clicked) => {
                    sender.output(DashboardOut::ClickedAlbum(clicked)).unwrap();
                }
                AlbumElementOut::DisplayToast(title) => sender
                    .output(DashboardOut::DisplayToast(title))
                    .expect("sending failed"),
                AlbumElementOut::FavoriteClicked(id, state) => sender
                    .output(DashboardOut::FavoriteClicked(id, state))
                    .expect("sending failed"),
            },
            DashboardIn::ClickedRandomize => {
                self.random_album_list.clear();
                self.random_album.remove_all();
                let mut rng = rand::thread_rng();
                let mut albums = self.subsonic.borrow().albums().clone();
                albums.shuffle(&mut rng);
                self.random_album_list = albums
                    .iter()
                    .take(10)
                    .map(|album| {
                        AlbumElement::builder()
                            .launch((
                                self.subsonic.clone(),
                                AlbumElementInit::Child(Box::new(album.clone())),
                            ))
                            .forward(sender.input_sender(), DashboardIn::AlbumElement)
                    })
                    .collect();
                for album in &self.random_album_list {
                    self.random_album.append(album.widget());
                }
            }
            DashboardIn::FavoritedAlbum(id, state) => {
                for album in &self.recently_added_list {
                    album.emit(AlbumElementIn::Favorited(id.clone(), state));
                }
                for album in &self.recently_played_list {
                    album.emit(AlbumElementIn::Favorited(id.clone(), state));
                }
                for album in &self.random_album_list {
                    album.emit(AlbumElementIn::Favorited(id.clone(), state));
                }
                for album in &self.most_played_list {
                    album.emit(AlbumElementIn::Favorited(id.clone(), state));
                }
            }
        }
    }

    fn update_cmd(
        &mut self,
        msg: Self::CommandOutput,
        sender: relm4::ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match msg {
            DashboardCmd::Error(msg) => sender.output(DashboardOut::DisplayToast(msg)).unwrap(),
            DashboardCmd::LoadedRecentlyPlayed(Err(_e)) => {}
            DashboardCmd::LoadedRecentlyPlayed(Ok(list)) => {
                self.recently_played_list = list
                    .iter()
                    .map(|album| {
                        AlbumElement::builder()
                            .launch((
                                self.subsonic.clone(),
                                AlbumElementInit::Child(Box::new(album.clone())),
                            ))
                            .forward(sender.input_sender(), DashboardIn::AlbumElement)
                    })
                    .collect();
                for album in &self.recently_played_list {
                    self.recently_played.append(album.widget());
                }
            }
        }
    }
}
