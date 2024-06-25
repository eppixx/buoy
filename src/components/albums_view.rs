use std::{cell::RefCell, rc::Rc};

use fuzzy_matcher::FuzzyMatcher;
use relm4::{
    gtk::{
        self, glib,
        prelude::{OrientableExt, WidgetExt},
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
    DisplayToast(String),
}

#[derive(Debug)]
pub enum AlbumsViewIn {
    AlbumElement(AlbumElementOut),
    SearchChanged(String),
}

#[relm4::component(pub)]
impl relm4::component::Component for AlbumsView {
    type Init = Rc<RefCell<Subsonic>>;
    type Input = AlbumsViewIn;
    type Output = AlbumsViewOut;
    type CommandOutput = ();

    fn init(
        init: Self::Init,
        root: Self::Root,
        sender: relm4::ComponentSender<Self>,
    ) -> relm4::component::ComponentParts<Self> {
        let mut model = Self::default();
        let widgets = view_output!();

        // add albums with cover and title
        for album in init.borrow().albums() {
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

            gtk::WindowHandle {
                gtk::Label {
                    add_css_class: "h2",
                    set_label: "Albums",
                    set_halign: gtk::Align::Center,
                },
            },

            gtk::ScrolledWindow {
                set_hexpand: true,
                set_vexpand: true,

                #[wrap(Some)]
                set_child = &model.albums.clone() -> gtk::FlowBox {
                    set_valign: gtk::Align::Start,
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
                    sender.output(AlbumsViewOut::Clicked(clicked)).unwrap();
                }
                AlbumElementOut::DisplayToast(title) => sender
                    .output(AlbumsViewOut::DisplayToast(title))
                    .expect("sending failed"),
            },
            AlbumsViewIn::SearchChanged(search) => {
                self.albums.set_filter_func(move |element| {
                    use glib::object::Cast;

                    // get the Label of the FlowBoxChild
                    let button = element.first_child().unwrap();
                    let bo = button.first_child().unwrap();
                    let cover = bo.first_child().unwrap();
                    let title = cover.next_sibling().unwrap();
                    let title = title.downcast::<gtk::Label>().expect("unepected element");

                    let artist = title.next_sibling().unwrap();
                    let artist = artist.downcast::<gtk::Label>().expect("unexpected element");
                    let title_artist = format!("{} {}", title.text(), artist.text());

                    //actual matching
                    let matcher = fuzzy_matcher::skim::SkimMatcherV2::default();
                    let score = matcher.fuzzy_match(&title_artist, &search);
                    score.is_some()
                });
            }
        }
    }
}
