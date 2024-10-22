use std::{cell::RefCell, rc::Rc};

use relm4::gtk::{
    self,
    gdk::{self},
    prelude::WidgetExt,
};

use crate::{gtk_helper::stack::StackExt, subsonic::Subsonic, subsonic_cover};

#[derive(Debug)]
pub struct Cover {
    subsonic: Rc<RefCell<Subsonic>>,

    // stack shows either a stock image, a loading wheel or a loaded cover
    stack: gtk::Stack,
    cover: gtk::Image,
}

impl Cover {
    pub fn add_css_class_image(&self, class: &str) {
        self.stack.add_css_class(class);
    }

    pub fn change_size(&self, size: i32) {
        self.cover.set_width_request(size);
        self.cover.set_height_request(size);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum State {
    Stock,
    Image,
    Loading,
}

impl std::fmt::Display for State {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Stock => write!(f, "Stock"),
            Self::Image => write!(f, "Image"),
            Self::Loading => write!(f, "Loading"),
        }
    }
}

impl TryFrom<String> for State {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.as_ref() {
            "Stock" => Ok(Self::Stock),
            "Image" => Ok(Self::Image),
            "Loading" => Ok(Self::Loading),
            e => Err(format!("\"{e}\" is not a State")),
        }
    }
}

#[derive(Debug)]
pub enum CoverIn {
    LoadId(Option<String>),
    LoadSong(Box<submarine::data::Child>),
    LoadAlbumId3(Box<submarine::data::AlbumWithSongsId3>),
    LoadPlaylist(Box<submarine::data::PlaylistWithSongs>),
    LoadArtist(Box<submarine::data::ArtistId3>),
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

#[derive(Debug, Clone)]
pub enum CoverOut {
    DisplayToast(String),
}

#[derive(Debug)]
pub enum CoverCmd {
    CoverLoaded(String, Option<gdk::Texture>, Option<Vec<u8>>),
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
        };

        let widgets = view_output!();

        sender.input(CoverIn::LoadId(id));
        relm4::ComponentParts { model, widgets }
    }

    view! {
        gtk::Box {
            model.stack.clone() -> gtk::Stack {
                add_enumed[State::Stock] = &gtk::Box {
                    add_css_class: "cover",
                },
                add_enumed[State::Image] = &model.cover.clone() -> gtk::Image {
                    add_css_class: "card",
                },
                add_enumed[State::Loading] = &gtk::CenterBox {
                    add_css_class: "card",
                    #[wrap(Some)]
                    set_center_widget = &gtk::Spinner {
                        start: (),
                    }
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
            CoverIn::ChangeImage(response) => match response {
                subsonic_cover::Response::Empty => self.stack.set_visible_child_enum(&State::Stock),
                subsonic_cover::Response::Loaded(pixbuf) => {
                    self.cover.set_from_paintable(Some(&pixbuf));
                    self.stack.set_visible_child_enum(&State::Image);
                }
                subsonic_cover::Response::Processing(receiver) => {
                    self.stack.set_visible_child_enum(&State::Loading);
                    sender.oneshot_command(async move {
                        match receiver.recv().await {
                            Ok((id, Some(texture), Some(buffer))) => {
                                CoverCmd::CoverLoaded(id, Some(texture), Some(buffer))
                            }
                            Ok((id, _, _)) => CoverCmd::CoverLoaded(id, None, None),
                            Err(_) => panic!(),
                        }
                    });
                }
            },
            CoverIn::LoadId(None) => self.stack.set_visible_child_enum(&State::Stock),
            CoverIn::LoadId(Some(id)) => {
                sender.input(CoverIn::ChangeImage(self.subsonic.borrow_mut().cover(&id)));
            }
            CoverIn::LoadSong(child) => match child.album_id {
                None => self.stack.set_visible_child_enum(&State::Stock),
                Some(album_id) => {
                    let album = self.subsonic.borrow().find_album(album_id);
                    match album {
                        None => self.stack.set_visible_child_enum(&State::Stock),
                        Some(album) => match &album.cover_art {
                            Some(id) => sender
                                .input(CoverIn::ChangeImage(self.subsonic.borrow_mut().cover(id))),
                            None => self.stack.set_visible_child_enum(&State::Stock),
                        },
                    }
                }
            },
            CoverIn::LoadAlbumId3(album) => match album.base.cover_art {
                None => self.stack.set_visible_child_enum(&State::Stock),
                Some(id) => {
                    sender.input(CoverIn::ChangeImage(self.subsonic.borrow_mut().cover(&id)));
                }
            },
            CoverIn::LoadPlaylist(playlist) => match playlist.base.cover_art {
                None => self.stack.set_visible_child_enum(&State::Stock),
                Some(id) => {
                    sender.input(CoverIn::ChangeImage(self.subsonic.borrow_mut().cover(&id)));
                }
            },
            CoverIn::LoadArtist(artist) => match artist.cover_art {
                None => self.stack.set_visible_child_enum(&State::Stock),
                Some(id) => {
                    sender.input(CoverIn::ChangeImage(self.subsonic.borrow_mut().cover(&id)));
                }
            },
        }
    }

    fn update_cmd(
        &mut self,
        message: Self::CommandOutput,
        sender: relm4::ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match message {
            CoverCmd::CoverLoaded(id, Some(texture), Some(buffer)) => {
                sender.input(CoverIn::ChangeImage(subsonic_cover::Response::Loaded(
                    texture,
                )));

                self.subsonic.borrow_mut().cover_update(&id, Some(buffer));
            }
            CoverCmd::CoverLoaded(id, _, _) => {
                sender.input(CoverIn::LoadId(None));
                self.subsonic.borrow_mut().cover_update(&id, None);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gtk_helper::stack::test_self;

    #[test]
    fn state_enum_conversion() {
        test_self(State::Stock);
        test_self(State::Image);
    }
}
