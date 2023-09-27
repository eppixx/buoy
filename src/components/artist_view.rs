use relm4::{
    gtk::{
        self,
        prelude::ToValue,
        traits::{BoxExt, OrientableExt, WidgetExt},
    },
    view, ComponentController,
};

use crate::{
    client::Client,
    types::{Droppable, Id},
};

use super::{
    album_element::{AlbumElement, AlbumElementInit, AlbumElementOut},
    cover::{Cover, CoverIn},
};

#[derive(Debug)]
pub struct ArtistView {
    cover: relm4::Controller<Cover>,
    title: String,
    bio: String,
    loaded_albums: bool,
    albums: gtk::FlowBox,
    album_elements: Vec<relm4::Controller<AlbumElement>>,
}

#[derive(Debug)]
pub enum ArtistViewIn {
    AlbumElement(AlbumElementOut),
}

#[derive(Debug)]
pub enum ArtistViewOut {
    AlbumClicked(Id),
}

#[derive(Debug)]
pub enum ArtistViewCmd {
    LoadedAlbums(Result<submarine::data::ArtistWithAlbumsId3, submarine::SubsonicError>),
    LoadedArtist(Result<submarine::data::ArtistInfo, submarine::SubsonicError>),
}

#[relm4::component(pub)]
impl relm4::Component for ArtistView {
    type Init = submarine::data::ArtistId3;
    type Input = ArtistViewIn;
    type Output = ArtistViewOut;
    type Widgets = ArtistViewWidgets;
    type CommandOutput = ArtistViewCmd;

    fn init(
        init: Self::Init,
        root: &Self::Root,
        sender: relm4::ComponentSender<Self>,
    ) -> relm4::ComponentParts<Self> {
        let model = Self {
            cover: Cover::builder().launch(()).detach(),
            title: init.name.clone(),
            bio: String::new(),
            loaded_albums: false,
            albums: gtk::FlowBox::default(),
            album_elements: vec![],
        };
        let widgets = view_output!();

        //setup DropSource
        let drop = Droppable::Artist(Box::new(init.clone()));
        let content = gtk::gdk::ContentProvider::for_value(&drop.to_value());
        let drag_src = gtk::DragSource::new();
        drag_src.set_actions(gtk::gdk::DragAction::MOVE);
        drag_src.set_content(Some(&content));
        model.cover.widget().add_controller(drag_src);

        // load cover
        model.cover.emit(CoverIn::LoadImage(init.cover_art.clone()));
        model.cover.model().add_css_class_image("size100");

        // load albums
        let id = init.id.clone();
        sender.oneshot_command(async move {
            let client = Client::get().lock().unwrap().inner.clone().unwrap();
            ArtistViewCmd::LoadedAlbums(client.get_artist(id).await)
        });

        // load metainfo on artist
        let id = init.id.clone();
        sender.oneshot_command(async move {
            let client = Client::get().lock().unwrap().inner.clone().unwrap();
            ArtistViewCmd::LoadedArtist(client.get_artist_info2(id, Some(5), Some(false)).await)
        });

        relm4::ComponentParts { model, widgets }
    }

    view! {
        gtk::Box {
            add_css_class: "artist-view",
            set_orientation: gtk::Orientation::Vertical,

            gtk::Box {
                set_spacing: 15,

                model.cover.widget().clone() -> gtk::Box {},

                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_spacing: 5,

                    gtk::Label {
                        add_css_class: "h3",
                        #[watch]
                        set_label: &model.title,
                        set_halign: gtk::Align::Start,
                    },
                    gtk::Label {
                        #[watch]
                        set_label: &model.bio,
                        set_halign: gtk::Align::Start,
                    }
                }
            },

            gtk::ScrolledWindow {
                set_vexpand: true,

                model.albums.clone() {},
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
            ArtistViewIn::AlbumElement(msg) => match msg {
                AlbumElementOut::Clicked(id) => {
                    sender.output(ArtistViewOut::AlbumClicked(id)).unwrap()
                }
            },
        }
    }

    fn update_cmd(
        &mut self,
        msg: Self::CommandOutput,
        sender: relm4::ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match msg {
            ArtistViewCmd::LoadedArtist(Err(e)) => {} //TODO error handling
            ArtistViewCmd::LoadedArtist(Ok(artist)) => {
                // self.bio = artist.base.biography;
                // TODO do smth with biography
                // TODO do smth with similar artists
            }
            ArtistViewCmd::LoadedAlbums(Err(e)) => {} //TODO error handling
            ArtistViewCmd::LoadedAlbums(Ok(artist)) => {
                for album in artist.album {
                    let element = AlbumElement::builder()
                        .launch(AlbumElementInit::AlbumId3(album))
                        .forward(sender.input_sender(), |msg| ArtistViewIn::AlbumElement(msg));
                    self.albums.append(element.widget());
                    self.album_elements.push(element);
                }
            }
        }
    }
}
