use std::{cell::RefCell, collections::HashSet, rc::Rc};

use gettextrs::gettext;
use relm4::{
    gtk::{
        self, glib,
        prelude::{BoxExt, ButtonExt, ListModelExt, OrientableExt, WidgetExt},
    },
    RelmWidgetExt,
};

use crate::{
    common::{
        self,
        filter_categories::Category,
        types::{Droppable, Id},
    },
    components::cover::CoverOut,
    factory::{
        artist_row::{AlbumCountColumn, ArtistRow, CoverColumn, FavColumn, TitleColumn},
        filter_row::{FilterRow, FilterRowIn, FilterRowOut},
    },
    settings::Settings,
    subsonic::Subsonic,
};

#[derive(Debug)]
pub struct ArtistsView {
    subsonic: Rc<RefCell<Subsonic>>,
    filters: Rc<RefCell<relm4::factory::FactoryVecDeque<FilterRow>>>,
    entries: relm4::typed_view::column::TypedColumnView<ArtistRow, gtk::SingleSelection>,
    shown_artists: HashSet<String>,
}

impl ArtistsView {
    fn active_filters(&self) -> bool {
        self.filters.borrow().iter().any(|f| f.active())
    }

    fn calc_sensitivity_of_buttons(&self, widgets: &<ArtistsView as relm4::Component>::Widgets) {
        let allowed_queue_modifier_len = 5;

        if (!self.active_filters() && self.entries.len() >= allowed_queue_modifier_len)
            || (self.active_filters()
                && self.shown_artists.len() >= allowed_queue_modifier_len as usize)
        {
            widgets.add_to_queue.set_sensitive(false);
            widgets
                .add_to_queue
                .set_tooltip(&gettext("There are too many artists to add to queue"));
            widgets.append_to_queue.set_sensitive(false);
            widgets
                .append_to_queue
                .set_tooltip(&gettext("There are too many artists to append to queue"));
            widgets.replace_queue.set_sensitive(false);
            widgets
                .replace_queue
                .set_tooltip(&gettext("There are too many artists to replace queue"));
        } else {
            widgets.add_to_queue.set_sensitive(true);
            widgets
                .add_to_queue
                .set_tooltip(&gettext("Append shown artists to end of queue"));
            widgets.append_to_queue.set_sensitive(true);
            widgets.append_to_queue.set_tooltip(&gettext(
                "Insert shown artists after currently played or paused item",
            ));
            widgets.replace_queue.set_sensitive(true);
            widgets
                .replace_queue
                .set_tooltip(&gettext("Replaces current queue with shown artists"));
        }
    }
}

#[derive(Debug)]
pub enum ArtistsViewOut {
    ClickedArtist(Id),
    DisplayToast(String),
    FavoriteClicked(String, bool),
    AddToQueue(Droppable),
    AppendToQueue(Droppable),
    ReplaceQueue(Droppable),
}

#[derive(Debug)]
pub enum ArtistsViewIn {
    SearchChanged,
    FilterChanged,
    UpdateWidgetsSearchFilterChanged,
    UpdateFavoriteArtist(String, bool),
    Cover(CoverOut),
    FilterRow(FilterRowOut),
    FilterAdd,
    AppendToQueue,
    AddToQueue,
    ReplaceQueue,
    ArtistClicked(u32),
    ToggleFilters,
}

#[relm4::component(pub)]
impl relm4::component::Component for ArtistsView {
    type Init = Rc<RefCell<Subsonic>>;
    type Input = ArtistsViewIn;
    type Output = ArtistsViewOut;
    type CommandOutput = ();

    fn init(
        subsonic: Self::Init,
        root: Self::Root,
        sender: relm4::ComponentSender<Self>,
    ) -> relm4::component::ComponentParts<Self> {
        let mut entries =
            relm4::typed_view::column::TypedColumnView::<ArtistRow, gtk::SingleSelection>::new();
        entries.append_column::<CoverColumn>();
        entries.append_column::<TitleColumn>();
        entries.append_column::<AlbumCountColumn>();
        entries.append_column::<FavColumn>();

        let columns = entries.get_columns();
        columns
            .get("Cover")
            .unwrap()
            .set_title(Some(&gettext("Cover")));
        columns
            .get("Name")
            .unwrap()
            .set_title(Some(&gettext("Name")));
        columns
            .get("Albums")
            .unwrap()
            .set_title(Some(&gettext("Albums")));
        columns
            .get("Favorite")
            .unwrap()
            .set_title(Some(&gettext("Favorite")));

        let mut model = Self {
            subsonic,
            entries,
            filters: Rc::new(RefCell::new(
                relm4::factory::FactoryVecDeque::builder()
                    .launch(gtk::ListBox::default())
                    .forward(sender.input_sender(), Self::Input::FilterRow),
            )),
            shown_artists: HashSet::new(),
        };

        model
            .filters
            .borrow_mut()
            .guard()
            .push_back(Category::Favorite);

        //add artists
        let list = model.subsonic.borrow().artists().to_vec();
        for artist in list.iter() {
            model.shown_artists.insert(artist.name.clone());
            let artist = ArtistRow::new(&model.subsonic, artist.clone(), sender.clone());
            model.entries.append(artist);
        }

        // create view
        let widgets = view_output!();

        // update labels and buttons
        widgets.shown_artists.set_label(&format!(
            "{}: {}",
            gettext("Shown artists"),
            model.shown_artists.len()
        ));
        model.calc_sensitivity_of_buttons(&widgets);

        // add filter
        let filters = model.filters.clone();
        let show_filters = widgets.filters.clone();
        model.entries.add_filter(move |row| {
            if filters.borrow().is_empty() || !show_filters.reveals_child() {
                return true;
            }

            if filters
                .borrow()
                .iter()
                .filter_map(|row| row.filter().as_ref())
                .any(|filter| !filter.match_artist(row.item()))
            {
                return false;
            }

            true
        });

        // add search filter
        model.entries.add_filter(move |track| {
            let search = Settings::get().lock().unwrap().search_text.clone();
            let artist = track.item().name.clone();
            common::search_matching(artist, search)
        });

        relm4::component::ComponentParts { model, widgets }
    }

    view! {
        gtk::Box {
            gtk::Box {
                add_css_class: "tracks-view",
                set_orientation: gtk::Orientation::Vertical,

                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_spacing: 8,

                    // info
                    gtk::WindowHandle {
                        gtk::CenterBox {
                            #[wrap(Some)]
                            set_start_widget = &gtk::Box {
                                set_spacing: 15,
                                set_margin_horizontal: 7,

                                //tracks info
                                gtk::Box {
                                    set_orientation: gtk::Orientation::Vertical,
                                    set_spacing: 8,

                                    append: shown_artists = &gtk::Label {
                                        set_halign: gtk::Align::Start,
                                        set_text: &format!("{}: {}", gettext("Shown artists:"), model.shown_artists.len()),
                                    },
                                    gtk::Box {
                                        set_spacing: 15,

                                        #[name = "append_to_queue"]
                                        gtk::Button {
                                            gtk::Box {
                                                gtk::Image {
                                                    set_icon_name: Some("queue-append-symbolic"),
                                                    set_pixel_size: 20,
                                                },
                                            },
                                            set_tooltip: &gettext("Append artists to end of queue"),
                                            connect_clicked => ArtistsViewIn::AppendToQueue,
                                        },
                                        #[name = "add_to_queue"]
                                        gtk::Button {
                                            gtk::Box {
                                                gtk::Image {
                                                    set_icon_name: Some("queue-insert-symbolic"),
                                                    set_pixel_size: 20,
                                                },
                                            },
                                            set_tooltip: &gettext("Insert artists after currently played or paused item"),
                                            connect_clicked => ArtistsViewIn::AddToQueue,
                                        },
                                        #[name = "replace_queue"]
                                        gtk::Button {
                                            gtk::Box {
                                                gtk::Image {
                                                    set_icon_name: Some("queue-replace-symbolic"),
                                                    set_pixel_size: 20,
                                                },
                                            },
                                            set_tooltip: &gettext("Replaces current queue with artists"),
                                            connect_clicked => ArtistsViewIn::ReplaceQueue,
                                        }
                                    }
                                }
                            },
                            #[wrap(Some)]
                            set_end_widget = &gtk::Box {
                                set_spacing: 10,
                                set_margin_end: 10,
                                set_tooltip: &gettext("Activate to show filter panel"),

                                gtk::Label {
                                    set_text: &gettext("Filters:"),
                                },
                                gtk::Switch {
                                    set_valign: gtk::Align::Center,
                                    connect_active_notify => ArtistsViewIn::ToggleFilters,
                                }
                            }
                        }
                    },
                    gtk::ScrolledWindow {
                        set_hexpand: true,
                        set_vexpand: true,

                        model.entries.view.clone() {
                            set_widget_name: "artists-view-tracks",
                            set_vexpand: true,
                            set_single_click_activate: true,

                            connect_activate[sender] => move |_column_view, index| {
                                sender.input(ArtistsViewIn::ArtistClicked(index));
                            },
                        }
                    }
                },
            },
            //filters
            append: filters = &gtk::Revealer {
                set_transition_duration: 200,
                set_transition_type: gtk::RevealerTransitionType::SlideLeft,

                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_size_request: (400, -1),

                    gtk::WindowHandle {
                        gtk::Label {
                            add_css_class: granite::STYLE_CLASS_H2_LABEL,
                            set_text: &gettext("Active Filters"),
                        }
                    },

                    model.filters.borrow().widget().clone() -> gtk::ListBox {
                        set_margin_all: 5,
                        add_css_class: granite::STYLE_CLASS_FRAME,
                        add_css_class: granite::STYLE_CLASS_RICH_LIST,
                        set_vexpand: true,
                        set_selection_mode: gtk::SelectionMode::None,

                        // display new filter button
                        gtk::ListBoxRow {
                            set_focusable: false,
                            set_valign: gtk::Align::Center,


                            gtk::Box {
                                set_spacing: 15,
                                set_halign: gtk::Align::Center,

                                gtk::Label {
                                    set_text: &gettext("New filter:"),
                                },

                                append: new_filter = &gtk::DropDown {
                                    set_model: Some(&Category::artists()),
                                    set_factory: Some(&Category::factory()),
                                },

                                gtk::Button {
                                    set_icon_name: "list-add-symbolic",
                                    connect_clicked => ArtistsViewIn::FilterAdd,
                                }
                            }
                        },
                    }
                }
            },
        }
    }

    fn update_with_view(
        &mut self,
        widgets: &mut Self::Widgets,
        msg: Self::Input,
        sender: relm4::ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match msg {
            ArtistsViewIn::SearchChanged => {
                self.entries.notify_filter_changed(1);
                sender.input(ArtistsViewIn::UpdateWidgetsSearchFilterChanged);
            }
            ArtistsViewIn::FilterChanged => {
                self.entries.notify_filter_changed(0);
                sender.input(ArtistsViewIn::UpdateWidgetsSearchFilterChanged);
            }
            ArtistsViewIn::UpdateWidgetsSearchFilterChanged => {
                // update widgets
                self.shown_artists.clear();
                (0..self.entries.len())
                    .filter_map(|i| self.entries.get_visible(i))
                    .for_each(|track| {
                        self.shown_artists
                            .insert(track.borrow().item().name.clone());
                    });

                widgets.shown_artists.set_text(&format!(
                    "{}: {}",
                    gettext("Shown artists"),
                    self.shown_artists.len()
                ));
                self.calc_sensitivity_of_buttons(widgets);
            }
            ArtistsViewIn::UpdateFavoriteArtist(id, state) => {
                (0..self.entries.len())
                    .filter_map(|i| self.entries.get(i))
                    .filter(|t| t.borrow().item().id == id)
                    .for_each(|track| match state {
                        true => {
                            track.borrow_mut().item_mut().starred =
                                Some(chrono::offset::Local::now().into());
                            if let Some(fav) = &track.borrow().fav_btn() {
                                fav.set_icon_name("starred-symbolic");
                            }
                        }
                        false => {
                            if let Some(fav) = &track.borrow().fav_btn() {
                                fav.set_icon_name("non-starred-symbolic");
                            }
                            track.borrow_mut().item_mut().starred = None;
                        }
                    });
            }
            ArtistsViewIn::Cover(msg) => match msg {
                CoverOut::DisplayToast(msg) => {
                    sender.output(ArtistsViewOut::DisplayToast(msg)).unwrap();
                }
            },
            ArtistsViewIn::FilterAdd => {
                use glib::object::Cast;

                let Some(list_item) = widgets.new_filter.selected_item() else {
                    sender
                        .output(ArtistsViewOut::DisplayToast(
                            "no filter selected".to_string(),
                        ))
                        .unwrap();
                    return;
                };
                let boxed = list_item
                    .downcast_ref::<glib::BoxedAnyObject>()
                    .expect("is not a BoxedAnyObject");
                let category: std::cell::Ref<Category> = boxed.borrow();

                let index = self
                    .filters
                    .borrow_mut()
                    .guard()
                    .push_back(category.clone());
                self.filters
                    .borrow()
                    .send(index.current_index(), FilterRowIn::SetTo(category.clone()));
                sender.input(ArtistsViewIn::FilterChanged);
            }
            ArtistsViewIn::FilterRow(msg) => match msg {
                FilterRowOut::RemoveFilter(index) => {
                    self.filters
                        .borrow_mut()
                        .guard()
                        .remove(index.current_index());
                    sender.input(ArtistsViewIn::FilterChanged);
                }
                FilterRowOut::ParameterChanged => sender.input(ArtistsViewIn::FilterChanged),
                FilterRowOut::DisplayToast(msg) => {
                    sender.output(ArtistsViewOut::DisplayToast(msg)).unwrap()
                }
            },
            ArtistsViewIn::AddToQueue => {
                if self.shown_artists.is_empty() {
                    return;
                }
                let artists: Vec<submarine::data::ArtistId3> =
                    (0..self.entries.selection_model.n_items())
                        .filter_map(|i| self.entries.get_visible(i))
                        .map(|i| i.borrow().item().clone())
                        .collect();
                for artist in artists {
                    let drop = Droppable::Artist(Box::new(artist));
                    sender.output(ArtistsViewOut::AddToQueue(drop)).unwrap();
                }
            }
            ArtistsViewIn::AppendToQueue => {
                if self.shown_artists.is_empty() {
                    return;
                }
                let artists: Vec<submarine::data::ArtistId3> =
                    (0..self.entries.selection_model.n_items())
                        .filter_map(|i| self.entries.get_visible(i))
                        .map(|i| i.borrow().item().clone())
                        .collect();
                for artist in artists {
                    let drop = Droppable::Artist(Box::new(artist));
                    sender.output(ArtistsViewOut::AppendToQueue(drop)).unwrap();
                }
            }
            ArtistsViewIn::ReplaceQueue => {
                if self.shown_artists.is_empty() {
                    return;
                }
                let artists: Vec<submarine::data::ArtistId3> =
                    (0..self.entries.selection_model.n_items())
                        .filter_map(|i| self.entries.get_visible(i))
                        .map(|i| i.borrow().item().clone())
                        .collect();
                for artist in artists {
                    let drop = Droppable::Artist(Box::new(artist));
                    sender.output(ArtistsViewOut::ReplaceQueue(drop)).unwrap();
                }
            }
            ArtistsViewIn::ArtistClicked(index) => {
                if let Some(clicked_artist) = self.entries.get_visible(index) {
                    let id = Id::artist(clicked_artist.borrow().item().id.clone());
                    sender.output(ArtistsViewOut::ClickedArtist(id)).unwrap();
                }
            }
            ArtistsViewIn::ToggleFilters => {
                sender.input(ArtistsViewIn::FilterChanged);
                widgets
                    .filters
                    .set_reveal_child(!widgets.filters.reveals_child());
            }
        }
    }
}
