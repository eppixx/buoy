use gtk::prelude::{BoxExt, ButtonExt, OrientableExt};
use relm4::{
    factory::FactoryVecDeque,
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
    factory::queue_item::QueueSong,
    play_state::PlayState,
    types::Id,
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

    fn shuffle(&self) -> bool {
        self.shuffle.model().current() == &Shuffle::Sequential
    }
}

#[derive(Debug)]
pub enum QueueIn {
    Activated(DynamicIndex, submarine::data::Child),
    Clicked(DynamicIndex),
    ShiftClicked(DynamicIndex),
    Append(Id),
    Clear,
    Remove,
    DropAbove {
        src: DynamicIndex,
        dest: DynamicIndex,
    },
    DropBelow {
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
}

#[derive(Debug)]
pub enum QueueOut {
    Play(submarine::data::Child),
}

#[derive(Debug)]
pub enum QueueCmd {
    FetchedQueue(Option<submarine::data::PlayQueue>),
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
                        //TODO add DragDest
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
                    connect_clicked => QueueIn::Append(Id::song("5555555")),
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
                    song.new_play_state(PlayState::Stop);
                }

                sender.output(QueueOut::Play(info)).unwrap();
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
                let _ = self.songs.guard().push_back(id);
                self.update_clear_btn_sensitivity();
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
            QueueIn::DropAbove { src, dest } => {
                let mut guard = self.songs.guard();
                let src = src.current_index();
                let dest = dest.current_index();
                guard.move_to(src, dest);
            }
            QueueIn::DropBelow { src, dest } => {
                let mut guard = self.songs.guard();
                let src = src.current_index();
                let dest = dest.current_index();
                if src <= dest {
                    guard.move_to(src, dest);
                } else {
                    guard.move_to(src, dest + 1);
                }
            }
            QueueIn::NewState(state) => {
                if self.songs.is_empty() {
                    return;
                }

                match state {
                    PlayState::Play => {
                        // let (index, id) = if let Some(index) = &self.playing_index {
                        //     // play current index
                        //     let index = self.playing_index.unwrap().current_index();
                        //     self.songs.get(index).unwrap().new_play_state(state)
                        // } else {
                        //     // no song playing, start at front of playlist
                        //     self.songs.front().unwrap().new_play_state(state)
                        // };
                        // self.playing_index = index;
                        // sender.output(QueueOutput::Play(id.unwrap()));
                        println!("TODO implement play in queue");
                    }
                    PlayState::Pause => println!("TODO implement pause in queue"),
                    PlayState::Stop => println!("TODO implement stop in queue"),
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
                //TODO fth useful
            }
            QueueIn::PlayPrevious => {
                //TODO fth useful
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

                for entry in &queue.entry {
                    self.songs.guard().push_back(Id::song(&entry.id));
                }
                // TODO jump to current song
                // TODO set seekbar
                // TODO save queue

                self.loading_queue = false;
            }
        }
    }
}
