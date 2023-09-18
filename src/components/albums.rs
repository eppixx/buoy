use relm4::component::AsyncComponent;
use relm4::factory::{AsyncFactoryVecDeque, FactoryVecDeque};
use relm4::gtk::gdk_pixbuf::Pixbuf;
use relm4::gtk::pango;
use relm4::gtk::traits::{BoxExt, OrientableExt, WidgetExt};
use relm4::loading_widgets::LoadingWidgets;
use relm4::prelude::{DynamicIndex, FactoryComponent};
use relm4::{component::AsyncComponentParts, factory::AsyncFactoryComponent, AsyncComponentSender};
use relm4::{gtk, AsyncFactorySender, FactorySender};

use crate::client::Client;
use crate::types::Id;

#[derive(Debug)]
pub struct Albums {
    // albums: AsyncFactoryVecDeque<AlbumElement>,
    albums: FactoryVecDeque<AlbumElement>,
}

#[derive(Debug)]
pub enum AlbumsOut {
    Clicked(Id),
}

#[derive(Debug)]
pub enum AlbumsIn {
    Clicked(Id),
    AddAlbum(submarine::data::Child),
}

#[relm4::component(async, pub)]
impl AsyncComponent for Albums {
    type Input = AlbumsIn;
    type Output = AlbumsOut;
    type Init = ();
    type CommandOutput = ();

    async fn init(
        _init: Self::Init,
        root: Self::Root,
        sender: AsyncComponentSender<Self>,
    ) -> AsyncComponentParts<Self> {
        let albumss: Vec<submarine::data::Child> = {
            let client = crate::Client::get().lock().unwrap().inner.clone().unwrap();
            //TODO fetch all albums
            client
                .get_album_list(
                    submarine::api::get_album_list::Order::AlphabeticalByName,
                    Some(500),
                    Some(0),
                    None::<&str>,
                )
                .await
                .unwrap()
        };
        println!("fetched albums");
        let mut album_list = FactoryVecDeque::new(gtk::Box::default(), sender.input_sender());
        // let mut album_list =
        //     AsyncFactoryVecDeque::new(gtk::FlowBox::default(), sender.input_sender());
        // {
        //     let mut guard = album_list.guard();
        //     for album in albums {
        //         guard.push_back(album);
        //     }
        // }

        let mut model = Albums { albums: album_list };
        let albums = model.albums.widget();
        let widgets = view_output!();
        // {

        for album in albumss {
            // sender.input(AlbumsIn::AddAlbum(album));
            // println!("add album");
            let mut guard = model.albums.guard();
            guard.push_back(album);
        }
        // }

        relm4::component::AsyncComponentParts { model, widgets }
    }

    fn init_loading_widgets(root: &mut Self::Root) -> Option<LoadingWidgets> {
        relm4::view! {
            #[local_ref]
            root {
                add_css_class: "albums",

                // gtk::Box {
                //     gtk::Label {
                //         set_label: "sdfsdf",
                //     },

                    #[name = "loading_box"]
                    append = &gtk::Box {
                        set_hexpand: true,
                        set_spacing: 30,
                        set_halign: gtk::Align::Center,
                        set_orientation: gtk::Orientation::Vertical,

                        gtk::Label {
                            add_css_class: "h2",
                            set_label: "Loading albums",
                        },

                        gtk::Spinner {
                            add_css_class: "size100",
                            set_halign: gtk::Align::Center,
                            start: (),
                        }
                    }
                // },
            }
        }

        Some(LoadingWidgets::new(root, loading_box))
    }

    view! {
        gtk::Box {
            set_hexpand: true,
            set_orientation: gtk::Orientation::Vertical,

            gtk::Label {
                set_label: "sdf",
            },

            #[name = "reveal_after_init"]
            gtk::Label {
                set_visible: false,
                add_css_class: "h2",
                set_halign: gtk::Align::Center,
                set_label: "Artists",
            },

            gtk::ScrolledWindow {
                set_vexpand: true,
                set_hexpand: true,

                #[wrap(Some)]
                set_child = &model.albums.widget().clone() {
                },
            }
        }
    }

    async fn update(
        &mut self,
        msg: Self::Input,
        sender: AsyncComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match msg {
            AlbumsIn::AddAlbum(child) => {
                let mut guard = self.albums.guard();
                guard.push_back(child);
            }
            AlbumsIn::Clicked(id) => sender.output(AlbumsOut::Clicked(id)).unwrap(),
        }
    }
}

#[derive(Debug)]
struct AlbumElement {
    info: submarine::data::Child,
    image: gtk::Image,
}

#[derive(Debug)]
enum AlbumElementOut {
    Clicked(Id),
}

#[relm4::factory]
impl FactoryComponent for AlbumElement {
    type Input = ();
    type Output = AlbumElementOut;
    type Init = submarine::data::Child;
    type CommandOutput = ();
    type ParentInput = AlbumsIn;
    type ParentWidget = gtk::Box;

    fn init_model(init: Self::Init, _index: &DynamicIndex, _sender: FactorySender<Self>) -> Self {
        let model = Self {
            info: init,
            image: gtk::Image::from_icon_name("go-home"),
        };
        // println!("sdf {:?}", model.info.cover_art);
        // {
        //     let client = Client::get().lock().unwrap().inner.clone().unwrap();
        //     if let Some(id) = &model.info.cover_art {
        //         let buffer: Vec<u8> = client.get_cover_art(id, Some(300)).await.unwrap();
        //         let bytes = gtk::glib::Bytes::from(&buffer);
        //         let stream = gtk::gio::MemoryInputStream::from_bytes(&bytes);
        //         match Pixbuf::from_stream(&stream, gtk::gio::Cancellable::NONE) {
        //             Ok(pixbuf) => model.image.set_from_pixbuf(Some(&pixbuf)),
        //             _ => {} // TODO replace with stock image
        //         }
        //     }
        // }
        model
    }

    // fn init_loading_widgets(root: &mut Self::Root) -> Option<LoadingWidgets> {
    //     relm4::view! {
    //         #[local_ref]
    //         root {
    //             // add_css_class: "albums-element",

    //                             #[name = "spinner"]
    //                             gtk::Label {
    //                                     set_label: "sdf",
    //                             }

    //             // #[name(spinner)]
    //             // gtk::Box {
    //             //     add_css_class: "card",
    //             //     add_css_class: "size150",
    //             //     set_halign: gtk::Align::Center,

    //             //     gtk::Spinner {
    //             //         add_css_class: "size50",
    //             //         set_hexpand: true,
    //             //         set_halign: gtk::Align::Center,
    //             //         start: (),
    //             //     }
    //             // }
    //         }
    //     }
    //     Some(LoadingWidgets::new(root, spinner))
    // }

    view! {
        gtk::Box {
            gtk::Label {
                set_label: "test",
            },
            append = &self.image.clone() -> gtk::Image {}
            // add_css_class: "albums-element-cover",
            // set_orientation: gtk::Orientation::Vertical,
            // set_spacing: 5,

            // #[name = "actual_cover"]
            // gtk::Box {
            //     set_visible: false,
            //     set_halign: gtk::Align::Center,

            //     append = &self.image.clone() -> gtk::Image {
            //         add_css_class: "card",
            //         add_css_class: "size150",
            //     },
            // },
            // gtk::Label {
            //     set_text: &self.info.title,
            //     set_ellipsize: pango::EllipsizeMode::End,
            //     set_max_width_chars: 15,
            //     set_size_request: (150, -1),
            // },
            // gtk::Label {
            //     set_text: "Artist title",
            //     set_ellipsize: pango::EllipsizeMode::End,
            //     set_max_width_chars: 15,
            //     set_size_request: (150, -1),
            // }
        }
    }
}

//////////////////
// #[derive(Debug)]
// struct AlbumElement {
//     info: submarine::data::Child,
//     image: gtk::Image,
// }

// #[derive(Debug)]
// enum AlbumElementOut {
//     Clicked(Id),
// }

// #[relm4::factory(async, pub)]
// impl AsyncFactoryComponent for AlbumElement {
//     type Input = ();
//     type Output = AlbumElementOut;
//     type Init = submarine::data::Child;
//     type CommandOutput = ();
//     type ParentInput = AlbumsIn;
//     type ParentWidget = gtk::FlowBox;

//     async fn init_model(
//         init: Self::Init,
//         _index: &DynamicIndex,
//         _sender: AsyncFactorySender<Self>,
//     ) -> Self {
//         let model = Self {
//             info: init,
//             image: gtk::Image::default(),
//         };
//         // {
//         //     let client = Client::get().lock().unwrap().inner.clone().unwrap();
//         //     if let Some(id) = &model.info.cover_art {
//         //         let buffer: Vec<u8> = client.get_cover_art(id, Some(300)).await.unwrap();
//         //         let bytes = gtk::glib::Bytes::from(&buffer);
//         //         let stream = gtk::gio::MemoryInputStream::from_bytes(&bytes);
//         //         match Pixbuf::from_stream(&stream, gtk::gio::Cancellable::NONE) {
//         //             Ok(pixbuf) => model.image.set_from_pixbuf(Some(&pixbuf)),
//         //             _ => {} // TODO replace with stock image
//         //         }
//         //     }
//         // }
//         model
//     }

//     fn init_loading_widgets(root: &mut Self::Root) -> Option<LoadingWidgets> {
//         relm4::view! {
//             #[local_ref]
//             root {
//                 add_css_class: "albums-element",

//                 #[name(spinner)]
//                 gtk::Box {
//                     add_css_class: "card",
//                     add_css_class: "size150",
//                     set_halign: gtk::Align::Center,

//                     gtk::Spinner {
//                         add_css_class: "size50",
//                         set_hexpand: true,
//                         set_halign: gtk::Align::Center,
//                         start: (),
//                     }
//                 }
//             }
//         }
//         Some(LoadingWidgets::new(root, spinner))
//     }

//     view! {
//         gtk::Box {
//             // add_css_class: "albums-element-cover",
//             // set_orientation: gtk::Orientation::Vertical,
//             // set_spacing: 5,

//             // #[name = "actual_cover"]
//             // gtk::Box {
//             //     set_visible: false,
//             //     set_halign: gtk::Align::Center,

//             //     append = &self.image.clone() -> gtk::Image {
//             //         add_css_class: "card",
//             //         add_css_class: "size150",
//             //     },
//             // },
//             // gtk::Label {
//             //     set_text: &self.info.title,
//             //     set_ellipsize: pango::EllipsizeMode::End,
//             //     set_max_width_chars: 15,
//             //     set_size_request: (150, -1),
//             // },
//             // gtk::Label {
//             //     set_text: "Artist title",
//             //     set_ellipsize: pango::EllipsizeMode::End,
//             //     set_max_width_chars: 15,
//             //     set_size_request: (150, -1),
//             // }
//         }
//     }
// }
