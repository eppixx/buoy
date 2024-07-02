use relm4::gtk::{
    self, gio, glib,
    prelude::{BoxExt, ButtonExt, EditableExt, EntryExt, ListBoxRowExt, ListItemExt, WidgetExt},
};

use std::cmp::Ordering;

#[derive(Debug)]
pub enum Category {
    Favorite,
    Year,
    Cd,
    TrackNumber,
    Artist,
    Album,
    Genre,
    Duration,
    BitRate,
}

impl Category {
    pub fn all() -> Vec<Self> {
        vec![
            Self::Favorite,
            Self::Year,
            Self::Cd,
            Self::TrackNumber,
            Self::Artist,
            Self::Album,
            Self::Genre,
            Self::Duration,
            Self::BitRate,
        ]
    }
}

#[derive(Debug)]
pub enum Filter {
    Favorited(bool),
    Year(Ordering, usize),
    Cd(bool, usize),
    TrackNumber(Ordering, usize),
    Artist(bool, String),
    Album(bool, String),
}

#[derive(Debug, Clone)]
struct OrderRow {
    order: Ordering,
    label: String,
}

// adapted from https://gtk-rs.org/gtk4-rs/stable/latest/book/list_widgets.html
impl Filter {
    pub fn order() -> gio::ListStore {
        let data: [OrderRow; 3] = [
            OrderRow {
                order: Ordering::Equal,
                label: String::from("equal to"),
            },
            OrderRow {
                order: Ordering::Greater,
                label: String::from("greater than"),
            },
            OrderRow {
                order: Ordering::Less,
                label: String::from("less than"),
            },
        ];
        let store = gtk::gio::ListStore::new::<glib::BoxedAnyObject>();
        for d in data {
            store.append(&glib::BoxedAnyObject::new(d));
        }
        store
    }

    pub fn factory() -> gtk::SignalListItemFactory {
        use glib::object::Cast;
        use granite::prelude::CastNone;

        let factory = gtk::SignalListItemFactory::new();
        factory.connect_setup(move |_, list_item| {
            let label = gtk::Label::new(None);
            list_item
                .downcast_ref::<gtk::ListItem>()
                .expect("Needs to be ListItem")
                .set_child(Some(&label));
        });

        factory.connect_bind(move |_, list_item| {
            // Get `BoxedAnyObject` from `ListItem`
            let boxed = list_item
                .downcast_ref::<gtk::ListItem>()
                .expect("Needs to be ListItem")
                .item()
                .and_downcast::<glib::BoxedAnyObject>()
                .expect("The item has to be an `IntegerObject`.");

            // Get `Label` from `ListItem`
            let label = list_item
                .downcast_ref::<gtk::ListItem>()
                .expect("Needs to be ListItem")
                .child()
                .and_downcast::<gtk::Label>()
                .expect("The child has to be a `Label`.");

            // Set "label" to "number"
            label.set_label(&boxed.borrow::<OrderRow>().label);
        });

        factory
    }
}

#[derive(Debug)]
pub struct FilterRow {
    filter: Option<Filter>,
    index: relm4::factory::DynamicIndex,
}

#[derive(Debug)]
pub enum FilterRowIn {
    ParameterChanged,
    RemoveFilter,
}

#[derive(Debug)]
pub enum FilterRowOut {
    ParameterChanged,
    RemoveFilter(relm4::factory::DynamicIndex),
}

#[relm4::factory(pub)]
impl relm4::factory::FactoryComponent for FilterRow {
    type Init = ();
    type Input = FilterRowIn;
    type Output = FilterRowOut;
    type ParentWidget = gtk::ListBox;
    type CommandOutput = ();

    fn init_model(
        _init: Self::Init,
        index: &relm4::factory::DynamicIndex,
        _sender: relm4::FactorySender<Self>,
    ) -> Self {
        Self {
            filter: None,
            index: index.clone(),
        }
    }

    view! {
        gtk::ListBoxRow {
            set_selectable: false,
            set_activatable: false,
            set_margin_top: 5,
            set_margin_bottom: 5,

            gtk::CenterBox {
                #[wrap(Some)]
                set_start_widget = &gtk::Label {
                    set_text: "Year",
                },
                #[wrap(Some)]
                set_center_widget = &gtk::DropDown {
                    set_focus_on_click: false,
                    set_margin_start: 15,
                    set_margin_end: 15,
                    set_model: Some(&Filter::order()),
                    set_factory: Some(&Filter::factory()),
                },
                #[wrap(Some)]
                set_end_widget = &gtk::Box {
                    set_spacing: 5,

                    gtk::Entry {
                        set_focus_on_click: false,
                        set_placeholder_text: Some("2000"),

                        connect_text_notify => Self::Input::ParameterChanged,
                    },
                    gtk::Button {
                        set_icon_name: "user-trash-symbolic",
                        set_tooltip_text: Some("remove this filter"),

                        connect_clicked => Self::Input::RemoveFilter,
                    }
                }
            }
        }
    }

    fn update(&mut self, msg: Self::Input, sender: relm4::FactorySender<Self>) {
        match msg {
            Self::Input::RemoveFilter => sender
                .output(FilterRowOut::RemoveFilter(self.index.clone()))
                .expect("sending failed"),
            Self::Input::ParameterChanged => sender
                .output(Self::Output::ParameterChanged)
                .expect("sending failed"),
            _ => {}
        }
    }
}
