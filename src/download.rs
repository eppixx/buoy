use std::io::Write;

use relm4::gtk::{
    self,
    prelude::{
        DialogExt, DialogExtManual, FileChooserExt, FileExt, GtkApplicationExt, GtkWindowExt,
        WidgetExt,
    },
};

use crate::{app::App, client::Client, types::Droppable};

pub enum DownloadStructure {
    ArtistAlbums,
    Albums,
    AlbumZip,
    Flat,
}

pub struct Download {}

impl Download {
    pub fn download(sender: relm4::component::AsyncComponentSender<App>, drop: Droppable) {
        match drop {
            Droppable::Child(child) => {
                let builder = gtk::FileChooserDialog::builder();
                let file_dialog = builder
                    .name("Choose location for download")
                    .create_folders(true)
                    .modal(true)
                    .use_header_bar(1)
                    .action(gtk::FileChooserAction::SelectFolder)
                    .transient_for(&relm4::main_application().windows()[0])
                    .build();
                file_dialog.add_buttons(&[
                    ("Choose folder", gtk::ResponseType::Accept),
                    ("Cancel", gtk::ResponseType::Cancel),
                ]);
                file_dialog.show();
                file_dialog.connect_response(move |dialog, response| {
                    dialog.close();

                    let mut path = match (response, dialog.file()) {
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
                    let child = child.clone();
                    let client = Client::get().unwrap();
                    // new thread for downloading file on
                    tokio::spawn(async move {
                        sender.input(
                            <App as relm4::component::AsyncComponent>::Input::DisplayToast(format!(
                                "start downloading: {}",
                                child.title
                            )),
                        );
                        match client.download(child.id).await {
                            Err(e) => sender.input(
                                <App as relm4::component::AsyncComponent>::Input::DisplayToast(
                                    format!("Download of Album {} failed: {e}", child.title),
                                ),
                            ),
                            Ok(buffer) => {
                                path.push(format!("{}.zip", child.title));
                                let mut file = std::fs::OpenOptions::new()
                                    .create(true)
                                    .truncate(true)
                                    .write(true)
                                    .open(&path)
                                    .unwrap();
                                if let Err(e) = file.write_all(&buffer) {
                                    sender.input(
                                        <App as relm4::component::AsyncComponent>::Input::DisplayToast(
                                            format!("saving of Album {} failed: {e}", child.title),
                                        ),
                                    );
                                }
                                sender.input(
                                    <App as relm4::component::AsyncComponent>::Input::DisplayToast(
                                        format!("finished downloading: {}", child.title),
                                    ),
                                );
                            }
                        }
                    });
                });
            }
            Droppable::Album(id3) => println!("sdfsdfsd"),
            _ => {}
        }
    }
}
