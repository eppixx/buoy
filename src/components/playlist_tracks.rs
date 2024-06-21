use std::{cell::RefCell, rc::Rc};

use relm4::gtk::{
    self,
    prelude::{ButtonExt, GridExt, OrientableExt, WidgetExt},
};

use crate::{factory::playlist_song::{PlaylistSong, PlaylistSongOut}, subsonic::Subsonic};

#[derive(Debug)]
pub struct PlaylistTracks {
    subsonic: Rc<RefCell<Subsonic>>,
    songs: relm4::factory::FactoryVecDeque<PlaylistSong>,
    size_groups: [gtk::SizeGroup; 6], // sizegroup for every collum
}

#[derive(Debug)]
pub enum PlaylistTracksIn {
    DisplayToast(String),
    SetTracks(submarine::data::PlaylistWithSongs),
}

#[derive(Debug)]
pub enum PlaylistTracksOut {
    DisplayToast(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Column {
    Cover,
    Title,
    Artist,
    Album,
    Length,
    Favorited,
}

#[relm4::component(pub)]
impl relm4::SimpleComponent for PlaylistTracks {
    type Init = Rc<RefCell<Subsonic>>;
    type Input = PlaylistTracksIn;
    type Output = PlaylistTracksOut;

    fn init(
        subsonic: Self::Init,
        root: Self::Root,
        sender: relm4::ComponentSender<Self>,
    ) -> relm4::ComponentParts<Self> {
        let model = Self {
            subsonic,
            songs: relm4::factory::FactoryVecDeque::builder()
                .launch(gtk::ListBox::default())
                .forward(sender.input_sender(), |output| match output {
                    PlaylistSongOut::DisplayToast(title) => PlaylistTracksIn::DisplayToast(title),
                }),
            size_groups: [
                gtk::SizeGroup::new(gtk::SizeGroupMode::Horizontal),
                gtk::SizeGroup::new(gtk::SizeGroupMode::Horizontal),
                gtk::SizeGroup::new(gtk::SizeGroupMode::Horizontal),
                gtk::SizeGroup::new(gtk::SizeGroupMode::Horizontal),
                gtk::SizeGroup::new(gtk::SizeGroupMode::Horizontal),
                gtk::SizeGroup::new(gtk::SizeGroupMode::Horizontal),
            ],
        };
        let widgets = view_output!();

        // add widgets to group to make them the same size
        model.size_groups[0].add_widget(&widgets.title);
        model.size_groups[1].add_widget(&widgets.artist);
        model.size_groups[2].add_widget(&widgets.album);
        model.size_groups[3].add_widget(&widgets.length);
        model.size_groups[4].add_widget(&widgets.favorite);

        relm4::ComponentParts { model, widgets }
    }

    view! {
        gtk::Box {
            add_css_class: "album-tracks",
            set_orientation: gtk::Orientation::Vertical,

            gtk::Grid {
                set_column_spacing: 10,
                add_css_class: "album-tracks-header",

                #[name = "title"]
                attach[1, 0, 1, 1] = &gtk::Label {
                    add_css_class: "h4",
                    set_halign: gtk::Align::Start,
                    set_label: "Title",
                    set_hexpand: true,
                },
                #[name = "artist"]
                attach[2, 0, 1, 1] = &gtk::Label {
                    add_css_class: "h4",
                    set_halign: gtk::Align::Start,
                    set_label: "Artist",
                    set_hexpand: true,
                },
                #[name = "album"]
                attach[3, 0, 1, 1] = &gtk::Label {
                    add_css_class: "h4",
                    set_halign: gtk::Align::Start,
                    set_label: "Album",
                    set_hexpand: true,
                },
                #[name = "length"]
                attach[4, 0, 1, 1] = &gtk::Label {
                    add_css_class: "h4",
                    set_halign: gtk::Align::Start,
                    set_label: "Length",
                },
                #[name = "favorite"]
                attach[5, 0, 1, 1] = &gtk::Label {
                    add_css_class: "h4",
                    set_halign: gtk::Align::Start,
                    set_label: "Favorite",
                },
            },

            gtk::ScrolledWindow {
                set_vexpand: true,

                model.songs.widget().clone() -> gtk::ListBox {},
            },
        }
    }

    fn update(&mut self, msg: Self::Input, sender: relm4::ComponentSender<Self>) {
        match msg {
            PlaylistTracksIn::DisplayToast(title) => sender
                .output(PlaylistTracksOut::DisplayToast(title))
                .expect("sending failed"),
            PlaylistTracksIn::SetTracks(tracks) => {
                self.songs.guard().clear();
                for track in &tracks.entry {
                    let sizes = self.size_groups.clone();
                    self.songs.guard().push_back((self.subsonic.clone(), track.clone(), sizes));
                }
            }
        }
    }
}
