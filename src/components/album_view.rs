use relm4::{
    gtk::{
        self,
        traits::{BoxExt, OrientableExt, WidgetExt},
    },
    Component, ComponentController,
};

use super::cover::Cover;
use crate::{client::Client, components::cover::CoverIn, types::Id};

#[derive(Debug)]
pub struct AlbumView {
    cover: relm4::Controller<Cover>,
    title: String,
    artist: Option<String>,
    info: String,
}

#[derive(Debug)]
pub enum AlbumViewOut {
    AppendAlbum(submarine::data::Child),
    InsertAfterCurrentPLayed(submarine::data::Child),
}

#[derive(Debug)]
pub enum AlbumViewIn {
    LoadChild(Id),
}

#[derive(Debug)]
pub enum AlbumViewCmd {
    LoadedChild(Option<submarine::data::Child>),
}

#[relm4::component(pub)]
impl relm4::Component for AlbumView {
    type Init = Id;
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
        };

        let widgets = view_output!();
        sender.input(AlbumViewIn::LoadChild(init));

        relm4::ComponentParts { model, widgets }
    }

    view! {
        gtk::Box {
            add_css_class: "album-view",
            set_orientation: gtk::Orientation::Vertical,

            gtk::Box {
                set_spacing: 15,

                model.cover.widget().clone() -> gtk::Box {
                },

                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_spacing: 5,

                    gtk::Label {
                        #[watch]
                        set_label: &model.title,
                    },

                    gtk::Label {
                        #[watch]
                        set_label: &model.artist.as_deref().unwrap_or("Unkonwn Artist"),
                    },

                    gtk::Label {
                        #[watch]
                        set_label: &model.info,
                    }
                }
            },

            gtk::Label {
                set_label: "track here",
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
            AlbumViewIn::LoadChild(id) => {
                sender.oneshot_command(async move {
                    let client = Client::get().lock().unwrap().inner.clone().unwrap();
                    match client.get_song(id.inner()).await {
                        Ok(child) => AlbumViewCmd::LoadedChild(Some(child)),
                        Err(_) => AlbumViewCmd::LoadedChild(None),
                    }
                });
            }
        }
    }

    fn update_cmd(
        &mut self,
        msg: Self::CommandOutput,
        _sender: relm4::ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match msg {
            AlbumViewCmd::LoadedChild(None) => {} //TODO error handling
            AlbumViewCmd::LoadedChild(Some(child)) => {
                self.info = build_info_string(&child);
                self.title = child.title;
                self.artist = child.artist;
                self.cover.emit(CoverIn::LoadImage(child.cover_art));
            }
        }
    }
}

fn build_info_string(child: &submarine::data::Child) -> String {
    format!("TODO build info string")
}
