use relm4::{
    gtk::{
        self,
        traits::{BoxExt, WidgetExt},
    },
    Component, ComponentController,
};

use super::cover::{Cover, CoverIn};

#[derive(Debug)]
pub struct PlayInfo {
    covers: relm4::Controller<Cover>,
    pub cover: Option<String>,
    pub title: String,
    pub artist: Option<String>,
    pub album: Option<String>,
}

impl Default for PlayInfo {
    fn default() -> Self {
        Self {
            covers: Cover::builder().launch(()).detach(),
            title: String::from("Nothing is played currently"),
            cover: None,
            artist: None,
            album: None,
        }
    }
}

#[derive(Debug)]
pub enum PlayInfoIn {
    NewState(submarine::data::Child),
}

#[relm4::component(pub)]
impl relm4::SimpleComponent for PlayInfo {
    type Input = PlayInfoIn;
    type Output = ();
    type Init = Option<submarine::data::Child>;

    fn init(
        init: Self::Init,
        root: &Self::Root,
        sender: relm4::ComponentSender<Self>,
    ) -> relm4::ComponentParts<Self> {
        let model = Self::default();
        let widgets = view_output!();

        //init widget
        if let Some(child) = init {
            sender.input(PlayInfoIn::NewState(child));
        }
        model.covers.model().add_css_class_image("size150");

        relm4::ComponentParts { model, widgets }
    }

    view! {
        #[root]
        gtk::Box {
            add_css_class: "play-info",

            append = &model.covers.widget().clone() {
                add_css_class: "play-info-cover",
            },

            gtk::Label {
                add_css_class: "play-info-info",
                set_hexpand: true,
                #[watch]
                set_markup: &style_label(&model.title, model.artist.as_deref(), model.album.as_deref()),
            },
        }
    }

    fn update(&mut self, msg: Self::Input, _sender: relm4::ComponentSender<Self>) {
        match msg {
            PlayInfoIn::NewState(child) => {
                self.title = child.title;
                if let Some(artist) = child.artist {
                    self.artist = Some(artist);
                }
                if let Some(cover_id) = child.cover_art {
                    self.covers.emit(CoverIn::LoadImage(Some(cover_id)));
                }
            }
        }
    }
}

fn style_label(title: &str, artist: Option<&str>, album: Option<&str>) -> String {
    let mut result = format!(
        "<span font_size=\"x-large\" weight=\"bold\">{}</span>",
        title
    );
    if artist.is_some() || album.is_some() {
        result.push('\n');
    }
    if let Some(ref artist) = artist {
        result.push_str(&format!(
            "from <span font_size=\"large\" style=\"italic\">{}</span>",
            artist
        ));
    }
    if artist.is_some() || album.is_some() {
        result.push(' ');
    }
    if let Some(album) = album {
        result.push_str(&format!(
            "on <span font_size=\"large\" style=\"italic\">{}</span>",
            album
        ));
    }
    result
}
