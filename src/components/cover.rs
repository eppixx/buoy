use std::{cell::RefCell, rc::Rc};

use relm4::{gtk, gtk::prelude::{WidgetExt, ButtonExt}};

use crate::{gtk_helper::stack::StackExt, subsonic::{Subsonic, Sync}, subsonic_cover};
use crate::types::Id;

#[derive(Debug)]
pub struct Cover {
    subsonic: Rc<RefCell<Subsonic>>,
    id: Option<Id>,

    // stack shows either a stock image, a loading wheel or a loaded cover
    stack: gtk::Stack,
    cover: gtk::Image,
    favorite: gtk::Button,
}

impl Cover {
    pub fn add_css_class_image(&self, class: &str) {
        self.stack.add_css_class(class);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum State {
    Stock,
    Image,
}

impl std::fmt::Display for State {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Stock => write!(f, "Stock"),
            Self::Image => write!(f, "Image"),
        }
    }
}

impl TryFrom<String> for State {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.as_ref() {
            "Stock" => Ok(Self::Stock),
            "Image" => Ok(Self::Image),
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
    SetFavorite(bool),
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
pub enum CoverCmd {}

#[relm4::component(pub)]
impl relm4::Component for Cover {
    type Init = (Rc<RefCell<Subsonic>>, Option<String>, bool, Option<Id>);
    type Input = CoverIn;
    type Output = CoverOut;
    type Widgets = CoverWidgets;
    type CommandOutput = CoverCmd;

    fn init(
        (subsonic, id, show_favorite, typ): Self::Init,
        root: Self::Root,
        sender: relm4::ComponentSender<Self>,
    ) -> relm4::ComponentParts<Self> {
        let model = Self {
            subsonic,
            id: typ,
            stack: gtk::Stack::default(),
            cover: gtk::Image::default(),
            favorite: gtk::Button::default(),
        };

        let widgets = view_output!();

        // set favorite icon
        if show_favorite {
            model.favorite.set_halign(gtk::Align::End);
            model.favorite.set_valign(gtk::Align::End);
            model.favorite.set_width_request(24);
            model.favorite.set_height_request(24);

            if let Some(typ) = &model.id {
                let mut starred = false;
                match typ {
                    Id::Album(id) => {
                        let album = model.subsonic.borrow().find_album(id);
                        if let Some(album) = album {
                            starred = album.starred.is_some();
                        }
                    }
                    Id::Artist(id) => {
                        let artist = model.subsonic.borrow().find_artist(id);
                        if let Some(artist) = artist {
                            starred = artist.starred.is_some();
                        }
                    }
                    Id::Song(_id) | Id::Playlist(_id) => {} // cant be favorited
                }
                if starred {
                    model.favorite.set_icon_name("starred-symbolic");
                }
            } else {
                model.favorite.set_icon_name("non-starred-symbolic");
            }
            widgets.overlay.add_overlay(&model.favorite.clone());

            if let Some(id) = model.id.clone() {
                let subsonic = model.subsonic.clone();
                model.favorite.connect_clicked(move|btn| {
                    println!("clicked on {id:?}");
                    if btn.icon_name().as_deref() == Some("starred-symbolic") {
                        subsonic.borrow().send(Sync::Favorited(id.inner().to_string(), false));
                    } else {
                        subsonic.borrow().send(Sync::Favorited(id.inner().to_string(), true));
                    }
                });
            }
        }

        // receive changes
        let sender = sender.clone();
        let receiver = model.subsonic.borrow().receiver().clone();
        let relm_sender = sender.clone();
        let local_id = model.id.clone();
        if let Some(local_id) = local_id {
            gtk::glib::spawn_future_local(async move {
                while let Ok(msg) = receiver.recv().await {
                    println!("received msg {msg:?}");
                    match msg {
                        Sync::Favorited(changed_id, value) if local_id.inner() == changed_id => {
                            println!("received signal to be {value}");
                            relm_sender.input(CoverIn::SetFavorite(value));
                        }
                        _ => {}
                    }
                }
            });
        }

        sender.input(CoverIn::LoadId(id));
        relm4::ComponentParts { model, widgets }
    }

    view! {
        gtk::Box {
            #[name = "overlay"]
            gtk::Overlay {
                #[wrap(Some)]
                set_child = &model.stack.clone() -> gtk::Stack {
                    add_enumed[State::Stock] = &gtk::Box {
                        add_css_class: "cover",
                    },
                    add_enumed[State::Image] = &model.cover.clone() -> gtk::Image {
                        add_css_class: "card",
                    },
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
            },
            CoverIn::LoadId(None) => self.stack.set_visible_child_enum(&State::Stock),
            CoverIn::LoadId(Some(id)) => {
                sender.input(CoverIn::ChangeImage(self.subsonic.borrow_mut().cover(&id)));
                // update favorite
                // sender.oneshot_command(async move {
                    
                // });
            }
            CoverIn::LoadSong(child) => {
                sender.input(CoverIn::SetFavorite(child.starred.is_some()));
                match child.album_id {
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
                }
            },
            CoverIn::LoadAlbumId3(album) => {
                sender.input(CoverIn::SetFavorite(album.base.starred.is_some()));
                match album.base.cover_art {
                    None => self.stack.set_visible_child_enum(&State::Stock),
                    Some(id) => {
                        sender.input(CoverIn::ChangeImage(self.subsonic.borrow_mut().cover(&id)));
                    }
                }
            },
            CoverIn::LoadPlaylist(playlist) => match playlist.base.cover_art {
                None => self.stack.set_visible_child_enum(&State::Stock),
                Some(id) => {
                    sender.input(CoverIn::ChangeImage(self.subsonic.borrow_mut().cover(&id)));
                }
            },
            CoverIn::LoadArtist(artist) => {
                sender.input(CoverIn::SetFavorite(artist.starred.is_some()));
                match artist.cover_art {
                    None => self.stack.set_visible_child_enum(&State::Stock),
                    Some(id) => {
                        sender.input(CoverIn::ChangeImage(self.subsonic.borrow_mut().cover(&id)));
                    }
                }
            },
            CoverIn::SetFavorite(true) => self.favorite.set_icon_name("starred-symbolic"),
            CoverIn::SetFavorite(false) => self.favorite.set_icon_name("non-starred-symbolic"),
        }
    }

    fn update_cmd(
        &mut self,
        message: Self::CommandOutput,
        _sender: relm4::ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match message {}
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
