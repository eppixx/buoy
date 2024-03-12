use std::{cell::RefCell, rc::Rc, sync::mpsc::Receiver};

use relm4::{gtk, gtk::traits::WidgetExt};

use crate::{
    subsonic::Subsonic,
    subsonic_cover::{self},
};

#[derive(Debug)]
pub struct Cover {
    subsonic: Rc<RefCell<Subsonic>>,

    // stack shows either a stock image, a loading wheel or a loaded cover
    stack: gtk::Stack,
    cover: gtk::Image,

		//raw cover id
    id: Option<String>,
}

impl Cover {
    pub fn add_css_class_image(&self, class: &str) {
        self.stack.add_css_class(class);
    }
}

#[derive(Debug)]
pub enum CoverIn {
    LoadImage(Option<String>),
    // LoadCoverForChild(submarine::data::Child),
    // LoadId(Option<Id>),
}

// use tuple struct to keep the logging small
pub struct Image(Vec<u8>);

impl std::fmt::Debug for Image {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Image buffer")
            .field(&format!("size: {}", self.0.len()))
            .finish()
    }
}

#[derive(Debug)]
pub enum CoverOut {}

#[derive(Debug)]
pub enum CoverCmd {
    // LoadChild(submarine::data::Child),
}

#[relm4::component(pub)]
impl relm4::Component for Cover {
    type Init = (Rc<RefCell<Subsonic>>, Option<String>);
    type Input = CoverIn;
    type Output = CoverOut;
    type Widgets = CoverWidgets;
    type CommandOutput = CoverCmd;

    fn init(
        (subsonic, id): Self::Init,
        root: &Self::Root,
        sender: relm4::ComponentSender<Self>,
    ) -> relm4::ComponentParts<Self> {
        let model = Self {
            subsonic,
            stack: gtk::Stack::default(),
            cover: gtk::Image::default(),

            id,
        };

        let widgets = view_output!();

				sender.input(CoverIn::LoadImage(model.id.clone()));
        relm4::ComponentParts { model, widgets }
    }

    view! {
        gtk::Box {
            model.stack.clone() -> gtk::Stack {
                add_named[Some("stock")] = &gtk::Box {
                    add_css_class: "card",
                    add_css_class: "cover",
                },
                add_named[Some("cover")] = &model.cover.clone(),
            }
        }
    }

    fn update(
        &mut self,
        msg: Self::Input,
        _sender: relm4::ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match msg {
            CoverIn::LoadImage(None) => self.stack.set_visible_child_name("stock"),
            CoverIn::LoadImage(Some(id)) => match self.subsonic.borrow_mut().coverss.cover(&id) {
                subsonic_cover::Response::Empty => self.stack.set_visible_child_name("stock"),
                subsonic_cover::Response::Loaded(pixbuf) => {
                    self.cover.set_from_pixbuf(Some(&pixbuf));
                    self.stack.set_visible_child_name("cover");
                }
            }, // CoverIn::LoadCoverForChild(child) => {
               // 		sender.clone().oneshot_command(async move {
               // 				match child.album_id {
               // 						None => sender.input(CoverIn::LoadImage(child.cover_art)),
               // 						Some(album_id) => {
               // 								let client = Client::get().unwrap();
               // 								match client.get_album(album_id).await {
               // 										Err(e) => sender.input(CoverIn::LoadImage(child.cover_art)),
               // 										Ok(album) => sender.input(CoverIn::LoadImage(album.base.cover_art)),
               // 								}
               // 						}
               // 				}
               // 				CoverCmd::LoadedImage(false)
               // 		})
               // }
               // CoverIn::LoadId(None) => self.stack.set_visible_child_name("stock"),
               // CoverIn::LoadId(Some(Id::Song(id))) => {

               // }
               // CoverIn::LoadId(_) => {}
        }
    }

    fn update_cmd(
        &mut self,
        message: Self::CommandOutput,
        _sender: relm4::ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match message {
            // CoverCmd::LoadChild(child) => {
            // }
        }
    }
}
