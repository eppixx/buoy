use gtk::prelude::{BoxExt, ButtonExt, OrientableExt};
use relm4::{
    factory::FactoryVecDeque,
    gtk::gdk,
    gtk::{
        self,
        traits::{ListBoxRowExt, WidgetExt},
    },
    prelude::DynamicIndex,
    ComponentController, ComponentParts, ComponentSender, RelmWidgetExt,
};

use crate::{
    client::Client,
    components::{
        sequence_button::{SequenceButton, SequenceButtonOut},
        sequence_button_impl::{repeat::Repeat, shuffle::Shuffle},
    },
    factory::queue_song::QueueSong,
    play_state::PlayState,
    types::Droppable,
};

#[derive(Debug)]
pub struct Queue {
    songs: FactoryVecDeque<QueueSong>,
    loading_queue: bool,
    playing_index: Option<DynamicIndex>,
    remove_items: gtk::Button,
    clear_items: gtk::Button,
    last_selected: Option<DynamicIndex>,
    shuffle: relm4::Controller<SequenceButton<Shuffle>>,
    repeat: relm4::Controller<SequenceButton<Repeat>>,
}

impl Queue {
    fn update_clear_btn_sensitivity(&mut self) {
        self.clear_items
            .set_sensitive(!self.songs.guard().is_empty());
    }
}

#[derive(Debug)]
pub enum QueueIn {
    Activated(DynamicIndex, Box<submarine::data::Child>),
    Clicked(DynamicIndex),
    ShiftClicked(DynamicIndex),
    Clear,
    Remove,
    MoveAbove {
        src: DynamicIndex,
        dest: DynamicIndex,
    },
    MoveBelow {
        src: DynamicIndex,
        dest: DynamicIndex,
    },
    NewState(PlayState),
    SomeIsSelected(bool),
    ToggleShuffle,
    RepeatPressed,
    PlayNext,
    PlayPrevious,
    LoadPlayQueue,
    DropAbove {
        src: Vec<submarine::data::Child>,
        dest: DynamicIndex,
    },
    DropBelow {
        src: Vec<submarine::data::Child>,
        dest: DynamicIndex,
    },
    Append(Droppable),
}

#[derive(Debug)]
pub enum QueueOut {
    Play(Box<submarine::data::Child>),
    Stop,
}

#[derive(Debug)]
pub enum QueueCmd {
    FetchedQueue(Option<submarine::data::PlayQueue>),
    FetchedAppendItems(Result<Vec<submarine::data::Child>, submarine::SubsonicError>),
}

#[relm4::component(pub)]
impl relm4::Component for Queue {
    type Input = QueueIn;
    type Output = QueueOut;
    type Init = ();
    type Widgets = QueueWidgets;
    type CommandOutput = QueueCmd;

    fn init(
        _init: Self::Init,
        root: &Self::Root,
        sender: relm4::ComponentSender<Self>,
    ) -> relm4::ComponentParts<Self> {
        let shuffle: relm4::Controller<SequenceButton<Shuffle>> =
            SequenceButton::<Shuffle>::builder()
                .launch(Shuffle::Sequential)
                .forward(sender.input_sender(), |msg| match msg {
                    SequenceButtonOut::Clicked => QueueIn::ToggleShuffle,
                });
        let repeat: relm4::Controller<SequenceButton<Repeat>> = SequenceButton::<Repeat>::builder()
            .launch(Repeat::Normal)
            .forward(sender.input_sender(), |msg| match msg {
                SequenceButtonOut::Clicked => QueueIn::RepeatPressed,
            });

        let mut model = Queue {
            songs: FactoryVecDeque::new(gtk::ListBox::default(), sender.input_sender()),
            loading_queue: false,
            playing_index: None,
            remove_items: gtk::Button::new(),
            clear_items: gtk::Button::new(),
            last_selected: None,
            shuffle,
            repeat,
        };

        //init queue
        sender.input(QueueIn::LoadPlayQueue);

        let widgets = view_output!();

        model.update_clear_btn_sensitivity();

        ComponentParts { model, widgets }
    }

    view! {
        gtk::Box {
            add_css_class: "queue",
            set_orientation: gtk::Orientation::Vertical,
            set_spacing: 5,

            gtk::ScrolledWindow {
                set_vexpand: true,

                if model.loading_queue {
                    gtk::Box {
                        set_hexpand: true,
                        set_orientation: gtk::Orientation::Vertical,
                        set_spacing: 20,

                        gtk::Label {
                            add_css_class: "h3",
                            set_label: "Loading queue",
                        },
                        gtk::Spinner {
                            add_css_class: "size100",
                            start: (),
                        }
                    }
                } else if model.songs.is_empty() {
                    gtk::Label {
                        add_css_class: "h3",
                        set_label: "Queue is empty\nDrop music here",

                        add_controller = gtk::DropTarget {
                            set_actions: gdk::DragAction::MOVE,
                            set_types: &[<Droppable as gtk::prelude::StaticType>::static_type()],
                            connect_drop[sender] => move |_target, value, _x, _y| {
                                if let Ok(drop) = value.get::<Droppable>() {
                                    sender.input(QueueIn::Append(drop));
                                }
                                true
                            },
                        }
                    }
                } else {
                    model.songs.widget().clone() -> gtk::ListBox {
                        set_selection_mode: gtk::SelectionMode::Multiple,

                        connect_selected_rows_changed[sender] => move |widget| {
                            sender.input(QueueIn::SomeIsSelected(!widget.selected_rows().is_empty()));
                        },
                    }
                },
            },

            gtk::ActionBar {
                pack_start = &model.shuffle.widget().clone() {},
                pack_start = &model.repeat.widget().clone() {},

                pack_end = &gtk::Button {
                    set_icon_name: "document-new-symbolic",
                    set_tooltip: "add queue to playlists",
                    set_focus_on_click: false,
                    // TODO add new playlist
                },

                pack_end = &gtk::Label {
                    add_css_class: "destructive-button-spacer",
                },

                pack_end = &model.remove_items.clone() {
                    set_icon_name: "list-remove-symbolic",
                    set_tooltip: "remove song from queue",
                    set_sensitive: false,
                    set_focus_on_click: false,
                    connect_clicked => QueueIn::Remove,
                },

                pack_end = &gtk::Label {
                    add_css_class: "destructive-button-spacer",
                },

                pack_end = &model.clear_items.clone() {
                    set_icon_name: "user-trash-symbolic",
                    set_tooltip: "clear queue",
                    set_sensitive: false,
                    set_focus_on_click: false,
                    connect_clicked => QueueIn::Clear,
                },
            }
        }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>, _root: &Self::Root) {
        match msg {
            QueueIn::Activated(index, info) => {
                // remove play icon and selection from other indexes
                for (_i, song) in self
                    .songs
                    .iter()
                    .enumerate()
                    .filter(|(i, _)| i != &index.current_index())
                {
                    self.songs.widget().unselect_row(song.root_widget());
                    song.new_play_state(&PlayState::Stop);
                }

                self.playing_index = Some(index);
                sender.output(QueueOut::Play(Box::new(*info))).unwrap();
            }
            QueueIn::Clicked(index) => {
                for (_i, song) in self
                    .songs
                    .iter()
                    .enumerate()
                    .filter(|(i, _)| i != &index.current_index())
                {
                    self.songs.widget().unselect_row(song.root_widget());
                }
                self.last_selected = Some(index.clone());
            }
            QueueIn::ShiftClicked(target) => {
                if let Some(index) = &self.last_selected {
                    let (lower, bigger) = if index.current_index() < target.current_index() {
                        (index.clone(), target)
                    } else {
                        (target, index.clone())
                    };

                    let items: Vec<gtk::ListBoxRow> = self
                        .songs
                        .iter()
                        .enumerate()
                        .filter_map(|(i, s)| {
                            if i >= lower.current_index() && i <= bigger.current_index() {
                                return Some(s.root_widget().clone());
                            }
                            None
                        })
                        .collect();
                    for item in items {
                        self.songs.widget().select_row(Some(&item));
                    }
                } else {
                    self.last_selected = Some(target)
                }
            }
            QueueIn::Append(id) => {
                let songs: Vec<submarine::data::Child> = match id {
                    Droppable::Child(c) => vec![*c],
                    Droppable::AlbumWithSongs(album) => album.song,
                    Droppable::Artist(artist) => {
                        sender.oneshot_command(async move {
                            let client = Client::get().lock().unwrap().inner.clone().unwrap();
                            let artist_with_albums = match client.get_artist(artist.id).await {
                                Err(e) => return QueueCmd::FetchedAppendItems(Err(e)),
                                Ok(artist) => artist,
                            };

                            let mut result = vec![];
                            for album in artist_with_albums.album {
                                match client.get_album(album.id).await {
                                    Ok(mut album) => result.append(&mut album.song),
                                    Err(e) => return QueueCmd::FetchedAppendItems(Err(e)),
                                }
                            }
                            QueueCmd::FetchedAppendItems(Ok(result))
                        });
                        return;
                    }
                    Droppable::ArtistWithAlbums(artist) => {
                        sender.oneshot_command(async move {
                            let client = Client::get().lock().unwrap().inner.clone().unwrap();
                            let mut result = vec![];
                            for album in artist.album {
                                match client.get_album(album.id).await {
                                    Ok(mut album) => result.append(&mut album.song),
                                    Err(e) => return QueueCmd::FetchedAppendItems(Err(e)),
                                }
                            }
                            QueueCmd::FetchedAppendItems(Ok(result))
                        });
                        return;
                    }
                    Droppable::Playlist(playlist) => playlist.entry,
                    Droppable::AlbumChild(child) => {
                        sender.oneshot_command(async move {
                            let client = Client::get().lock().unwrap().inner.clone().unwrap();
                            match client.get_album(child.id).await {
                                Err(e) => QueueCmd::FetchedAppendItems(Err(e)),
                                Ok(album) => QueueCmd::FetchedAppendItems(Ok(album.song)),
                            }
                        });
                        return;
                    }
                    Droppable::Album(album) => {
                        sender.oneshot_command(async move {
                            let client = Client::get().lock().unwrap().inner.clone().unwrap();
                            match client.get_album(album.id).await {
                                Err(e) => QueueCmd::FetchedAppendItems(Err(e)),
                                Ok(album) => QueueCmd::FetchedAppendItems(Ok(album.song)),
                            }
                        });
                        return;
                    }
                    Droppable::Id(_) => vec![], //TODO remove eventually
                };

                for song in songs.into_iter() {
                    self.songs.guard().push_back(song);
                }
            }
            QueueIn::Clear => {
                self.songs.guard().clear();
                self.update_clear_btn_sensitivity();
                self.last_selected = None;
            }
            QueueIn::Remove => {
                let selected_indices: Vec<usize> = self
                    .songs
                    .iter()
                    .enumerate()
                    .filter_map(|(i, s)| {
                        if s.root_widget().is_selected() {
                            Some(i)
                        } else {
                            None
                        }
                    })
                    .collect();
                for index in selected_indices.iter().rev() {
                    let mut guard = self.songs.guard();
                    guard.remove(*index);
                }

                self.update_clear_btn_sensitivity();
            }
            QueueIn::MoveAbove { src, dest } => {
                let mut guard = self.songs.guard();
                let src = src.current_index();
                let dest = dest.current_index();
                guard.move_to(src, dest);
            }
            QueueIn::MoveBelow { src, dest } => {
                let mut guard = self.songs.guard();
                let src = src.current_index();
                let dest = dest.current_index();
                if src <= dest {
                    guard.move_to(src, dest);
                } else {
                    guard.move_to(src, dest + 1);
                }
            }
            QueueIn::DropAbove { src, dest } => {
                let mut guard = self.songs.guard();
                for (i, child) in src.iter().enumerate() {
                    guard.insert(dest.current_index() + i, child.clone());
                }
            }
            QueueIn::DropBelow { src, dest } => {
                let mut guard = self.songs.guard();
                for (i, child) in src.iter().enumerate() {
                    guard.insert(dest.current_index() + i + 1, child.clone());
                }
            }
            QueueIn::NewState(state) => {
                if self.songs.is_empty() {
                    return;
                }

                if let Some(index) = &self.playing_index {
                    if let Some(song) = self.songs.get(index.current_index()) {
                        song.new_play_state(&state);
                    }
                }
            }
            QueueIn::SomeIsSelected(state) => self.remove_items.set_sensitive(state),
            QueueIn::ToggleShuffle => {
                //TODO sth useful
            }
            QueueIn::RepeatPressed => {
                //TODO sth useful
            }
            QueueIn::PlayNext => {
                if self.songs.is_empty() {
                    return;
                }

                match &self.playing_index {
                    None => self.songs.front().unwrap().activate(),
                    Some(index) => {
                        match index.current_index() {
                            // at end of queue with repeat current song
                            i if *self.repeat.model().current() == Repeat::RepeatOne => {
                                self.songs.get(i).unwrap().activate();
                            }
                            // at end of queue with repeat queue
                            i if i + 1 == self.songs.len()
                                && *self.repeat.model().current() == Repeat::RepeatAll =>
                            {
                                self.songs.get(0).unwrap().activate();
                            }
                            // at end of queue
                            i if i + 1 == self.songs.len() => {
                                sender.output(QueueOut::Stop).unwrap();
                                self.songs.get(i).unwrap().new_play_state(&PlayState::Stop);
                                self.playing_index = None;
                            }
                            i => self.songs.get(i + 1).unwrap().activate(),
                        }
                    }
                }
            }
            QueueIn::PlayPrevious => {
                if self.songs.is_empty() {
                    return;
                }

                if let Some(index) = &self.playing_index {
                    match index.current_index() {
                        // previous with repeat current song
                        i if *self.repeat.model().current() == Repeat::RepeatOne => {
                            self.songs.get(i).unwrap().activate();
                        }
                        // at start of queue with repeat queue
                        0 if *self.repeat.model().current() == Repeat::RepeatAll => {
                            self.songs.get(self.songs.len() - 1).unwrap().activate();
                        }
                        // at start of queue
                        0 => self.songs.get(0).unwrap().activate(),
                        i => self.songs.get(i - 1).unwrap().activate(),
                    }
                }
            }
            QueueIn::LoadPlayQueue => {
                self.loading_queue = true;
                sender.oneshot_command(async move {
                    let client = Client::get().lock().unwrap().inner.clone().unwrap();
                    if let Ok(Ok(queue)) = client.get_play_queue().await {
                        QueueCmd::FetchedQueue(Some(queue))
                    } else {
                        QueueCmd::FetchedQueue(None)
                    }
                });
            }
        }
    }

    fn update_cmd(
        &mut self,
        message: Self::CommandOutput,
        _sender: relm4::ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match message {
            QueueCmd::FetchedQueue(queue) => {
                let queue = if let Some(queue) = queue {
                    queue
                } else {
                    return;
                };

                for entry in queue.entry {
                    self.songs.guard().push_back(entry);
                }
                // TODO jump to current song
                // TODO set seekbar
                // TODO save queue

                self.loading_queue = false;
            }
            QueueCmd::FetchedAppendItems(Err(e)) => {} //TODO error handling
            QueueCmd::FetchedAppendItems(Ok(children)) => {
                for child in children {
                    self.songs.guard().push_back(child);
                }
            }
        }
    }
}
