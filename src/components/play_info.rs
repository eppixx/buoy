use relm4::gtk::{self, traits::WidgetExt};

#[derive(Debug)]
pub struct PlayInfoModel {
    pub cover: Option<String>,
    pub title: String,
    pub artist: Option<String>,
    pub album: Option<String>,
}

impl Default for PlayInfoModel {
    fn default() -> Self {
        Self {
            title: String::from("Nothing is played currently"),
            cover: None,
            artist: None,
            album: None,
        }
    }
}

#[relm4::component(pub)]
impl relm4::SimpleComponent for PlayInfoModel {
    type Input = PlayInfoModel;
    type Output = ();
    type Init = Option<PlayInfoModel>;

    fn init(
        init: Self::Init,
        root: &Self::Root,
        _sender: relm4::ComponentSender<Self>,
    ) -> relm4::ComponentParts<Self> {
        let model = if let Some(init) = init {
            init
        } else {
            Self::default()
        };

        let widgets = view_output!();

        relm4::ComponentParts { model, widgets }
    }

    view! {
        #[root]
        gtk::Box {
            add_css_class: "play-info",

            gtk::Image {
                add_css_class: "play-info-cover",
                set_icon_name: Some("folder-music-symbolic"),
            },

            gtk::Label {
                add_css_class: "play-info-info",
                #[watch]
                set_markup: &style_label(&model.title, model.artist.as_deref(), model.album.as_deref()),
            },
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
