use std::{cell::RefCell, rc::Rc};

use gtk::prelude::OrientableExt;
use relm4::{
    gtk::{
        self, glib, pango,
        prelude::{BoxExt, WidgetExt},
    },
    Component, ComponentController,
};

use crate::components::cover::{Cover, CoverIn, CoverOut};
use crate::subsonic::Subsonic;

#[derive(Debug)]
pub struct PlayInfo {
    covers: relm4::Controller<Cover>,
    title: String,
    artist: String,
    album: String,
}

#[derive(Debug)]
pub enum PlayInfoIn {
    NewState(Box<Option<submarine::data::Child>>),
    Cover(CoverOut),
}

#[derive(Debug)]
pub enum PlayInfoOut {
    DisplayToast(String),
}

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
                .launch((subsonic, child.clone().and_then(|child| child.cover_art), false))
                .forward(sender.input_sender(), PlayInfoIn::Cover),
            title: String::from("Nothing is played currently"),
            artist: String::new(),
            album: String::new(),
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
            set_spacing: 10,

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
                set_ellipsize: pango::EllipsizeMode::End,

                #[watch]
                set_markup: &style_label(&model.title, &model.artist, &model.album),
            },
        }
    }

    fn update(&mut self, msg: Self::Input, sender: relm4::ComponentSender<Self>) {
        match msg {
            PlayInfoIn::NewState(child) => match *child {
                None => {
                    self.covers.emit(CoverIn::LoadId(None));
                    self.title = String::from("Nothing is played currently");
                    self.artist = String::new();
                    self.album = String::new();
                }
                Some(child) => {
                    self.covers.emit(CoverIn::LoadSong(Box::new(child.clone())));
                    self.title = child.title;
                    self.artist = child.artist.unwrap_or_default();
                    self.album = child.album.unwrap_or_default();
                }
            },
            PlayInfoIn::Cover(msg) => match msg {
                CoverOut::DisplayToast(title) => sender
                    .output(PlayInfoOut::DisplayToast(title))
                    .expect("sending failed"),
            },
        }
    }
}

fn style_label(title: &str, artist: &str, album: &str) -> String {
    let mut result = format!(
        "<span font_size=\"xx-large\" weight=\"bold\">{}</span>",
        glib::markup_escape_text(title)
    );
    if !artist.is_empty() {
        result.push_str(&format!(
            "\nby <span font_size=\"large\" style=\"italic\" weight=\"bold\">{}</span>",
            glib::markup_escape_text(artist)
        ));
    } else {
        result.push('\n')
    }
    if !album.is_empty() {
        result.push_str(&format!(
            "\non <span font_size=\"large\" style=\"italic\" weight=\"bold\">{}</span>",
            glib::markup_escape_text(album)
        ));
    } else {
        result.push('\n')
    }
    result
}
