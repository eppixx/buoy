use relm4::{
    gtk::{
        self,
        traits::{BoxExt, ButtonExt, EditableExt, OrientableExt, ToggleButtonExt, WidgetExt},
    },
    Component, ComponentController,
};

use crate::{components::dashboard::Dashboard, types::Id};

use super::dashboard::DashboardOutput;

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
    content: gtk::Stack,
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
            gtk::ScrolledWindow {
                add_css_class: "browser-content",
                set_vexpand: true,
                set_child: Some(&model.content.clone()),
            }
        }
    }

    fn update(&mut self, msg: Self::Input, sender: relm4::ComponentSender<Self>) {
        match msg {
            BrowserInput::SearchChanged(search) => {
                tracing::warn!("new search {search}");
            }
            BrowserInput::BackClicked => {
                if self.history.len() > 1 {
                    let previous = self.history.pop();

                    // untoggle all buttons
                    self.dashboard_btn.set_active(false);
                    self.artists_btn.set_active(false);
                    self.albums_btn.set_active(false);
                    self.tracks_btn.set_active(false);
                    self.playlists_btn.set_active(false);

                    //toggle the right one if its active
                    match self.history.last() {
                        Some(View::Dashboard) => self.dashboard_btn.set_active(true),
                        Some(View::Artists) => self.artists_btn.set_active(true),
                        Some(View::Albums) => self.albums_btn.set_active(true),
                        Some(View::Tracks) => self.tracks_btn.set_active(true),
                        Some(View::Playlists) => self.playlists_btn.set_active(true),
                        _ => {}
                    }

                    // TODO show previous
                    tracing::error!("new view {previous:?}");
                }

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

                if self.history.is_empty() {
                    self.back_btn.set_sensitive(false);
                } else {
                    self.back_btn.set_sensitive(true);
                }
                self.history.push(view.clone());
                match view {
                    View::Dashboard => {
                        let dashboard: relm4::Controller<Dashboard> = Dashboard::builder()
                            .launch(())
                            .forward(sender.input_sender(), BrowserInput::Dashboard);
                        self.content.add_child(dashboard.widget());
                    }
                    _ => todo!("add other views"),
                }
                //TODO show new view
                tracing::error!("new view {view:?}");
            }
            BrowserInput::Dashboard(output) => {
                //TODO react to output
            }
        }
    }
}
