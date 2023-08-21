use relm4::{
    gtk::{
        self, gdk, glib, pango,
        prelude::ToValue,
        traits::{
            BoxExt, ButtonExt, EventControllerExt, GestureSingleExt, ListBoxRowExt, OrientableExt,
            WidgetExt,
        },
    },
    prelude::{DynamicIndex, FactoryComponent},
    FactorySender,
};

use crate::{components::queue::QueueInput, css::DragState, play_state::PlayState, types::Id};

#[derive(Clone, Debug, PartialEq, Eq, glib::Boxed)]
#[boxed_type(name = "QueueSongIndex")]
struct Index(DynamicIndex);

#[derive(Debug)]
pub enum QueueSongInput {
    Activated,
    DraggedOver(f64),
    DragDropped {
        src: DynamicIndex,
        dest: DynamicIndex,
        y: f64,
    },
    DragLeave,
    StartPlay,
    PausePlay,
    StopPlay,
}

#[derive(Debug)]
pub enum QueueSongOutput {
    Activated(DynamicIndex, Id),
    Clicked(DynamicIndex),
    ShiftClicked(DynamicIndex),
    Remove(DynamicIndex),
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
}

#[derive(Debug)]
pub struct QueueSong {
    root_widget: gtk::ListBoxRow,
    id: Id,
    index: DynamicIndex,
    sender: FactorySender<Self>,
    drag_src: gtk::DragSource,
    play_icon: gtk::Image,
}

impl QueueSong {
    pub fn new_play_state(&self, state: PlayState) -> (Option<DynamicIndex>, Option<Id>) {
        match state {
            PlayState::Play => {
                self.sender.input(QueueSongInput::StartPlay);
                (Some(self.index.clone()), Some(self.id.clone()))
            }
            PlayState::Pause => {
                self.sender.input(QueueSongInput::PausePlay);
                (Some(self.index.clone()), None)
            }
            PlayState::Stop => {
                self.sender.input(QueueSongInput::StopPlay);
                (None, None)
            }
        }
    }

    pub fn root_widget(&self) -> &gtk::ListBoxRow {
        &self.root_widget
    }
}

#[relm4::factory(pub)]
impl FactoryComponent for QueueSong {
    type ParentWidget = gtk::ListBox;
    type ParentInput = QueueInput;
    type CommandOutput = ();
    type Input = QueueSongInput;
    type Output = QueueSongOutput;
    type Init = Id;
    type Widgets = QueueSongWidgets;

    fn init_model(id: Self::Init, index: &DynamicIndex, sender: FactorySender<Self>) -> Self {
        let mut model = Self {
            root_widget: gtk::ListBoxRow::new(),
            id,
            index: index.clone(),
            sender: sender.clone(),
            drag_src: gtk::DragSource::new(),
            play_icon: gtk::Image::new(),
        };

        DragState::reset(&mut model.root_widget);

        // setup DragSource
        let index = Index(index.clone());
        let content = gdk::ContentProvider::for_value(&index.to_value());
        model.drag_src.set_content(Some(&content));
        model.drag_src.set_actions(gdk::DragAction::MOVE);

        model
    }

    fn output_to_parent_input(output: Self::Output) -> Option<QueueInput> {
        match output {
            QueueSongOutput::Activated(index, id) => Some(QueueInput::Activated(index, id)),
            QueueSongOutput::Clicked(index) => Some(QueueInput::Clicked(index)),
            QueueSongOutput::ShiftClicked(index) => Some(QueueInput::ShiftClicked(index)),
            QueueSongOutput::Remove(index) => Some(QueueInput::Remove),
            QueueSongOutput::KeyUp => Some(QueueInput::KeyUp),
            QueueSongOutput::KeyDown => Some(QueueInput::KeyDown),
            QueueSongOutput::DropAbove { src, dest } => Some(QueueInput::DropAbove { src, dest }),
            QueueSongOutput::DropBelow { src, dest } => Some(QueueInput::DropBelow { src, dest }),
        }
    }

    fn update(&mut self, message: Self::Input, sender: FactorySender<Self>) {
        match message {
            QueueSongInput::Activated => {
                self.play_icon.set_icon_name(Some("audio-volume-high"));
                sender.output(QueueSongOutput::Activated(
                    self.index.clone(),
                    self.id.clone(),
                ));
            }
            QueueSongInput::DraggedOver(y) => {
                let widget_height = self.root_widget.height();
                if y < widget_height as f64 * 0.5f64 {
                    DragState::drop_shadow_top(&mut self.root_widget);
                } else {
                    DragState::drop_shadow_bottom(&mut self.root_widget);
                }
            }
            QueueSongInput::DragDropped { src, dest, y } => {
                let widget_height = self.root_widget.height();
                if y < widget_height as f64 * 0.5f64 {
                    sender.output(QueueSongOutput::DropAbove { src, dest });
                } else {
                    sender.output(QueueSongOutput::DropBelow { src, dest });
                }
            }
            QueueSongInput::DragLeave => DragState::reset(&mut self.root_widget),
            QueueSongInput::StartPlay => {
                self.play_icon.set_from_icon_name(Some("audio-volume-high"));
            }
            QueueSongInput::PausePlay => {
                self.play_icon
                    .set_from_icon_name(Some("media-playback-pause-symbolic"));
            }
            QueueSongInput::StopPlay => {
                self.play_icon.set_from_icon_name(None);
            }
        }
    }

    view! {
        #[root]
        self.root_widget.clone() -> gtk::ListBoxRow {
            add_css_class: "queue-song",

            gtk::Box {
                set_orientation: gtk::Orientation::Horizontal,
                set_spacing: 10,

                self.play_icon.clone() -> gtk::Image {
                    set_icon_name: None,
                },

                gtk::Label {
                    set_label: &self.id.serialize(),
                    set_width_chars: 3,
                    set_hexpand: true,
                    set_halign: gtk::Align::Start,
                    set_ellipsize: pango::EllipsizeMode::End,
                },

                gtk::Button {
                    set_icon_name: "view-more-symbolic",
                    set_tooltip_text: Some("drag to reorder"),
                    add_controller: &self.drag_src,
                    set_margin_end: 3,
                }
            },

            // activate is when pressed enter on item
            connect_activate => QueueSongInput::Activated,

            // accept drop from queue items and id's and render drop indicators
            add_controller = &gtk::DropTarget {
                set_actions: gdk::DragAction::MOVE,
                set_types: &[<Index as gtk::prelude::StaticType>::static_type(),
                             <Id as gtk::prelude::StaticType>::static_type(),
                ],

                connect_drop[sender, index] => move |_target, value, _x, y| {
                    sender.input(QueueSongInput::DragLeave);

                    // drop is a index
                    if let Ok(src_index) = value.get::<Index>() {
                        sender.input(QueueSongInput::DragDropped {
                            src: src_index.0.clone(),
                            dest: index.clone(),
                            y,
                        });
                        return true;
                    }

                    // drop is a id
                    if let Ok(id) = value.get::<Id>() {
                        todo!();
                        // return true;
                    }

                    false
                },

                connect_motion[sender] => move |_widget, _x, y| {
                    sender.input(QueueSongInput::DraggedOver(y));
                    //may need to return other value for drag in future
                    gdk::DragAction::MOVE
                },

                connect_leave => QueueSongInput::DragLeave,
            },

            // double left click activates item
            add_controller = &gtk::GestureClick {
                set_button: 1,
                connect_pressed[sender, index] => move |_widget, n, _x, _y|{
                    if n == 1 {
                        let state = _widget.current_event_state();
                        if !(state.contains(gdk::ModifierType::SHIFT_MASK)
                             || state.contains(gdk::ModifierType::CONTROL_MASK) ) {
                            // normal click
                            sender.output(QueueSongOutput::Clicked(index.clone()));
                        } else if state.contains(gdk::ModifierType::SHIFT_MASK) {
                            // shift click
                            sender.output(QueueSongOutput::ShiftClicked(index.clone()));
                        }
                    }
                    else if n == 2 {
                        sender.input(QueueSongInput::Activated);
                    }
                }
            },

            // connect key presses
            add_controller = &gtk::EventControllerKey {
                connect_key_pressed[sender, index] => move |_widget, key, _code, state| {
                    use gtk::gdk::Key;
                    println!("state {state:?}");
                    if !state.contains(gdk::ModifierType::CONTROL_MASK) {
                        println!("true");
                    }
                    match (key, state.contains(gdk::ModifierType::CONTROL_MASK)) {
                        // (Key::Down, true) => sender.output(QueueSongOutput::KeyDown),
                        // (Key::Down, false) => println!("normal press"),
                        _ => {}
                    }
                    match key {
                        Key::Delete => sender.output(QueueSongOutput::Remove(index.clone())),
                        // Key::Down if state.contains(gdk::ModifierType::CONTROL_MASK) => sender.output(QueueSongOutput::KeyDown),
                        // Key::Up if state.contains(gdk::ModifierType::CONTROL_MASK) => sender.output(QueueSongOutput::KeyUp),
                        _ => println!("{:?}", key),
                        // _ => {}
                    }
                    gtk::Inhibit(false)
                }
            },
        }
    }
}
