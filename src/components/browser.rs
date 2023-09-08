use relm4::gtk::{self, traits::{ButtonExt, OrientableExt, WidgetExt, EditableExt}};

use crate::types::Id;

#[derive(Debug, Default)]
pub struct Browser {
    content: gtk::Stack,
    // back_btn: 
}

#[derive(Debug)]
pub enum BrowserInput {
    SearchChanged(String),
}

#[relm4::component(pub)]
impl relm4::SimpleComponent for Browser {
    type Input = BrowserInput;
    type Output = ();
    type Init = Vec<Id>;

    fn init(
        path: Self::Init,
        root: &Self::Root,
        sender: relm4::ComponentSender<Self>,
    ) -> relm4::ComponentParts<Self> {
        let model = Browser::default();
        let widgets = view_output!();

        relm4::ComponentParts { model, widgets }

    }

    view! {
        gtk::Box {
            add_css_class: "browser",
            set_orientation: gtk::Orientation::Vertical,

            gtk::Box {
                add_css_class: "pathbar",

                gtk::Button {
                    set_icon_name: "go-home-symbolic",
                    set_visible: false,
                },
                gtk::Button {
                    set_icon_name: "go-previous-symbolic",
                    set_label: "Back",
                    set_visible: false,
                },
                //TODO add path
                gtk::Label {
                    set_hexpand: true,
                },
                gtk::SearchEntry {
                    set_placeholder_text: Some("Search..."),
                    grab_focus: (),
                    connect_search_changed[sender] => move |w| {
                        sender.input(BrowserInput::SearchChanged(w.text().to_string()));
                    }
                }
            },

            //TODO implement stack of view here
            gtk::Label {
                set_label: "sdfdsfdsf",
            }
        }
    }

    fn update(&mut self, msg: Self::Input, _sender: relm4::ComponentSender<Self>) {
        match msg {
            BrowserInput::SearchChanged(search) => {
                //TODO send to active view
                tracing::warn!("new search {search}");
            }
        }
    }
}
