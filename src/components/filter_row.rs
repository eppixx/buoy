use std::cmp::Ordering;

use relm4::{
    gtk::{
        self, gio, glib,
        prelude::{
            BoxExt, ButtonExt, EditableExt, EntryExt, ListBoxRowExt, ListItemExt, WidgetExt,
        },
    },
    RelmWidgetExt,
};

use crate::components::filter_categories::Category;
use crate::{common::store_from_category, gtk_helper::stack::StackExt};

#[derive(Debug, Clone)]
pub enum Filter {
    Favorite(bool),
    Title(TextRelation, String),
    Year(Ordering, i32),
    Cd(Ordering, i32),
    TrackNumber(Ordering, usize),
    Artist(TextRelation, String),
    Album(TextRelation, String),
    Genre(TextRelation, String),
    BitRate(Ordering, usize),
    Duration(Ordering, i32),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TextRelation {
    Contains,
    ContainsNot,
    ExactNot,
    Exact,
}

#[derive(Debug, Clone)]
struct TextRow {
    relation: TextRelation,
    label: String,
}

impl TextRow {
    pub fn store() -> gio::ListStore {
        let data: [TextRow; 4] = [
            TextRow {
                relation: TextRelation::Contains,
                label: String::from("Contains"),
            },
            TextRow {
                relation: TextRelation::ContainsNot,
                label: String::from("Contains not"),
            },
            TextRow {
                relation: TextRelation::ExactNot,
                label: String::from("Matches not"),
            },
            TextRow {
                relation: TextRelation::Exact,
                label: String::from("Matches"),
            },
        ];
        store_from_category(&data)
    }

    pub fn factory() -> gtk::SignalListItemFactory {
        use glib::object::Cast;
        use granite::prelude::CastNone;

        let factory = gtk::SignalListItemFactory::new();
        factory.connect_setup(move |_, list_item| {
            let label = gtk::Label::new(Some("Selection"));
            list_item
                .downcast_ref::<gtk::ListItem>()
                .expect("is not a ListItem")
                .set_child(Some(&label));
        });
        factory.connect_bind(move |_, list_item| {
            // get BoxedAnyObject from ListItem
            let boxed = list_item
                .downcast_ref::<gtk::ListItem>()
                .expect("ist not a ListItem")
                .item()
                .and_downcast::<glib::BoxedAnyObject>()
                .expect("item is not a BoxedAnyObject");
            // get label from ListItem
            let label = list_item
                .downcast_ref::<gtk::ListItem>()
                .expect("is not a ListItem")
                .child()
                .and_downcast::<gtk::Label>()
                .expect("is not a Label");
            // set label from OrderRow
            label.set_label(&boxed.borrow::<TextRow>().label);
        });

        factory
    }
}

#[derive(Debug, Clone)]
struct OrderRow {
    order: Ordering,
    label: String,
}

// adapted from https://gtk-rs.org/gtk4-rs/stable/latest/book/list_widgets.html
impl OrderRow {
    pub fn store() -> gio::ListStore {
        let data: [OrderRow; 3] = [
            OrderRow {
                order: Ordering::Greater,
                label: String::from(">"),
            },
            OrderRow {
                order: Ordering::Equal,
                label: String::from("=="),
            },
            OrderRow {
                order: Ordering::Less,
                label: String::from("<"),
            },
        ];
        store_from_category(&data)
    }

    pub fn factory() -> gtk::SignalListItemFactory {
        use glib::object::Cast;
        use granite::prelude::CastNone;

        let factory = gtk::SignalListItemFactory::new();
        factory.connect_setup(move |_, list_item| {
            let label = gtk::Label::new(Some("Selection"));
            list_item
                .downcast_ref::<gtk::ListItem>()
                .expect("is not a ListItem")
                .set_child(Some(&label));
        });
        factory.connect_bind(move |_, list_item| {
            // get BoxedAnyObject from ListItem
            let boxed = list_item
                .downcast_ref::<gtk::ListItem>()
                .expect("ist not a ListItem")
                .item()
                .and_downcast::<glib::BoxedAnyObject>()
                .expect("item is not a BoxedAnyObject");
            // get label from ListItem
            let label = list_item
                .downcast_ref::<gtk::ListItem>()
                .expect("is not a ListItem")
                .child()
                .and_downcast::<gtk::Label>()
                .expect("is not a Label");
            // set label from OrderRow
            label.set_label(&boxed.borrow::<OrderRow>().label);
        });

        factory
    }
}

#[derive(Debug)]
pub struct FilterRow {
    category: Category,
    filter: Option<Filter>,
    index: relm4::factory::DynamicIndex,
    stack: gtk::Stack,

    year_entry: gtk::SpinButton,
    year_dropdown: gtk::DropDown,
    cd_entry: gtk::Entry,
    cd_dropdown: gtk::DropDown,
    track_number_entry: gtk::Entry,
    track_number_dropdown: gtk::DropDown,
    duration_entry: gtk::Entry,
    duration_dropdown: gtk::DropDown,
    bit_rate_entry: gtk::SpinButton,
    bit_rate_dropdown: gtk::DropDown,
    title_entry: gtk::Entry,
    title_dropdown: gtk::DropDown,
    artist_entry: gtk::Entry,
    artist_dropdown: gtk::DropDown,
    album_entry: gtk::Entry,
    album_dropdown: gtk::DropDown,
    genre_entry: gtk::Entry,
    genre_dropdown: gtk::DropDown,
}

impl FilterRow {
    pub fn filter(&self) -> &Option<Filter> {
        &self.filter
    }
}

#[derive(Debug)]
pub enum FilterRowIn {
    ParameterChanged,
    RemoveFilter,
    SetTo(Category),
}

#[derive(Debug)]
pub enum FilterRowOut {
    ParameterChanged,
    RemoveFilter(relm4::factory::DynamicIndex),
}

#[relm4::factory(pub)]
impl relm4::factory::FactoryComponent for FilterRow {
    type Init = Category;
    type Input = FilterRowIn;
    type Output = FilterRowOut;
    type ParentWidget = gtk::ListBox;
    type CommandOutput = ();

    fn init_model(
        init: Self::Init,
        index: &relm4::factory::DynamicIndex,
        _sender: relm4::FactorySender<Self>,
    ) -> Self {
        Self {
            category: init.clone(),
            filter: None,
            index: index.clone(),
            stack: gtk::Stack::default(),

            year_entry: gtk::SpinButton::default(),
            year_dropdown: gtk::DropDown::default(),
            cd_entry: gtk::Entry::default(),
            cd_dropdown: gtk::DropDown::default(),
            track_number_entry: gtk::Entry::default(),
            track_number_dropdown: gtk::DropDown::default(),
            duration_entry: gtk::Entry::default(),
            duration_dropdown: gtk::DropDown::default(),
            bit_rate_entry: gtk::SpinButton::default(),
            bit_rate_dropdown: gtk::DropDown::default(),
            title_entry: gtk::Entry::default(),
            title_dropdown: gtk::DropDown::default(),
            artist_entry: gtk::Entry::default(),
            artist_dropdown: gtk::DropDown::default(),
            album_entry: gtk::Entry::default(),
            album_dropdown: gtk::DropDown::default(),
            genre_entry: gtk::Entry::default(),
            genre_dropdown: gtk::DropDown::default(),
        }
    }

    view! {
        gtk::ListBoxRow {
            set_selectable: false,
            set_activatable: false,
            set_margin_all: 10,

            self.stack.clone() {
                add_enumed[Category::Year] = &gtk::Box {
                    set_spacing: 10,
                    gtk::Box {
                        set_hexpand: true,

                        gtk::Label {
                            set_text: "By year",
                        },
                    },

                    self.year_dropdown.clone() -> gtk::DropDown {
                        set_model: Some(&OrderRow::store()),
                        set_factory: Some(&OrderRow::factory()),
                        connect_selected_item_notify => Self::Input::ParameterChanged,
                    },

                    self.year_entry.clone() -> gtk::SpinButton {
                        set_digits: 0,
                        set_value: 2010f64,
                        set_adjustment: &gtk::Adjustment::new(2010f64, 0f64, 3000f64, 1f64, 1f64, 1f64),
                        connect_text_notify => Self::Input::ParameterChanged,
                    },
                    gtk::Button {
                        set_icon_name: "user-trash-symbolic",
                        set_tooltip_text: Some("remove this filter"),
                        connect_clicked => Self::Input::RemoveFilter,
                    }
                },
                add_enumed[Category::Cd] = &gtk::Box {
                    set_spacing: 10,
                    gtk::Box {
                        set_hexpand: true,

                        gtk::Label {
                            set_text: "By cd number",
                        },
                    },

                    self.cd_dropdown.clone() -> gtk::DropDown {
                        set_model: Some(&OrderRow::store()),
                        set_factory: Some(&OrderRow::factory()),
                        connect_selected_item_notify => Self::Input::ParameterChanged,
                    },

                    self.cd_entry.clone() -> gtk::Entry {
                        set_text: "0",
                        connect_text_notify => Self::Input::ParameterChanged,
                    },
                    gtk::Button {
                        set_icon_name: "user-trash-symbolic",
                        set_tooltip_text: Some("remove this filter"),
                        connect_clicked => Self::Input::RemoveFilter,
                    }
                },
                add_enumed[Category::TrackNumber] = &gtk::Box {
                    set_spacing: 10,
                    gtk::Box {
                        set_hexpand: true,

                        gtk::Label {
                            set_text: "By track number",
                        }
                    },

                    self.track_number_dropdown.clone() -> gtk::DropDown {
                        set_model: Some(&OrderRow::store()),
                        set_factory: Some(&OrderRow::factory()),
                        connect_selected_item_notify => Self::Input::ParameterChanged,
                    },

                    self.track_number_entry.clone() -> gtk::Entry {
                        set_text: "0",
                        connect_text_notify => Self::Input::ParameterChanged,
                    },
                    gtk::Button {
                        set_icon_name: "user-trash-symbolic",
                        set_tooltip_text: Some("remove this filter"),
                        connect_clicked => Self::Input::RemoveFilter,
                    }
                },
                add_enumed[Category::Duration] = &gtk::Box {
                    set_spacing: 10,
                    gtk::Box {
                        set_hexpand: true,

                        gtk::Label {
                            set_text: "By duration",
                        },
                    },

                    self.duration_dropdown.clone() -> gtk::DropDown {
                        set_model: Some(&OrderRow::store()),
                        set_factory: Some(&OrderRow::factory()),
                        connect_selected_item_notify => Self::Input::ParameterChanged,
                    },

                    self.duration_entry.clone() -> gtk::Entry {
                        set_text: "0",
                        connect_text_notify => Self::Input::ParameterChanged,
                    },
                    gtk::Button {
                        set_icon_name: "user-trash-symbolic",
                        set_tooltip_text: Some("remove this filter"),
                        connect_clicked => Self::Input::RemoveFilter,
                    }
                },
                add_enumed[Category::BitRate] = &gtk::Box {
                    set_spacing: 10,
                    gtk::Box {
                        set_hexpand: true,

                        gtk::Label {
                            set_text: "By bit rate",
                        },
                    },

                    self.bit_rate_dropdown.clone() -> gtk::DropDown {
                        set_model: Some(&OrderRow::store()),
                        set_factory: Some(&OrderRow::factory()),
                        connect_selected_item_notify => Self::Input::ParameterChanged,
                    },
                    self.bit_rate_entry.clone() -> gtk::SpinButton {
                        set_digits: 0,
                        set_value: 128f64,
                        set_adjustment: &gtk::Adjustment::new(128f64, 0f64, 3000f64, 4f64, 1f64, 1f64),
                        connect_text_notify => Self::Input::ParameterChanged,
                    },
                    gtk::Button {
                        set_icon_name: "user-trash-symbolic",
                        set_tooltip_text: Some("remove this filter"),
                        connect_clicked => Self::Input::RemoveFilter,
                    }
                },
                add_enumed[Category::Title] = &gtk::Box {
                    set_spacing: 10,
                    gtk::Box {
                        set_hexpand: true,

                        gtk::Label {
                            set_text: "By title name",
                        },
                    },

                    self.title_dropdown.clone() -> gtk::DropDown {
                        set_model: Some(&TextRow::store()),
                        set_factory: Some(&TextRow::factory()),
                        connect_selected_item_notify => Self::Input::ParameterChanged,
                    },
                    self.title_entry.clone() -> gtk::Entry {
                        set_text: "",
                        set_placeholder_text: Some("Title"),
                        connect_text_notify => Self::Input::ParameterChanged,
                    },
                    gtk::Button {
                        set_icon_name: "user-trash-symbolic",
                        set_tooltip_text: Some("remove this filter"),
                        connect_clicked => Self::Input::RemoveFilter,
                    }
                },
                add_enumed[Category::Artist] = &gtk::Box {
                    set_spacing: 10,
                    gtk::Box {
                        set_hexpand: true,

                        gtk::Label {
                            set_text: "By artist name",
                        }
                    },

                    self.artist_dropdown.clone() -> gtk::DropDown {
                        set_model: Some(&TextRow::store()),
                        set_factory: Some(&TextRow::factory()),
                        connect_selected_item_notify => Self::Input::ParameterChanged,
                    },
                    self.artist_entry.clone() -> gtk::Entry {
                        set_text: "",
                        set_placeholder_text: Some("Artist"),
                        connect_text_notify => Self::Input::ParameterChanged,
                    },
                    gtk::Button {
                        set_icon_name: "user-trash-symbolic",
                        set_tooltip_text: Some("remove this filter"),
                        connect_clicked => Self::Input::RemoveFilter,
                    }
                },
                add_enumed[Category::Album] = &gtk::Box {
                    set_spacing: 10,
                    gtk::Box {
                        set_hexpand: true,

                        gtk::Label {
                            set_text: "By album name",
                        }
                    },

                    self.album_dropdown.clone() -> gtk::DropDown {
                        set_model: Some(&TextRow::store()),
                        set_factory: Some(&TextRow::factory()),
                        connect_selected_item_notify => Self::Input::ParameterChanged,
                    },
                    self.album_entry.clone() -> gtk::Entry {
                        set_text: "",
                        set_placeholder_text: Some("Album"),
                        connect_text_notify => Self::Input::ParameterChanged,
                    },
                    gtk::Button {
                        set_icon_name: "user-trash-symbolic",
                        set_tooltip_text: Some("remove this filter"),
                        connect_clicked => Self::Input::RemoveFilter,
                    }
                },
                add_enumed[Category::Genre] = &gtk::Box {
                    set_spacing: 10,

                    gtk::Box {
                        set_hexpand: true,

                        gtk::Label {
                            set_text: "By genre",
                        }
                    },

                    self.genre_dropdown.clone() -> gtk::DropDown {
                        set_model: Some(&TextRow::store()),
                        set_factory: Some(&TextRow::factory()),
                        connect_selected_item_notify => Self::Input::ParameterChanged,
                    },
                    self.genre_entry.clone() -> gtk::Entry {
                        set_text: "",
                        set_placeholder_text: Some("Genre"),
                        connect_text_notify => Self::Input::ParameterChanged,
                    },
                    gtk::Button {
                        set_icon_name: "user-trash-symbolic",
                        set_tooltip_text: Some("remove this filter"),
                        connect_clicked => Self::Input::RemoveFilter,
                    }
                },
            }
        }
    }

    fn update(&mut self, msg: Self::Input, sender: relm4::FactorySender<Self>) {
        match msg {
            Self::Input::RemoveFilter => sender
                .output(FilterRowOut::RemoveFilter(self.index.clone()))
                .unwrap(),
            Self::Input::SetTo(category) => {
                self.stack.set_visible_child_enum(&category);
            }
            Self::Input::ParameterChanged => {
                use glib::object::Cast;

                // update local filter
                match &self.category {
                    Category::Title => {
                        let relation = self.title_dropdown.selected_item().unwrap();
                        let relation = relation
                            .downcast_ref::<glib::BoxedAnyObject>()
                            .expect("Needs to be ListItem");
                        let relation: std::cell::Ref<TextRow> = relation.borrow();

                        self.filter = Some(Filter::Title(
                            relation.relation.clone(),
                            self.title_entry.text().into(),
                        ))
                    }
                    Category::Artist => {
                        let relation = self.artist_dropdown.selected_item().unwrap();
                        let relation = relation
                            .downcast_ref::<glib::BoxedAnyObject>()
                            .expect("Needs to be ListItem");
                        let relation: std::cell::Ref<TextRow> = relation.borrow();

                        self.filter = Some(Filter::Artist(
                            relation.relation.clone(),
                            self.artist_entry.text().into(),
                        ))
                    }
                    Category::Album => {
                        let relation = self.album_dropdown.selected_item().unwrap();
                        let relation = relation
                            .downcast_ref::<glib::BoxedAnyObject>()
                            .expect("Needs to be ListItem");
                        let relation: std::cell::Ref<TextRow> = relation.borrow();

                        self.filter = Some(Filter::Album(
                            relation.relation.clone(),
                            self.album_entry.text().into(),
                        ))
                    }
                    Category::Genre => {
                        let relation = self.genre_dropdown.selected_item().unwrap();
                        let relation = relation
                            .downcast_ref::<glib::BoxedAnyObject>()
                            .expect("Needs to be ListItem");
                        let relation: std::cell::Ref<TextRow> = relation.borrow();

                        self.filter = Some(Filter::Genre(
                            relation.relation.clone(),
                            self.genre_entry.text().into(),
                        ))
                    }
                    Category::Year => {
                        let order = self.year_dropdown.selected_item().unwrap();
                        let order = order
                            .downcast_ref::<glib::BoxedAnyObject>()
                            .expect("Needs to be ListItem");
                        let order: std::cell::Ref<OrderRow> = order.borrow();
                        if let Ok(number) = self.year_entry.text().parse::<i32>() {
                            self.filter = Some(Filter::Year(order.order, number));
                            self.year_entry.set_tooltip_text(None);
                        self.year_entry.remove_css_class("entry-error");
                        } else {
                            self.filter = None;
                            self.year_entry.add_css_class("entry-error");
                            self.year_entry
                                .set_tooltip_text(Some("Please enter a valid number"));
                        }
                    }
                    Category::Cd => {
                        let order = self.cd_dropdown.selected_item().unwrap();
                        let order = order
                            .downcast_ref::<glib::BoxedAnyObject>()
                            .expect("Needs to be ListItem");
                        let order: std::cell::Ref<OrderRow> = order.borrow();
                        if let Ok(number) = self.cd_entry.text().parse::<i32>() {
                            self.filter = Some(Filter::Cd(order.order, number));
                            self.cd_entry.set_secondary_icon_name(None);
                            self.cd_entry.set_tooltip_text(None);
                        } else {
                            self.filter = None;
                            self.cd_entry
                                .set_secondary_icon_name(Some("dialog-error-symbolic"));
                            self.cd_entry.set_secondary_icon_tooltip_text(Some(
                                "Needs to input a valid number",
                            ));
                            self.cd_entry
                                .set_tooltip_text(Some("Needs to input a valid number"));
                        }
                    }
                    Category::TrackNumber => {
                        let order = self.track_number_dropdown.selected_item().unwrap();
                        let order = order
                            .downcast_ref::<glib::BoxedAnyObject>()
                            .expect("Needs to be ListItem");
                        let order: std::cell::Ref<OrderRow> = order.borrow();
                        if let Ok(number) = self.track_number_entry.text().parse::<usize>() {
                            self.filter = Some(Filter::TrackNumber(order.order, number));
                            self.track_number_entry.set_secondary_icon_name(None);
                            self.track_number_entry.set_tooltip_text(None);
                        } else {
                            self.filter = None;
                            self.track_number_entry
                                .set_secondary_icon_name(Some("dialog-error-symbolic"));
                            self.track_number_entry
                                .set_secondary_icon_tooltip_text(Some(
                                    "Needs to input a valid number",
                                ));
                            self.track_number_entry
                                .set_tooltip_text(Some("Needs to input a valid number"));
                        }
                    }
                    Category::Duration => {
                        let order = self.duration_dropdown.selected_item().unwrap();
                        let order = order
                            .downcast_ref::<glib::BoxedAnyObject>()
                            .expect("Needs to be ListItem");
                        let order: std::cell::Ref<OrderRow> = order.borrow();
                        if let Ok(number) = self.duration_entry.text().parse::<i32>() {
                            self.filter = Some(Filter::Duration(order.order, number));
                            self.duration_entry.set_secondary_icon_name(None);
                            self.duration_entry.set_tooltip_text(None);
                        } else {
                            self.filter = None;
                            self.duration_entry
                                .set_secondary_icon_name(Some("dialog-error-symbolic"));
                            self.duration_entry.set_secondary_icon_tooltip_text(Some(
                                "Needs to input a valid number",
                            ));
                            self.duration_entry
                                .set_tooltip_text(Some("Needs to input a valid number"));
                        }
                    }
                    Category::BitRate => {
                        let order = self.bit_rate_dropdown.selected_item().unwrap();
                        let order = order
                            .downcast_ref::<glib::BoxedAnyObject>()
                            .expect("Needs to be ListItem");
                        let order: std::cell::Ref<OrderRow> = order.borrow();
                        if let Ok(number) = self.bit_rate_entry.text().parse::<usize>() {
                            self.filter = Some(Filter::BitRate(order.order, number));
                            self.bit_rate_entry.set_tooltip_text(None);
                            self.bit_rate_entry.remove_css_class("entry-error");
                        } else {
                            self.filter = None;
                            self.bit_rate_entry.add_css_class("entry-error");
                            self.bit_rate_entry
                                .set_tooltip_text(Some("Please enter a valid number"));
                        }
                    }
                }
                sender.output(Self::Output::ParameterChanged).unwrap();
            }
        }
    }
}
