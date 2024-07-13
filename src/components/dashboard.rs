use rand::prelude::SliceRandom;
use relm4::gtk::{
    self,
    prelude::{BoxExt, ButtonExt, OrientableExt, WidgetExt},
};
use relm4::{ComponentController, RelmRemoveAllExt, RelmWidgetExt};

use std::cell::RefCell;
use std::rc::Rc;

use crate::client::Client;
use crate::components::album_element::{AlbumElement, AlbumElementInit, AlbumElementOut};
use crate::subsonic::Subsonic;

#[derive(Debug)]
pub struct Dashboard {
    subsonic: Rc<RefCell<Subsonic>>,
    recently_added: gtk::Box,
    recently_played: gtk::Box,
    random_album: gtk::Box,
    most_played: gtk::Box,
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
            recently_played: gtk::Box::default(),
            random_album: gtk::Box::default(),
            most_played: gtk::Box::default(),
        };

        //load recently added albums
        let mut albums = subsonic.borrow().albums().clone();
        albums.sort_by(|a, b| a.created.cmp(&b.created));
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

        let widgets = view_output!();
        relm4::ComponentParts { model, widgets }
    }

    view! {
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            add_css_class: "dashboard-view",
            set_margin_horizontal: 7,

            gtk::WindowHandle {
                gtk::Label {
                    add_css_class: "h2",
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
                        add_css_class: "h3",
                        set_halign: gtk::Align::Start,
                        set_text: "Newly added",
                    },
                    gtk::ScrolledWindow {
                        set_vscrollbar_policy: gtk::PolicyType::Never,

                        model.recently_added.clone() {
                            set_halign: gtk::Align::Start,
                            set_vexpand: true,
                        }
                    },

                    gtk::Label {
                        add_css_class: "h3",
                        set_halign: gtk::Align::Start,
                        set_text: "Recently Played",
                    },
                    gtk::ScrolledWindow {
                        set_vscrollbar_policy: gtk::PolicyType::Never,

                        model.recently_played.clone() {
                            set_halign: gtk::Align::Start,
                            set_vexpand: true,
                        }
                    },

                    gtk::Box {
                        set_spacing: 7,

                        gtk::Label {
                            add_css_class: "h3",
                            set_halign: gtk::Align::Start,
                            set_text: "Random"
                        },
                        gtk::Button {
                            set_icon_name: "media-playlist-shuffle-symbolic",
                            set_tooltip: "Rerandomize albums",
                            connect_clicked => DashboardIn::ClickedRandomize,
                        }
                    },
                    gtk::ScrolledWindow {
                        set_vscrollbar_policy: gtk::PolicyType::Never,

                        gtk::Box {
                            set_halign: gtk::Align::Start,

                            model.random_album.clone() {
                                set_halign: gtk::Align::Start,
                                set_vexpand: true,
                            },
                        }
                    },

                    gtk::Label {
                        set_halign: gtk::Align::Start,
                        add_css_class: "h3",
                        set_text: "Most Played",
                    },
                    gtk::ScrolledWindow {
                        set_vscrollbar_policy: gtk::PolicyType::Never,

                        gtk::Box {
                            set_halign: gtk::Align::Start,

                            model.most_played.clone() {
                                set_halign: gtk::Align::Start,
                                set_vexpand: true,
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
                AlbumElementOut::FavoriteClicked(id, state) => sender.output(DashboardOut::FavoriteClicked(id, state)).expect("sending failed"),
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
            DashboardIn::Favorited(id, state) => {
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
