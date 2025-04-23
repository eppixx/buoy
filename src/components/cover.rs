use std::{cell::RefCell, rc::Rc};

use relm4::gtk::{
    self,
    gdk::{self},
    prelude::WidgetExt,
};

use crate::{
    gtk_helper::{loading_widget::LoadingWidgetState, stack::StackExt},
    subsonic::Subsonic,
    subsonic_cover,
};

pub struct Cover {
    subsonic: Rc<RefCell<Subsonic>>,

    // stack shows either a stock image, a loading wheel or a loaded cover
    stack: gtk::Stack,
    cover: gtk::Image,
}

impl std::fmt::Debug for Cover {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Cover")
            .field(
                "state",
                &self.stack.visible_child_enum::<LoadingWidgetState>(),
            )
            .finish()
    }
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

#[derive(Debug)]
pub enum CoverIn {
    LoadId(Option<String>),
    LoadSong(Box<submarine::data::Child>),
    LoadAlbumId3(Box<submarine::data::AlbumWithSongsId3>),
    LoadPlaylist(Box<submarine::data::PlaylistWithSongs>),
    LoadArtist(Box<submarine::data::ArtistId3>),
    ChangeImage(subsonic_cover::Response),
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
                add_enumed[LoadingWidgetState::Empty] = &gtk::Box {
                    add_css_class: "bordered",
                    add_css_class: "stock-cover",
                },
                add_enumed[LoadingWidgetState::NotEmpty] = &model.cover.clone() -> gtk::Image {
                    add_css_class: "card",
                },
                add_enumed[LoadingWidgetState::Loading] = &gtk::CenterBox {
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
                subsonic_cover::Response::Empty => self
                    .stack
                    .set_visible_child_enum(&LoadingWidgetState::Empty),
                subsonic_cover::Response::Loaded(pixbuf) => {
                    self.cover.set_paintable(Some(&pixbuf));
                    self.stack
                        .set_visible_child_enum(&LoadingWidgetState::NotEmpty);
                }
                subsonic_cover::Response::Processing(receiver) => {
                    self.stack
                        .set_visible_child_enum(&LoadingWidgetState::Loading);
                    sender.oneshot_command(async move {
                        match receiver.recv().await {
                            Ok((id, Some(texture), Some(buffer))) => {
                                CoverCmd::CoverLoaded(id, Some(texture), Some(buffer))
                            }
                            Ok((id, _, _)) => CoverCmd::CoverLoaded(id, None, None),
                            Err(e) => panic!("Cover error: {e}"),
                        }
                    });
                }
            },
            CoverIn::LoadId(None) => self
                .stack
                .set_visible_child_enum(&LoadingWidgetState::Empty),
            CoverIn::LoadId(Some(id)) => {
                sender.input(CoverIn::ChangeImage(self.subsonic.borrow_mut().cover(&id)));
            }
            CoverIn::LoadSong(child) => match child.album_id {
                None => self
                    .stack
                    .set_visible_child_enum(&LoadingWidgetState::Empty),
                Some(album_id) => {
                    let album = self.subsonic.borrow().find_album(album_id);
                    match album {
                        None => self
                            .stack
                            .set_visible_child_enum(&LoadingWidgetState::Empty),
                        Some(album) => match &album.cover_art {
                            Some(id) => sender
                                .input(CoverIn::ChangeImage(self.subsonic.borrow_mut().cover(id))),
                            None => self
                                .stack
                                .set_visible_child_enum(&LoadingWidgetState::Empty),
                        },
                    }
                }
            },
            CoverIn::LoadAlbumId3(album) => match album.base.cover_art {
                None => self
                    .stack
                    .set_visible_child_enum(&LoadingWidgetState::Empty),
                Some(id) => {
                    sender.input(CoverIn::ChangeImage(self.subsonic.borrow_mut().cover(&id)));
                }
            },
            CoverIn::LoadPlaylist(playlist) => match playlist.base.cover_art {
                None => self
                    .stack
                    .set_visible_child_enum(&LoadingWidgetState::Empty),
                Some(id) => {
                    sender.input(CoverIn::ChangeImage(self.subsonic.borrow_mut().cover(&id)));
                }
            },
            CoverIn::LoadArtist(artist) => match artist.cover_art {
                None => self
                    .stack
                    .set_visible_child_enum(&LoadingWidgetState::Empty),
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
