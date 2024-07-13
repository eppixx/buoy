use std::{cell::RefCell, rc::Rc};

use relm4::{
    gtk::{
        self,
        prelude::{ButtonExt, ToValue, WidgetExt},
    },
    Component, ComponentController,
};

use super::descriptive_cover::DescriptiveCoverOut;
use crate::{
    common::convert_for_label,
    components::descriptive_cover::{DescriptiveCover, DescriptiveCoverInit},
    subsonic::Subsonic,
    types::{Droppable, Id},
};

#[derive(Debug)]
pub struct AlbumElement {
    cover: relm4::Controller<DescriptiveCover>,
    init: AlbumElementInit,
    favorite: gtk::Button,
}

impl AlbumElement {
    pub fn info(&self) -> &AlbumElementInit {
        &self.init
    }
}

#[derive(Debug, Clone)]
pub enum AlbumElementIn {
    DescriptiveCover(DescriptiveCoverOut),
    Favorited(String, bool),
}

#[derive(Debug)]
pub enum AlbumElementOut {
    Clicked(AlbumElementInit),
    FavoriteClicked(String, bool),
    DisplayToast(String),
}

#[derive(Debug, Clone)]
pub enum AlbumElementInit {
    Child(Box<submarine::data::Child>),
    AlbumId3(Box<submarine::data::AlbumId3>),
}

#[relm4::component(pub)]
impl relm4::SimpleComponent for AlbumElement {
    type Init = (Rc<RefCell<Subsonic>>, AlbumElementInit);
    type Input = AlbumElementIn;
    type Output = AlbumElementOut;

    fn init(
        (subsonic, init): Self::Init,
        root: Self::Root,
        sender: relm4::ComponentSender<Self>,
    ) -> relm4::ComponentParts<Self> {
        // init cover
        let (builder, drop, id) = match &init {
            AlbumElementInit::AlbumId3(id3) => {
                let builder = DescriptiveCoverInit::new(
                    id3.name.clone(),
                    id3.cover_art.clone(),
                    id3.artist.clone(),
                );
                (builder, Droppable::Album(id3.clone()), id3.id.clone())
            }
            AlbumElementInit::Child(child) => {
                let builder = DescriptiveCoverInit::new(
                    child.title.clone(),
                    child.cover_art.clone(),
                    child.artist.clone(),
                );
                (builder, Droppable::AlbumChild(child.clone()), child.id.clone())
            }
        };

        let cover: relm4::Controller<DescriptiveCover> = DescriptiveCover::builder()
            .launch((subsonic.clone(), builder, true, Some(Id::album(id))))
            .forward(sender.input_sender(), AlbumElementIn::DescriptiveCover);
        let model = Self {
            cover,
            init: init.clone(),
            favorite: gtk::Button::default(),
        };

        // tooltip string
        let tooltip = match &init {
            AlbumElementInit::AlbumId3(album) => {
                let year = match album.year {
                    Some(year) => format!("Year: {year} • "),
                    None => String::new(),
                };
                format!(
                    "{year}{} songs • Length: {}",
                    album.song_count,
                    convert_for_label(i64::from(album.duration) * 1000)
                )
            }
            AlbumElementInit::Child(child) => {
                let year = match child.year {
                    Some(year) => format!("Year: {year} • "),
                    None => String::new(),
                };
                let duration = match child.duration {
                    Some(duration) => {
                        format!("Length: {}", convert_for_label(i64::from(duration) * 1000))
                    }
                    None => String::new(),
                };
                format!("{year}{duration}")
            }
        };

        let info = init.clone();
        let widgets = view_output!();

        //setup DropSource
        let content = gtk::gdk::ContentProvider::for_value(&drop.to_value());
        let drag_src = gtk::DragSource::new();
        drag_src.set_actions(gtk::gdk::DragAction::COPY);
        drag_src.set_content(Some(&content));
        let cover_art = match &init {
            AlbumElementInit::AlbumId3(album) => album.cover_art.clone(),
            AlbumElementInit::Child(album) => album.cover_art.clone(),
        };
        drag_src.connect_drag_begin(move |src, _drag| {
            if let Some(cover_id) = &cover_art {
                let cover = subsonic.borrow().cover_icon(&cover_id);
                match cover {
                    Some(tex) => {
                        src.set_icon(Some(&tex), 0, 0);
                    }
                    None => {}
                }
            }
        });
        model.cover.widget().add_controller(drag_src);

        relm4::ComponentParts { model, widgets }
    }

    view! {
        gtk::FlowBoxChild {
            add_css_class: "album-element",

            gtk::Overlay {
                add_overlay = &gtk::Box {
                    set_halign: gtk::Align::Center,
                    set_valign: gtk::Align::Start,
                    set_margin_top: 85,
                    set_margin_start: 75,

                    model.favorite.clone() -> gtk::Button {
                        set_width_request: 24,
                        set_height_request: 24,
                        set_icon_name: "non-starred-symbolic",

                        connect_clicked[sender] => move |btn| {
                            let id = match &info {
                                AlbumElementInit::AlbumId3(album) => album.id.clone(),
                                AlbumElementInit::Child(child) => child.id.clone(),
                            };
                            match btn.icon_name().as_deref() {
                                Some("starred-symbolic") => sender.output(AlbumElementOut::FavoriteClicked(id, false)).expect("sending failed"),
                                Some("non-starred-symbolic") => sender.output(AlbumElementOut::FavoriteClicked(id, true)).expect("sending failed"),
                                _ => {}
                            }
                        }
                    }
                },

                #[wrap(Some)]
                set_child = &gtk::Button {
                    add_css_class: "flat",
                    set_halign: gtk::Align::Center,

                    connect_clicked[sender, init] => move |_btn| {
                        sender.output(AlbumElementOut::Clicked(init.clone())).unwrap();
                    },

                    #[wrap(Some)]
                    set_child = &model.cover.widget().clone() {
                        set_tooltip_text: Some(&tooltip),
                    }
                }
            },
        }
    }

    fn update(&mut self, msg: Self::Input, sender: relm4::ComponentSender<Self>) {
        match msg {
            AlbumElementIn::DescriptiveCover(msg) => match msg {
                DescriptiveCoverOut::DisplayToast(title) => sender
                    .output(AlbumElementOut::DisplayToast(title))
                    .expect("sending failed"),
            },
            AlbumElementIn::Favorited(id, state) => {
                let local_id = match &self.init {
                    AlbumElementInit::AlbumId3(album) => album.id.clone(),
                    AlbumElementInit::Child(child) => child.id.clone(),
                };
                if local_id == id {
                    match state {
                        true => self.favorite.set_icon_name("starred-symbolic"),
                        false => self.favorite.set_icon_name("non-starred-symbolic"),
                    }
                }
            }
        }
    }
}
