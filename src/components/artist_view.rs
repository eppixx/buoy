use relm4::{
    gtk::{
        self,
        traits::{BoxExt, OrientableExt, WidgetExt},
    },
    view, ComponentController,
};

use crate::{client::Client, types::Id};

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
    LoadArtist(Id),
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
    LoadedArtistInfo(Result<submarine::data::ArtistId3, submarine::SubsonicError>),
}

#[relm4::component(pub)]
impl relm4::Component for ArtistView {
    type Init = Id;
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
            title: String::from("Unkonwn Title"),
            bio: String::new(),
            loaded_albums: false,
            albums: gtk::FlowBox::default(),
            album_elements: vec![],
        };
        let widgets = view_output!();

        // model.cover.model().add_css_class_image("size100");
        sender.input(ArtistViewIn::LoadArtist(init.clone()));

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
            ArtistViewIn::LoadArtist(id) => {
                // load albums of artist
                let id2 = id.clone();
                sender.oneshot_command(async move {
                    let client = Client::get().lock().unwrap().inner.clone().unwrap();
                    ArtistViewCmd::LoadedAlbums(client.get_artist(id2.inner()).await)
                });
                // load metainfo on artist
                let id2 = id.clone();
                sender.oneshot_command(async move {
                    let client = Client::get().lock().unwrap().inner.clone().unwrap();
                    ArtistViewCmd::LoadedArtist(
                        client
                            .get_artist_info2(id2.inner(), Some(5), Some(false))
                            .await,
                    )
                });
                // load artist info
                sender.oneshot_command(async move {
                    let client = Client::get().lock().unwrap().inner.clone().unwrap();
                    ArtistViewCmd::LoadedArtistInfo(
                        client
                            .get_artist(id.inner())
                            .await
                            .map(|artist| artist.base),
                    )
                });
            }
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
            ArtistViewCmd::LoadedArtistInfo(Err(e)) => {} //TODO error handling
            ArtistViewCmd::LoadedArtistInfo(Ok(info)) => {
                self.title = info.name;
                self.cover.emit(CoverIn::LoadImage(info.cover_art));
            }
        }
    }
}
