use relm4::gtk::{
    self,
    traits::{OrientableExt, WidgetExt},
};

use crate::types::Id;

#[derive(Debug, Default)]
pub struct Artists {}

#[derive(Debug)]
pub enum ArtistsOut {
    ChangeTo(Id),
}

#[relm4::component(pub)]
impl relm4::SimpleComponent for Artists {
    type Input = ();
    type Output = ArtistsOut;
    type Init = ();

    fn init(
        _init: Self::Init,
        root: &Self::Root,
        sender: relm4::ComponentSender<Self>,
    ) -> relm4::ComponentParts<Self> {
        let model = Artists::default();
        let widgets = view_output!();

        relm4::ComponentParts { model, widgets }
    }

    view! {
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,

            gtk::Label {
                set_label: "usdfsdf",
            },
            gtk::Label {
                set_label: "sdfs",
            }
        }
    }
}
