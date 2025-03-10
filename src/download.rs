use std::io::Write;

use gettextrs::gettext;
use relm4::gtk::{
    self,
    prelude::{
        DialogExt, DialogExtManual, FileChooserExt, FileExt, GtkApplicationExt, GtkWindowExt,
        WidgetExt,
    },
};

use crate::{
    app::App,
    client::Client,
    types::{Droppable, Id},
};

pub struct Download {}

impl Download {
    pub fn download(sender: relm4::component::AsyncComponentSender<App>, drop: Droppable) {
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
        file_dialog.add_buttons(&[
            (&gettext("Choose folder"), gtk::ResponseType::Accept),
            (&gettext("Cancel"), gtk::ResponseType::Cancel),
        ]);
        file_dialog.show();

        // extract relevant info from Droppable
        let ids: Vec<(String, Id)> = match drop {
            Droppable::Child(child) => vec![(format!("{}.zip", child.title), Id::album(child.id))],
            Droppable::QueueSongs(_) => unreachable!(),
            Droppable::QueueSong(_) => unreachable!(), //TODO
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
            Droppable::PlaylistItems(_items) => todo!(),
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
                            <App as relm4::component::AsyncComponent>::Input::DisplayToast(
                                format!(
                                    "{} {} {name} {}: {e}",
                                    gettext("Download of"),
                                    id.kind(),
                                    gettext("failed")
                                ),
                            ),
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
        });
    }
}
