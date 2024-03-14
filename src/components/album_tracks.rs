use relm4::gtk::{
    self,
    prelude::{ButtonExt, GridExt, OrientableExt, WidgetExt},
};

use crate::factory::album_song::AlbumSong;

#[derive(Debug)]
pub struct AlbumTracks {
    songs: relm4::factory::FactoryVecDeque<AlbumSong>,
    size_groups: [gtk::SizeGroup; 5], // sizegroup for every collum
}

#[derive(Debug)]
pub enum AlbumTracksIn {
    Sort(Column),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Column {
    TrackNumber,
    Title,
    Artist,
    Length,
    Favorited,
}

#[relm4::component(pub)]
impl relm4::SimpleComponent for AlbumTracks {
    type Init = Vec<submarine::data::Child>;
    type Input = AlbumTracksIn;
    type Output = ();

    fn init(
        init: Self::Init,
        root: Self::Root,
        sender: relm4::ComponentSender<Self>,
    ) -> relm4::ComponentParts<Self> {
        let mut model = Self {
            songs: relm4::factory::FactoryVecDeque::builder()
                .launch(gtk::ListBox::default())
                .detach(),
            size_groups: [
                gtk::SizeGroup::new(gtk::SizeGroupMode::Horizontal),
                gtk::SizeGroup::new(gtk::SizeGroupMode::Horizontal),
                gtk::SizeGroup::new(gtk::SizeGroupMode::Horizontal),
                gtk::SizeGroup::new(gtk::SizeGroupMode::Horizontal),
                gtk::SizeGroup::new(gtk::SizeGroupMode::Horizontal),
            ],
        };
        let widgets = view_output!();

        // add widgets to group to make them the same size
        model.size_groups[0].add_widget(&widgets.track_number);
        model.size_groups[1].add_widget(&widgets.title);
        model.size_groups[2].add_widget(&widgets.artist);
        model.size_groups[3].add_widget(&widgets.length);
        model.size_groups[4].add_widget(&widgets.favorite);

        for track in &init {
            let sizes = model.size_groups.clone();
            model.songs.guard().push_back((track.clone(), sizes));
        }

        relm4::ComponentParts { model, widgets }
    }

    view! {
        gtk::Box {
            add_css_class: "album-tracks",
            set_orientation: gtk::Orientation::Vertical,

            gtk::Grid {
                set_column_spacing: 10,
                add_css_class: "album-tracks-header",

                #[name = "track_number"]
                attach[0, 0, 1, 1] = &gtk::Button {
                    add_css_class: "flat",
                    connect_clicked => AlbumTracksIn::Sort(Column::TrackNumber),

                    gtk::Label {
                        add_css_class: "h4",
                        set_halign: gtk::Align::Start,
                        set_label: "#",
                    }
                },
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
                #[name = "length"]
                attach[3, 0, 1, 1] = &gtk::Label {
                    add_css_class: "h4",
                    set_halign: gtk::Align::Start,
                    set_label: "Length",
                },
                #[name = "favorite"]
                attach[4, 0, 1, 1] = &gtk::Label {
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

    fn update(&mut self, msg: Self::Input, _sender: relm4::ComponentSender<Self>) {
        tracing::error!("msg in tracks");
        match msg {
            _ => {} //TODO sort
        }
    }
}
