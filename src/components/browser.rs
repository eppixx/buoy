use relm4::{
    gtk::{
        self,
        traits::{BoxExt, ButtonExt, EditableExt, OrientableExt, ToggleButtonExt, WidgetExt},
    },
    Component, ComponentController,
};

use crate::{components::artists_view::Artists, components::dashboard::Dashboard, types::Id};

use super::{artists_view::ArtistsOut, dashboard::DashboardOutput};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum View {
    Dashboard,
    Artists,
    Albums,
    Tracks,
    Playlists,
    Id(Id),
}

#[derive(Debug, Default)]
pub struct Browser {
    history: Vec<View>, //includes current View; so should never be empty
    content: gtk::ScrolledWindow,
    back_btn: gtk::Button,
    dashboard_btn: gtk::ToggleButton,
    artists_btn: gtk::ToggleButton,
    albums_btn: gtk::ToggleButton,
    tracks_btn: gtk::ToggleButton,
    playlists_btn: gtk::ToggleButton,
}

#[derive(Debug)]
pub enum BrowserInput {
    SearchChanged(String),
    BackClicked,
    DashboardClicked,
    ArtistClicked,
    AlbumClicked,
    TrackClicked,
    PlaylistClicked,
    NewView(View),
    Dashboard(DashboardOutput),
    Artists(ArtistsOut),
}

#[relm4::component(pub)]
impl relm4::SimpleComponent for Browser {
    type Input = BrowserInput;
    type Output = ();
    type Init = ();

    fn init(
        _path: Self::Init,
        root: &Self::Root,
        sender: relm4::ComponentSender<Self>,
    ) -> relm4::ComponentParts<Self> {
        let model = Browser::default();
        let widgets = view_output!();

        sender.input(BrowserInput::NewView(View::Dashboard));

        relm4::ComponentParts { model, widgets }
    }

    view! {
        gtk::Box {
            add_css_class: "browser",
            set_orientation: gtk::Orientation::Vertical,

            gtk::Box {
                add_css_class: "pathbar",

                append = &model.back_btn.clone() {
                    gtk::Box {
                        gtk::Image {
                            set_icon_name: Some("go-previous-symbolic"),
                        },
                        gtk::Label {
                            set_label: "Back",
                        },
                    },
                    connect_clicked => Self::Input::BackClicked,
                },

                gtk::Label {
                    add_css_class: "pathbar-space",
                },

                gtk::Box {
                    set_spacing: 7,
                    set_hexpand: true,

                    append = &model.dashboard_btn.clone() -> gtk::ToggleButton {
                        set_icon_name: "go-home-symbolic",
                        set_tooltip_text: Some("Go to dashboard"),
                        set_active: true,
                        connect_clicked => Self::Input::DashboardClicked,
                    },
                    append = &model.artists_btn.clone() -> gtk::ToggleButton {
                        set_icon_name: "avatar-default-symbolic",
                        set_tooltip_text: Some("Show Artists"),
                        connect_clicked => Self::Input::ArtistClicked,
                    },
                    append = &model.albums_btn.clone() -> gtk::ToggleButton {
                        set_icon_name: "media-optical-cd-audio-symbolic",
                        set_tooltip_text: Some("Show Albums"),
                        connect_clicked => Self::Input::AlbumClicked,
                    },
                    append = &model.tracks_btn.clone() -> gtk::ToggleButton {
                        set_icon_name: "audio-x-generic-symbolic",
                        set_tooltip_text: Some("Show Tracks"),
                        connect_clicked => Self::Input::TrackClicked,
                    },
                    append = &model.playlists_btn.clone() -> gtk::ToggleButton {
                        set_icon_name: "playlist-symbolic",
                        set_tooltip_text: Some("Show playlists"),
                        connect_clicked => Self::Input::PlaylistClicked,
                    },
                },

                gtk::Label {
                    set_hexpand: true,
                    add_css_class: "pathbar-space",
                },

                gtk::SearchEntry {
                    set_placeholder_text: Some("Search..."),
                    grab_focus: (),
                    connect_search_changed[sender] => move |w| {
                        sender.input(BrowserInput::SearchChanged(w.text().to_string()));
                    }
                }
            },

            //TODO implement stack of view here
            append = &model.content.clone() -> gtk::ScrolledWindow {
                set_hexpand: true,
                set_vexpand: true,
            }
            // gtk::ScrolledWindow {
            //     add_css_class: "browser-content",
            //     set_vexpand: true,
            //     set_child: Some(&model.content.clone()),
            // }
        }
    }

    fn update(&mut self, msg: Self::Input, sender: relm4::ComponentSender<Self>) {
        match msg {
            BrowserInput::SearchChanged(search) => {
                tracing::warn!("new search {search}");
            }
            BrowserInput::BackClicked => {
                if self.history.len() > 1 {
                    // remove current view from history
                    let _ = self.history.pop();

                    // untoggle all buttons
                    self.dashboard_btn.set_active(false);
                    self.artists_btn.set_active(false);
                    self.albums_btn.set_active(false);
                    self.tracks_btn.set_active(false);
                    self.playlists_btn.set_active(false);

                    //toggle the right button one if its active
                    match self.history.last() {
                        Some(View::Dashboard) => self.dashboard_btn.set_active(true),
                        Some(View::Artists) => self.artists_btn.set_active(true),
                        Some(View::Albums) => self.albums_btn.set_active(true),
                        Some(View::Tracks) => self.tracks_btn.set_active(true),
                        Some(View::Playlists) => self.playlists_btn.set_active(true),
                        _ => {}
                    }

                    //change view
                    if let Some(view) = self.history.last() {
                        self.set_active_view(&view.clone(), &sender);
                    }
                }

                //change back button sensitivity
                if self.history.len() == 1 {
                    self.back_btn.set_sensitive(false);
                }
            }
            BrowserInput::DashboardClicked => {
                sender.input(BrowserInput::NewView(View::Dashboard));
                self.dashboard_btn.set_active(true);
            }
            BrowserInput::ArtistClicked => {
                sender.input(BrowserInput::NewView(View::Artists));
                self.artists_btn.set_active(true);
            }
            BrowserInput::AlbumClicked => {
                sender.input(BrowserInput::NewView(View::Albums));
                self.albums_btn.set_active(true);
            }
            BrowserInput::TrackClicked => {
                sender.input(BrowserInput::NewView(View::Tracks));
                self.tracks_btn.set_active(true);
            }
            BrowserInput::PlaylistClicked => {
                sender.input(BrowserInput::NewView(View::Playlists));
                self.playlists_btn.set_active(true);
            }
            BrowserInput::NewView(view) => {
                match self.history.last() {
                    Some(View::Dashboard) => self.dashboard_btn.set_active(false),
                    Some(View::Artists) => self.artists_btn.set_active(false),
                    Some(View::Albums) => self.albums_btn.set_active(false),
                    Some(View::Tracks) => self.tracks_btn.set_active(false),
                    Some(View::Playlists) => self.playlists_btn.set_active(false),
                    _ => {}
                }

                //set back button sensitivity
                if self.history.is_empty() {
                    self.back_btn.set_sensitive(false);
                } else {
                    self.back_btn.set_sensitive(true);
                }
                //remember new view
                self.history.push(view.clone());
                //show new view
                self.set_active_view(&view, &sender);
            }
            BrowserInput::Dashboard(output) => {
                //TODO react to output
            }
            BrowserInput::Artists(out) => {
                //TODO react to output
            }
        }
    }
}

impl Browser {
    fn set_active_view(&mut self, view: &View, sender: &relm4::ComponentSender<Self>) {
        match view {
            View::Dashboard => {
                let dashboard: relm4::Controller<Dashboard> = Dashboard::builder()
                    .launch(())
                    .forward(sender.input_sender(), BrowserInput::Dashboard);
                self.content.set_child(Some(dashboard.widget()));
            }
            View::Artists => {
                let artists: relm4::Controller<Artists> = Artists::builder()
                    .launch(())
                    .forward(sender.input_sender(), BrowserInput::Artists);
                self.content.set_child(Some(artists.widget()));
            }
            // View::Albums => todo!(),
            // View::Tracks => todo!(),
            // View::Playlists => todo!(),
            // View::Id(_) => todo!(),
            _ => todo!("implement view"), //TODO show new view
        }
    }
}
