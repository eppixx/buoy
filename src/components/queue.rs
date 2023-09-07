use gtk::prelude::{BoxExt, ButtonExt, OrientableExt};
use relm4::{
    factory::FactoryVecDeque,
    gtk::{
        self,
        traits::{ListBoxRowExt, WidgetExt},
    },
    prelude::DynamicIndex,
    Component, ComponentController, ComponentParts, ComponentSender, SimpleComponent,
};

use crate::{
    components::{
        sequence_button::SequenceButtonOutput,
        sequence_button_impl::{repeat::Repeat, shuffle::Shuffle},
    },
    factory::queue_item::QueueSong,
    play_state::PlayState,
    types::Id,
};

use super::sequence_button::SequenceButtonModel;

#[derive(Debug)]
pub struct Uri {}

pub struct QueueModel {
    songs: FactoryVecDeque<QueueSong>,
    playing_index: Option<DynamicIndex>,
    remove_items: gtk::Button,
    clear_items: gtk::Button,
    last_selected: Option<DynamicIndex>,
    shuffle: relm4::Controller<SequenceButtonModel<Shuffle>>,
    repeat: relm4::Controller<SequenceButtonModel<Repeat>>,
}

impl QueueModel {
    fn update_clear_btn_sensitivity(&mut self) {
        self.clear_items
            .set_sensitive(!self.songs.guard().is_empty());
    }

    fn shuffle(&self) -> bool {
        self.shuffle.model().current() == &Shuffle::Sequential
    }
}

#[derive(Debug)]
pub enum QueueInput {
    Activated(DynamicIndex, Id),
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
}

#[derive(Debug)]
pub enum QueueOutput {
    Play(Id),
}

#[relm4::component(pub)]
impl SimpleComponent for QueueModel {
    type Input = QueueInput;
    type Output = QueueOutput;
    type Init = ();

    fn init(
        _queue: Self::Init,
        root: &Self::Root,
        sender: relm4::ComponentSender<Self>,
    ) -> relm4::ComponentParts<Self> {
        let shuffle: relm4::Controller<SequenceButtonModel<Shuffle>> =
            SequenceButtonModel::<Shuffle>::builder()
                .launch(Shuffle::Sequential)
                .forward(sender.input_sender(), |msg| match msg {
                    SequenceButtonOutput::Clicked => QueueInput::ToggleShuffle,
                });
        let repeat: relm4::Controller<SequenceButtonModel<Repeat>> =
            SequenceButtonModel::<Repeat>::builder()
                .launch(Repeat::Normal)
                .forward(sender.input_sender(), |msg| match msg {
                    SequenceButtonOutput::Clicked => QueueInput::RepeatPressed,
                });

        let mut model = QueueModel {
            songs: FactoryVecDeque::new(gtk::ListBox::default(), sender.input_sender()),
            playing_index: None,
            remove_items: gtk::Button::new(),
            clear_items: gtk::Button::new(),
            last_selected: None,
            shuffle,
            repeat,
        };

        model.songs.guard().push_back(Id::song("1"));
        model.songs.guard().push_back(Id::song("2"));
        model.songs.guard().push_back(Id::song("3"));
        model.songs.guard().push_back(Id::song("4"));
        model.songs.guard().push_back(Id::song("5"));

        let queue_songs = model.songs.widget();
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

                #[local_ref]
                queue_songs -> gtk::ListBox {
                    set_selection_mode: gtk::SelectionMode::Multiple,

                    connect_selected_rows_changed[sender] => move |widget| {
                        sender.input(QueueInput::SomeIsSelected(!widget.selected_rows().is_empty()));
                    },
                },
            },

            gtk::ActionBar {
                pack_start = &model.shuffle.widget().clone() {},
                pack_start = &model.repeat.widget().clone() {},

                pack_end = &gtk::Button {
                    set_icon_name: "document-new-symbolic",
                    set_tooltip_text: Some("add queue to playlists"),
                    connect_clicked => QueueInput::Append(Id::song("5555555")),
                },

                pack_end = &gtk::Label {
                    add_css_class: "destructive-button-spacer",
                },

                pack_end = &model.remove_items.clone() {
                    set_icon_name: "list-remove-symbolic",
                    set_tooltip_text: Some("remove song from queue"),
                    set_sensitive: false,
                    connect_clicked => QueueInput::Remove,
                },

                pack_end = &gtk::Label {
                    add_css_class: "destructive-button-spacer",
                },

                pack_end = &model.clear_items.clone() {
                    set_icon_name: "user-trash-symbolic",
                    set_tooltip_text: Some("clear queue"),
                    set_sensitive: false,
                    connect_clicked => QueueInput::Clear,
                },
            }
        }
    }

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>) {
        match msg {
            QueueInput::Activated(index, id) => {
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

                // TODO play song
                println!("playing id: {id:?}");
            }
            QueueInput::Clicked(index) => {
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
            QueueInput::ShiftClicked(target) => {
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
            QueueInput::Append(id) => {
                let _ = self.songs.guard().push_back(id);
                self.update_clear_btn_sensitivity();
            }
            QueueInput::Clear => {
                self.songs.guard().clear();
                self.update_clear_btn_sensitivity();
                self.last_selected = None;
            }
            QueueInput::Remove => {
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
            QueueInput::DropAbove { src, dest } => {
                let mut guard = self.songs.guard();
                let src = src.current_index();
                let dest = dest.current_index();
                guard.move_to(src, dest);
            }
            QueueInput::DropBelow { src, dest } => {
                let mut guard = self.songs.guard();
                let src = src.current_index();
                let dest = dest.current_index();
                if src <= dest {
                    guard.move_to(src, dest);
                } else {
                    guard.move_to(src, dest + 1);
                }
            }
            QueueInput::NewState(state) => {
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
            QueueInput::SomeIsSelected(state) => self.remove_items.set_sensitive(state),
            QueueInput::ToggleShuffle => {
                //TODO sth useful
            }
            QueueInput::RepeatPressed => {
                //TODO sth useful
            }
            QueueInput::PlayNext => {
                //TODO fth useful
            }
            QueueInput::PlayPrevious => {
                //TODO fth useful
            }
        }
    }
}
