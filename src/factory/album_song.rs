use relm4::gtk::{
    self, gdk, pango,
    prelude::{BoxExt, ToValue, WidgetExt},
};

use crate::{client::Client, common::convert_for_label, types::Droppable};

#[derive(Debug)]
pub struct AlbumSong {
    info: submarine::data::Child,
    track_number: gtk::Label,
    title: gtk::Label,
    artist: gtk::Label,
    length: gtk::Label,
    favorited: bool,
    fav_widget: gtk::Image,
    unfav_widget: gtk::Image,
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
    type Init = (submarine::data::Child, [gtk::SizeGroup; 5]);
    type Input = AlbumSongIn;
    type Output = AlbumSongOut;
    type ParentWidget = gtk::ListBox;
    // type ParentInput = AlbumTracksIn;
    type Widgets = AlbumSongWidgets;
    type CommandOutput = AlbumSongCmd;

    fn init_model(
        (child, groups): Self::Init,
        _index: &relm4::prelude::DynamicIndex,
        _sender: relm4::FactorySender<Self>,
    ) -> Self {
        let model = Self {
            info: child.clone(),
            track_number: gtk::Label::default(),
            title: gtk::Label::default(),
            artist: gtk::Label::default(),
            length: gtk::Label::default(),
            favorited: child.starred.is_some(),
            fav_widget: gtk::Image::default(),
            unfav_widget: gtk::Image::default(),
            drag_src: gtk::DragSource::default(),
        };

        // add widgets to group to make them the same size
        groups[0].add_widget(&model.track_number);
        groups[1].add_widget(&model.title);
        groups[2].add_widget(&model.artist);
        groups[3].add_widget(&model.length);
        groups[4].add_widget(&model.fav_widget);
        groups[4].add_widget(&model.unfav_widget);

        let src = Droppable::Child(Box::new(model.info.clone()));
        let content = gdk::ContentProvider::for_value(&src.to_value());
        model.drag_src.set_content(Some(&content));
        model.drag_src.set_actions(gdk::DragAction::MOVE);

        model
    }

    view! {
        gtk::ListBoxRow {
            add_css_class: "album-tracks-song",
            add_controller: self.drag_src.clone(),

            gtk::Box {
                set_spacing: 10,

                self.track_number.clone() -> gtk::Label {
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
                    set_label: self.info.artist.as_deref().unwrap_or("Unknown Artist"),
                    set_widget_name: "artist",
                },
                self.length.clone() -> gtk::Label {
                    set_halign: gtk::Align::Start,
                    set_label: &convert_for_label(self.info.duration.unwrap_or(0) as i64 * 1000),
                },
                if self.favorited {
                    self.fav_widget.clone() -> gtk::Image {
                        set_icon_name: Some("starred"),
                    }
                } else {
                    self.unfav_widget.clone() -> gtk::Image {
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
                    let client = Client::get().unwrap();
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

    fn update_cmd(&mut self, msg: Self::CommandOutput, _sender: relm4::FactorySender<Self>) {
        match msg {
            AlbumSongCmd::Favorited(Err(e)) => {} //TODO error handling
            AlbumSongCmd::Favorited(Ok(state)) => self.favorited = state,
        }
    }
}
