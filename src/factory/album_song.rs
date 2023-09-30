use relm4::{
    gtk::{
        self, gdk, pango,
        prelude::ToValue,
        traits::{BoxExt, WidgetExt},
    },
    Component, ComponentController,
};

use crate::{
    client::Client,
    components::{
        album_tracks::AlbumTracksIn,
        cover::{Cover, CoverIn},
        seekbar,
    },
    types::Droppable,
};

#[derive(Debug)]
pub struct AlbumSong {
    info: submarine::data::Child,
    favorited: bool,
    title: gtk::Label,
    artist: gtk::Label,
    drag_src: gtk::DragSource,
}

#[derive(Debug)]
pub enum AlbumSongIn {
    Favorited,
}

#[derive(Debug)]
pub enum AlbumSongOut {
    //
}

#[derive(Debug)]
pub enum AlbumSongCmd {
    Favorited(Result<bool, submarine::SubsonicError>),
}

#[relm4::factory(pub)]
impl relm4::factory::FactoryComponent for AlbumSong {
    type Init = submarine::data::Child;
    type Input = AlbumSongIn;
    type Output = AlbumSongOut;
    type ParentWidget = gtk::ListBox;
    type ParentInput = AlbumTracksIn;
    type Widgets = AlbumSongWidgets;
    type CommandOutput = AlbumSongCmd;

    fn init_model(
        init: Self::Init,
        _index: &relm4::prelude::DynamicIndex,
        _sender: relm4::FactorySender<Self>,
    ) -> Self {
        let model = Self {
            info: init.clone(),
            favorited: init.starred.is_some(),
            title: gtk::Label::default(),
            artist: gtk::Label::default(),
            drag_src: gtk::DragSource::default(),
        };

        //TODO fix layout
        // let layout = gtk::ConstraintLayout::default();
        // let vfl = "V:-[title]-[artist(==title)]";
        // layout
        //     .add_constraints_from_description(
        //         vec![vfl],
        //         1,
        //         1,
        //         vec![("title", &model.title), ("artist", &model.artist)],
        //     )
        //     .unwrap();

        let src = Droppable::Child(Box::new(model.info.clone()));
        let content = gdk::ContentProvider::for_value(&src.to_value());
        model.drag_src.set_content(Some(&content));
        model.drag_src.set_actions(gdk::DragAction::MOVE);

        model
    }

    view! {
        gtk::ListBoxRow {
            add_css_class: "album-song",
            add_controller: self.drag_src.clone(),

            gtk::Box {
                set_spacing: 10,

                gtk::Label {
                    set_label: &self.info.track.map_or(String::from("-"), |t| t.to_string()),
                },

                self.title.clone() -> gtk::Label {
                    set_hexpand: true,
                    set_halign: gtk::Align::Start,
                    set_ellipsize: pango::EllipsizeMode::End,
                    set_width_chars: 3,
                    set_label: &self.info.title,
                    set_widget_name: "title",
                },
                self.artist.clone() -> gtk::Label {
                    set_hexpand: true,
                    set_halign: gtk::Align::Start,
                    set_ellipsize: pango::EllipsizeMode::End,
                    set_width_chars: 3,
                    set_label: &self.info.artist.as_deref().unwrap_or("Unknown Artist"),
                    set_widget_name: "artist",
                },
                gtk::Label {
                    set_halign: gtk::Align::Start,
                    set_label: &seekbar::convert_for_label(self.info.duration.unwrap_or(0) as i64 * 1000),
                },
                if self.favorited {
                    gtk::Image {
                        set_icon_name: Some("starred"),
                    }
                } else {
                    gtk::Image {
                        set_icon_name: Some("non-starred"),
                    }
                },
            }
        }
    }

    fn update(&mut self, msg: Self::Input, sender: relm4::FactorySender<Self>) {
        match msg {
            AlbumSongIn::Favorited => {
                let id = self.info.id.clone();
                let favorite = self.favorited;
                sender.oneshot_command(async move {
                    let client = Client::get().lock().unwrap().inner.clone().unwrap();
                    let empty: Vec<&str> = vec![];

                    let result = match favorite {
                        true => client.star(vec![id], empty.clone(), empty).await,
                        false => client.unstar(vec![id], empty.clone(), empty).await,
                    };
                    AlbumSongCmd::Favorited(result.map(|_| !favorite))
                });
            }
        }
    }

    fn forward_to_parent(output: Self::Output) -> Option<Self::ParentInput> {
        match output {
						//
        }
        None
    }

    fn update_cmd(&mut self, msg: Self::CommandOutput, sender: relm4::FactorySender<Self>) {
        match msg {
            AlbumSongCmd::Favorited(Err(e)) => {} //TODO error handling
            AlbumSongCmd::Favorited(Ok(state)) => self.favorited = state,
        }
    }
}
