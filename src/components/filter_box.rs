use std::cmp::Ordering;

use gstreamer::glib::BoxedAnyObject;
use gtk::prelude::OrientableExt;
use relm4::gtk::{
    self, glib,
    prelude::{BoxExt, ButtonExt, EntryExt, ListItemExt, WidgetExt},
};

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
    fn order() -> gtk::gio::ListStore {
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
        let store = gtk::gio::ListStore::new::<BoxedAnyObject>();
        for d in data {
            store.append(&BoxedAnyObject::new(d));
        }
        store
    }

    fn factory() -> gtk::SignalListItemFactory {
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
                .and_downcast::<BoxedAnyObject>()
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

#[derive(Debug, Default)]
pub struct FilterBox {
    categories: Vec<Category>,
    filters: Vec<Filter>,
}

#[derive(Debug)]
pub enum FilterBoxIn {
    ClearFilters,
    Favorited(bool),
}

#[derive(Debug)]
pub enum FilterBoxOut {
    FiltersChanged,
}

#[relm4::component(pub)]
impl relm4::SimpleComponent for FilterBox {
    type Init = Vec<Category>;
    type Input = FilterBoxIn;
    type Output = FilterBoxOut;

    fn init(
        init: Self::Init,
        root: Self::Root,
        sender: relm4::ComponentSender<Self>,
    ) -> relm4::ComponentParts<Self> {
        let model = Self::default();

        let widgets = view_output!();

        relm4::ComponentParts { model, widgets }
    }

    view! {
        #[root]
        gtk::Box {
            add_css_class: "filter-box",
            set_orientation: gtk::Orientation::Vertical,
            set_spacing: 10,

            gtk::Label {
                add_css_class: granite::STYLE_CLASS_H3_LABEL,
                set_hexpand: true,
                set_halign: gtk::Align::Center,
                set_text: "Active Filters",
            },

            gtk::CenterBox {
                #[wrap(Some)]
                set_start_widget = &gtk::Label {
                    set_margin_end: 15,
                    set_text: "Show Favorites",
                },
                #[wrap(Some)]
                set_end_widget = &gtk::Switch {
                    set_margin_start: 15,
                    connect_state_set[sender] => move |_btn, state| {
                        sender.input(Self::Input::Favorited(state));
                        glib::signal::Propagation::Proceed
                    }
                }
            },

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
                    gtk::Entry {
                        set_focus_on_click: false,
                        set_placeholder_text: Some("1990"),
                    },
                    gtk::Button {
                        set_icon_name: "user-trash-symbolic",
                    }
                }

            },

            gtk::Button {
                set_label: "Add new filter",
            }
        }
    }

    fn update(&mut self, msg: Self::Input, sender: relm4::ComponentSender<Self>) {
        match msg {
            Self::Input::ClearFilters => {
                self.filters.clear();
                sender
                    .output(Self::Output::FiltersChanged)
                    .expect("sending failed");
            }
            Self::Input::Favorited(value) => {
                self.filters.push(Filter::Favorited(value));
                sender
                    .output(Self::Output::FiltersChanged)
                    .expect("sending failed");
            }
        }
    }
}
