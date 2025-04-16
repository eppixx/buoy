use std::{cell::RefCell, rc::Rc};

use gettextrs::gettext;
use relm4::{
    gtk::{
        self,
        prelude::{ButtonExt, ToValue, WidgetExt},
        FlowBoxChild,
    },
    Component, ComponentController, RelmWidgetExt,
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
    subsonic: Rc<RefCell<Subsonic>>,
    cover: relm4::Controller<DescriptiveCover>,
    id: Id,
    favorite: gtk::Button,
    favorite_ribbon: gtk::Box,
}

impl AlbumElement {
    pub fn change_size(&self, size: i32) {
        self.cover.model().change_size(size);
    }
}

#[derive(Debug, Clone)]
pub enum AlbumElementIn {
    DescriptiveCover(DescriptiveCoverOut),
    Favorited(String, bool),
    Hover(bool),
    FavoriteClicked,
    Clicked,
}

#[derive(Debug)]
pub enum AlbumElementOut {
    Clicked(Id),
    FavoriteClicked(String, bool),
    DisplayToast(String),
}

#[relm4::factory(pub)]
impl relm4::factory::FactoryComponent for AlbumElement {
    type Init = (Rc<RefCell<Subsonic>>, Id);
    type Input = AlbumElementIn;
    type Output = AlbumElementOut;
    type CommandOutput = ();
    type ParentWidget = gtk::FlowBox;

    fn init_model(
        (subsonic, id): Self::Init,
        _index: &relm4::factory::DynamicIndex,
        sender: relm4::factory::FactorySender<Self>,
    ) -> Self {
        //check id
        let Id::Album(_) = &id else {
            panic!("given id: '{id}' is not an album");
        };

        let album = subsonic.borrow().find_album(id.as_ref()).unwrap();
        let drop = Droppable::AlbumChild(Box::new(album.clone()));
        let builder = DescriptiveCoverInit::new(
            album.title.clone(),
            album.cover_art.clone(),
            album.artist.clone(),
        );

        let cover: relm4::Controller<DescriptiveCover> = DescriptiveCover::builder()
            .launch((subsonic.clone(), builder))
            .forward(sender.input_sender(), AlbumElementIn::DescriptiveCover);
        let model = Self {
            subsonic,
            cover,
            id,
            favorite: gtk::Button::default(),
            favorite_ribbon: gtk::Box::default(),
        };

        let length_tr = gettext("Length");
        let year_tr = gettext("Year");

        // tooltip string
        let mut tooltip = album.name;
        if let Some(artist) = album.artist {
            tooltip.push_str(" - ");
            tooltip.push_str(&artist);
        }
        tooltip.push('\n');
        if let Some(year) = album.year {
            tooltip.push_str(&year_tr);
            tooltip.push_str(": ");
            tooltip.push_str(&year.to_string());
        }
        match album.duration {
            Some(duration) if !tooltip.is_empty() => {
                tooltip.push_str(" • ");
                tooltip.push_str(&length_tr);
                tooltip.push_str(": ");
                tooltip.push_str(&convert_for_label(i64::from(duration) * 1000));
            }
            Some(duration) => {
                tooltip.push_str(&length_tr);
                tooltip.push_str(": ");
                tooltip.push_str(&convert_for_label(i64::from(duration) * 1000));
            }
            None => {}
        };
        model.cover.widget().set_tooltip(&tooltip);

        //setup DropSource
        let content = gtk::gdk::ContentProvider::for_value(&drop.to_value());
        let drag_src = gtk::DragSource::new();
        drag_src.set_actions(gtk::gdk::DragAction::COPY);
        drag_src.set_content(Some(&content));
        let subsonic = model.subsonic.clone();
        drag_src.connect_drag_begin(move |src, _drag| {
            if let Some(cover_id) = &album.cover_art {
                let cover = subsonic.borrow().cover_icon(cover_id);
                if let Some(tex) = cover {
                    src.set_icon(Some(&tex), 0, 0);
                }
            }
        });
        model.cover.widget().add_controller(drag_src);

        // set favorite icon
        model.favorite.set_visible(false);
        model.favorite_ribbon.set_visible(false);
        if album.starred.is_some() {
            model.favorite.set_icon_name("starred-symbolic");
            model.favorite_ribbon.set_visible(true);
        } else {
            model.favorite.set_icon_name("non-starred-symbolic");
        }

        model
    }

    view! {
        gtk::FlowBoxChild {
            set_widget_name: "album-element",
            set_halign: gtk::Align::Center,

            add_controller = gtk::EventControllerMotion {
                connect_enter[sender] => move |_event, _x, _y| {
                    sender.input(AlbumElementIn::Hover(true));
                },
                connect_leave => AlbumElementIn::Hover(false),
            },

            gtk::Overlay {
                add_overlay = &gtk::Box {
                    set_halign: gtk::Align::Center,
                    set_valign: gtk::Align::Start,
                    set_margin_top: 135,
                    set_margin_start: 125,

                    #[name = "favorite"]
                    self.favorite.clone() -> gtk::Button {
                        add_css_class: "neutral-color",
                        set_width_request: 24,
                        set_height_request: 24,

                        connect_clicked => AlbumElementIn::FavoriteClicked,
                    }
                },

                #[wrap(Some)]
                set_child = &gtk::Button {
                    add_css_class: "flat",
                    set_halign: gtk::Align::Center,

                    connect_clicked => AlbumElementIn::Clicked,

                    #[wrap(Some)]
                    set_child = &gtk::Overlay {
                        add_overlay = &self.favorite_ribbon.clone() {
                            add_css_class: "cover-favorite-ribbon",
                            set_halign: gtk::Align::Start,
                            set_valign: gtk::Align::Start,
                            set_height_request: 35,
                            set_width_request: 35,
                        },

                        #[wrap(Some)]
                        set_child = &self.cover.widget().clone() {}
                    }
                }
            },
        }
    }

    fn update_with_view(
        &mut self,
        widgets: &mut Self::Widgets,
        msg: Self::Input,
        sender: relm4::FactorySender<Self>,
    ) {
        match msg {
            AlbumElementIn::DescriptiveCover(msg) => match msg {
                DescriptiveCoverOut::DisplayToast(title) => {
                    sender.output(AlbumElementOut::DisplayToast(title)).unwrap();
                }
            },
            AlbumElementIn::Favorited(id, state) => {
                if self.id.as_ref() == id {
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
            AlbumElementIn::Hover(false) => {
                self.favorite.set_visible(false);
            }
            AlbumElementIn::Hover(true) => {
                self.favorite.set_visible(true);
            }
            AlbumElementIn::FavoriteClicked => match widgets.favorite.icon_name().as_deref() {
                Some("starred-symbolic") => sender
                    .output(AlbumElementOut::FavoriteClicked(
                        self.id.inner().to_string(),
                        false,
                    ))
                    .unwrap(),
                Some("non-starred-symbolic") => sender
                    .output(AlbumElementOut::FavoriteClicked(
                        self.id.inner().to_string(),
                        true,
                    ))
                    .unwrap(),
                name => unreachable!("unkonwn icon name: {name:?}"),
            },
            AlbumElementIn::Clicked => {
                sender
                    .output(AlbumElementOut::Clicked(self.id.clone()))
                    .unwrap();
            }
        }
    }
}

pub fn get_info_of_flowboxchild(element: &FlowBoxChild) -> Option<(gtk::Label, gtk::Label)> {
    use gtk::glib::object::Cast;
    let overlay = element.first_child()?;
    let button = overlay.first_child()?;
    let overlay = button.first_child()?;
    let bo = overlay.first_child()?;
    let cover = bo.first_child()?;
    let title = cover.next_sibling()?;
    let title = title.downcast::<gtk::Label>().expect("unepected element");
    let artist = title.next_sibling()?;
    let artist = artist.downcast::<gtk::Label>().expect("unexpected element");

    Some((title, artist))
}
