use fuzzy_matcher::FuzzyMatcher;
use relm4::gtk::glib::prelude::ToValue;
use relm4::gtk::{
    self,
    prelude::{BoxExt, ButtonExt, OrientableExt, WidgetExt},
};
use relm4::{Component, ComponentController};

use std::{cell::RefCell, rc::Rc};

use super::cover::{Cover, CoverIn, CoverOut};
use super::playlist_element::PlaylistElementOut;
use crate::common::convert_for_label;
use crate::factory::playlist_tracks_row::{
    AlbumColumn, ArtistColumn, FavColumn, LengthColumn, PlaylistTracksRow, TitleColumn,
};
use crate::types::Droppable;
use crate::{components::playlist_element::{PlaylistElement, PlaylistElementIn}, subsonic::Subsonic};

#[derive(Debug)]
pub struct PlaylistsView {
    playlists: relm4::factory::FactoryVecDeque<PlaylistElement>,
    index_shown: Option<relm4::factory::DynamicIndex>,

    track_stack: gtk::Stack,
    tracks: relm4::typed_view::column::TypedColumnView<PlaylistTracksRow, gtk::SingleSelection>,
    info_cover: relm4::Controller<Cover>,
    info_cover_controller: gtk::DragSource,
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
    Cover(CoverOut),
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

            track_stack: gtk::Stack::default(),
            tracks,
            info_cover: Cover::builder()
                .launch((init.clone(), None))
                .forward(sender.input_sender(), PlaylistsViewIn::Cover),
            info_cover_controller: gtk::DragSource::default(),
            info_title: gtk::Label::default(),
            info_details: gtk::Label::default(),
        };

        let track_stack = &model.track_stack.clone();
        let column = &model.tracks.view;
        column.connect_activate(|_column_view, i| {
            //TODO play next or append to queue
            println!("activated index {i}");
        });
        let info_cover = model.info_cover.widget().clone();
        let info_title = model.info_title.clone();
        let info_details = model.info_details.clone();
        let widgets = view_output!();
        model.info_cover.model().add_css_class_image("size100");

        model
            .info_cover
            .widget()
            .add_controller(model.info_cover_controller.clone());

        // add playlists to list
        for playlist in init.borrow().playlists() {
            model.playlists.guard().push_back(playlist.clone());
        }

        relm4::ComponentParts { model, widgets }
    }

    view! {
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,

            gtk::WindowHandle {
                gtk::Label {
                    add_css_class: granite::STYLE_CLASS_H2_LABEL,
                    set_halign: gtk::Align::Center,
                    set_text: "Playlists",
                },
            },

            gtk::Paned {
                set_position: 300,
                set_shrink_start_child: false,
                set_resize_start_child: false,
                set_shrink_end_child: false,

                #[wrap(Some)]
                set_start_child = &model.playlists.widget().clone() -> gtk::ListBox {
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
                },

                #[wrap(Some)]
                set_end_child = &gtk::Box {
                    #[local_ref]
                    track_stack -> gtk::Stack {
                        add_named[Some("tracks-stock")] = &gtk::Box {
                            gtk::Label {
                                add_css_class: granite::STYLE_CLASS_H2_LABEL,
                                set_hexpand: true,

                                set_label: "Select a playlist to show its songs",
                            }
                        },
                        add_named[Some("tracks")] = &gtk::Box {
                            set_orientation: gtk::Orientation::Vertical,
                            set_spacing: 8,

                            gtk::Box {
                                add_css_class: "playlist-view-info",
                                set_spacing: 15,

                                #[local_ref]
                                info_cover -> gtk::Box {},

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

                                        gtk::Label {
                                            set_label: " ",
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
        }
    }

    fn update(&mut self, msg: Self::Input, sender: relm4::ComponentSender<Self>) {
        match msg {
            PlaylistsViewIn::SearchChanged(search) => {
                self.tracks.clear_filters();
                self.tracks.add_filter(move |row| {
                    let matcher = fuzzy_matcher::skim::SkimMatcherV2::default();
                    let test = format!(
                        "{} {} {}",
                        row.item.title,
                        row.item.artist.as_deref().unwrap_or_default(),
                        row.item.album.as_deref().unwrap_or_default()
                    );
                    let score = matcher.fuzzy_match(&test, &search);
                    score.is_some()
                });
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
                    self.track_stack.set_visible_child_name("tracks");
                    if self.index_shown == Some(index.clone()) {
                        return;
                    }

                    if let Some(i) = &self.index_shown {
                        self.playlists.guard().get(i.current_index()).unwrap().set_edit_area(false);
                    }
                    self.playlists.guard().get(index.current_index()).unwrap().set_edit_area(true);


                    // set info
                    self.info_cover
                        .emit(CoverIn::LoadImage(list.base.cover_art.clone()));
                    self.info_title.set_text(&list.base.name);
                    self.info_details.set_text(&build_info_string(&list));

                    //set drag controller
                    let drop = Droppable::Playlist(Box::new(list.clone()));
                    let content = gtk::gdk::ContentProvider::for_value(&drop.to_value());
                    self.info_cover_controller.set_content(Some(&content));
                    self.info_cover_controller
                        .set_actions(gtk::gdk::DragAction::MOVE);

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
            PlaylistsViewIn::Cover(msg) => match msg {
                CoverOut::DisplayToast(title) => sender
                    .output(PlaylistsViewOut::DisplayToast(title))
                    .expect("sending failed"),
            },
        }
    }
}

fn build_info_string(list: &submarine::data::PlaylistWithSongs) -> String {
    let created = list
        .base
        .created
        .format("Created at: %d.%m.%Y, %H:%M")
        .to_string();
    format!(
        "Songs: {} • Length: {} • {}",
        list.base.song_count,
        convert_for_label(i64::from(list.base.duration) * 1000),
        created
    )
}
