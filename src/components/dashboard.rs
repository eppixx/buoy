use relm4::gtk::{
    self,
    prelude::{BoxExt, OrientableExt, WidgetExt},
};
use relm4::{ComponentController, RelmWidgetExt};

use std::cell::RefCell;
use std::rc::Rc;

use crate::components::album_element::{AlbumElement, AlbumElementInit, AlbumElementOut};
use crate::subsonic::Subsonic;

#[derive(Debug, Default)]
pub struct Dashboard {
    recently_added: gtk::Box,
    recently_played: gtk::FlowBox,
    random_album: gtk::FlowBox,
    most_played: gtk::Box,
}

#[derive(Debug)]
pub enum DashboardOut {
    ClickedAlbum(AlbumElementInit),
    DisplayToast(String),
}

#[derive(Debug)]
pub enum DashboardIn {
    SearchChanged(String),
    AlbumElement(AlbumElementOut),
}

#[relm4::component(pub)]
impl relm4::Component for Dashboard {
    type Init = Rc<RefCell<Subsonic>>;
    type Input = DashboardIn;
    type Output = DashboardOut;
    type CommandOutput = ();

    fn init(
        subsonic: Self::Init,
        root: Self::Root,
        sender: relm4::ComponentSender<Self>,
    ) -> relm4::ComponentParts<Self> {
        let model = Dashboard::default();

        //load recently added albums
        let mut albums = subsonic.borrow().albums().clone();
        albums.sort_by(|a, b| a.created.cmp(&b.created));
        let albums: Vec<relm4::Controller<AlbumElement>> = albums
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
        for album in albums {
            model.recently_added.append(album.widget());
        }

        //load most played albums
        let mut albums = subsonic.borrow().albums().clone();
        albums.sort_by(|a, b| b.play_count.cmp(&a.play_count));
        let albums: Vec<relm4::Controller<AlbumElement>> = albums
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
        for album in albums {
            model.most_played.append(album.widget());
        }
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

                    gtk::Label {
                        add_css_class: "h3",
                        set_halign: gtk::Align::Start,
                        set_text: "Random"
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
            },
        }
    }
}
