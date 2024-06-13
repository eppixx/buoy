use std::{cell::RefCell, rc::Rc};

use gtk::prelude::OrientableExt;
use relm4::{
    gtk::{
        self,
        prelude::{BoxExt, WidgetExt},
    },
    Component, ComponentController,
};

use crate::components::cover::{Cover, CoverIn, CoverOut};
use crate::{subsonic::Subsonic, types::Id};

#[derive(Debug)]
pub struct PlayInfo {
    covers: relm4::Controller<Cover>,
    title: String,
    artist: Option<String>,
    album: Option<String>,
}

#[derive(Debug)]
pub enum PlayInfoIn {
    NewState(Box<Option<submarine::data::Child>>),
    Cover(CoverOut),
}

#[derive(Debug)]
pub enum PlayInfoOut {}

#[relm4::component(pub)]
impl relm4::SimpleComponent for PlayInfo {
    type Init = (Rc<RefCell<Subsonic>>, Option<submarine::data::Child>);
    type Input = PlayInfoIn;
    type Output = PlayInfoOut;

    fn init(
        (subsonic, child): Self::Init,
        root: Self::Root,
        sender: relm4::ComponentSender<Self>,
    ) -> relm4::ComponentParts<Self> {
        let model = Self {
            covers: Cover::builder()
                .launch((subsonic, child.clone().and_then(|child| child.cover_art)))
                .forward(sender.input_sender(), PlayInfoIn::Cover),
            title: String::from("Nothing is played currently"),
            artist: None,
            album: None,
        };

        let widgets = view_output!();

        //init widget
        sender.input(PlayInfoIn::NewState(Box::new(child)));
        model.covers.model().add_css_class_image("size100");

        relm4::ComponentParts { model, widgets }
    }

    view! {
        #[root]
        gtk::Box {
            add_css_class: "play-info",
            set_hexpand: true,
            set_orientation: gtk::Orientation::Vertical,

            append = &model.covers.widget().clone() {
                add_css_class: "play-info-cover",
                set_hexpand: true,
                set_halign: gtk::Align::Center,
            },

            gtk::Label {
                add_css_class: "play-info-info",
                set_hexpand: true,
                set_halign: gtk::Align::Center,
                set_justify: gtk::Justification::Center,

                #[watch]
                set_markup: &style_label(&model.title, model.artist.as_deref(), model.album.as_deref()),
            },
        }
    }

    fn update(&mut self, msg: Self::Input, _sender: relm4::ComponentSender<Self>) {
        match msg {
            PlayInfoIn::NewState(child) => match *child {
                None => {
                    self.covers.emit(CoverIn::LoadImage(None));
                    self.title = String::from("Nothing is played currently");
                    self.artist = None;
                    self.album = None;
                }
                Some(child) => {
                    self.title = child.title;
                    self.artist = child.artist;
                    self.album = child.album;
                    self.covers
                        .emit(CoverIn::LoadId(Some(Id::song(child.id.clone()))));
                }
            },
            PlayInfoIn::Cover(msg) => match msg {},
        }
    }
}

fn style_label(title: &str, artist: Option<&str>, album: Option<&str>) -> String {
    let mut result = format!(
        "<span font_size=\"xx-large\" weight=\"bold\">{}</span>",
        glib::markup_escape_text(title)
    );
    if artist.is_some() || album.is_some() {
        result.push('\n');
    }
    if let Some(artist) = artist {
        result.push_str(&format!(
            "by <span font_size=\"large\" style=\"italic\" weight=\"bold\">{}</span>",
            glib::markup_escape_text(artist)
        ));
    }
    if artist.is_some() || album.is_some() {
        result.push(' ');
    }
    if let Some(album) = album {
        result.push_str(&format!(
            "\non <span font_size=\"large\" style=\"italic\" weight=\"bold\">{}</span>",
            glib::markup_escape_text(album)
        ));
    }
    result
}
