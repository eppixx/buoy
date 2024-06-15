use std::{cell::RefCell, rc::Rc};

use relm4::{gtk, gtk::prelude::WidgetExt};

use crate::{
    client::Client,
    subsonic::Subsonic,
    subsonic_cover::{self},
    types::Id,
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
    LoadId(Option<Id>),
    ChangeImage(subsonic_cover::Response),
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
pub enum CoverOut {
    DisplayToast(String),
}

#[derive(Debug)]
pub enum CoverCmd {
    ChangeImage(Option<String>),
    ErrorOccured(String),
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
        root: Self::Root,
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
                    add_css_class: "cover",
                },
                add_named[Some("cover")] = &model.cover.clone() -> gtk::Image {
                    add_css_class: "card",
                },
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
            CoverIn::ChangeImage(response) => match response {
                subsonic_cover::Response::Empty => self.stack.set_visible_child_name("stock"),
                subsonic_cover::Response::Loaded(pixbuf) => {
                    self.cover.set_from_pixbuf(Some(&pixbuf));
                    self.stack.set_visible_child_name("cover");
                }
            },
            CoverIn::LoadImage(None) => self.stack.set_visible_child_name("stock"),
            CoverIn::LoadImage(Some(id)) => {
                sender.input(CoverIn::ChangeImage(self.subsonic.borrow_mut().cover(&id)))
            }
            CoverIn::LoadId(None) => self.stack.set_visible_child_name("stock"),
            CoverIn::LoadId(Some(Id::Song(id))) => sender.oneshot_command(async move {
                let client = Client::get().unwrap();
                match client.get_song(id).await {
                    Err(e) => CoverCmd::ErrorOccured(format!("could not get song: {e:?}")),
                    Ok(child) => match child.album_id {
                        None => CoverCmd::ChangeImage(child.cover_art),
                        Some(album_id) => match client.get_album(album_id).await {
                            Err(e) => {
                                CoverCmd::ErrorOccured(format!("could not fetch album: {e:?}"))
                            }
                            Ok(album) => CoverCmd::ChangeImage(album.base.cover_art),
                        },
                    },
                }
            }),
            CoverIn::LoadId(Some(Id::Album(id))) => {
                sender.oneshot_command(async move {
                    let client = Client::get().unwrap();
                    match client.get_album(id).await {
                        Err(e) => CoverCmd::ErrorOccured(format!("could not fetch album: {e:?}")),
                        Ok(album) => CoverCmd::ChangeImage(album.base.cover_art),
                    }
                });
            }
            CoverIn::LoadId(Some(Id::Artist(id))) => {
                sender.oneshot_command(async move {
                    let client = Client::get().unwrap();
                    match client.get_artist(id).await {
                        Err(e) => CoverCmd::ErrorOccured(format!("could not fetch artist: {e:?}")),
                        Ok(artist) => CoverCmd::ChangeImage(artist.base.cover_art),
                    }
                });
            }
            CoverIn::LoadId(Some(Id::Playlist(id))) => {
                sender.oneshot_command(async move {
                    let client = Client::get().unwrap();
                    match client.get_playlist(id).await {
                        Err(e) => {
                            CoverCmd::ErrorOccured(format!("could not fetch playlist: {e:?}"))
                        }
                        Ok(playlist) => CoverCmd::ChangeImage(playlist.base.cover_art),
                    }
                });
            }
        }
    }

    fn update_cmd(
        &mut self,
        message: Self::CommandOutput,
        sender: relm4::ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match message {
            CoverCmd::ChangeImage(id) => match id {
                None => self.stack.set_visible_child_name("stock"),
                Some(id) => {
                    sender.input(CoverIn::ChangeImage(self.subsonic.borrow_mut().cover(&id)))
                }
            },
            CoverCmd::ErrorOccured(title) => {
                self.stack.set_visible_child_name("stock");
                sender
                    .output(CoverOut::DisplayToast(title))
                    .expect("sending failed");
            }
        }
    }
}
