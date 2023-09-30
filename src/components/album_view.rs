use relm4::{
    gtk::{
        self,
        prelude::ToValue,
        traits::{BoxExt, OrientableExt, WidgetExt},
    },
    ComponentController,
};

use super::cover::Cover;
use crate::{
    client::Client,
    components::{album_tracks::AlbumTracks, cover::CoverIn},
    types::Droppable,
};

#[derive(Debug)]
pub struct AlbumView {
    cover: relm4::Controller<Cover>,
    title: String,
    artist: Option<String>,
    info: String,
    view: gtk::Viewport,
    loaded_tracks: bool,
    tracks: Option<relm4::Controller<AlbumTracks>>,
}

#[derive(Debug)]
pub enum AlbumViewInit {
    Child(Box<submarine::data::Child>),
    AlbumId3(Box<submarine::data::AlbumId3>),
}

#[derive(Debug)]
pub enum AlbumViewOut {
    AppendAlbum(submarine::data::AlbumWithSongsId3),
    InsertAfterCurrentPLayed(submarine::data::AlbumWithSongsId3),
}

#[derive(Debug)]
pub enum AlbumViewIn {
    AlbumTracks,
}

#[derive(Debug)]
pub enum AlbumViewCmd {
    LoadedAlbum(Result<submarine::data::AlbumWithSongsId3, submarine::SubsonicError>),
}

#[relm4::component(pub)]
impl relm4::Component for AlbumView {
    type Init = AlbumViewInit;
    type Input = AlbumViewIn;
    type Output = AlbumViewOut;
    type Widgets = AlbumViewWidgets;
    type CommandOutput = AlbumViewCmd;

    fn init(
        init: Self::Init,
        root: &Self::Root,
        sender: relm4::ComponentSender<Self>,
    ) -> relm4::ComponentParts<Self> {
        let model = Self {
            cover: Cover::builder().launch(()).detach(),
            title: String::from("Unkonwn Title"),
            artist: None,
            info: String::new(),
            view: gtk::Viewport::default(),
            loaded_tracks: false,
            tracks: None,
        };

        let widgets = view_output!();
        model.cover.model().add_css_class_image("size100");

        //load albums
        sender.oneshot_command(async move {
            let client = Client::get().lock().unwrap().inner.clone().unwrap();
            match &init {
                AlbumViewInit::Child(child) => {
                    AlbumViewCmd::LoadedAlbum(client.get_album(&child.id).await)
                }
                AlbumViewInit::AlbumId3(album) => {
                    AlbumViewCmd::LoadedAlbum(client.get_album(&album.id).await)
                }
            }
        });

        relm4::ComponentParts { model, widgets }
    }

    view! {
        gtk::Box {
            add_css_class: "album-view",
            set_orientation: gtk::Orientation::Vertical,

            gtk::Box {
                set_spacing: 15,
                add_css_class: "album-view-info",

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
                        set_label: &format!("by {}", model.artist.as_deref().unwrap_or("Unkown Artist")),
                        set_halign: gtk::Align::Start,
                    },
                    gtk::Label {
                        #[watch]
                        set_label: &model.info,
                        set_halign: gtk::Align::Start,
                    }
                }
            },

            model.view.clone() {
                #[wrap(Some)]
                set_child = &gtk::Spinner {
                    add_css_class: "size50",
                    set_halign: gtk::Align::Center,
                    set_valign: gtk::Align::Center,
                    start: (),
                }
            },
        }
    }

    fn update(
        &mut self,
        msg: Self::Input,
        _sender: relm4::ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match msg {
            AlbumViewIn::AlbumTracks => {} //do nothing
        }
    }

    fn update_cmd(
        &mut self,
        msg: Self::CommandOutput,
        sender: relm4::ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match msg {
            AlbumViewCmd::LoadedAlbum(Err(e)) => {
                //TODO error handling
                tracing::error!("No child on server found");
            }
            AlbumViewCmd::LoadedAlbum(Ok(album)) => {
                // update dragSource
                let drop = Droppable::AlbumWithSongs(Box::new(album.clone()));
                let content = gtk::gdk::ContentProvider::for_value(&drop.to_value());
                let drag_src = gtk::DragSource::new();
                drag_src.set_actions(gtk::gdk::DragAction::MOVE);
                drag_src.set_content(Some(&content));
                self.cover.widget().add_controller(drag_src);

                //update self
                self.info = build_info_string(&album);
                self.title = album.base.name;
                self.artist = album.base.artist;
                self.cover.emit(CoverIn::LoadImage(album.base.cover_art));
                let tracks = AlbumTracks::builder()
                    .launch(album.song)
                    .forward(sender.input_sender(), |_| AlbumViewIn::AlbumTracks);
                self.view.set_child(Some(tracks.widget()));
                self.tracks = Some(tracks);
                self.loaded_tracks = true;
            }
        }
    }
}

fn build_info_string(child: &submarine::data::AlbumWithSongsId3) -> String {
    format!("TODO build info string")
}
