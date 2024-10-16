use std::{cell::RefCell, rc::Rc};

use rand::prelude::SliceRandom;
use relm4::{
    factory::FactoryVecDeque,
    gtk::{
        self, gdk,
        prelude::{AdjustmentExt, BoxExt, ButtonExt, ListBoxRowExt, OrientableExt, WidgetExt},
    },
    prelude::DynamicIndex,
    ComponentParts, ComponentSender, RelmWidgetExt,
};

use crate::{
    client::Client,
    components::sequence_button_impl::{repeat::Repeat, shuffle::Shuffle},
    factory::queue_song::{QueueSong, QueueSongIn, QueueSongOut},
    play_state::PlayState,
    player::Command,
    settings::Settings,
    subsonic::Subsonic,
    types::Droppable,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScrollMotion {
    None,
    Down,
    Up,
}

#[derive(Debug)]
pub struct Queue {
    subsonic: Rc<RefCell<Subsonic>>,
    scrolled: gtk::ScrolledWindow,
    songs: FactoryVecDeque<QueueSong>,
    randomized_indices: Vec<usize>,
    loading_queue: bool,
    playing_index: Option<DynamicIndex>,
    remove_items: gtk::Button,
    clear_items: gtk::Button,
    last_selected: Option<DynamicIndex>,
}

impl Queue {
    pub fn songs(&self) -> Vec<submarine::data::Child> {
        self.songs.iter().map(|c| c.info().clone()).collect()
    }

    pub fn playing_index(&self) -> &Option<DynamicIndex> {
        &self.playing_index
    }

    pub fn can_play(&self) -> bool {
        !self.songs.is_empty()
    }

    pub fn can_play_next(&self) -> bool {
        if self.songs.is_empty() {
            return false;
        }

        let settings = Settings::get().lock().unwrap();
        if settings.repeat != Repeat::Normal {
            return true;
        }

        if settings.shuffle == Shuffle::Shuffle {
            return true; //TODO might change later
        }
        drop(settings);

        if let Some(index) = &self.playing_index {
            if index.current_index() + 1 == self.songs.len() {
                return false;
            }
        }

        true
    }

    pub fn can_play_previous(&self) -> bool {
        if self.songs.is_empty() {
            return false;
        }

        let settings = Settings::get().lock().unwrap();
        if settings.repeat != Repeat::Normal {
            return true;
        }

        if settings.shuffle == Shuffle::Shuffle {
            return true;
        }
        drop(settings);

        if let Some(index) = &self.playing_index {
            if index.current_index() == 0 {
                return false;
            }
        }

        true
    }

    pub fn current_song(&self) -> Option<submarine::data::Child> {
        match &self.playing_index {
            None => None,
            Some(index) => self
                .songs
                .get(index.current_index())
                .as_ref()
                .map(|queue_song| queue_song.info().clone()),
        }
    }
}

#[derive(Debug)]
pub enum QueueIn {
    SetCurrent(Option<usize>),
    Clear,
    Remove,
    NewState(PlayState),
    SomeIsSelected(bool),
    ToggleShuffle(Shuffle),
    PlayNext,
    PlayPrevious,
    Append(Droppable),
    QueueSong(QueueSongOut),
    InsertAfterCurrentlyPlayed(Droppable),
    Replace(Droppable),
    DisplayToast(String),
    Favorite(String, bool),
    JumpToCurrent,
    Rerandomize,
}

#[derive(Debug)]
pub enum QueueOut {
    Play(Box<submarine::data::Child>),
    QueueEmpty,
    QueueNotEmpty,
    Player(Command),
    CreatePlaylist,
    DisplayToast(String),
    DesktopNotification(Box<submarine::data::Child>),
    FavoriteClicked(String, bool),
}

#[derive(Debug)]
pub enum QueueCmd {
    Error(String, submarine::SubsonicError),
    FetchedAppendItems(Vec<submarine::data::Child>),
    FetchedInsertItems(Vec<submarine::data::Child>),
}

#[relm4::component(pub)]
impl relm4::Component for Queue {
    type Init = (
        Rc<RefCell<Subsonic>>,
        Vec<submarine::data::Child>,
        Option<usize>,
    );
    type Input = QueueIn;
    type Output = QueueOut;
    type Widgets = QueueWidgets;
    type CommandOutput = QueueCmd;

    fn init(
        (subsonic, songs, index): Self::Init,
        root: Self::Root,
        sender: relm4::ComponentSender<Self>,
    ) -> relm4::ComponentParts<Self> {
        let model = Queue {
            subsonic,
            scrolled: gtk::ScrolledWindow::default(),
            songs: FactoryVecDeque::builder()
                .launch(gtk::ListBox::default())
                .forward(sender.input_sender(), QueueIn::QueueSong),
            randomized_indices: vec![],
            loading_queue: false,
            playing_index: None,
            remove_items: gtk::Button::new(),
            clear_items: gtk::Button::new(),
            last_selected: None,
        };

        //init queue
        sender.input(QueueIn::Append(Droppable::Queue(songs)));
        sender.input(QueueIn::SetCurrent(index));

        let songs = model.songs.widget().clone();
        let scrolled = model.scrolled.clone();
        let scrolling = Rc::new(RefCell::new(ScrollMotion::None));
        let (scroll_sender, receiver) = async_channel::unbounded::<ScrollMotion>();
        let widgets = view_output!();

        gtk::glib::spawn_future_local(async move {
            let scrolling = scrolling.clone();

            while let Ok(msg) = receiver.recv().await {
                scrolling.replace(msg.clone());

                match msg {
                    ScrollMotion::None => {}
                    _ => {
                        let scrolled = scrolled.clone();
                        let scrolling = scrolling.clone();

                        gtk::glib::source::timeout_add_local(
                            core::time::Duration::from_millis(15),
                            move || {
                                const SCROLL_MOVE: f64 = 5f64;
                                match *scrolling.borrow() {
                                    ScrollMotion::None => return gtk::glib::ControlFlow::Break,
                                    ScrollMotion::Up => {
                                        let vadj = scrolled.vadjustment();
                                        vadj.set_value(vadj.value() - SCROLL_MOVE);
                                        scrolled.set_vadjustment(Some(&vadj));
                                    }
                                    ScrollMotion::Down => {
                                        let vadj = scrolled.vadjustment();
                                        vadj.set_value(vadj.value() + SCROLL_MOVE);
                                        scrolled.set_vadjustment(Some(&vadj));
                                    }
                                }
                                gtk::glib::ControlFlow::Continue
                            },
                        );
                    }
                }
            }
        });

        model.clear_items.set_sensitive(!model.songs.is_empty());

        ComponentParts { model, widgets }
    }

    view! {
        gtk::Box {
            add_css_class: "queue",
            set_orientation: gtk::Orientation::Vertical,

            model.scrolled.clone() -> gtk::ScrolledWindow {
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
                } else {
                    model.songs.widget().clone() -> gtk::ListBox {
                        set_selection_mode: gtk::SelectionMode::Multiple,

                        connect_selected_rows_changed[sender] => move |widget| {
                            sender.input(QueueIn::SomeIsSelected(!widget.selected_rows().is_empty()));
                        },

                        add_controller = gtk::DropControllerMotion {
                            connect_motion[scrolled, scrolling, scroll_sender] => move |_self, x, y| {
                                if *scrolling.borrow() != ScrollMotion::None {
                                    return;
                                }

                                const SCROLL_ZONE: f32 = 60f32;

                                let point = gtk::graphene::Point::new(x as f32, y as f32);
                                let computed = songs.compute_point(&scrolled, &point).unwrap();
                                if computed.y() >= 0f32 && computed.y() <= SCROLL_ZONE {
                                    println!("up");
                                    scroll_sender.try_send(ScrollMotion::Up).unwrap();
                                } else if computed.y() >= scrolled.height() as f32 - SCROLL_ZONE && computed.y() <= scrolled.height() as f32 {
                                    println!("down");
                                    scroll_sender.try_send(ScrollMotion::Down).unwrap();
                                } else {
                                    println!("none");
                                    scroll_sender.try_send(ScrollMotion::None).unwrap();
                                }
                            },

                            //TODO fix continued scrolling when droping while scrolling
                            connect_leave[scroll_sender] => move |_self| {
                                scroll_sender.try_send(ScrollMotion::None).unwrap();
                            },

                            connect_drop_notify[scroll_sender, scrolling] => move |_self| {
                                scroll_sender.try_send(ScrollMotion::None).unwrap();
                                scrolling.replace(ScrollMotion::None);
                            }
                        },

                        #[wrap(Some)]
                        set_placeholder = &gtk::Box {
                            set_valign: gtk::Align::Center,
                            set_orientation: gtk::Orientation::Vertical,

                            gtk::Label {
                                add_css_class: granite::STYLE_CLASS_H2_LABEL,
                                set_label: "Queue is empty",
                            },
                            gtk::Label {
                                add_css_class: granite::STYLE_CLASS_H3_LABEL,
                                set_label: "Drag music here to add it",
                            },

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
                    }
                },

                add_controller = gtk::DropTarget {
                    set_actions: gdk::DragAction::MOVE | gdk::DragAction::COPY,
                    set_types: &[<Droppable as gtk::prelude::StaticType>::static_type()],
                    connect_drop[sender] => move |_target, value, _x, _y| {
                        if let Ok(drop) = value.get::<Droppable>() {
                            sender.input(QueueIn::Append(drop));
                        }
                        true
                    },
                }
            },

            gtk::ActionBar {
                pack_start = &gtk::Box {
                    model.remove_items.clone() {
                        set_icon_name: "list-remove-symbolic",
                        set_tooltip: "Remove selected songs from queue",
                        set_sensitive: false,
                        set_focus_on_click: false,
                        connect_clicked => QueueIn::Remove,
                    },

                    model.clear_items.clone() {
                        set_margin_start: 15,
                        set_icon_name: "user-trash-symbolic",
                        set_tooltip: "Clear queue",
                        set_sensitive: false,
                        set_focus_on_click: false,
                        connect_clicked => QueueIn::Clear,
                    },
                },

                #[wrap(Some)]
                set_center_widget = &gtk::Button {
                        set_icon_name: "view-continuous-symbolic",
                        set_tooltip: "Jump to played track in queue",
                        set_focus_on_click: false,
                        connect_clicked => QueueIn::JumpToCurrent,
                },

                pack_end = &gtk::Button {
                    set_icon_name: "document-new-symbolic",
                    set_tooltip: "Create new playlist from queue",
                    set_focus_on_click: false,
                    connect_clicked[sender] => move |_btn| {
                        sender.output(QueueOut::CreatePlaylist).unwrap();
                    },
                },
            }
        }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>, _root: &Self::Root) {
        match msg {
            QueueIn::SetCurrent(None) | QueueIn::NewState(PlayState::Stop) => {
                if let Some(index) = &self.playing_index {
                    if let Some(song) = self.songs.get(index.current_index()) {
                        song.new_play_state(&PlayState::Stop);
                    }
                }

                self.playing_index = None;
            }
            QueueIn::SetCurrent(Some(index)) => {
                if let Some(song) = self.songs.get(index) {
                    self.playing_index = Some(song.index().clone());
                    sender.input(QueueIn::NewState(PlayState::Pause));
                }
            }
            QueueIn::Replace(drop) => {
                sender.input(QueueIn::Clear);
                sender.input(QueueIn::Append(drop));
            }
            QueueIn::Append(id) => {
                let songs: Vec<submarine::data::Child> = match id {
                    Droppable::Queue(ids) => ids,
                    Droppable::Child(c) => vec![*c],
                    Droppable::AlbumWithSongs(album) => album.song,
                    Droppable::Artist(artist) => {
                        sender.oneshot_command(async move {
                            let client = Client::get().unwrap();
                            let artist_with_albums = match client.get_artist(artist.id).await {
                                Err(e) => {
                                    return QueueCmd::Error(String::from("Could not append"), e)
                                }
                                Ok(artist) => artist,
                            };

                            let mut result = vec![];
                            for album in artist_with_albums.album {
                                match client.get_album(album.id).await {
                                    Ok(mut album) => result.append(&mut album.song),
                                    Err(e) => {
                                        return QueueCmd::Error(String::from("could not append"), e)
                                    }
                                }
                            }
                            QueueCmd::FetchedAppendItems(result)
                        });
                        return;
                    }
                    Droppable::ArtistWithAlbums(artist) => {
                        sender.oneshot_command(async move {
                            let client = Client::get().unwrap();
                            let mut result = vec![];
                            for album in artist.album {
                                match client.get_album(album.id).await {
                                    Ok(mut album) => result.append(&mut album.song),
                                    Err(e) => {
                                        return QueueCmd::Error(String::from("could not append"), e)
                                    }
                                }
                            }
                            QueueCmd::FetchedAppendItems(result)
                        });
                        return;
                    }
                    Droppable::Playlist(playlist) => playlist.entry,
                    Droppable::AlbumChild(child) => {
                        sender.oneshot_command(async move {
                            let client = Client::get().unwrap();
                            match client.get_album(child.id).await {
                                Err(e) => QueueCmd::Error(String::from("could not append"), e),
                                Ok(album) => QueueCmd::FetchedAppendItems(album.song),
                            }
                        });
                        return;
                    }
                    Droppable::Album(album) => {
                        sender.oneshot_command(async move {
                            let client = Client::get().unwrap();
                            match client.get_album(album.id).await {
                                Err(e) => QueueCmd::Error(String::from("could not append"), e),
                                Ok(album) => QueueCmd::FetchedAppendItems(album.song),
                            }
                        });
                        return;
                    }
                };

                for song in songs {
                    self.songs.guard().push_back((self.subsonic.clone(), song));
                }
                sender.input(QueueIn::Rerandomize);

                if !self.songs.is_empty() {
                    sender.output(QueueOut::QueueNotEmpty).unwrap();
                }
                self.clear_items.set_sensitive(!self.songs.is_empty());
            }
            QueueIn::InsertAfterCurrentlyPlayed(drop) => {
                let songs: Vec<submarine::data::Child> = match drop {
                    Droppable::Queue(ids) => ids,
                    Droppable::Child(c) => vec![*c],
                    Droppable::AlbumWithSongs(album) => album.song,
                    Droppable::Artist(artist) => {
                        sender.oneshot_command(async move {
                            let client = Client::get().unwrap();
                            let artist_with_albums = match client.get_artist(artist.id).await {
                                Err(e) => {
                                    return QueueCmd::Error(String::from("could not insert"), e)
                                }
                                Ok(artist) => artist,
                            };

                            let mut result = vec![];
                            for album in artist_with_albums.album {
                                match client.get_album(album.id).await {
                                    Ok(mut album) => result.append(&mut album.song),
                                    Err(e) => {
                                        return QueueCmd::Error(String::from("could not insert"), e)
                                    }
                                }
                            }
                            QueueCmd::FetchedInsertItems(result)
                        });
                        return;
                    }
                    Droppable::ArtistWithAlbums(artist) => {
                        sender.oneshot_command(async move {
                            let client = Client::get().unwrap();
                            let mut result = vec![];
                            for album in artist.album {
                                match client.get_album(album.id).await {
                                    Ok(mut album) => result.append(&mut album.song),
                                    Err(e) => {
                                        return QueueCmd::Error(String::from("could not insert"), e)
                                    }
                                }
                            }
                            QueueCmd::FetchedInsertItems(result)
                        });
                        return;
                    }
                    Droppable::Playlist(playlist) => playlist.entry,
                    Droppable::AlbumChild(child) => {
                        sender.oneshot_command(async move {
                            let client = Client::get().unwrap();
                            match client.get_album(child.id).await {
                                Err(e) => QueueCmd::Error(String::from("could not insert"), e),
                                Ok(album) => QueueCmd::FetchedInsertItems(album.song),
                            }
                        });
                        return;
                    }
                    Droppable::Album(album) => {
                        sender.oneshot_command(async move {
                            let client = Client::get().unwrap();
                            match client.get_album(album.id).await {
                                Err(e) => QueueCmd::Error(String::from("could not insert"), e),
                                Ok(album) => QueueCmd::FetchedInsertItems(album.song),
                            }
                        });
                        return;
                    }
                };

                if let Some(index) = &self.playing_index {
                    for song in songs.into_iter().rev() {
                        self.songs
                            .guard()
                            .insert(index.current_index() + 1, (self.subsonic.clone(), song));
                    }
                } else {
                    for song in songs.into_iter().rev() {
                        self.songs.guard().push_back((self.subsonic.clone(), song));
                    }
                }
                sender.input(QueueIn::Rerandomize);

                if !self.songs.is_empty() {
                    sender.output(QueueOut::QueueNotEmpty).unwrap();
                }
                self.clear_items.set_sensitive(!self.songs.is_empty());
            }
            QueueIn::Clear => {
                self.songs.guard().clear();
                self.randomized_indices.clear();
                self.clear_items.set_sensitive(!self.songs.is_empty());
                self.last_selected = None;
                self.playing_index = None;
                sender.output(QueueOut::QueueEmpty).unwrap();
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
                sender.input(QueueIn::Rerandomize);

                //set new state when deleting played index
                if let Some(current) = &self.playing_index {
                    if selected_indices.contains(&current.current_index()) {
                        sender.output(QueueOut::Player(Command::Stop)).unwrap();
                        sender.input(QueueIn::SetCurrent(None));
                    }
                }

                if self.songs.is_empty() {
                    sender.output(QueueOut::QueueEmpty).unwrap();
                }

                self.clear_items.set_sensitive(!self.songs.is_empty());
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
            QueueIn::ToggleShuffle(shuffle) => {
                {
                    let mut settings = Settings::get().lock().unwrap();
                    settings.shuffle = shuffle.clone();
                }
                sender
                    .output(QueueOut::Player(Command::Shuffle(shuffle)))
                    .unwrap();
            }
            QueueIn::PlayNext => {
                if self.songs.is_empty() {
                    return;
                }

                let settings = Settings::get().lock().unwrap();
                let repeat = settings.repeat.clone();
                let shuffle = settings.shuffle.clone();
                drop(settings);

                match &self.playing_index {
                    None => self.songs.front().unwrap().activate(),
                    Some(index) => {
                        match index.current_index() {
                            // at end of queue with repeat current song
                            i if repeat == Repeat::One => {
                                self.songs.get(i).unwrap().activate();
                            }
                            // at end of queue with repeat queue
                            i if i + 1 == self.songs.len()
                                && shuffle != Shuffle::Shuffle
                                && repeat == Repeat::All =>
                            {
                                self.songs.get(0).unwrap().activate();
                            }
                            // shuffle ignores repeat all
                            i if shuffle == Shuffle::Shuffle => {
                                let idx = self
                                    .randomized_indices
                                    .iter()
                                    .position(|idx| &i == idx)
                                    .unwrap();
                                let idx =
                                    self.randomized_indices.iter().cycle().nth(idx + 1).unwrap();
                                self.songs.get(*idx).unwrap().activate();
                            }
                            // at end of queue
                            i if i + 1 == self.songs.len() => {
                                self.songs.get(i).unwrap().new_play_state(&PlayState::Stop);
                                self.playing_index = None;
                                sender.output(QueueOut::Player(Command::Stop)).unwrap();
                            }
                            // play next song
                            i => self.songs.get(i + 1).unwrap().activate(),
                        }
                    }
                }
            }
            QueueIn::PlayPrevious => {
                if self.songs.is_empty() {
                    return;
                }

                let settings = Settings::get().lock().unwrap();
                let repeat = settings.repeat.clone();
                let shuffle = settings.shuffle.clone();
                drop(settings);

                if let Some(index) = &self.playing_index {
                    match index.current_index() {
                        // previous with repeat current song
                        i if repeat == Repeat::One => {
                            self.songs.get(i).unwrap().activate();
                        }
                        // at start of queue with repeat queue
                        0 if repeat == Repeat::All => {
                            self.songs.get(self.songs.len() - 1).unwrap().activate();
                        }
                        // shuffle ignores repeat all
                        i if shuffle == Shuffle::Shuffle => {
                            let idx = self
                                .randomized_indices
                                .iter()
                                .position(|idx| &i == idx)
                                .unwrap();
                            if idx == 0 {
                                let idx = self.randomized_indices.last().unwrap();
                                self.songs.get(*idx).unwrap().activate();
                            } else {
                                let idx = self.randomized_indices.get(idx - 1).unwrap();
                                self.songs.get(*idx).unwrap().activate();
                            }
                        }
                        // at start of queue
                        0 => self.songs.get(0).unwrap().activate(),
                        i => self.songs.get(i - 1).unwrap().activate(),
                    }
                }
            }
            QueueIn::QueueSong(msg) => match msg {
                QueueSongOut::Activated(index, info) => {
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
                QueueSongOut::Clicked(index) => {
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
                QueueSongOut::DisplayToast(msg) => {
                    sender.output(QueueOut::DisplayToast(msg)).unwrap()
                }
                QueueSongOut::DropAbove { src, dest } => {
                    let mut guard = self.songs.guard();
                    for (i, child) in src.iter().enumerate() {
                        guard.insert(
                            dest.current_index() + i,
                            (self.subsonic.clone(), child.clone()),
                        );
                    }
                    sender.input(QueueIn::Rerandomize);
                }
                QueueSongOut::DropBelow { src, dest } => {
                    let mut guard = self.songs.guard();
                    for (i, child) in src.iter().enumerate() {
                        guard.insert(
                            dest.current_index() + i + 1,
                            (self.subsonic.clone(), child.clone()),
                        );
                    }
                    sender.input(QueueIn::Rerandomize);
                }
                QueueSongOut::MoveAbove { src, dest } => {
                    let mut guard = self.songs.guard();
                    let src = src.current_index();
                    let dest = dest.current_index();
                    guard.move_to(src, dest);
                }
                QueueSongOut::MoveBelow { src, dest } => {
                    let mut guard = self.songs.guard();
                    let src = src.current_index();
                    let dest = dest.current_index();
                    if src <= dest {
                        guard.move_to(src, dest);
                    } else {
                        guard.move_to(src, dest + 1);
                    }
                }
                QueueSongOut::Remove => sender.input(QueueIn::Remove),
                QueueSongOut::ShiftClicked(target) => {
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
                        self.last_selected = Some(target);
                    }
                }
                QueueSongOut::FavoriteClicked(id, state) => {
                    sender.output(QueueOut::FavoriteClicked(id, state)).unwrap()
                }
            },
            QueueIn::DisplayToast(title) => sender.output(QueueOut::DisplayToast(title)).unwrap(),
            QueueIn::Favorite(id, state) => {
                self.songs.broadcast(QueueSongIn::FavoriteSong(id, state));
            }
            QueueIn::JumpToCurrent => {
                // where the current song in the window will up end from start
                const CURRENT_POSITION: f64 = 0.4;
                if let Some(current) = &self.playing_index {
                    let height = self.songs.widget().height();
                    let adj = self.scrolled.vadjustment();
                    let scroll_y = height as f64 / self.songs.len() as f64
                        * current.current_index() as f64
                        - self.scrolled.height() as f64 * CURRENT_POSITION;
                    adj.set_value(scroll_y);
                    self.scrolled.set_vadjustment(Some(&adj));
                }
            }
            QueueIn::Rerandomize => {
                self.randomized_indices = (0..self.songs.len()).collect();
                let mut rng = rand::thread_rng();
                self.randomized_indices.shuffle(&mut rng);
            }
        }
    }

    fn update_cmd(
        &mut self,
        message: Self::CommandOutput,
        sender: relm4::ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match message {
            QueueCmd::Error(msg, e) => {
                sender
                    .output(QueueOut::DisplayToast(format!("{msg}: {e:?}")))
                    .unwrap();
            }
            QueueCmd::FetchedAppendItems(children) => {
                for child in children {
                    self.songs.guard().push_back((self.subsonic.clone(), child));
                }
                self.clear_items.set_sensitive(!self.songs.is_empty());
                sender.input(QueueIn::Rerandomize);

                if !self.songs.is_empty() {
                    sender.output(QueueOut::QueueNotEmpty).unwrap();
                }
            }
            QueueCmd::FetchedInsertItems(children) => {
                for (i, child) in children.iter().enumerate() {
                    let current = match &self.playing_index {
                        None => 0,
                        Some(i) => i.current_index(),
                    };
                    self.songs
                        .guard()
                        .insert(current + i + 1, (self.subsonic.clone(), child.clone()));
                }
                self.clear_items.set_sensitive(!self.songs.is_empty());
                sender.input(QueueIn::Rerandomize);

                if !self.songs.is_empty() {
                    sender.output(QueueOut::QueueNotEmpty).unwrap();
                }
            }
        }
    }
}
