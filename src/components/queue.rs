use gtk::prelude::{BoxExt, ButtonExt, OrientableExt};
use relm4::{
    factory::FactoryVecDeque,
    gtk::{
        self,
        traits::{ListBoxRowExt, WidgetExt},
    },
    prelude::DynamicIndex,
    ComponentParts, ComponentSender, SimpleComponent,
};

use crate::{factory::queue_item::QueueSong, play_state::PlayState, types::Id};

#[derive(Debug)]
pub struct Uri {}

pub struct QueueModel {
    songs: FactoryVecDeque<QueueSong>,
    playing_index: Option<DynamicIndex>,
    remove_items: gtk::Button,
    clear_items: gtk::Button,
}

impl QueueModel {
    pub fn append(&mut self, id: Id) {
        self.songs.guard().push_back(id);
        self.update_clear_btn_sensitivity();
    }

    fn update_clear_btn_sensitivity(&mut self) {
        self.clear_items
            .set_sensitive(!self.songs.guard().is_empty());
    }
}

#[derive(Debug)]
pub enum QueueInput {
    Activated(DynamicIndex, Id),
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
    KeyUp,
    KeyDown,
    NewState(PlayState),
    SomeIsSelected(bool),
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
        let mut model = QueueModel {
            songs: FactoryVecDeque::new(gtk::ListBox::default(), sender.input_sender()),
            playing_index: None,
            remove_items: gtk::Button::new(),
            clear_items: gtk::Button::new(),
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

            #[local_ref]
            queue_songs -> gtk::ListBox {
                set_selection_mode: gtk::SelectionMode::Multiple,

                connect_selected_rows_changed[sender] => move |widget| {
                    sender.input(QueueInput::SomeIsSelected(!widget.selected_rows().is_empty()));
                },
            },

            // dummy for moving the actionbar to the end of the queue
            gtk::Label {
                set_vexpand: true,
            },

            gtk::ActionBar {
                pack_start = &model.clear_items.clone() {
                    set_icon_name: "user-trash-symbolic",
                    set_tooltip_text: Some("clear queue"),
                    set_sensitive: false,
                    connect_clicked => QueueInput::Clear,
                },

                pack_start = &gtk::Label {
                    add_css_class: "destructive-button-spacer",
                },

                pack_start = &model.remove_items.clone() {
                    set_icon_name: "list-remove-symbolic",
                    set_tooltip_text: Some("remove song from queue"),
                    set_sensitive: false,
                    connect_clicked => QueueInput::Remove,
                },

                pack_end = &gtk::Button {
                    set_icon_name: "document-new-symbolic",
                    set_tooltip_text: Some("add queue to playlists"),
                    connect_clicked => QueueInput::Append(Id::song("5555555")),
                }
            }
        }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
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
            QueueInput::Append(id) => {
                let _ = self.songs.guard().push_back(id);
                self.update_clear_btn_sensitivity();
            }
            QueueInput::Clear => {
                self.songs.guard().clear();
                self.update_clear_btn_sensitivity();
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
            QueueInput::KeyUp => todo!("up without ctrl"),
            QueueInput::KeyDown => todo!("down without ctrl"),
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
                        todo!();
                    }
                    PlayState::Pause => {
                        todo!();
                    }
                    PlayState::Stop => todo!(),
                }
            }
            QueueInput::SomeIsSelected(state) => _ = self.remove_items.set_sensitive(state),
        }
    }
}
