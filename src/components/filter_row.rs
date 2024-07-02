use relm4::gtk::{
    self, gio, glib,
    prelude::{BoxExt, ButtonExt, EditableExt, EntryExt, ListBoxRowExt, ListItemExt, WidgetExt},
};

use std::cmp::Ordering;

#[derive(Debug, Clone)]
pub enum Category {
    Title,
    Year,
    Cd,
    TrackNumber,
    Artist,
    Album,
    Genre,
    Duration,
    BitRate,
}

impl std::fmt::Display for Category {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Title => write!(f, "Title"),
            Self::Year => write!(f, "Year"),
            Self::Cd => write!(f, "Cd"),
            Self::TrackNumber => write!(f, "Track Number"),
            Self::Artist => write!(f, "Artist"),
            Self::Album => write!(f, "Album"),
            Self::Genre => write!(f, "Genre"),
            Self::Duration => write!(f, "Duration"),
            Self::BitRate => write!(f, "Bit Rate"),
        }
    }
}

impl Category {
    pub fn all() -> Vec<Self> {
        vec![
            Self::Title,
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

    pub fn albums_view() -> Vec<Self> {
        vec![
            Self::Title,
            Self::Year,
            Self::Cd,
            Self::Artist,
            Self::Album,
            Self::Genre,
            Self::Duration,
        ]
    }

    pub fn to_str(&self) -> &str {
        match self {
            Self::Title => "title",
            Self::Year => "year",
            Self::Cd => "cd",
            Self::TrackNumber => "track_number",
            Self::Artist => "artist",
            Self::Album => "Album",
            Self::Genre => "genre",
            Self::Duration => "duration",
            Self::BitRate => "bit_rate",
        }
    }

    pub fn store(categories: Vec<Self>) -> gio::ListStore {
        let store = gio::ListStore::new::<glib::BoxedAnyObject>();
        for category in categories.iter() {
            store.append(&glib::BoxedAnyObject::new(category.clone()));
        }
        store
    }

    pub fn factory() -> gtk::SignalListItemFactory {
        use glib::object::Cast;
        use granite::prelude::CastNone;

        let factory = gtk::SignalListItemFactory::new();
        factory.connect_setup(move |_, list_item| {
            let label = gtk::Label::new(Some("Category"));
            list_item
                .downcast_ref::<gtk::ListItem>()
                .expect("is not a ListItem")
                .set_child(Some(&label));
        });
        factory.connect_bind(move |_, list_item| {
            // get BoxedAnyObject from ListItem
            let boxed = list_item
                .downcast_ref::<gtk::ListItem>()
                .expect("is not a ListItem")
                .item()
                .and_downcast::<glib::BoxedAnyObject>()
                .expect("item is not a BoxedAnyObject");
            // get label from ListItem
            let label = list_item
                .downcast_ref::<gtk::ListItem>()
                .expect("is not a ListItem")
                .child()
                .and_downcast::<gtk::Label>()
                .expect("child has to be a Label");
            // set label from category
            label.set_label(&boxed.borrow::<Category>().to_string());
        });

        factory
    }
}

#[derive(Debug, Clone)]
pub enum Filter {
    Favorite(bool),
    Title(String),
    Year(Ordering, usize),
    Cd(Ordering, usize),
    TrackNumber(Ordering, usize),
    Artist(String),
    Album(String),
    Genre(String),
    BitRate(Ordering, usize),
    Duration(Ordering, usize),
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
                label: String::from("greater than"),
            },
            OrderRow {
                order: Ordering::Equal,
                label: String::from("equal to"),
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
            let label = gtk::Label::new(Some("Selection"));
            list_item
                .downcast_ref::<gtk::ListItem>()
                .expect("is not a
ListItem")
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

    year_entry: gtk::Entry,
    year_dropdown: gtk::DropDown,
    cd_entry: gtk::Entry,
    cd_dropdown: gtk::DropDown,
    track_number_entry: gtk::Entry,
    track_number_dropdown: gtk::DropDown,
    duration_entry: gtk::Entry,
    duration_dropdown: gtk::DropDown,
    bit_rate_entry: gtk::Entry,
    bit_rate_dropdown: gtk::DropDown,
    title_entry: gtk::Entry,
    artist_entry: gtk::Entry,
    album_entry: gtk::Entry,
    genre_entry: gtk::Entry,
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

            year_entry: gtk::Entry::default(),
            year_dropdown: gtk::DropDown::default(),
            cd_entry: gtk::Entry::default(),
            cd_dropdown: gtk::DropDown::default(),
            track_number_entry: gtk::Entry::default(),
            track_number_dropdown: gtk::DropDown::default(),
            duration_entry: gtk::Entry::default(),
            duration_dropdown: gtk::DropDown::default(),
            bit_rate_entry: gtk::Entry::default(),
            bit_rate_dropdown: gtk::DropDown::default(),
            title_entry: gtk::Entry::default(),
            artist_entry: gtk::Entry::default(),
            album_entry: gtk::Entry::default(),
            genre_entry: gtk::Entry::default(),
        }
    }

    view! {
        gtk::ListBoxRow {
            set_selectable: false,
            set_activatable: false,
            set_margin_top: 10,
            set_margin_bottom: 10,

            self.stack.clone() {
                add_named[Some(Category::Year.to_str())] = &gtk::CenterBox {
                    #[wrap(Some)]
                    set_start_widget = &gtk::Label {
                        set_text: "By year",
                    },
                    #[wrap(Some)]
                    set_center_widget = &self.year_dropdown.clone() -> gtk::DropDown {
                        set_focus_on_click: false,
                        set_margin_start: 15,
                        set_margin_end: 15,
                        set_model: Some(&OrderRow::store()),
                        set_factory: Some(&OrderRow::factory()),
                        connect_selected_item_notify => Self::Input::ParameterChanged,
                    },
                    #[wrap(Some)]
                    set_end_widget = &gtk::Box {
                        set_spacing: 5,
                        self.year_entry.clone() -> gtk::Entry {
                            set_focus_on_click: false,
                            set_text: "0",
                            connect_text_notify => Self::Input::ParameterChanged,
                        },
                        gtk::Button {
                            set_icon_name: "user-trash-symbolic",
                            set_tooltip_text: Some("remove this filter"),
                            connect_clicked => Self::Input::RemoveFilter,
                        }
                    }
                },
                add_named[Some(Category::Cd.to_str())] = &gtk::CenterBox {
                    #[wrap(Some)]
                    set_start_widget = &gtk::Label {
                        set_text: "By cd number",
                    },
                    #[wrap(Some)]
                    set_center_widget = &self.cd_dropdown.clone() -> gtk::DropDown {
                        set_focus_on_click: false,
                        set_margin_start: 15,
                        set_margin_end: 15,
                        set_model: Some(&OrderRow::store()),
                        set_factory: Some(&OrderRow::factory()),
                        connect_selected_item_notify => Self::Input::ParameterChanged,
                    },
                    #[wrap(Some)]
                    set_end_widget = &gtk::Box {
                        set_spacing: 5,
                        self.cd_entry.clone() -> gtk::Entry {
                            set_focus_on_click: false,
                            set_text: "0",
                            connect_text_notify => Self::Input::ParameterChanged,
                        },
                        gtk::Button {
                            set_icon_name: "user-trash-symbolic",
                            set_tooltip_text: Some("remove this filter"),
                            connect_clicked => Self::Input::RemoveFilter,
                        }
                    }
                },
                add_named[Some(Category::TrackNumber.to_str())] = &gtk::CenterBox {
                    #[wrap(Some)]
                    set_start_widget = &gtk::Label {
                        set_text: "By track number",
                    },
                    #[wrap(Some)]
                    set_center_widget = &self.track_number_dropdown.clone() -> gtk::DropDown {
                        set_focus_on_click: false,
                        set_margin_start: 15,
                        set_margin_end: 15,
                        set_model: Some(&OrderRow::store()),
                        set_factory: Some(&OrderRow::factory()),
                        connect_selected_item_notify => Self::Input::ParameterChanged,
                    },
                    #[wrap(Some)]
                    set_end_widget = &gtk::Box {
                        set_spacing: 5,
                        self.track_number_entry.clone() -> gtk::Entry {
                            set_focus_on_click: false,
                            set_text: "0",
                            connect_text_notify => Self::Input::ParameterChanged,
                        },
                        gtk::Button {
                            set_icon_name: "user-trash-symbolic",
                            set_tooltip_text: Some("remove this filter"),
                            connect_clicked => Self::Input::RemoveFilter,
                        }
                    }
                },
                add_named[Some(Category::Duration.to_str())] = &gtk::CenterBox {
                    #[wrap(Some)]
                    set_start_widget = &gtk::Label {
                        set_text: "By duration",
                    },
                    #[wrap(Some)]
                    set_center_widget = &self.duration_dropdown.clone() -> gtk::DropDown {
                        set_focus_on_click: false,
                        set_margin_start: 15,
                        set_margin_end: 15,
                        set_model: Some(&OrderRow::store()),
                        set_factory: Some(&OrderRow::factory()),
                        connect_selected_item_notify => Self::Input::ParameterChanged,
                    },
                    #[wrap(Some)]
                    set_end_widget = &gtk::Box {
                        set_spacing: 5,
                        self.duration_entry.clone() -> gtk::Entry {
                            set_focus_on_click: false,
                            set_text: "0",
                            connect_text_notify => Self::Input::ParameterChanged,
                        },
                        gtk::Button {
                            set_icon_name: "user-trash-symbolic",
                            set_tooltip_text: Some("remove this filter"),
                            connect_clicked => Self::Input::RemoveFilter,
                        }
                    }
                },
                add_named[Some(Category::BitRate.to_str())] = &gtk::CenterBox {
                    #[wrap(Some)]
                    set_start_widget = &gtk::Label {
                        set_text: "By bit rate",
                    },
                    #[wrap(Some)]
                    set_center_widget = &self.bit_rate_dropdown.clone() -> gtk::DropDown {
                        set_focus_on_click: false,
                        set_margin_start: 15,
                        set_margin_end: 15,
                        set_model: Some(&OrderRow::store()),
                        set_factory: Some(&OrderRow::factory()),
                        connect_selected_item_notify => Self::Input::ParameterChanged,
                    },
                    #[wrap(Some)]
                    set_end_widget = &gtk::Box {
                        set_spacing: 5,
                        self.bit_rate_entry.clone() -> gtk::Entry {
                            set_focus_on_click: false,
                            set_text: "0",
                            connect_text_notify => Self::Input::ParameterChanged,
                        },
                        gtk::Button {
                            set_icon_name: "user-trash-symbolic",
                            set_tooltip_text: Some("remove this filter"),
                            connect_clicked => Self::Input::RemoveFilter,
                        }
                    }
                },
                add_named[Some(Category::Title.to_str())] = &gtk::CenterBox {
                    #[wrap(Some)]
                    set_start_widget = &gtk::Label {
                        set_text: "By title name",
                    },
                    #[wrap(Some)]
                    set_end_widget = &gtk::Box {
                        set_spacing: 5,
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
                    }
                },
                add_named[Some(Category::Artist.to_str())] = &gtk::CenterBox {
                    #[wrap(Some)]
                    set_start_widget = &gtk::Label {
                        set_text: "By artist name",
                    },
                    #[wrap(Some)]
                    set_end_widget = &gtk::Box {
                        set_spacing: 5,
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
                    }
                },
                add_named[Some(Category::Album.to_str())] = &gtk::CenterBox {
                    #[wrap(Some)]
                    set_start_widget = &gtk::Label {
                        set_text: "By album name",
                    },
                    #[wrap(Some)]
                    set_end_widget = &gtk::Box {
                        set_spacing: 5,
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
                    }
                },
                add_named[Some(Category::Genre.to_str())] = &gtk::CenterBox {
                    #[wrap(Some)]
                    set_start_widget = &gtk::Label {
                        set_text: "By genre",
                    },
                    #[wrap(Some)]
                    set_end_widget = &gtk::Box {
                        set_spacing: 5,
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
                    }
                },
            }
        }
    }

    fn update(&mut self, msg: Self::Input, sender: relm4::FactorySender<Self>) {
        match msg {
            Self::Input::RemoveFilter => sender
                .output(FilterRowOut::RemoveFilter(self.index.clone()))
                .expect("sending failed"),
            Self::Input::SetTo(category) => {
                self.stack.set_visible_child_name(category.to_str());
            }
            Self::Input::ParameterChanged => {
                use glib::object::Cast;

                // update local filter
                match &self.category {
                    Category::Title => {
                        self.filter = Some(Filter::Title(self.title_entry.text().into()))
                    }
                    Category::Artist => {
                        self.filter = Some(Filter::Artist(self.artist_entry.text().into()))
                    }
                    Category::Album => {
                        self.filter = Some(Filter::Album(self.album_entry.text().into()))
                    }
                    Category::Genre => {
                        self.filter = Some(Filter::Genre(self.genre_entry.text().into()))
                    }
                    Category::Year => {
                        let order = self.year_dropdown.selected_item().unwrap();
                        let order = order
                            .downcast_ref::<glib::BoxedAnyObject>()
                            .expect("Needs to be ListItem");
                        let order: std::cell::Ref<OrderRow> = order.borrow();
                        if let Ok(number) = self.year_entry.text().parse::<usize>() {
                            self.filter = Some(Filter::Year(order.order, number));
                            self.year_entry.set_secondary_icon_name(None);
                            self.year_entry.set_tooltip_text(None);
                        } else {
                            self.filter = None;
                            self.year_entry
                                .set_secondary_icon_name(Some("dialog-error-symbolic"));
                            self.year_entry.set_secondary_icon_tooltip_text(Some(
                                "Needs to input a valid number",
                            ));
                            self.year_entry
                                .set_tooltip_text(Some("Needs to input a valid number"));
                        }
                    }
                    Category::Cd => {
                        let order = self.cd_dropdown.selected_item().unwrap();
                        let order = order
                            .downcast_ref::<glib::BoxedAnyObject>()
                            .expect("Needs to be ListItem");
                        let order: std::cell::Ref<OrderRow> = order.borrow();
                        if let Ok(number) = self.cd_entry.text().parse::<usize>() {
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
                        if let Ok(number) = self.duration_entry.text().parse::<usize>() {
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
                            self.bit_rate_entry.set_secondary_icon_name(None);
                            self.bit_rate_entry.set_tooltip_text(None);
                        } else {
                            self.filter = None;
                            self.bit_rate_entry
                                .set_secondary_icon_name(Some("dialog-error-symbolic"));
                            self.bit_rate_entry.set_secondary_icon_tooltip_text(Some(
                                "Needs to input a valid number",
                            ));
                            self.bit_rate_entry
                                .set_tooltip_text(Some("Needs to input a valid number"));
                        }
                    }
                }
                sender
                    .output(Self::Output::ParameterChanged)
                    .expect("sending failed");
            }
        }
    }
}