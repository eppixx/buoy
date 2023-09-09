use relm4::gtk::{
    self,
    traits::{BoxExt, ButtonExt, EditableExt, OrientableExt, WidgetExt},
};

use crate::types::Id;

#[derive(Debug, Default)]
pub struct Browser {
    content: gtk::Stack,
    back_btn: gtk::Button,
}

#[derive(Debug)]
pub enum BrowserInput {
    SearchChanged(String),
    BackClicked,
    HomeClicked,
    ArtistClicked,
    AlbumClicked,
    TrackClicked,
    PlaylistClicked,
}

#[relm4::component(pub)]
impl relm4::SimpleComponent for Browser {
    type Input = BrowserInput;
    type Output = ();
    type Init = Vec<Id>;

    fn init(
        path: Self::Init,
        root: &Self::Root,
        sender: relm4::ComponentSender<Self>,
    ) -> relm4::ComponentParts<Self> {
        let model = Browser::default();
        let widgets = view_output!();

        if path.is_empty() {
            model.back_btn.set_sensitive(false);
        } else {
            todo!("set view according to path");
        }

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

                    gtk::Button {
                        set_icon_name: "go-home-symbolic",
                        set_tooltip_text: Some("Go to dashboard"),
                        connect_clicked => Self::Input::HomeClicked,
                    },
                    gtk::Button {
                        set_icon_name: "avatar-default-symbolic",
                        set_tooltip_text: Some("Show Artists"),
                        connect_clicked => Self::Input::ArtistClicked,
                    },
                    gtk::Button {
                        set_icon_name: "media-optical-cd-audio-symbolic",
                        set_tooltip_text: Some("Show Albums"),
                        connect_clicked => Self::Input::AlbumClicked,
                    },
                    gtk::Button {
                        set_icon_name: "audio-x-generic-symbolic",
                        set_tooltip_text: Some("Show Tracks"),
                        connect_clicked => Self::Input::TrackClicked,
                    },
                    gtk::Button {
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
            gtk::Label {
                set_label: "sdfdsfdsf",
            }
        }
    }

    fn update(&mut self, msg: Self::Input, _sender: relm4::ComponentSender<Self>) {
        match msg {
            BrowserInput::SearchChanged(search) => {
                tracing::warn!("new search {search}");
            }
            BrowserInput::BackClicked => todo!(),
            BrowserInput::HomeClicked => todo!(),
            BrowserInput::ArtistClicked => todo!(),
            BrowserInput::AlbumClicked => todo!(),
            BrowserInput::TrackClicked => todo!(),
            BrowserInput::PlaylistClicked => todo!(),
        }
    }
}
