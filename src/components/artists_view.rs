use std::{cell::RefCell, rc::Rc};

use fuzzy_matcher::FuzzyMatcher;
use relm4::{
    gtk::{
        self, glib,
        prelude::{BoxExt, ButtonExt, OrientableExt, ToggleButtonExt, WidgetExt},
        FlowBoxChild,
    },
    loading_widgets::LoadingWidgets,
    view, Component, ComponentController,
};

use crate::{
    components::artist_element::{ArtistElement, ArtistElementIn, ArtistElementOut},
    subsonic::Subsonic,
};

#[derive(Debug)]
pub struct ArtistsView {
    artists: gtk::FlowBox,
    artist_list: Vec<relm4::Controller<ArtistElement>>,
}

#[derive(Debug)]
pub enum ArtistsViewOut {
    ClickedArtist(submarine::data::ArtistId3),
    DisplayToast(String),
    FavoriteClicked(String, bool),
}

#[derive(Debug)]
pub enum ArtistsViewIn {
    ArtistElement(ArtistElementOut),
    SearchChanged(String),
    ShowStarred(bool),
    Favorited(String, bool),
}

#[relm4::component(async, pub)]
impl relm4::component::AsyncComponent for ArtistsView {
    type Init = Rc<RefCell<Subsonic>>;
    type Input = ArtistsViewIn;
    type Output = ArtistsViewOut;
    type CommandOutput = ();

    fn init_loading_widgets(root: Self::Root) -> Option<LoadingWidgets> {
        view! {
            append = root.clone() -> gtk::Box {
                add_css_class: "artists-view",

                #[name(loading_box)]
                gtk::Box {
                    set_hexpand: true,
                    set_spacing: 30,
                    set_halign: gtk::Align::Center,
                    set_orientation: gtk::Orientation::Vertical,

                    gtk::Spinner {
                        add_css_class: "size100",
                        set_halign: gtk::Align::Center,
                        start: (),
                    }
                }
            }
        }

        // removes widget loading_box when function init finishes
        Some(LoadingWidgets::new(root, loading_box))
    }

    async fn init(
        init: Self::Init,
        root: Self::Root,
        sender: relm4::AsyncComponentSender<Self>,
    ) -> relm4::component::AsyncComponentParts<Self> {
        let mut model = Self {
            artists: gtk::FlowBox::default(),
            artist_list: vec![],
        };
        let widgets = view_output!();

        // add artists with cover and title
        for (i, artist) in init.borrow().artists().iter().enumerate() {
            let cover: relm4::Controller<ArtistElement> = ArtistElement::builder()
                .launch((init.clone(), artist.clone()))
                .forward(sender.input_sender(), ArtistsViewIn::ArtistElement);
            model.artists.insert(cover.widget(), i as i32);
            model.artist_list.insert(i, cover);
        }

        relm4::component::AsyncComponentParts { model, widgets }
    }

    view! {
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            set_hexpand: true,

            gtk::WindowHandle {
                gtk::CenterBox {
                    #[wrap(Some)]
                    set_center_widget = &gtk::Label {
                        add_css_class: "h2",
                        set_label: "Artists",
                        set_halign: gtk::Align::Center,
                    },

                    #[wrap(Some)]
                    set_end_widget = &gtk::Box {
                        gtk::ToggleButton {
                            set_icon_name: "non-starred-symbolic",
                            set_width_request: 50,
                            connect_clicked[sender] => move |btn| {
                                if btn.is_active() {
                                    btn.set_icon_name("starred-symbolic");
                                } else {
                                    btn.set_icon_name("non-starred-symbolic");
                                }
                                sender.input(Self::Input::ShowStarred(btn.is_active()));
                            },
                            set_tooltip_text: Some("Toggle showing favortited albums"),
                        },
                    }
                },
            },

            gtk::ScrolledWindow {
                set_hexpand: true,
                set_vexpand: true,

                #[wrap(Some)]
                set_child = &model.artists.clone() -> gtk::FlowBox {
                    set_valign: gtk::Align::Start,
                    set_row_spacing: 20,
                }
            }
        }
    }

    async fn update(
        &mut self,
        msg: Self::Input,
        sender: relm4::AsyncComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match msg {
            ArtistsViewIn::ArtistElement(msg) => match msg {
                ArtistElementOut::Clicked(id) => {
                    sender.output(ArtistsViewOut::ClickedArtist(id)).unwrap();
                }
                ArtistElementOut::DisplayToast(msg) => sender
                    .output(ArtistsViewOut::DisplayToast(msg))
                    .expect("sending failed"),
                ArtistElementOut::FavoriteClicked(id, state) => sender
                    .output(ArtistsViewOut::FavoriteClicked(id, state))
                    .expect("sending failed"),
            },
            ArtistsViewIn::SearchChanged(search) => {
                self.artists.set_filter_func(move |element| {
                    let title = get_title_of_flowboxchild(element);

                    //fuzzy matching
                    let matcher = fuzzy_matcher::skim::SkimMatcherV2::default();
                    let score = matcher.fuzzy_match(&title.text(), &search);
                    score.is_some()
                });
            }
            ArtistsViewIn::ShowStarred(false) => {
                self.artists.set_filter_func(move |_element| true);
            }
            ArtistsViewIn::ShowStarred(true) => {
                //TODO find a better way to match which element is starred
                //gather artists
                let artists: Vec<_> = self
                    .artist_list
                    .iter()
                    .map(|controller| controller.model().info().clone())
                    .collect();
                self.artists.set_filter_func(move |element| {
                    let title = get_title_of_flowboxchild(element);
                    for artist in &artists {
                        // if artist matches text check starred
                        if artist.name == title.text() {
                            return artist.starred.is_some();
                        }
                    }
                    true
                });
            }
            ArtistsViewIn::Favorited(id, state) => {
                for artist in &self.artist_list {
                    artist.emit(ArtistElementIn::Favorited(id.clone(), state));
                }
            }
        }
    }
}

fn get_title_of_flowboxchild(element: &FlowBoxChild) -> gtk::Label {
    use glib::object::Cast;
    let overlay = element.first_child().unwrap();
    let bo = overlay.first_child().unwrap();
    let button = bo.first_child().unwrap();
    let bo = button.first_child().unwrap();
    let cover = bo.first_child().unwrap();
    let title = cover.next_sibling().unwrap();
    title.downcast::<gtk::Label>().expect("unepected element")
}
