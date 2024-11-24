use std::cell::RefCell;
use std::rc::Rc;

use fuzzy_matcher::FuzzyMatcher;
use rand::prelude::SliceRandom;
use relm4::{
    gtk::{
        self,
        prelude::{AdjustmentExt, BoxExt, ButtonExt, OrientableExt, WidgetExt},
    },
    RelmWidgetExt,
};

use crate::{client::Client, subsonic::Subsonic};
use crate::{
    components::album_element::{AlbumElement, AlbumElementIn, AlbumElementInit, AlbumElementOut},
    settings::Settings,
};

use super::albums_view::get_info_of_flowboxchild;

#[derive(Debug, Clone)]
enum Scrolling {
    None,
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

    recently_added_scroll: gtk::ScrolledWindow,
    recently_added_list: relm4::factory::FactoryVecDeque<AlbumElement>,

    recently_played_scroll: gtk::ScrolledWindow,
    recently_played_list: relm4::factory::FactoryVecDeque<AlbumElement>,

    random_album_scroll: gtk::ScrolledWindow,
    random_album_list: relm4::factory::FactoryVecDeque<AlbumElement>,

    most_played_scroll: gtk::ScrolledWindow,
    most_played_list: relm4::factory::FactoryVecDeque<AlbumElement>,
}

#[derive(Debug)]
pub enum DashboardOut {
    ClickedAlbum(AlbumElementInit),
    DisplayToast(String),
    FavoriteClicked(String, bool),
}

#[derive(Debug)]
pub enum DashboardIn {
    FilterChanged,
    AlbumElement(AlbumElementOut),
    ClickedRandomize,
    FavoritedAlbum(String, bool),
    CoverSizeChanged,
    ScrollOuter(f64),
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

            recently_added_scroll: gtk::ScrolledWindow::default(),
            recently_added_list: relm4::factory::FactoryVecDeque::builder()
                .launch(gtk::FlowBox::default())
                .forward(sender.input_sender(), DashboardIn::AlbumElement),

            recently_played_scroll: gtk::ScrolledWindow::default(),
            recently_played_list: relm4::factory::FactoryVecDeque::builder()
                .launch(gtk::FlowBox::default())
                .forward(sender.input_sender(), DashboardIn::AlbumElement),

            random_album_scroll: gtk::ScrolledWindow::default(),
            random_album_list: relm4::factory::FactoryVecDeque::builder()
                .launch(gtk::FlowBox::default())
                .forward(sender.input_sender(), DashboardIn::AlbumElement),

            most_played_scroll: gtk::ScrolledWindow::default(),
            most_played_list: relm4::factory::FactoryVecDeque::builder()
                .launch(gtk::FlowBox::default())
                .forward(sender.input_sender(), DashboardIn::AlbumElement),
        };

        //load recently added albums
        let mut albums = subsonic.borrow().albums().clone();
        albums.sort_by(|a, b| b.created.cmp(&a.created));
        let list: Vec<(Rc<RefCell<Subsonic>>, AlbumElementInit)> = albums
            .iter()
            .take(10)
            .map(|album| {
                (
                    subsonic.clone(),
                    AlbumElementInit::Child(Box::new(album.clone())),
                )
            })
            .collect();
        let mut guard = model.recently_added_list.guard();
        for infos in list {
            guard.push_back(infos);
        }
        drop(guard);

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
        let list: Vec<AlbumElementInit> = albums
            .iter()
            .take(10)
            .map(|album| AlbumElementInit::Child(Box::new(album.clone())))
            .collect();
        let mut guard = model.most_played_list.guard();
        for infos in list {
            guard.push_back((model.subsonic.clone(), infos));
        }
        drop(guard);

        let (scroll_sender, receiver) = async_channel::unbounded::<Scrolling>();
        let widgets = view_output!();

        //update scrolling of boxes
        let recently_added_scroll = model.recently_added_scroll.clone();
        let recently_played_scroll = model.recently_played_scroll.clone();
        let random_album_scroll = model.random_album_scroll.clone();
        let most_played_scroll = model.most_played_scroll.clone();

        gtk::glib::spawn_future_local(async move {
            let scrollings = Rc::new(RefCell::new(Scrolling::None));

            while let Ok(msg) = receiver.recv().await {
                scrollings.replace(msg.clone());

                if let Scrolling::None = msg {} else {
                    let scrolling = scrollings.clone();
                    let recently_added_scroll = recently_added_scroll.clone();
                    let recently_played_scroll = recently_played_scroll.clone();
                    let random_album_scroll = random_album_scroll.clone();
                    let most_played_scroll = most_played_scroll.clone();

                    //scroll the albums when arrow is hovered
                    gtk::glib::source::timeout_add_local(
                        core::time::Duration::from_millis(15),
                        move || {
                            const SCROLL_MOVE: f64 = 5f64;
                            enum Op {
                                Add,
                                Sub,
                            }
                            let adj_fn = |scroll: &gtk::ScrolledWindow, op: Op| {
                                let vadj = scroll.hadjustment();
                                match op {
                                    Op::Add => vadj.set_value(vadj.value() + SCROLL_MOVE),
                                    Op::Sub => vadj.set_value(vadj.value() - SCROLL_MOVE),
                                }
                                scroll.set_hadjustment(Some(&vadj));
                            };
                            match *scrolling.borrow() {
                                // when no scrolling end closure
                                Scrolling::None => return gtk::glib::ControlFlow::Break,
                                Scrolling::RecentlyAddedLeft => {
                                    adj_fn(&recently_added_scroll, Op::Sub);
                                }
                                Scrolling::RecentlyAddedRight => {
                                    adj_fn(&recently_added_scroll, Op::Add);
                                }
                                Scrolling::RecentlyPlayedLeft => {
                                    adj_fn(&recently_played_scroll, Op::Sub);
                                }
                                Scrolling::RecentlyPlayedRight => {
                                    adj_fn(&recently_played_scroll, Op::Add);
                                }
                                Scrolling::RandomAlbumLeft => {
                                    adj_fn(&random_album_scroll, Op::Sub);
                                }
                                Scrolling::RandomAlbumRight => {
                                    adj_fn(&random_album_scroll, Op::Add);
                                }
                                Scrolling::MostPlayedLeft => {
                                    adj_fn(&most_played_scroll, Op::Sub);
                                }
                                Scrolling::MostPlayedRight => {
                                    adj_fn(&most_played_scroll, Op::Add);
                                }
                            }
                            gtk::glib::ControlFlow::Continue
                        },
                    );
                }
            }
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

            #[name = "outer_scroll"]
            gtk::ScrolledWindow {
                set_hexpand: true,
                set_vexpand: true,
                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_halign: gtk::Align::Fill,
                    set_spacing: 30,

                    gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,

                        gtk::CenterBox {
                            #[wrap(Some)]
                            set_start_widget = &gtk::Label {
                                add_css_class: granite::STYLE_CLASS_H2_LABEL,
                                set_halign: gtk::Align::Start,
                                set_text: "Newly added",
                            },
                            #[wrap(Some)]
                            set_end_widget = &gtk::Box {
                                gtk::Image {
                                    set_icon_name: Some("go-previous-symbolic"),
                                    set_size_request: (40, 30),

                                    add_controller = gtk::GestureClick {
                                        connect_pressed[scroll_sender] => move |_btn, _, _, _| {
                                            scroll_sender.try_send(Scrolling::RecentlyAddedLeft).unwrap();
                                        },
                                        connect_released[scroll_sender] => move |_btn, _, _, _| {
                                            scroll_sender.try_send(Scrolling::None).unwrap();
                                        }
                                    },
                                },
                                gtk::Image {
                                    set_icon_name: Some("go-next-symbolic"),
                                    set_size_request: (40, 30),
                                    set_margin_end: 10,

                                    add_controller = gtk::GestureClick {
                                        connect_pressed[scroll_sender] => move |_btn, _, _, _| {
                                            scroll_sender.try_send(Scrolling::RecentlyAddedRight).unwrap();
                                        },
                                        connect_released[scroll_sender] => move |_btn, _, _, _| {
                                            scroll_sender.try_send(Scrolling::None).unwrap();
                                        }
                                    },
                                },
                            },
                        },
                        model.recently_added_scroll.clone() -> gtk::ScrolledWindow {
                            set_vscrollbar_policy: gtk::PolicyType::Never,
                            set_hscrollbar_policy: gtk::PolicyType::External,
                            set_hexpand: true,

                            add_controller = gtk::EventControllerScroll {
                                set_flags: gtk::EventControllerScrollFlags::VERTICAL,
                                connect_scroll[sender] => move |_event, _x, y| {
                                    sender.input(DashboardIn::ScrollOuter(y));
                                    gtk::glib::signal::Propagation::Stop
                                }
                            },

                            model.recently_added_list.widget().clone() {
                                set_valign: gtk::Align::Start,
                                set_vexpand: true,
                                set_max_children_per_line: 100,
                                set_min_children_per_line: 20,
                            },
                        },
                    },

                    gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,

                        gtk::CenterBox {
                            #[wrap(Some)]
                            set_start_widget = &gtk::Label {
                                add_css_class: granite::STYLE_CLASS_H2_LABEL,
                                set_halign: gtk::Align::Start,
                                set_text: "Recently Played",
                            },
                            #[wrap(Some)]
                            set_end_widget = &gtk::Box {
                                gtk::Image {
                                    set_icon_name: Some("go-previous-symbolic"),
                                    set_size_request: (40, 30),

                                    add_controller = gtk::GestureClick {
                                        connect_pressed[scroll_sender] => move |_btn, _, _, _| {
                                            scroll_sender.try_send(Scrolling::RecentlyPlayedLeft).unwrap();
                                        },
                                        connect_released[scroll_sender] => move |_btn, _, _, _| {
                                            scroll_sender.try_send(Scrolling::None).unwrap();
                                        }
                                    }
                                },
                                gtk::Image {
                                    set_icon_name: Some("go-next-symbolic"),
                                    set_size_request: (40, 30),
                                    set_margin_end: 10,

                                    add_controller = gtk::GestureClick {
                                        connect_pressed[scroll_sender] => move |_btn, _, _, _| {
                                            scroll_sender.try_send(Scrolling::RecentlyPlayedRight).unwrap();
                                        },
                                        connect_released[scroll_sender] => move |_btn, _, _, _| {
                                            scroll_sender.try_send(Scrolling::None).unwrap();
                                        }
                                    }
                                },
                            }
                        },
                        model.recently_played_scroll.clone() -> gtk::ScrolledWindow {
                            set_vscrollbar_policy: gtk::PolicyType::Never,
                            set_hscrollbar_policy: gtk::PolicyType::External,
                            set_hexpand: true,

                            add_controller = gtk::EventControllerScroll {
                                set_flags: gtk::EventControllerScrollFlags::VERTICAL,
                                connect_scroll[sender] => move |_event, _x, y| {
                                    sender.input(DashboardIn::ScrollOuter(y));
                                    gtk::glib::signal::Propagation::Stop
                                }
                            },

                            model.recently_played_list.widget().clone() {
                                set_valign: gtk::Align::Start,
                                set_vexpand: true,
                                set_max_children_per_line: 100,
                                set_min_children_per_line: 20,
                            },
                        },
                    },

                    gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,

                        gtk::CenterBox {
                            #[wrap(Some)]
                            set_start_widget = &gtk::Box {
                                set_spacing: 10,

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
                            #[wrap(Some)]
                            set_end_widget = &gtk::Box {
                                gtk::Image {
                                    set_icon_name: Some("go-previous-symbolic"),
                                    set_size_request: (40, 30),

                                    add_controller = gtk::GestureClick {
                                        connect_pressed[scroll_sender] => move |_btn, _, _, _| {
                                            scroll_sender.try_send(Scrolling::RandomAlbumLeft).unwrap();
                                        },
                                        connect_released[scroll_sender] => move |_btn, _, _, _| {
                                            scroll_sender.try_send(Scrolling::None).unwrap();
                                        }
                                    }
                                },

                                gtk::Image {
                                    set_icon_name: Some("go-next-symbolic"),
                                    set_size_request: (40, 30),
                                    set_margin_end: 10,

                                    add_controller = gtk::GestureClick {
                                        connect_pressed[scroll_sender] => move |_btn, _, _, _| {
                                            scroll_sender.try_send(Scrolling::RandomAlbumRight).unwrap();
                                        },
                                        connect_released[scroll_sender] => move |_btn, _, _, _| {
                                            scroll_sender.try_send(Scrolling::None).unwrap();
                                        }
                                    }
                                }
                            },
                        },

                        model.random_album_scroll.clone() -> gtk::ScrolledWindow {
                            set_vscrollbar_policy: gtk::PolicyType::Never,
                            set_hscrollbar_policy: gtk::PolicyType::External,
                            set_hexpand: true,

                            add_controller = gtk::EventControllerScroll {
                                set_flags: gtk::EventControllerScrollFlags::VERTICAL,
                                connect_scroll[sender] => move |_event, _x, y| {
                                    sender.input(DashboardIn::ScrollOuter(y));
                                    gtk::glib::signal::Propagation::Stop
                                }
                            },

                            model.random_album_list.widget().clone() {
                                set_halign: gtk::Align::Start,
                                set_vexpand: true,
                                set_max_children_per_line: 100,
                                set_min_children_per_line: 20,
                            }
                        },
                    },

                    gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,

                        gtk::CenterBox {
                            #[wrap(Some)]
                            set_start_widget = &gtk::Label {
                                set_halign: gtk::Align::Start,
                                add_css_class: granite::STYLE_CLASS_H2_LABEL,
                                set_text: "Most Played",
                            },

                            #[wrap(Some)]
                            set_end_widget = &gtk::Box {
                                gtk::Image {
                                    set_icon_name: Some("go-previous-symbolic"),
                                    set_size_request: (40, 30),

                                    add_controller = gtk::GestureClick {
                                        connect_pressed[scroll_sender] => move |_btn, _, _, _| {
                                            scroll_sender.try_send(Scrolling::MostPlayedLeft).unwrap();
                                        },
                                        connect_released[scroll_sender] => move |_btn, _, _, _| {
                                            scroll_sender.try_send(Scrolling::None).unwrap();
                                        }
                                    }
                                },
                                gtk::Image {
                                    set_icon_name: Some("go-next-symbolic"),
                                    set_size_request: (40, 30),
                                    set_margin_end: 10,

                                    add_controller = gtk::GestureClick {
                                        connect_pressed[scroll_sender] => move |_btn, _, _, _| {
                                            scroll_sender.try_send(Scrolling::MostPlayedRight).unwrap();
                                        },
                                        connect_released[scroll_sender] => move |_btn, _, _, _| {
                                            scroll_sender.try_send(Scrolling::None).unwrap();
                                        }
                                    }
                                },
                            }
                        },
                        model.most_played_scroll.clone() -> gtk::ScrolledWindow {
                            set_vscrollbar_policy: gtk::PolicyType::Never,
                            set_hscrollbar_policy: gtk::PolicyType::External,
                            set_hexpand: true,

                            add_controller = gtk::EventControllerScroll {
                                set_flags: gtk::EventControllerScrollFlags::VERTICAL,
                                connect_scroll[sender] => move |_event, _x, y| {
                                    sender.input(DashboardIn::ScrollOuter(y));
                                    gtk::glib::signal::Propagation::Stop
                                }
                            },

                            model.most_played_list.widget().clone() {
                                set_halign: gtk::Align::Start,
                                set_vexpand: true,
                                set_max_children_per_line: 100,
                                set_min_children_per_line: 20,
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
            DashboardIn::FilterChanged => {
                let search_fn = |element: &gtk::FlowBoxChild| -> bool {
                    let mut search = Settings::get().lock().unwrap().search_text.clone();
                    let (title, artist) = get_info_of_flowboxchild(element);
                    let mut title_artist = format!("{} {}", title.text(), artist.text());

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
                };

                self.recently_added_list.widget().set_filter_func(search_fn);
                self.recently_played_list
                    .widget()
                    .set_filter_func(search_fn);
                self.random_album_list.widget().set_filter_func(search_fn);
                self.most_played_list.widget().set_filter_func(search_fn);
            }
            DashboardIn::AlbumElement(msg) => match msg {
                AlbumElementOut::Clicked(clicked) => {
                    sender.output(DashboardOut::ClickedAlbum(clicked)).unwrap();
                }
                AlbumElementOut::DisplayToast(title) => {
                    sender.output(DashboardOut::DisplayToast(title)).unwrap();
                }
                AlbumElementOut::FavoriteClicked(id, state) => sender
                    .output(DashboardOut::FavoriteClicked(id, state))
                    .unwrap(),
            },
            DashboardIn::ClickedRandomize => {
                self.random_album_list.guard().clear();
                let mut rng = rand::thread_rng();
                let mut albums = self.subsonic.borrow().albums().clone();
                albums.shuffle(&mut rng);

                let infos: Vec<AlbumElementInit> = albums
                    .iter()
                    .take(10)
                    .map(|album| AlbumElementInit::Child(Box::new(album.clone())))
                    .collect();
                let mut guard = self.random_album_list.guard();
                for info in infos {
                    guard.push_back((self.subsonic.clone(), info));
                }
            }
            DashboardIn::FavoritedAlbum(id, state) => {
                self.recently_added_list
                    .broadcast(AlbumElementIn::Favorited(id.clone(), state));
                self.recently_played_list
                    .broadcast(AlbumElementIn::Favorited(id.clone(), state));
                self.random_album_list
                    .broadcast(AlbumElementIn::Favorited(id.clone(), state));
                self.most_played_list
                    .broadcast(AlbumElementIn::Favorited(id, state));
            }
            DashboardIn::CoverSizeChanged => {
                let size = Settings::get().lock().unwrap().cover_size;
                for album in self.recently_added_list.iter() {
                    album.change_size(size);
                }
                for album in self.recently_played_list.iter() {
                    album.change_size(size);
                }
                for album in self.random_album_list.iter() {
                    album.change_size(size);
                }
                for album in self.most_played_list.iter() {
                    album.change_size(size);
                }
            }
            DashboardIn::ScrollOuter(y) => {
                if y < 0.0 {
                    widgets
                        .outer_scroll
                        .emit_scroll_child(gtk::ScrollType::StepUp, false);
                } else {
                    widgets
                        .outer_scroll
                        .emit_scroll_child(gtk::ScrollType::StepDown, false);
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
                let infos: Vec<AlbumElementInit> = list
                    .iter()
                    .map(|album| AlbumElementInit::Child(Box::new(album.clone())))
                    .collect();
                let mut guard = self.recently_played_list.guard();
                for info in infos {
                    guard.push_back((self.subsonic.clone(), info));
                }
            }
        }
    }
}
