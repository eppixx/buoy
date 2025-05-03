use std::cell::RefCell;
use std::rc::Rc;

use fuzzy_matcher::FuzzyMatcher;
use gettextrs::gettext;
use rand::prelude::SliceRandom;
use relm4::{
    gtk::{
        self,
        prelude::{AdjustmentExt, BoxExt, ButtonExt, OrientableExt, WidgetExt},
    },
    RelmWidgetExt,
};

use crate::{
    client::Client,
    factory::album_element::{
        get_info_of_flowboxchild, AlbumElement, AlbumElementIn, AlbumElementOut,
    },
    gtk_helper::{loading_widget::LoadingWidgetState, stack::StackExt},
    settings::Settings,
    subsonic::Subsonic,
    types::Id,
};

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
    recently_added_list: relm4::factory::FactoryVecDeque<AlbumElement>,
    recently_played_list: relm4::factory::FactoryVecDeque<AlbumElement>,
    random_album_list: relm4::factory::FactoryVecDeque<AlbumElement>,
    most_played_list: relm4::factory::FactoryVecDeque<AlbumElement>,
}

#[derive(Debug)]
pub enum DashboardOut {
    ClickedAlbum(Id),
    DisplayToast(String),
    FavoriteClicked(String, bool),
}

#[derive(Debug)]
pub enum DashboardIn {
    FilterChanged,
    AlbumElement(AlbumElementOut),
    ClickedRandomize,
    UpdateFavoriteAlbum(String, bool),
    ScrollOuter(f64),
    UpdateRecentlyPlayed,
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
            recently_added_list: relm4::factory::FactoryVecDeque::builder()
                .launch(gtk::FlowBox::default())
                .forward(sender.input_sender(), DashboardIn::AlbumElement),
            recently_played_list: relm4::factory::FactoryVecDeque::builder()
                .launch(gtk::FlowBox::default())
                .forward(sender.input_sender(), DashboardIn::AlbumElement),
            random_album_list: relm4::factory::FactoryVecDeque::builder()
                .launch(gtk::FlowBox::default())
                .forward(sender.input_sender(), DashboardIn::AlbumElement),
            most_played_list: relm4::factory::FactoryVecDeque::builder()
                .launch(gtk::FlowBox::default())
                .forward(sender.input_sender(), DashboardIn::AlbumElement),
        };

        //load recently added albums
        let mut albums = subsonic.borrow().albums().clone();
        albums.sort_by(|a, b| b.created.cmp(&a.created));
        let list: Vec<(Rc<RefCell<Subsonic>>, Id)> = albums
            .iter()
            .take(Settings::get().lock().unwrap().dashboard_line_items)
            .map(|album| (subsonic.clone(), Id::album(&album.id)))
            .collect();
        let mut guard = model.recently_added_list.guard();
        list.into_iter().for_each(|info| _ = guard.push_back(info));
        drop(guard);

        //load random albums
        sender.input(DashboardIn::ClickedRandomize);

        //load most played albums
        albums.sort_by(|a, b| b.play_count.cmp(&a.play_count));
        let ids: Vec<Id> = albums
            .iter()
            .take(Settings::get().lock().unwrap().dashboard_line_items)
            .map(|album| Id::album(&album.id))
            .collect();
        let mut guard = model.most_played_list.guard();
        ids.into_iter()
            .for_each(|id| _ = guard.push_back((model.subsonic.clone(), id)));
        drop(guard);

        let (scroll_sender, receiver) = async_channel::unbounded::<Scrolling>();
        let widgets = view_output!();

        //update scrolling of boxes
        let recently_added_scroll = widgets.recently_added_scroll.clone();
        let recently_played_scroll = widgets.recently_played_scroll.clone();
        let random_album_scroll = widgets.random_album_scroll.clone();
        let most_played_scroll = widgets.most_played_scroll.clone();

        gtk::glib::spawn_future_local(async move {
            let scrollings = Rc::new(RefCell::new(Scrolling::None));

            while let Ok(msg) = receiver.recv().await {
                scrollings.replace(msg.clone());

                if let Scrolling::None = msg {
                } else {
                    let scrolling = scrollings.clone();
                    let recently_added_scroll = recently_added_scroll.clone();
                    let recently_played_scroll = recently_played_scroll.clone();
                    let random_album_scroll = random_album_scroll.clone();
                    let most_played_scroll = most_played_scroll.clone();

                    //scroll the albums when arrow is activated
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

        // set uniform size
        let group = gtk::SizeGroup::new(gtk::SizeGroupMode::Vertical);
        group.add_widget(&widgets.recently_added_scroll);
        group.add_widget(&widgets.recently_stack);
        group.add_widget(&widgets.random_album_scroll);
        group.add_widget(&widgets.most_played_scroll);

        relm4::ComponentParts { model, widgets }
    }

    view! {
        gtk::Box {
            append: outer_scroll = &gtk::ScrolledWindow {
                add_css_class: "dashboard-view",
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
                                set_margin_horizontal: 7,
                                set_text: &gettext("Newly added"),
                            },
                            #[wrap(Some)]
                            set_end_widget = &gtk::Box {
                                gtk::Image {
                                    set_icon_name: Some("go-previous-symbolic"),
                                    set_size_request: (40, 30),
                                    set_tooltip: &gettext("Press and hold mouse button to scroll"),

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
                                    set_tooltip: &gettext("Press and hold mouse button to scroll"),

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
                        append: recently_added_scroll = &gtk::ScrolledWindow {
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
                                set_margin_horizontal: 7,
                                set_text: &gettext("Recently Played"),
                            },
                            #[wrap(Some)]
                            set_end_widget = &gtk::Box {
                                gtk::Image {
                                    set_icon_name: Some("go-previous-symbolic"),
                                    set_size_request: (40, 30),
                                    set_tooltip: &gettext("Press and hold mouse button to scroll"),

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
                                    set_tooltip: &gettext("Press and hold mouse button to scroll"),

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
                        append: recently_stack = &gtk::Stack {
                            set_transition_type: gtk::StackTransitionType::Crossfade,
                            set_transition_duration: 100,

                            add_enumed[LoadingWidgetState::NotEmpty]: recently_played_scroll = &gtk::ScrolledWindow {
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
                            add_enumed[LoadingWidgetState::Loading] = &gtk::Box {
                                set_orientation: gtk::Orientation::Vertical,
                                set_valign: gtk::Align::Center,

                                gtk::Spinner {
                                    add_css_class: "size32",
                                    set_spinning: true,
                                    start: (),
                                }
                            },
                            add_enumed[LoadingWidgetState::Empty] = &gtk::Box {
                                set_orientation: gtk::Orientation::Vertical,
                                set_valign: gtk::Align::Center,
                                set_spacing: 20,

                                gtk::Label {
                                    set_label: &gettext("Nothing played recently"),
                                    add_css_class: granite::STYLE_CLASS_H2_LABEL,
                                },
                                gtk::Label {
                                    set_label: &gettext("You might need to turn on scrobbling to populate this area"),
                                    add_css_class: granite::STYLE_CLASS_H3_LABEL,
                                }
                            },
                            set_visible_child_enum: &LoadingWidgetState::Loading,
                        }
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
                                    set_margin_horizontal: 7,
                                    set_text: &gettext("Random"),
                                },
                                gtk::Button {
                                    set_icon_name: "media-playlist-shuffle-symbolic",
                                    set_tooltip: &gettext("Rerandomize albums"),
                                    connect_clicked => DashboardIn::ClickedRandomize,
                                }
                            },
                            #[wrap(Some)]
                            set_end_widget = &gtk::Box {
                                gtk::Image {
                                    set_icon_name: Some("go-previous-symbolic"),
                                    set_size_request: (40, 30),
                                    set_tooltip: &gettext("Press and hold mouse button to scroll"),

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
                                    set_tooltip: &gettext("Press and hold mouse button to scroll"),

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

                        append: random_album_scroll = &gtk::ScrolledWindow {
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
                                set_margin_horizontal: 7,
                                set_text: &gettext("Most Played"),
                            },

                            #[wrap(Some)]
                            set_end_widget = &gtk::Box {
                                gtk::Image {
                                    set_icon_name: Some("go-previous-symbolic"),
                                    set_size_request: (40, 30),
                                    set_tooltip: &gettext("Press and hold mouse button to scroll"),

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
                                    set_tooltip: &gettext("Press and hold mouse button to scroll"),

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
                        append: most_played_scroll = &gtk::ScrolledWindow {
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
                    let Some((title, artist)) = get_info_of_flowboxchild(element) else {
                        return true;
                    };
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

                // apply search_fn as filter to FlowBox
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
                let mut rng = rand::rng();
                let mut albums = self.subsonic.borrow().albums().clone();
                albums.shuffle(&mut rng);

                let ids: Vec<Id> = albums
                    .iter()
                    .take(Settings::get().lock().unwrap().dashboard_line_items)
                    .map(|album| Id::album(&album.id))
                    .collect();
                let mut guard = self.random_album_list.guard();
                ids.into_iter()
                    .for_each(|id| _ = guard.push_back((self.subsonic.clone(), id)));
            }
            DashboardIn::UpdateFavoriteAlbum(id, state) => {
                self.recently_added_list
                    .broadcast(AlbumElementIn::Favorited(id.clone(), state));
                self.recently_played_list
                    .broadcast(AlbumElementIn::Favorited(id.clone(), state));
                self.random_album_list
                    .broadcast(AlbumElementIn::Favorited(id.clone(), state));
                self.most_played_list
                    .broadcast(AlbumElementIn::Favorited(id, state));
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
            DashboardIn::UpdateRecentlyPlayed => {
                let dashboard_line_items = Settings::get().lock().unwrap().dashboard_line_items;
                sender.oneshot_command(async move {
                    let client = match Client::get() {
                        None => return DashboardCmd::Error(String::from("no client found")),
                        Some(client) => client,
                    };
                    DashboardCmd::LoadedRecentlyPlayed(
                        client
                            .get_album_list2(
                                submarine::api::get_album_list::Order::Recent,
                                Some(dashboard_line_items),
                                None,
                                None::<String>,
                            )
                            .await,
                    )
                });
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
            DashboardCmd::Error(msg) => sender.output(DashboardOut::DisplayToast(msg)).unwrap(),
            DashboardCmd::LoadedRecentlyPlayed(Err(_e)) => {}
            DashboardCmd::LoadedRecentlyPlayed(Ok(list)) => {
                if list.is_empty() {
                    widgets
                        .recently_stack
                        .set_visible_child_enum(&LoadingWidgetState::Empty);
                    return;
                }

                // remove previous entries
                self.recently_played_list.guard().clear();

                // add new entries
                let ids: Vec<Id> = list.iter().map(|album| Id::album(&album.id)).collect();
                widgets
                    .recently_stack
                    .set_visible_child_enum(&LoadingWidgetState::NotEmpty);
                let mut guard = self.recently_played_list.guard();
                ids.into_iter()
                    .for_each(|id| _ = guard.push_back((self.subsonic.clone(), id)));
            }
        }
    }
}
