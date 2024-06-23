use crate::{components::playlist_element::PlaylistElement, subsonic::Subsonic};
use relm4::gtk::{
    self,
    prelude::{BoxExt, ButtonExt, OrientableExt, WidgetExt},
};

use std::{cell::RefCell, rc::Rc};

use super::playlist_element::PlaylistElementOut;
use crate::common::convert_for_label;
use crate::factory::playlist_tracks_row::{
    AlbumColumn, ArtistColumn, FavColumn, LengthColumn, PlaylistTracksRow, TitleColumn,
};

#[derive(Debug)]
pub struct PlaylistsView {
    playlists: relm4::factory::FactoryVecDeque<PlaylistElement>,
    index_shown: Option<relm4::factory::DynamicIndex>,
    tracks: relm4::typed_view::column::TypedColumnView<PlaylistTracksRow, gtk::SingleSelection>,
    info_title: gtk::Label,
    info_details: gtk::Label,
}

#[derive(Debug)]
pub enum PlaylistsViewOut {
    ReplaceQueue(Vec<submarine::data::Child>),
    AppendToQueue(Vec<submarine::data::Child>),
    DisplayToast(String),
}

#[derive(Debug)]
pub enum PlaylistsViewIn {
    SearchChanged(String),
    NewPlaylist(Vec<submarine::data::Child>),
    PlaylistElement(PlaylistElementOut),
}

#[relm4::component(pub)]
impl relm4::SimpleComponent for PlaylistsView {
    type Init = Rc<RefCell<Subsonic>>;
    type Input = PlaylistsViewIn;
    type Output = PlaylistsViewOut;

    fn init(
        init: Self::Init,
        root: Self::Root,
        sender: relm4::ComponentSender<Self>,
    ) -> relm4::ComponentParts<Self> {
        let mut tracks = relm4::typed_view::column::TypedColumnView::<
            PlaylistTracksRow,
            gtk::SingleSelection,
        >::new();
        tracks.append_column::<TitleColumn>();
        tracks.append_column::<ArtistColumn>();
        tracks.append_column::<AlbumColumn>();
        tracks.append_column::<LengthColumn>();
        tracks.append_column::<FavColumn>();

        let mut model = PlaylistsView {
            playlists: relm4::factory::FactoryVecDeque::builder()
                .launch(gtk::ListBox::default())
                .forward(sender.input_sender(), PlaylistsViewIn::PlaylistElement),
            index_shown: None,
            tracks,
            info_title: gtk::Label::default(),
            info_details: gtk::Label::default(),
        };

        let column = &model.tracks.view;
        let info_title = model.info_title.clone();
        let info_details = model.info_details.clone();
        let widgets = view_output!();

        // add playlists to list
        for playlist in init.borrow().playlists() {
            model
                .playlists
                .guard()
                .push_back((init.clone(), playlist.clone()));
        }

        relm4::ComponentParts { model, widgets }
    }

    view! {
        gtk::Box {
            gtk::Paned {
                set_position: 300,
                set_shrink_start_child: false,
                set_resize_start_child: false,
                set_shrink_end_child: false,

                #[wrap(Some)]
                set_start_child = &gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_spacing: 7,

                    gtk::WindowHandle {
                        gtk::Label {
                            add_css_class: granite::STYLE_CLASS_H2_LABEL,
                            set_halign: gtk::Align::Center,
                            set_text: "Playlists",
                        },
                    },

                    model.playlists.widget().clone() -> gtk::ListBox {
                        add_css_class: "playlist-view-playlist-list",
                        set_vexpand: true,

                        gtk::ListBoxRow {
                            add_css_class: "playlist-view-add-playlist",

                            gtk::Button {
                                gtk::Box {
                                    set_halign: gtk::Align::Center,

                                    gtk::Image {
                                        set_icon_name: Some("list-add-symbolic"),
                                    },
                                    gtk::Label {
                                        set_text: "New Playlist",
                                    }
                                },

                                connect_clicked => PlaylistsViewIn::NewPlaylist(vec![]),
                            }
                        }
                    }
                },

                // set_end_child = Some(model.tracks.widget()),
                #[wrap(Some)]
                set_end_child = &gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_spacing: 8,

                    gtk::Box {
                        set_spacing: 15,

                        //TODO add cover
                        gtk::Image {
                            set_icon_name: Some("starred"),
                        },

                        // playlist info
                        gtk::WindowHandle {
                            set_hexpand: true,

                            gtk::Box {
                                set_orientation: gtk::Orientation::Vertical,
                                set_spacing: 8,

                                #[local_ref]
                                info_title -> gtk::Label {
                                    add_css_class: granite::STYLE_CLASS_H3_LABEL,
                                    set_label: "title",
                                    set_halign: gtk::Align::Start,
                                },

                                #[local_ref]
                                info_details -> gtk::Label {
                                    set_label: "more info",
                                    set_halign: gtk::Align::Start,
                                },

                                gtk::Box {
                                    set_spacing: 15,
                                    gtk::Button {
                                        gtk::Box {
                                            gtk::Image {
                                                set_icon_name: Some("list-add-symbolic"),
                                            },
                                            gtk::Label {
                                                set_label: "Append",
                                            }
                                        },
                                        set_tooltip_text: Some("Append Album to end of queue"),
                                        connect_clicked[sender, init] => move |_btn| {
                                        }
                                    },
                                    gtk::Button {
                                        gtk::Box {
                                            gtk::Image {
                                                set_icon_name: Some("list-add-symbolic"),
                                            },
                                            gtk::Label {
                                                set_label: "Play next"
                                            }
                                        },
                                        set_tooltip_text: Some("Insert Album after currently played or paused item"),
                                        connect_clicked[sender, init] => move |_btn| {
                                        }
                                    }
                                }
                            }
                        }
                    },

                    gtk::ScrolledWindow {
                        set_hexpand: true,
                        set_vexpand: true,

                        #[local_ref]
                        column -> gtk::ColumnView {
                            add_css_class: "playlist-view-tracks-row",
                        }
                    }
                }
            }
        }
    }

    fn update(&mut self, msg: Self::Input, sender: relm4::ComponentSender<Self>) {
        match msg {
            PlaylistsViewIn::SearchChanged(_search) => {
                // unimplemented!("search in dashboard"); //TODO implement
            }
            PlaylistsViewIn::NewPlaylist(list) => {
                sender
                    .output(PlaylistsViewOut::DisplayToast(String::from(
                        "new playlist clicked",
                    )))
                    .expect("sending failed");
            }
            PlaylistsViewIn::PlaylistElement(msg) => match msg {
                PlaylistElementOut::Clicked(index, list) => {
                    if self.index_shown == Some(index.clone()) {
                        return;
                    }

                    // set info
                    self.info_title.set_text(&list.base.name);
                    self.info_details.set_text(&build_info_string(&list));

                    //set tracks
                    self.tracks.clear();
                    for track in list.entry {
                        self.tracks.append(PlaylistTracksRow::new(track));
                    }
                    self.index_shown = Some(index);
                }
                PlaylistElementOut::DisplayToast(msg) => sender
                    .output(PlaylistsViewOut::DisplayToast(msg))
                    .expect("sending failed"),
            },
        }
    }
}

fn build_info_string(list: &submarine::data::PlaylistWithSongs) -> String {
    let songs = format!(
        "Songs: {} • Length: {}",
        list.base.song_count,
        convert_for_label(i64::from(list.base.duration) * 1000)
    );
    let time = list.base.created;
    let created = time.format(" • Created at: %d.%m.%Y, %H:%M").to_string();
    format!("{songs}{created}")
}
