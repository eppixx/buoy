use std::{cell::RefCell, rc::Rc};

use gettextrs::gettext;
use gtk::prelude::OrientableExt;
use relm4::{
    gtk::{
        self, glib, pango,
        prelude::{BoxExt, WidgetExt},
    },
    ComponentController,
};

use crate::subsonic::Subsonic;
use crate::{
    components::cover::{Cover, CoverIn, CoverOut},
    types::Id,
};

#[derive(Debug)]
pub struct PlayInfo {
    covers: relm4::Controller<Cover>,
    child: Option<submarine::data::Child>,
}

#[derive(Debug)]
pub enum PlayInfoIn {
    NewState(Box<Option<submarine::data::Child>>),
    Cover(CoverOut),
    CoverClicked,
}

#[derive(Debug)]
pub enum PlayInfoOut {
    DisplayToast(String),
    ShowArtist(Id),
    ShowAlbum(Id),
    CoverClicked(String),
}

#[relm4::component(pub)]
impl relm4::component::Component for PlayInfo {
    type Init = (Rc<RefCell<Subsonic>>, Option<submarine::data::Child>);
    type Input = PlayInfoIn;
    type Output = PlayInfoOut;
    type CommandOutput = ();

    fn init(
        (subsonic, child): Self::Init,
        root: Self::Root,
        sender: relm4::ComponentSender<Self>,
    ) -> relm4::ComponentParts<Self> {
        let model = Self {
            covers: Cover::builder()
                .launch((subsonic, child.clone().and_then(|child| child.cover_art)))
                .forward(sender.input_sender(), PlayInfoIn::Cover),
            child: None,
        };

        let widgets = view_output!();

        //init widget
        sender.input(PlayInfoIn::NewState(Box::new(child)));
        model.covers.model().add_css_class_image("size150");

        relm4::ComponentParts { model, widgets }
    }

    view! {
        #[root]
        gtk::Box {
            set_widget_name: "play-info",
            set_hexpand: true,
            set_orientation: gtk::Orientation::Vertical,
            set_spacing: 10,

            append = &model.covers.widget().clone() {
                set_hexpand: true,
                set_halign: gtk::Align::Center,

                add_controller = gtk::GestureClick {
                    connect_pressed[sender] => move |_ctrl, _btn, _x, _y| {
                        sender.input(PlayInfoIn::CoverClicked);
                    },
                }
            },

            append: info = &gtk::Label {
                set_hexpand: true,
                set_halign: gtk::Align::Center,
                set_justify: gtk::Justification::Center,
                set_ellipsize: pango::EllipsizeMode::End,

                set_text: &gettext("Nothing is playing"),

                connect_activate_link[sender] => move |_label, text| {
                    let id = Id::try_from(text);
                    match &id {
                        Err(e) => unreachable!("text is not an id: {e:?}"),
                        Ok(Id::Playlist(id)) | Ok(Id::Song(id)) => unreachable!("found wrong id: {id}"),
                        Ok(Id::Artist(_)) => sender.output(PlayInfoOut::ShowArtist(id.unwrap())).unwrap(),
                        Ok(Id::Album(_)) => sender.output(PlayInfoOut::ShowAlbum(id.unwrap())).unwrap(),
                    }
                    gtk::glib::signal::Propagation::Stop
                },
            },
        }
    }

    fn update_with_view(
        &mut self,
        widgets: &mut Self::Widgets,
        msg: Self::Input,
        sender: relm4::ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match msg {
            PlayInfoIn::NewState(child) => {
                self.child = *child;
                widgets
                    .info
                    .set_markup(&style_label_from_child(&self.child));

                match &self.child {
                    None => self.covers.emit(CoverIn::LoadId(None)),
                    Some(child) => {
                        self.covers.emit(CoverIn::LoadSong(Box::new(child.clone())));
                    }
                }
            }
            PlayInfoIn::Cover(msg) => match msg {
                CoverOut::DisplayToast(title) => {
                    sender.output(PlayInfoOut::DisplayToast(title)).unwrap();
                }
            },
            PlayInfoIn::CoverClicked => {
                if let Some(child) = &self.child {
                    if let Some(cover_art) = &child.cover_art {
                        sender
                            .output(PlayInfoOut::CoverClicked(cover_art.clone()))
                            .unwrap();
                    }
                }
            }
        }
    }
}

fn style_label_from_child(child: &Option<submarine::data::Child>) -> String {
    let Some(child) = &child else {
        return gettext("Nothing is playing");
    };

    let mut result = format!(
        "<span font_size=\"xx-large\" weight=\"bold\">{}</span>",
        glib::markup_escape_text(&child.title)
    );

    match &child.artist {
        None => result.push('\n'),
        Some(artist) => {
            // insert link if artist_id exists for child
            let artist = match &child.artist_id {
                None => glib::markup_escape_text(artist),
                Some(id) => format!(
                    "<a href=\"{}\">{}</a>",
                    Id::artist(id).serialize(),
                    glib::markup_escape_text(artist)
                )
                .into(),
            };

            // build artist markup string
            result.push_str(&format!(
                "\n{} <span font_size=\"large\" style=\"italic\" weight=\"bold\">{}</span>",
                gettext("by"),
                artist
            ));
        }
    }

    match &child.album {
        None => result.push('\n'),
        Some(album) => {
            // insert link if album_id exists for child
            let album = match &child.album_id {
                None => glib::markup_escape_text(album),
                Some(id) => format!(
                    "<a href=\"{}\">{}</a>",
                    Id::album(id).serialize(),
                    glib::markup_escape_text(album)
                )
                .into(),
            };

            // build album markup string
            result.push_str(&format!(
                " {} <span font_size=\"large\" style=\"italic\" weight=\"bold\">{}</span>",
                gettext("on"),
                album
            ));
        }
    }
    result
}
