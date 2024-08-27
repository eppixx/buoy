use std::{cell::RefCell, rc::Rc};

use relm4::{
    gtk::{
        self,
        prelude::{ButtonExt, ToValue, WidgetExt},
    },
    Component, ComponentController,
};

use crate::{
    components::descriptive_cover::{DescriptiveCover, DescriptiveCoverInit, DescriptiveCoverOut},
    subsonic::Subsonic,
    types::Droppable,
};

#[derive(Debug)]
pub struct ArtistElement {
    cover: relm4::Controller<DescriptiveCover>,
    init: submarine::data::ArtistId3,
    favorite: gtk::Button,
    favorite_ribbon: gtk::Box,
}

impl ArtistElement {
    pub fn info(&self) -> &submarine::data::ArtistId3 {
        &self.init
    }
}

#[derive(Debug)]
pub enum ArtistElementIn {
    DescriptiveCover(DescriptiveCoverOut),
    Favorited(String, bool),
    Hover(bool),
}

#[derive(Debug)]
pub enum ArtistElementOut {
    Clicked(submarine::data::ArtistId3),
    DisplayToast(String),
    FavoriteClicked(String, bool),
}

#[relm4::component(pub)]
impl relm4::SimpleComponent for ArtistElement {
    type Init = (Rc<RefCell<Subsonic>>, submarine::data::ArtistId3);
    type Input = ArtistElementIn;
    type Output = ArtistElementOut;

    fn init(
        (subsonic, init): Self::Init,
        root: Self::Root,
        sender: relm4::ComponentSender<Self>,
    ) -> relm4::ComponentParts<Self> {
        // init cover
        let builder =
            DescriptiveCoverInit::new(init.name.clone(), init.cover_art.clone(), None::<&str>);
        let cover: relm4::Controller<DescriptiveCover> = DescriptiveCover::builder()
            .launch((subsonic.clone(), builder))
            .forward(sender.input_sender(), ArtistElementIn::DescriptiveCover);
        let model = Self {
            cover,
            init: init.clone(),
            favorite: gtk::Button::default(),
            favorite_ribbon: gtk::Box::default(),
        };

        let widgets = view_output!();

        //setup DropSource
        let drop = Droppable::Artist(Box::new(init.clone()));
        let content = gtk::gdk::ContentProvider::for_value(&drop.to_value());
        let drag_src = gtk::DragSource::new();
        drag_src.set_actions(gtk::gdk::DragAction::COPY);
        drag_src.set_content(Some(&content));
        drag_src.connect_drag_begin(move |src, _drag| {
            if let Some(cover_id) = &init.cover_art {
                let cover = subsonic.borrow().cover_icon(cover_id);
                if let Some(tex) = cover {
                    src.set_icon(Some(&tex), 0, 0);
                }
            }
        });

        //setup favorite button
        model.favorite.set_visible(false);
        model.favorite_ribbon.set_visible(false);
        if init.starred.is_some() {
            model.favorite.set_icon_name("starred-symbolic");
            model.favorite_ribbon.set_visible(true);
        }

        model.cover.widget().add_controller(drag_src);

        relm4::ComponentParts { model, widgets }
    }

    view! {
        gtk::Box {
            add_css_class: "artist-element",
            set_halign: gtk::Align::Center,

            add_controller = gtk::EventControllerMotion {
                connect_enter[sender] => move |_event, _x, _y| {
                    sender.input(ArtistElementIn::Hover(true));
                },
                connect_leave => ArtistElementIn::Hover(false),
            },

            gtk::Overlay {
                add_overlay = &gtk::Box {
                    set_halign: gtk::Align::Center,
                    set_valign: gtk::Align::Start,
                    set_margin_top: 135,
                    set_margin_start: 125,

                    model.favorite.clone() -> gtk::Button {
                        add_css_class: "cover-favorite",
                        set_width_request: 24,
                        set_height_request: 24,
                        set_icon_name: "non-starred-symbolic",

                        connect_clicked[sender, init] => move |btn| {
                            match btn.icon_name().as_deref() {
                                Some("starred-symbolic") => sender.output(ArtistElementOut::FavoriteClicked(init.id.clone(), false)).expect("sending failed"),
                                Some("non-starred-symbolic") => sender.output(ArtistElementOut::FavoriteClicked(init.id.clone(), true)).expect("sending failed"),
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
                        sender.output(ArtistElementOut::Clicked(init.clone())).unwrap();
                    },

                    #[wrap(Some)]
                    set_child = &gtk::Overlay {
                        add_overlay = &model.favorite_ribbon.clone() {
                            add_css_class: "cover-favorite-ribbon",
                            set_halign: gtk::Align::End,
                            set_valign: gtk::Align::End,
                            set_height_request: 35,
                            set_width_request: 35,
                            set_margin_bottom: 25,
                            set_margin_end: 5,
                        },

                        #[wrap(Some)]
                        set_child = &model.cover.widget().clone(),
                    }
                }
            }
        }
    }

    fn update(&mut self, msg: Self::Input, sender: relm4::ComponentSender<Self>) {
        match msg {
            ArtistElementIn::DescriptiveCover(msg) => match msg {
                DescriptiveCoverOut::DisplayToast(msg) => sender
                    .output(ArtistElementOut::DisplayToast(msg))
                    .expect("sending failed"),
            },
            ArtistElementIn::Favorited(id, state) => {
                if self.init.id == id {
                    match state {
                        true => {
                            self.favorite.set_icon_name("starred-symbolic");
                            self.favorite_ribbon.set_visible(true);
                        }
                        false => {
                            self.favorite.set_icon_name("non-starred-symbolic");
                            self.favorite_ribbon.set_visible(false);
                        }
                    }
                }
            }
            ArtistElementIn::Hover(false) => {
                self.favorite.set_visible(false);
            }
            ArtistElementIn::Hover(true) => {
                self.favorite.set_visible(true);
            }
        }
    }
}
