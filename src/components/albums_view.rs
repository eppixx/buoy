use std::{cell::RefCell, rc::Rc};

use relm4::{
    gtk::{
        self,
        traits::{OrientableExt, WidgetExt},
    },
    ComponentController,
};

use super::album_element::AlbumElementOut;
use crate::{
    components::album_element::{AlbumElement, AlbumElementInit},
    subsonic::Subsonic,
};

#[derive(Debug, Default)]
pub struct AlbumsView {
    albums: gtk::FlowBox,
    album_list: Vec<relm4::Controller<AlbumElement>>,
}

#[derive(Debug)]
pub enum AlbumsViewOut {
    Clicked(AlbumElementInit),
}

#[derive(Debug)]
pub enum AlbumsViewIn {
    AlbumElement(AlbumElementOut),
}

#[relm4::component(pub)]
impl relm4::component::Component for AlbumsView {
    type Init = Rc<RefCell<Subsonic>>;
    type Input = AlbumsViewIn;
    type Output = AlbumsViewOut;
    type CommandOutput = ();

    fn init(
        init: Self::Init,
        root: &Self::Root,
        sender: relm4::ComponentSender<Self>,
    ) -> relm4::component::ComponentParts<Self> {
        let mut model = Self::default();
        let widgets = view_output!();

        // add albums with cover and title
        for album in init.borrow().albums().iter() {
            let cover: relm4::Controller<AlbumElement> = AlbumElement::builder()
                .launch((
                    init.clone(),
                    AlbumElementInit::Child(Box::new(album.clone())),
                ))
                .forward(sender.input_sender(), AlbumsViewIn::AlbumElement);
            model.albums.append(cover.widget());
            model.album_list.push(cover);
        }

        relm4::component::ComponentParts { model, widgets }
    }

    view! {
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            set_hexpand: true,

            gtk::Label {
                add_css_class: "h2",
                set_label: "Albums",
                set_halign: gtk::Align::Center,
            },

            gtk::ScrolledWindow {
                set_hexpand: true,
                set_vexpand: true,

                #[wrap(Some)]
                set_child = &model.albums.clone() -> gtk::FlowBox {
                }
            }
        }
    }

    fn update(
        &mut self,
        msg: Self::Input,
        sender: relm4::ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match msg {
            AlbumsViewIn::AlbumElement(msg) => match msg {
                AlbumElementOut::Clicked(clicked) => {
                    sender.output(AlbumsViewOut::Clicked(clicked)).unwrap()
                }
            },
        }
    }
}
