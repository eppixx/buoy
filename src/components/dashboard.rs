use rand::prelude::SliceRandom;
use relm4::gtk::{
    self,
    prelude::{AdjustmentExt, BoxExt, ButtonExt, OrientableExt, WidgetExt},
};
use relm4::{ComponentController, RelmRemoveAllExt, RelmWidgetExt};

use std::cell::RefCell;
use std::rc::Rc;

use crate::client::Client;
use crate::components::album_element::{AlbumElement, AlbumElementInit, AlbumElementOut};
use crate::subsonic::Subsonic;

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
    // recently_added_list: Vec<submarine::data::Child>,
    recently_played: gtk::Box,
    recently_played_scroll: gtk::ScrolledWindow,

    random_album: gtk::Box,
    random_album_scroll: gtk::ScrolledWindow,

    most_played: gtk::Box,
    most_played_scroll: gtk::ScrolledWindow,
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
    Favorited(String, bool),
}

#[derive(Debug)]
pub enum DashboardCmd {
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
        let model = Self {
            subsonic: subsonic.clone(),

            recently_added: gtk::Box::default(),
            recently_added_scroll: gtk::ScrolledWindow::default(),
            // recently_added_list: vec![],
            recently_played: gtk::Box::default(),
            recently_played_scroll: gtk::ScrolledWindow::default(),

            random_album: gtk::Box::default(),
            random_album_scroll: gtk::ScrolledWindow::default(),

            most_played: gtk::Box::default(),
            most_played_scroll: gtk::ScrolledWindow::default(),
        };

        //load recently added albums
        let mut albums = subsonic.borrow().albums().clone();
        albums.sort_by(|a, b| a.created.cmp(&b.created));
        albums
            .iter()
            .take(10)
            .map(|album| {
                // model.recently_added_list.push(album.clone());
                AlbumElement::builder()
                    .launch((
                        subsonic.clone(),
                        AlbumElementInit::Child(Box::new(album.clone())),
                    ))
                    .forward(sender.input_sender(), DashboardIn::AlbumElement)
            })
            .for_each(|album| {
                model.recently_added.append(album.widget());
            });

        //load recently played albums
        sender.oneshot_command(async move {
            let client = Client::get().unwrap();
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
        albums
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
            .for_each(|album| {
                model.most_played.append(album.widget());
            });

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
                    set_spacing: 20,

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
            DashboardIn::SearchChanged(_search) => {
                // unimplemented!("search in dashboard"); //TODO implement
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
                self.random_album.remove_all();
                let mut rng = rand::thread_rng();
                let mut albums = self.subsonic.borrow().albums().clone();
                albums.shuffle(&mut rng);
                albums
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
                    .for_each(|album| self.random_album.append(album.widget()));
            }
            DashboardIn::Favorited(_id, state) => {
                tracing::error!("implement favorite dashboard");
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
            DashboardCmd::LoadedRecentlyPlayed(Err(_e)) => {}
            DashboardCmd::LoadedRecentlyPlayed(Ok(list)) => {
                list.iter()
                    .map(|album| {
                        AlbumElement::builder()
                            .launch((
                                self.subsonic.clone(),
                                AlbumElementInit::Child(Box::new(album.clone())),
                            ))
                            .forward(sender.input_sender(), DashboardIn::AlbumElement)
                    })
                    .for_each(|album| {
                        self.recently_played.append(album.widget());
                    });
            }
        }
    }
}
