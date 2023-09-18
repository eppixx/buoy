// use crate::{client::Client, types::Id};

// pub struct AlbumsView {}

// pub enum AlbumsViewIn {
//     ClickedAlbum(Id),
// }

// pub enum AlbumsViewOut {
//     ClickedAlbum(Id),
// }

// #[relm4::component(async, pub)]
// impl relm4::component::AsyncComponent for AlbumsView {
//     type Input = AlbumsViewIn;
//     type Output = AlbumsViewOut;
//     type Init = ();
//     type CommandOutput = ();

//     async fn init(
//         _init: Self::Init,
//         root: Self::Root,
//         sender: relm4::AsyncComponentSender<Self>,
//     ) -> relm4::component::AsyncComponentParts<Self> {
//         let albums: Vec<submarine::data::ArtistId3> = {
//             let client = Client::get().lock().unwrap().inner.clone().unwrap();
//             let indexes: Vec<submarine::data::IndexId3> = client.get_album_list2(None).await.unwrap();
//             indexes.into_iter().flat_map(|i| i.artist).collect()
//         };
//     }

//     view! {}
// }
