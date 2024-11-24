use std::cmp::Ordering;

use relm4::gtk::{
    self, gio, glib,
    prelude::{BoxExt, ButtonExt, EditableExt, EntryExt, ListBoxRowExt, ListItemExt, WidgetExt},
};

use crate::components::filter_categories::Category;
use crate::{common::store_from_category, gtk_helper::stack::StackExt};

#[derive(Debug, Clone)]
pub enum Filter {
    Favorite(Option<bool>),
    Title(TextRelation, String),
    Year(Ordering, i32),
    Cd(Ordering, i32),
    TrackNumber(Ordering, usize),
    Artist(TextRelation, String),
    Album(TextRelation, String),
    Genre(TextRelation, String),
    BitRate(Ordering, usize),
    Duration(Ordering, i32),
    AlbumCount(Ordering, i32),
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct BoolRow {
    relation: Option<bool>,
    label: String,
}

impl BoolRow {
    pub fn store() -> gio::ListStore {
        let data: [BoolRow; 3] = [
            BoolRow {
                relation: None,
                label: String::from("both"),
            },
            BoolRow {
                relation: Some(true),
                label: String::from("yes"),
            },
            BoolRow {
                relation: Some(false),
                label: String::from("no"),
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
            label.set_label(&boxed.borrow::<BoolRow>().label);
        });

        factory
    }
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
                label: String::from("contains"),
            },
            TextRow {
                relation: TextRelation::ContainsNot,
                label: String::from("contains not"),
            },
            TextRow {
                relation: TextRelation::ExactNot,
                label: String::from("matches not"),
            },
            TextRow {
                relation: TextRelation::Exact,
                label: String::from("matches"),
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
                label: String::from("="),
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
}

impl FilterRow {
    pub fn filter(&self) -> &Option<Filter> {
        &self.filter
    }

    pub fn active(&self) -> bool {
        match &self.filter {
            Some(Filter::Favorite(None)) => true,
            Some(Filter::Album(_, value)) if !value.is_empty() => true,
            Some(Filter::Artist(_, value)) if !value.is_empty() => true,
            Some(Filter::Genre(_, value)) if !value.is_empty() => true,
            Some(Filter::Title(_, value)) if !value.is_empty() => true,
            _ => false,
        }
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
        }
    }

    view! {
        gtk::ListBoxRow {
            set_selectable: false,
            set_activatable: false,
            set_focusable: false,

            self.stack.clone() {
                set_margin_start: 10,
                set_margin_end: 10,
                set_valign: gtk::Align::Center,

                add_enumed[Category::Favorite] = &gtk::Box {
                    set_spacing: 5,
                    gtk::Box {
                        set_hexpand: true,

                        gtk::Label {
                            set_text: "Favorites",
                        },
                    },
                    #[name = "favorites"]
                    gtk::DropDown {
                        set_model: Some(&BoolRow::store()),
                        set_factory: Some(&BoolRow::factory()),
                        connect_selected_item_notify => Self::Input::ParameterChanged,
                    },
                },
                add_enumed[Category::Year] = &gtk::Box {
                    set_spacing: 5,
                    gtk::Box {
                        set_hexpand: true,

                        gtk::Label {
                            set_text: "Year",
                        },
                    },

                    #[name = "year_dropdown"]
                    gtk::DropDown {
                        set_model: Some(&OrderRow::store()),
                        set_factory: Some(&OrderRow::factory()),
                        connect_selected_item_notify => Self::Input::ParameterChanged,
                    },

                    #[name = "year_entry"]
                    gtk::SpinButton {
                        set_digits: 0,
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
                    set_spacing: 5,
                    gtk::Box {
                        set_hexpand: true,

                        gtk::Label {
                            set_text: "CD number",
                        },
                    },

                    #[name = "cd_dropdown"]
                    gtk::DropDown {
                        set_model: Some(&OrderRow::store()),
                        set_factory: Some(&OrderRow::factory()),
                        connect_selected_item_notify => Self::Input::ParameterChanged,
                    },

                    #[name = "cd_entry"]
                    gtk::SpinButton {
                        set_digits: 0,
                        set_adjustment: &gtk::Adjustment::new(0f64, 0f64, 3000f64, 1f64, 1f64, 1f64),
                        connect_text_notify => Self::Input::ParameterChanged,
                    },
                    gtk::Button {
                        set_icon_name: "user-trash-symbolic",
                        set_tooltip_text: Some("remove this filter"),
                        connect_clicked => Self::Input::RemoveFilter,
                    }
                },
                add_enumed[Category::TrackNumber] = &gtk::Box {
                    set_spacing: 5,
                    gtk::Box {
                        set_hexpand: true,

                        gtk::Label {
                            set_text: "Track number",
                        }
                    },

                    #[name = "track_number_dropdown"]
                    gtk::DropDown {
                        set_model: Some(&OrderRow::store()),
                        set_factory: Some(&OrderRow::factory()),
                        connect_selected_item_notify => Self::Input::ParameterChanged,
                    },

                    #[name = "track_number_entry"]
                    gtk::SpinButton {
                        set_digits: 0,
                        set_adjustment: &gtk::Adjustment::new(0f64, 0f64, 3000f64, 1f64, 1f64, 1f64),
                        connect_text_notify => Self::Input::ParameterChanged,
                    },
                    gtk::Button {
                        set_icon_name: "user-trash-symbolic",
                        set_tooltip_text: Some("remove this filter"),
                        connect_clicked => Self::Input::RemoveFilter,
                    }
                },
                add_enumed[Category::AlbumCount] = &gtk::Box {
                    set_spacing: 5,

                    gtk::Box {
                        set_hexpand: true,

                        gtk::Label {
                            set_text: "Album count",
                        }
                    },

                    #[name = "album_count_dropdown"]
                    gtk::DropDown {
                        set_model: Some(&OrderRow::store()),
                        set_factory: Some(&OrderRow::factory()),
                        connect_selected_item_notify => Self::Input::ParameterChanged,
                    },

                    #[name = "album_count_entry"]
                    gtk::SpinButton {
                        set_digits: 0,
                        set_adjustment: &gtk::Adjustment::new(0f64, 0f64, 3000f64, 1f64, 1f64, 1f64),
                        connect_text_notify => Self::Input::ParameterChanged,
                    },
                    gtk::Button {
                        set_icon_name: "user-trash-symbolic",
                        set_tooltip_text: Some("remove this filter"),
                        connect_clicked => Self::Input::RemoveFilter,
                    }
                },
                add_enumed[Category::Duration] = &gtk::Box {
                    set_spacing: 5,
                    gtk::Box {
                        set_hexpand: true,

                        gtk::Label {
                            set_text: "Duration",
                        },
                    },

                    #[name = "duration_dropdown"]
                    gtk::DropDown {
                        set_model: Some(&OrderRow::store()),
                        set_factory: Some(&OrderRow::factory()),
                        connect_selected_item_notify => Self::Input::ParameterChanged,
                    },

                    #[name = "duration_entry"]
                    gtk::SpinButton {
                        set_digits: 0,
                        set_adjustment: &gtk::Adjustment::new(0f64, 0f64, 3000f64, 1f64, 1f64, 1f64),
                        connect_text_notify => Self::Input::ParameterChanged,
                    },
                    gtk::Button {
                        set_icon_name: "user-trash-symbolic",
                        set_tooltip_text: Some("remove this filter"),
                        connect_clicked => Self::Input::RemoveFilter,
                    }
                },
                add_enumed[Category::BitRate] = &gtk::Box {
                    set_spacing: 5,
                    gtk::Box {
                        set_hexpand: true,

                        gtk::Label {
                            set_text: "Bit rate",
                        },
                    },

                    #[name = "bit_rate_dropdown"]
                    gtk::DropDown {
                        set_model: Some(&OrderRow::store()),
                        set_factory: Some(&OrderRow::factory()),
                        connect_selected_item_notify => Self::Input::ParameterChanged,
                    },
                    #[name = "bit_rate_entry"]
                    gtk::SpinButton {
                        set_digits: 0,
                        set_adjustment: &gtk::Adjustment::new(124f64, 0f64, 3000f64, 4f64, 1f64, 1f64),
                        connect_text_notify => Self::Input::ParameterChanged,
                    },
                    gtk::Button {
                        set_icon_name: "user-trash-symbolic",
                        set_tooltip_text: Some("remove this filter"),
                        connect_clicked => Self::Input::RemoveFilter,
                    }
                },
                add_enumed[Category::Title] = &gtk::Box {
                    set_spacing: 5,
                    gtk::Box {
                        set_hexpand: true,

                        gtk::Label {
                            set_text: "Title name",
                        },
                    },

                    #[name = "title_dropdown"]
                    gtk::DropDown {
                        set_model: Some(&TextRow::store()),
                        set_factory: Some(&TextRow::factory()),
                        connect_selected_item_notify => Self::Input::ParameterChanged,
                    },
                    #[name = "title_entry"]
                    gtk::Entry {
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
                    set_spacing: 5,
                    gtk::Box {
                        set_hexpand: true,

                        gtk::Label {
                            set_text: "Artist name",
                        }
                    },

                    #[name = "artist_dropdown"]
                    gtk::DropDown {
                        set_model: Some(&TextRow::store()),
                        set_factory: Some(&TextRow::factory()),
                        connect_selected_item_notify => Self::Input::ParameterChanged,
                    },
                    #[name = "artist_entry"]
                    gtk::Entry {
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
                    set_spacing: 5,
                    gtk::Box {
                        set_hexpand: true,

                        gtk::Label {
                            set_text: "Album name",
                        }
                    },

                    #[name = "album_dropdown"]
                    gtk::DropDown {
                        set_model: Some(&TextRow::store()),
                        set_factory: Some(&TextRow::factory()),
                        connect_selected_item_notify => Self::Input::ParameterChanged,
                    },
                    #[name = "album_entry"]
                    gtk::Entry {
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
                    set_spacing: 5,

                    gtk::Box {
                        set_hexpand: true,

                        gtk::Label {
                            set_text: "Genre",
                        }
                    },

                    #[name = "genre_dropdown"]
                    gtk::DropDown {
                        set_model: Some(&TextRow::store()),
                        set_factory: Some(&TextRow::factory()),
                        connect_selected_item_notify => Self::Input::ParameterChanged,
                    },
                    #[name = "genre_entry"]
                    gtk::Entry {
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

    fn update_with_view(
        &mut self,
        widgets: &mut Self::Widgets,
        msg: Self::Input,
        sender: relm4::FactorySender<Self>,
    ) {
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
                    Category::Favorite => {
                        let relation = widgets.favorites.selected_item().unwrap();
                        let relation = relation
                            .downcast_ref::<glib::BoxedAnyObject>()
                            .expect("Needs to be ListItem");
                        let relation: std::cell::Ref<BoolRow> = relation.borrow();
                        self.filter = Some(Filter::Favorite(relation.relation));
                    }
                    Category::Title => {
                        let relation = widgets.title_dropdown.selected_item().unwrap();
                        let relation = relation
                            .downcast_ref::<glib::BoxedAnyObject>()
                            .expect("Needs to be ListItem");
                        let relation: std::cell::Ref<TextRow> = relation.borrow();

                        self.filter = Some(Filter::Title(
                            relation.relation.clone(),
                            widgets.title_entry.text().into(),
                        ))
                    }
                    Category::Artist => {
                        let relation = widgets.artist_dropdown.selected_item().unwrap();
                        let relation = relation
                            .downcast_ref::<glib::BoxedAnyObject>()
                            .expect("Needs to be ListItem");
                        let relation: std::cell::Ref<TextRow> = relation.borrow();

                        self.filter = Some(Filter::Artist(
                            relation.relation.clone(),
                            widgets.artist_entry.text().into(),
                        ))
                    }
                    Category::Album => {
                        let relation = widgets.album_dropdown.selected_item().unwrap();
                        let relation = relation
                            .downcast_ref::<glib::BoxedAnyObject>()
                            .expect("Needs to be ListItem");
                        let relation: std::cell::Ref<TextRow> = relation.borrow();

                        self.filter = Some(Filter::Album(
                            relation.relation.clone(),
                            widgets.album_entry.text().into(),
                        ))
                    }
                    Category::Genre => {
                        let relation = widgets.genre_dropdown.selected_item().unwrap();
                        let relation = relation
                            .downcast_ref::<glib::BoxedAnyObject>()
                            .expect("Needs to be ListItem");
                        let relation: std::cell::Ref<TextRow> = relation.borrow();

                        self.filter = Some(Filter::Genre(
                            relation.relation.clone(),
                            widgets.genre_entry.text().into(),
                        ))
                    }
                    Category::Year => {
                        let order = widgets.year_dropdown.selected_item().unwrap();
                        let order = order
                            .downcast_ref::<glib::BoxedAnyObject>()
                            .expect("Needs to be ListItem");
                        let order: std::cell::Ref<OrderRow> = order.borrow();
                        if let Ok(number) = widgets.year_entry.text().parse::<i32>() {
                            self.filter = Some(Filter::Year(order.order, number));
                            widgets.year_entry.set_tooltip_text(None);
                        } else {
                            self.filter = None;
                        }
                    }
                    Category::Cd => {
                        let order = widgets.cd_dropdown.selected_item().unwrap();
                        let order = order
                            .downcast_ref::<glib::BoxedAnyObject>()
                            .expect("Needs to be ListItem");
                        let order: std::cell::Ref<OrderRow> = order.borrow();
                        if let Ok(number) = widgets.cd_entry.text().parse::<i32>() {
                            self.filter = Some(Filter::Cd(order.order, number));
                            widgets.cd_entry.set_tooltip_text(None);
                        } else {
                            self.filter = None;
                        }
                    }
                    Category::TrackNumber => {
                        let order = widgets.track_number_dropdown.selected_item().unwrap();
                        let order = order
                            .downcast_ref::<glib::BoxedAnyObject>()
                            .expect("Needs to be ListItem");
                        let order: std::cell::Ref<OrderRow> = order.borrow();
                        if let Ok(number) = widgets.track_number_entry.text().parse::<usize>() {
                            self.filter = Some(Filter::TrackNumber(order.order, number));
                            widgets.track_number_entry.set_tooltip_text(None);
                        } else {
                            self.filter = None;
                        }
                    }
                    Category::Duration => {
                        let order = widgets.duration_dropdown.selected_item().unwrap();
                        let order = order
                            .downcast_ref::<glib::BoxedAnyObject>()
                            .expect("Needs to be ListItem");
                        let order: std::cell::Ref<OrderRow> = order.borrow();
                        if let Ok(number) = widgets.duration_entry.text().parse::<i32>() {
                            self.filter = Some(Filter::Duration(order.order, number));
                            widgets.duration_entry.set_tooltip_text(None);
                        } else {
                            self.filter = None;
                        }
                    }
                    Category::BitRate => {
                        let order = widgets.bit_rate_dropdown.selected_item().unwrap();
                        let order = order
                            .downcast_ref::<glib::BoxedAnyObject>()
                            .expect("Needs to be ListItem");
                        let order: std::cell::Ref<OrderRow> = order.borrow();
                        if let Ok(number) = widgets.bit_rate_entry.text().parse::<usize>() {
                            self.filter = Some(Filter::BitRate(order.order, number));
                            widgets.bit_rate_entry.set_tooltip_text(None);
                        } else {
                            self.filter = None;
                        }
                    }
                    Category::AlbumCount => {
                        let order = widgets.album_count_dropdown.selected_item().unwrap();
                        let order = order
                            .downcast_ref::<glib::BoxedAnyObject>()
                            .expect("Needs to be ListItem");
                        let order: std::cell::Ref<OrderRow> = order.borrow();
                        if let Ok(number) = widgets.album_count_entry.text().parse::<i32>() {
                            self.filter = Some(Filter::AlbumCount(order.order, number));
                            widgets.album_count_entry.set_tooltip_text(None);
                        } else {
                            self.filter = None;
                        }
                    }
                }
                sender.output(Self::Output::ParameterChanged).unwrap();
            }
        }
    }
}
