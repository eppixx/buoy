use relm4::gtk::{
    self,
    traits::{ButtonExt, GridExt, WidgetExt},
};
use relm4::{Component, ComponentController, FactorySender, RelmWidgetExt};

use crate::components::seekbar::convert_for_label;

#[derive(Debug)]
pub struct AlbumTracks {
    tracks: Vec<submarine::data::Child>,
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
        root: &Self::Root,
        sender: relm4::ComponentSender<Self>,
    ) -> relm4::ComponentParts<Self> {
        let model = Self { tracks: init };
        let widgets = view_output!();
        //TODO dragndrop
        //TODO highlighting rows

        for (index, track) in model.tracks.iter().enumerate() {
            //number
            let number = gtk::Label::new(Some(
                track.track.map(|t| t.to_string()).as_deref().unwrap_or(""),
            ));
            number.set_height_request(30);
            widgets.grid.attach(&number, 0, index as i32 + 1, 1, 1);

            //title
            let title = gtk::Label::new(Some(&track.title));
            title.set_halign(gtk::Align::Start);
            widgets.grid.attach(&title, 1, index as i32 + 1, 1, 1);

            //artist
            let artist = gtk::Label::new(Some(
                track
                    .artist
                    .as_ref()
                    .map(|t| t.to_string())
                    .as_deref()
                    .unwrap_or("Unknows Artist"),
            ));
            artist.set_halign(gtk::Align::Start);
            widgets.grid.attach(&artist, 2, index as i32 + 1, 1, 1);

            //length
            let length = gtk::Label::new(Some(
                track
                    .duration
                    .map(|l| convert_for_label(l as i64 * 1000))
                    .as_deref()
                    .unwrap_or("-:--"),
            ));
            widgets.grid.attach(&length, 3, index as i32 + 1, 1, 1);

            //favorite
            let favorite = gtk::Image::from_icon_name("starred");
            widgets.grid.attach(&favorite, 4, index as i32 + 1, 1, 1);
        }

        relm4::ComponentParts { model, widgets }
    }

    view! {
        gtk::Box {
            add_css_class: "album-view-tracks",

            #[transition = "Crossfade"]
            if !model.tracks.is_empty() {
                gtk::ScrolledWindow {
                    #[name = "grid"]
                    gtk::Grid {
                        set_column_spacing: 10,

                        attach[0, 0, 1, 1] = &gtk::Button {
                            add_css_class: "flat",
                            connect_clicked => AlbumTracksIn::Sort(Column::TrackNumber),

                            gtk::Label {
                                add_css_class: "h4",
                                set_halign: gtk::Align::Start,
                                set_label: "#",
                            }
                        },
                        attach[1, 0, 1, 1] = &gtk::Label {
                            add_css_class: "h4",
                            set_halign: gtk::Align::Start,
                            set_label: "Title",
                            set_hexpand: true,
                        },
                        attach[2, 0, 1, 1] = &gtk::Label {
                            add_css_class: "h4",
                            set_halign: gtk::Align::Start,
                            set_label: "Artist",
                            set_hexpand: true,
                        },
                        attach[3, 0, 1, 1] = &gtk::Label {
                            add_css_class: "h4",
                            set_halign: gtk::Align::Start,
                            set_label: "Length",
                        },
                        attach[4, 0, 1, 1] = &gtk::Label {
                            add_css_class: "h4",
                            set_halign: gtk::Align::Start,
                            set_label: "Favorite",
                        },
                    }
                }
            } else {
                gtk::Label {
                    add_css_class: "h3",
                    set_label: "Album is empty",
                    set_hexpand: true,
                    set_vexpand: true,
                    set_halign: gtk::Align::Center,
                    set_valign: gtk::Align::Center,
                }
            }
        }
    }

    fn update(&mut self, msg: Self::Input, _sender: relm4::ComponentSender<Self>) {
        tracing::error!("msg in tracks");
        match msg {
            _ => {} //TODO sort
        }
    }
}
