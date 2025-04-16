use std::{cell::RefCell, io::Write, rc::Rc};

use gettextrs::gettext;
use relm4::{
    gtk::{
        self,
        prelude::{
            BoxExt, ButtonExt, DialogExt, FileChooserExt, FileExt, GtkApplicationExt, GtkWindowExt,
            OrientableExt, WidgetExt,
        },
    },
    RelmWidgetExt,
};

use crate::{
    app::App,
    client::Client,
    settings::Settings,
    subsonic::Subsonic,
    types::{Droppable, Id},
};

pub struct Download {}

impl Download {
    pub fn download(
        subsonic: &Rc<RefCell<Subsonic>>,
        sender: relm4::component::AsyncComponentSender<App>,
        drop: Droppable,
    ) {
        let download_len = drop.len(subsonic);
        if download_len >= Settings::get().lock().unwrap().download_warning_threshold {
            Self::show_size_warning(sender, drop, download_len);
        } else {
            Self::show_file_chooser(sender, drop);
        }
    }

    fn show_size_warning(
        sender: relm4::component::AsyncComponentSender<App>,
        drop: Droppable,
        download_len: usize,
    ) {
        // create dialog for large files
        let warning = format!(
            "{} {} {}\n{}",
            gettext("You're about to download"),
            download_len,
            gettext("songs"),
            gettext("This may take a while. Do want to proceed?"),
        );

        relm4::view! {
            download_warning = gtk::Window {
                set_modal: true,
                set_transient_for: Some(&relm4::main_application().windows()[0]),

                #[wrap(Some)]
                set_titlebar = &gtk::HeaderBar {
                    add_css_class: granite::STYLE_CLASS_FLAT,
                    add_css_class: granite::STYLE_CLASS_DEFAULT_DECORATION,
                    set_show_title_buttons: false,
                    set_visible: false,
                },

                gtk::WindowHandle {
                    gtk::Box {
                        set_margin_all: 15,
                        set_spacing: 20,

                        gtk::Image {
                            set_icon_name: Some("dialog-warning"),
                            set_pixel_size: 64,
                        },

                        gtk::Box {
                            set_orientation: gtk::Orientation::Vertical,
                            set_spacing: 20,

                            gtk::Box {
                                set_orientation: gtk::Orientation::Vertical,
                                set_spacing: 5,

                                gtk::Label {
                                    set_label: "Warning",
                                    add_css_class: granite::STYLE_CLASS_H2_LABEL,
                                    set_halign: gtk::Align::Start,
                                },

                                gtk::Label {
                                    set_label: &warning,
                                }
                            },
                            gtk::Box {
                                set_halign: gtk::Align::End,
                                set_spacing: 10,


                                append: cancel_btn = &gtk::Button {
                                    set_label: &gettext("Cancel"),
                                },

                                append: proceed_btn = &gtk::Button {
                                    set_label: &gettext("Download"),
                                }
                            }
                        }
                    }
                }
            }
        };

        let win = download_warning.clone();
        cancel_btn.connect_clicked(move |_btn| {
            win.close();
        });

        let win = download_warning.clone();
        proceed_btn.connect_clicked(move |_btn| {
            win.close();
            Self::show_file_chooser(sender.clone(), drop.clone());
        });

        download_warning.show();
    }

    fn show_file_chooser(sender: relm4::component::AsyncComponentSender<App>, drop: Droppable) {
        // create dialog
        let builder = gtk::FileChooserDialog::builder();
        let file_dialog = builder
            .name(gettext("Choose location for download"))
            .create_folders(true)
            .modal(true)
            .use_header_bar(1)
            .action(gtk::FileChooserAction::SelectFolder)
            .transient_for(&relm4::main_application().windows()[0])
            .build();
        file_dialog.add_button(&gettext("Choose folder"), gtk::ResponseType::Accept);
        file_dialog.add_button(&gettext("Cancel"), gtk::ResponseType::Cancel);
        file_dialog.show();

        // extract relevant info from Droppable
        let ids: Vec<(String, Id)> = match drop {
            Droppable::Child(child) => vec![(format!("{}.zip", child.title), Id::album(child.id))],
            Droppable::QueueSongs(_) => unreachable!(),
            Droppable::Album(id3) => vec![(format!("{}.zip", id3.name), Id::album(id3.id))],
            Droppable::AlbumChild(child) => {
                vec![(format!("{}.zip", child.title), Id::album(child.id))]
            }
            Droppable::AlbumWithSongs(album) => {
                vec![(format!("{}.zip", album.base.name), Id::album(album.base.id))]
            }
            Droppable::Artist(artist) => {
                vec![(format!("{}.zip", artist.name), Id::artist(artist.id))]
            }
            Droppable::ArtistWithAlbums(artist) => {
                vec![(
                    format!("{}.zip", artist.base.name),
                    Id::artist(artist.base.id),
                )]
            }
            Droppable::Playlist(list) => vec![(
                format!("{}.zip", list.base.name),
                Id::playlist(list.base.id),
            )],
            Droppable::Queue(list) => list
                .iter()
                .map(|t| {
                    (
                        format!("{} - {}.mp3", t.artist.clone().unwrap_or_default(), t.title),
                        Id::song(&t.id),
                    )
                })
                .collect(),
            Droppable::PlaylistItems(items) => items
                .iter()
                .map(|item| {
                    (
                        format!(
                            "{} - {} - {}.mp3",
                            item.uid,
                            item.child.artist.clone().unwrap_or_default(),
                            item.child.title
                        ),
                        Id::song(&item.child.id),
                    )
                })
                .collect(),
        };

        //TODO sanitize file names, e.g "/"

        // respond to action of dialog
        file_dialog.connect_response(move |dialog, response| {
            dialog.close();

            let path = match (response, dialog.file()) {
                (gtk::ResponseType::Accept, Some(folder)) => {
                    if let Some(path) = folder.path() {
                        path
                    } else {
                        return;
                    }
                }
                (_, _) => return,
            };

            Self::do_download(sender.clone(), path, ids.clone());
        });
    }

    fn do_download(
        sender: relm4::component::AsyncComponentSender<App>,
        path: std::path::PathBuf,
        ids: Vec<(String, Id)>,
    ) {
        let sender = sender.clone();
        let ids = ids.clone();
        let client = Client::get().unwrap();
        // new thread for downloading files on
        tokio::spawn(async move {
            for (name, id) in ids {
                sender.input(
                    <App as relm4::component::AsyncComponent>::Input::DisplayToast(format!(
                        "{} {} {name}",
                        gettext("Start downloading:"),
                        id.kind(),
                    )),
                );
                // download from server
                match client.download(id.inner()).await {
                    Err(e) => sender.input(
                        <App as relm4::component::AsyncComponent>::Input::DisplayToast(format!(
                            "{} {} {name} {}: {e}",
                            gettext("Download of"),
                            id.kind(),
                            gettext("failed")
                        )),
                    ),
                    Ok(buffer) => {
                        let mut path = path.clone();
                        path.push(&name);
                        let mut file = std::fs::OpenOptions::new()
                            .create(true)
                            .truncate(true)
                            .write(true)
                            .open(&path)
                            .unwrap();
                        // save file
                        if let Err(e) = file.write_all(&buffer) {
                            sender.input(
                                <App as relm4::component::AsyncComponent>::Input::DisplayToast(
                                    format!(
                                        "{} {} {name} {}: {e}",
                                        gettext("Saving of"),
                                        id.kind(),
                                        gettext("failed")
                                    ),
                                ),
                            );
                        }
                        sender.input(
                            <App as relm4::component::AsyncComponent>::Input::DisplayToast(
                                format!(
                                    "{}: {} {name}",
                                    gettext("Finished downloading"),
                                    id.kind()
                                ),
                            ),
                        );
                    }
                }
            }
        });
    }
}
