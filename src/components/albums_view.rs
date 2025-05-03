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
    common,
    components::{
        cover::CoverOut,
        filter_categories::Category,
        filter_row::{FilterRow, FilterRowIn, FilterRowOut},
    },
    factory::album_row::{
        AlbumRow, ArtistColumn, CoverColumn, FavColumn, GenreColumn, LengthColumn, PlayCountColumn,
        TitleColumn, YearColumn,
    },
    settings::Settings,
    subsonic::Subsonic,
    types::{Droppable, Id},
};

#[derive(Debug)]
pub struct AlbumsView {
    subsonic: Rc<RefCell<Subsonic>>,
    entries: relm4::typed_view::column::TypedColumnView<AlbumRow, gtk::SingleSelection>,
    filters: Rc<RefCell<relm4::factory::FactoryVecDeque<FilterRow>>>,
    shown_artists: HashSet<Option<String>>,
    shown_albums: HashSet<Option<String>>,
}

impl AlbumsView {
    fn active_filters(&self) -> bool {
        self.filters.borrow().iter().any(|f| f.active())
    }

    fn calc_sensitivity_of_buttons(&self, widgets: &<AlbumsView as relm4::Component>::Widgets) {
        let allowed_queue_modifier_len = 10;

        if (!self.active_filters() && self.entries.len() >= allowed_queue_modifier_len)
            || (self.active_filters()
                && self.shown_artists.len() >= allowed_queue_modifier_len as usize)
        {
            widgets.add_to_queue.set_sensitive(false);
            widgets
                .add_to_queue
                .set_tooltip(&gettext("There are too many albums to add to queue"));
            widgets.append_to_queue.set_sensitive(false);
            widgets
                .append_to_queue
                .set_tooltip(&gettext("There are too many albums to append to queue"));
            widgets.replace_queue.set_sensitive(false);
            widgets
                .replace_queue
                .set_tooltip(&gettext("There are too many albums to replace queue"));
        } else {
            widgets.add_to_queue.set_sensitive(true);
            widgets
                .add_to_queue
                .set_tooltip(&gettext("Append shown albums to end of queue"));
            widgets.append_to_queue.set_sensitive(true);
            widgets.append_to_queue.set_tooltip(&gettext(
                "Insert shown albums after currently played or paused item",
            ));
            widgets.replace_queue.set_sensitive(true);
            widgets
                .replace_queue
                .set_tooltip(&gettext("Replaces current queue with shown albums"));
        }
    }
}

#[derive(Debug)]
pub enum AlbumsViewIn {
    SearchChanged,
    FilterChanged,
    UpdateWidgetsSearchFilterChanged,
    UpdateFavoriteAlbum(String, bool),
    UpdatePlayCountAlbum(String, Option<i64>),
    Cover(CoverOut),
    FilterRow(FilterRowOut),
    FilterAdd,
    AppendToQueue,
    AddToQueue,
    ReplaceQueue,
    ClickedAlbum(u32),
    ToggleFilters,
}

#[derive(Debug)]
pub enum AlbumsViewOut {
    ClickedAlbum(Id),
    ClickedArtist(Id),
    DisplayToast(String),
    FavoriteClicked(String, bool),
    AddToQueue(Droppable),
    AppendToQueue(Droppable),
    ReplaceQueue(Droppable),
}

#[relm4::component(pub)]
impl relm4::component::Component for AlbumsView {
    type Init = Rc<RefCell<Subsonic>>;
    type Input = AlbumsViewIn;
    type Output = AlbumsViewOut;
    type CommandOutput = ();

    fn init(
        subsonic: Self::Init,
        root: Self::Root,
        sender: relm4::ComponentSender<Self>,
    ) -> relm4::component::ComponentParts<Self> {
        let mut entries =
            relm4::typed_view::column::TypedColumnView::<AlbumRow, gtk::SingleSelection>::new();
        entries.append_column::<CoverColumn>();
        entries.append_column::<TitleColumn>();
        entries.append_column::<ArtistColumn>();
        entries.append_column::<GenreColumn>();
        entries.append_column::<LengthColumn>();
        entries.append_column::<YearColumn>();
        // entries.append_column::<CdColumn>();
        entries.append_column::<PlayCountColumn>();
        entries.append_column::<FavColumn>();

        let columns = entries.get_columns();
        columns
            .get("Cover")
            .unwrap()
            .set_title(Some(&gettext("Cover")));
        columns
            .get("Album")
            .unwrap()
            .set_title(Some(&gettext("Album")));
        columns
            .get("Artist")
            .unwrap()
            .set_title(Some(&gettext("Artist")));
        columns
            .get("Genre")
            .unwrap()
            .set_title(Some(&gettext("Genre")));
        columns
            .get("Length")
            .unwrap()
            .set_title(Some(&gettext("Length")));
        columns
            .get("Year")
            .unwrap()
            .set_title(Some(&gettext("Year")));
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
            shown_albums: HashSet::new(),
        };

        //add some albums
        let list = model.subsonic.borrow().albums().to_vec();
        for album in list.iter() {
            model.shown_albums.insert(album.album.clone());
            model.shown_artists.insert(album.artist.clone());
            let album = AlbumRow::new(&model.subsonic, album.clone(), sender.clone());
            model.entries.append(album);
        }

        model
            .filters
            .borrow_mut()
            .guard()
            .push_back(Category::Favorite);
        let widgets = view_output!();

        //update labels and buttons
        update_labels(
            &widgets.shown_albums,
            &model.shown_albums,
            &widgets.shown_artists,
            &model.shown_artists,
        );
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
                .any(|filter| !filter.match_album(row.item()))
            {
                return false;
            }

            true
        });

        // add search filter
        model.entries.add_filter(move |track| {
            let search = Settings::get().lock().unwrap().search_text.clone();
            let title_artist_album = format!(
                "{} {} {}",
                track.item().title.clone(),
                track.item().artist.clone().unwrap_or_default(),
                track.item().album.clone().unwrap_or_default()
            );
            common::search_matching(title_artist_album, search)
        });

        relm4::component::ComponentParts { model, widgets }
    }

    view! {
        gtk::Box {
            gtk::Box {
                add_css_class: "albums-view",
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

                                    append: shown_albums = &gtk::Label {
                                        set_halign: gtk::Align::Start,
                                        set_text: &format!("{}: {}", gettext("Shown albums"), model.shown_albums.len()),
                                    },
                                    append: shown_artists = &gtk::Label {
                                        set_halign: gtk::Align::Start,
                                        set_text: &format!("{}: {}", gettext("Shown artists"), model.shown_artists.len()),
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
                                            connect_clicked => AlbumsViewIn::AppendToQueue,
                                        },
                                        #[name = "add_to_queue"]
                                        gtk::Button {
                                            gtk::Box {
                                                gtk::Image {
                                                    set_icon_name: Some("queue-insert-symbolic"),
                                                    set_pixel_size: 20,
                                                },
                                            },
                                            connect_clicked => AlbumsViewIn::AddToQueue,
                                        },
                                        #[name = "replace_queue"]
                                        gtk::Button {
                                            gtk::Box {
                                                gtk::Image {
                                                    set_icon_name: Some("queue-replace-symbolic"),
                                                    set_pixel_size: 20,
                                                },
                                            },
                                            connect_clicked => AlbumsViewIn::ReplaceQueue,
                                        }
                                    }
                                }
                            },
                            #[wrap(Some)]
                            set_end_widget = &gtk::Box {
                                set_spacing: 10,
                                set_margin_end: 10,

                                gtk::Label {
                                    set_text: &gettext("Filters:"),
                                },
                                gtk::Switch {
                                    set_valign: gtk::Align::Center,
                                    connect_active_notify => AlbumsViewIn::ToggleFilters,
                                }
                            }
                        }
                    },
                    gtk::ScrolledWindow {
                        set_hexpand: true,
                        set_vexpand: true,

                        model.entries.view.clone() {
                            set_widget_name: "albums-view-tracks",
                            set_vexpand: true,
                            set_single_click_activate: true,

                            connect_activate[sender] => move |_column_view, index| {
                                sender.input(AlbumsViewIn::ClickedAlbum(index));
                            },
                        }
                    }
                }
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
                                    set_model: Some(&Category::albums()),
                                    set_factory: Some(&Category::factory()),
                                },

                                gtk::Button {
                                    set_icon_name: "list-add-symbolic",
                                    connect_clicked => AlbumsViewIn::FilterAdd,
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
            AlbumsViewIn::SearchChanged => {
                self.entries.notify_filter_changed(1);
                sender.input(AlbumsViewIn::UpdateWidgetsSearchFilterChanged);
            }
            AlbumsViewIn::FilterChanged => {
                self.entries.notify_filter_changed(0);
                sender.input(AlbumsViewIn::UpdateWidgetsSearchFilterChanged);
            }
            AlbumsViewIn::UpdateWidgetsSearchFilterChanged => {
                // recalc shown info
                self.shown_artists.clear();
                self.shown_albums.clear();
                (0..self.entries.len())
                    .filter_map(|i| self.entries.get_visible(i))
                    .for_each(|track| {
                        let track = track.borrow().item().clone();
                        self.shown_artists.insert(track.artist);
                        self.shown_albums.insert(track.album);
                    });

                // update widgets
                update_labels(
                    &widgets.shown_albums,
                    &self.shown_albums,
                    &widgets.shown_artists,
                    &self.shown_artists,
                );
                self.calc_sensitivity_of_buttons(widgets);
            }
            AlbumsViewIn::UpdateFavoriteAlbum(id, state) => {
                (0..self.entries.len())
                    .filter_map(|i| self.entries.get(i))
                    .filter(|a| a.borrow().item().id == id)
                    .for_each(|album| match state {
                        true => {
                            if let Some(fav) = &album.borrow().fav_btn() {
                                fav.set_icon_name("starred-symbolic");
                            }
                            album.borrow_mut().item_mut().starred =
                                Some(chrono::offset::Local::now().into());
                        }
                        false => {
                            if let Some(fav) = &album.borrow().fav_btn() {
                                fav.set_icon_name("non-starred-symbolic");
                            }
                            album.borrow_mut().item_mut().starred = None;
                        }
                    });
            }
            AlbumsViewIn::UpdatePlayCountAlbum(id, play_count) => {
                (0..self.entries.len())
                    .filter_map(|i| self.entries.get(i))
                    .filter(|a| a.borrow().item().id == id)
                    .for_each(|album| album.borrow_mut().item_mut().play_count = play_count);
            }
            AlbumsViewIn::Cover(msg) => match msg {
                CoverOut::DisplayToast(msg) => {
                    sender.output(AlbumsViewOut::DisplayToast(msg)).unwrap();
                }
            },
            AlbumsViewIn::FilterAdd => {
                use glib::object::Cast;

                let Some(list_item) = widgets.new_filter.selected_item() else {
                    sender
                        .output(AlbumsViewOut::DisplayToast(
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
                sender.input(AlbumsViewIn::FilterChanged);
            }
            AlbumsViewIn::FilterRow(msg) => match msg {
                FilterRowOut::RemoveFilter(index) => {
                    self.filters
                        .borrow_mut()
                        .guard()
                        .remove(index.current_index());
                    sender.input(AlbumsViewIn::FilterChanged);
                }
                FilterRowOut::ParameterChanged => sender.input(AlbumsViewIn::FilterChanged),
                FilterRowOut::DisplayToast(msg) => {
                    sender.output(AlbumsViewOut::DisplayToast(msg)).unwrap()
                }
            },
            AlbumsViewIn::AddToQueue => {
                if self.shown_albums.is_empty() {
                    return;
                }
                let albums: Vec<submarine::data::Child> =
                    (0..self.entries.selection_model.n_items())
                        .filter_map(|i| self.entries.get_visible(i))
                        .map(|i| i.borrow().item().clone())
                        .collect();
                for album in albums {
                    let drop = Droppable::Child(Box::new(album));
                    sender.output(AlbumsViewOut::AddToQueue(drop)).unwrap();
                }
            }
            AlbumsViewIn::AppendToQueue => {
                if self.shown_albums.is_empty() {
                    return;
                }
                let albums: Vec<submarine::data::Child> =
                    (0..self.entries.selection_model.n_items())
                        .filter_map(|i| self.entries.get_visible(i))
                        .map(|i| i.borrow().item().clone())
                        .collect();
                for album in albums {
                    let drop = Droppable::Child(Box::new(album));
                    sender.output(AlbumsViewOut::AppendToQueue(drop)).unwrap();
                }
            }
            AlbumsViewIn::ReplaceQueue => {
                if self.shown_albums.is_empty() {
                    return;
                }
                let albums: Vec<submarine::data::Child> =
                    (0..self.entries.selection_model.n_items())
                        .filter_map(|i| self.entries.get_visible(i))
                        .map(|i| i.borrow().item().clone())
                        .collect();
                for album in albums {
                    let drop = Droppable::Child(Box::new(album));
                    sender.output(AlbumsViewOut::ReplaceQueue(drop)).unwrap();
                }
            }
            AlbumsViewIn::ClickedAlbum(index) => {
                if let Some(clicked_album) = self.entries.get_visible(index) {
                    let id = Id::album(&clicked_album.borrow().item().id);
                    sender.output(AlbumsViewOut::ClickedAlbum(id)).unwrap();
                }
            }
            AlbumsViewIn::ToggleFilters => {
                sender.input(AlbumsViewIn::FilterChanged);
                widgets
                    .filters
                    .set_reveal_child(!widgets.filters.reveals_child());
            }
        }
    }
}

fn update_labels(
    album_label: &gtk::Label,
    albums: &HashSet<Option<String>>,
    artist_label: &gtk::Label,
    artists: &HashSet<Option<String>>,
) {
    album_label.set_text(&format!("{}: {}", gettext("Shown albums"), albums.len()));
    artist_label.set_text(&format!("{}: {}", gettext("Shown artists"), artists.len()));
}
