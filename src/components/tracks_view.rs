use std::{cell::RefCell, collections::HashSet, rc::Rc};

use gettextrs::gettext;
use relm4::{
    gtk::{
        self, glib,
        prelude::{
            BoxExt, ButtonExt, GtkWindowExt, ListModelExt, OrientableExt, SelectionModelExt,
            WidgetExt,
        },
    },
    ComponentController, RelmWidgetExt,
};

use crate::{
    common::{
        self,
        filter_categories::Category,
        types::{Droppable, Id},
    },
    components::{
        cover::{Cover, CoverIn, CoverOut},
        warning_dialog::WarningDialog,
    },
    factory::{
        filter_row::{FilterRow, FilterRowIn, FilterRowOut},
        track_row::{
            AlbumColumn, ArtistColumn, BitRateColumn, FavColumn, GenreColumn, LengthColumn,
            PlayCountColumn, PositionColumn, TitleColumn, TrackRow,
        },
    },
    settings::Settings,
    subsonic::Subsonic,
};

#[derive(Debug)]
pub struct TracksView {
    subsonic: Rc<RefCell<Subsonic>>,
    tracks: relm4::typed_view::column::TypedColumnView<TrackRow, gtk::MultiSelection>,
    filters: Rc<RefCell<relm4::factory::FactoryVecDeque<FilterRow>>>,

    info_cover: relm4::Controller<Cover>,
    shown_tracks: Vec<String>,
    shown_artists: HashSet<Option<String>>,
    shown_albums: HashSet<Option<String>>,
}

impl TracksView {
    fn active_filters(&self) -> bool {
        self.filters.borrow().iter().any(|f| f.active())
    }

    fn calc_sensitivity_of_buttons(&self, widgets: &<TracksView as relm4::Component>::Widgets) {
        let allowed_queue_modifier_len = 1000;

        if (!self.active_filters() && self.tracks.len() >= allowed_queue_modifier_len)
            || (self.active_filters()
                && self.shown_tracks.len() >= allowed_queue_modifier_len as usize)
        {
            widgets.add_to_queue.set_sensitive(false);
            widgets
                .add_to_queue
                .set_tooltip(&gettext("There are too many tracks to add to queue"));
            widgets.append_to_queue.set_sensitive(false);
            widgets
                .append_to_queue
                .set_tooltip(&gettext("There are too many tracks to append to queue"));
            widgets.replace_queue.set_sensitive(false);
            widgets
                .replace_queue
                .set_tooltip(&gettext("There are too many tracks to replace queue"));
        } else {
            widgets.add_to_queue.set_sensitive(true);
            widgets
                .add_to_queue
                .set_tooltip(&gettext("Append shown tracks to end of queue"));
            widgets.append_to_queue.set_sensitive(true);
            widgets.append_to_queue.set_tooltip(&gettext(
                "Insert shown after currently played or paused item",
            ));
            widgets.replace_queue.set_sensitive(true);
            widgets
                .replace_queue
                .set_tooltip(&gettext("Replaces current queue with shown tracks"));
        }
    }

    fn update_count_labels(
        &self,
        track_label: &gtk::Label,
        album_label: &gtk::Label,
        artist_label: &gtk::Label,
    ) {
        let tracks_len = self.shown_tracks.len();
        let artists_len = self.shown_artists.len();
        let albums_len = self.shown_albums.len();
        track_label.set_text(&format!("{}: {tracks_len}", gettext("Shown tracks")));
        artist_label.set_text(&format!("{}: {albums_len}", gettext("Shown artists")));
        album_label.set_text(&format!("{}: {artists_len}", gettext("Shown albums")));
    }
}

#[derive(Debug)]
pub enum TracksViewIn {
    SearchChanged,
    FilterChanged,
    UpdateWidgetsSearchFilterChanged,
    UpdateFavoriteSong(String, bool),
    UpdatePlayCountSong(String, Option<i64>),
    FilterAdd,
    FilterRow(FilterRowOut),
    Cover(CoverOut),
    AppendToQueue,
    AddToQueue,
    ReplaceQueue,
    ToggleFilters,
    TrackClicked(usize),
    RecalcDragSource,
    CreatePlaylist,
}

#[derive(Debug)]
pub enum TracksViewOut {
    DisplayToast(String),
    AddToQueue(Droppable),
    AppendToQueue(Droppable),
    ReplaceQueue(Droppable),
    Download(Droppable),
    FavoriteClicked(String, bool),
    ClickedArtist(Id),
    ClickedAlbum(Id),
    CreatePlaylist(String, Vec<submarine::data::Child>),
}

#[relm4::component(pub)]
impl relm4::Component for TracksView {
    type Init = Rc<RefCell<Subsonic>>;
    type Input = TracksViewIn;
    type Output = TracksViewOut;
    type CommandOutput = ();

    fn init(
        subsonic: Self::Init,
        root: Self::Root,
        sender: relm4::ComponentSender<Self>,
    ) -> relm4::ComponentParts<Self> {
        let mut tracks =
            relm4::typed_view::column::TypedColumnView::<TrackRow, gtk::MultiSelection>::new();
        tracks.append_column::<PositionColumn>();
        tracks.append_column::<TitleColumn>();
        tracks.append_column::<ArtistColumn>();
        tracks.append_column::<AlbumColumn>();
        tracks.append_column::<GenreColumn>();
        tracks.append_column::<LengthColumn>();
        tracks.append_column::<PlayCountColumn>();
        tracks.append_column::<BitRateColumn>();
        tracks.append_column::<FavColumn>();

        let columns = tracks.get_columns();
        columns
            .get("Title")
            .unwrap()
            .set_title(Some(&gettext("Title")));
        columns
            .get("Artist")
            .unwrap()
            .set_title(Some(&gettext("Artist")));
        columns
            .get("Album")
            .unwrap()
            .set_title(Some(&gettext("Album")));
        columns
            .get("Genre")
            .unwrap()
            .set_title(Some(&gettext("Genre")));
        columns
            .get("Length")
            .unwrap()
            .set_title(Some(&gettext("Length")));
        columns
            .get("Bitrate")
            .unwrap()
            .set_title(Some(&gettext("Bitrate")));
        columns
            .get("Favorite")
            .unwrap()
            .set_title(Some(&gettext("Favorite")));

        let mut model = Self {
            subsonic: subsonic.clone(),
            tracks,
            filters: Rc::new(RefCell::new(
                relm4::factory::FactoryVecDeque::builder()
                    .launch(gtk::ListBox::default())
                    .forward(sender.input_sender(), Self::Input::FilterRow),
            )),
            info_cover: Cover::builder()
                .launch((subsonic.clone(), None))
                .forward(sender.input_sender(), TracksViewIn::Cover),
            shown_tracks: Vec::with_capacity(subsonic.borrow().tracks().len()),
            shown_artists: HashSet::new(),
            shown_albums: HashSet::new(),
        };
        model.info_cover.model().add_css_class_image("size100");

        // add tracks
        let list = subsonic.borrow().tracks().to_vec();
        let tracks: Vec<TrackRow> = list
            .iter()
            .map(|track| {
                model.shown_tracks.push(track.id.clone());
                model.shown_albums.insert(track.album.clone());
                model.shown_artists.insert(track.artist.clone());
                TrackRow::new(&model.subsonic, track.clone(), &sender)
            })
            .collect();
        model.tracks.extend_from_iter(tracks);

        let widgets = view_output!();

        //update labels and buttons
        model.update_count_labels(
            &widgets.shown_tracks,
            &widgets.shown_albums,
            &widgets.shown_artists,
        );

        model
            .filters
            .borrow_mut()
            .guard()
            .push_back(Category::Favorite);
        model.calc_sensitivity_of_buttons(&widgets);

        // send signal on selection change
        model
            .tracks
            .selection_model
            .connect_selection_changed(move |_selection_model, _x, _y| {
                sender.input(TracksViewIn::RecalcDragSource);
            });

        // add filter
        let filters = model.filters.clone();
        let show_filters = widgets.filters.clone();
        model.tracks.add_filter(move |row| {
            if filters.borrow().is_empty() || !show_filters.reveals_child() {
                return true;
            }

            if filters
                .borrow()
                .iter()
                .filter_map(|row| row.filter().as_ref())
                .any(|filter| !filter.match_track(row.item()))
            {
                return false;
            }

            true
        });

        // add search filter
        model.tracks.add_filter(move |track| {
            let search = Settings::get().lock().unwrap().search_text.clone();
            let title_artist_album = format!(
                "{} {} {}",
                track.item().title.clone(),
                track.item().artist.clone().unwrap_or_default(),
                track.item().album.clone().unwrap_or_default()
            );
            common::search_matching(title_artist_album, search)
        });

        relm4::ComponentParts { model, widgets }
    }

    view! {
        gtk::Box {
            // tracks
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

                                model.info_cover.widget().clone() -> gtk::Box {
                                    set_valign: gtk::Align::Start,
                                },

                                //tracks info
                                gtk::Box {
                                    set_orientation: gtk::Orientation::Vertical,
                                    set_spacing: 8,

                                    append: shown_tracks = &gtk::Label {
                                        set_halign: gtk::Align::Start,
                                        set_text: &format!("{}: {}", gettext("Shown tracks"), model.shown_tracks.len()),
                                    },
                                    append: shown_artists = &gtk::Label {
                                        set_halign: gtk::Align::Start,
                                        set_text: &format!("{}: {}", gettext("Shown artists"), model.shown_artists.len()),
                                    },
                                    append: shown_albums = &gtk::Label {
                                        set_halign: gtk::Align::Start,
                                        set_text: &format!("{}: {}", gettext("Shown albums"), model.shown_albums.len()),
                                    },

                                    gtk::Box {
                                        set_spacing: 15,

                                        append: append_to_queue = &gtk::Button {
                                            gtk::Image {
                                                set_icon_name: Some("queue-append-symbolic"),
                                                set_pixel_size: 20,
                                            },
                                            set_tooltip: &gettext("Append Tracks to end of queue"),
                                            connect_clicked => TracksViewIn::AppendToQueue,
                                        },

                                        append: add_to_queue = &gtk::Button {
                                            gtk::Image {
                                                set_icon_name: Some("queue-insert-symbolic"),
                                                set_pixel_size: 20,
                                            },
                                            set_tooltip: &gettext("Insert Tracks after currently played or paused item"),
                                            connect_clicked => TracksViewIn::AddToQueue,
                                        },

                                        append: replace_queue = &gtk::Button {
                                            gtk::Image {
                                                set_icon_name: Some("queue-replace-symbolic"),
                                                set_pixel_size: 20,
                                            },
                                            set_tooltip: &gettext("Replaces current queue with tracks"),
                                            connect_clicked => TracksViewIn::ReplaceQueue,
                                        },
                                        gtk::Button {
                                            gtk::Image {
                                                set_icon_name: Some("list-add-symbolic"),
                                                set_pixel_size: 20,
                                            },
                                            set_tooltip: &gettext("Create a playlist with shown tracks"),
                                            connect_clicked => TracksViewIn::CreatePlaylist,
                                        },
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
                                    connect_active_notify => TracksViewIn::ToggleFilters,
                                }
                            }
                        }
                    },
                    gtk::ScrolledWindow {
                        set_hexpand: true,
                        set_vexpand: true,

                        model.tracks.view.clone() {
                            set_widget_name: "tracks-view-tracks",
                            set_vexpand: true,

                            add_controller = gtk::DragSource {
                                connect_prepare[sender] => move |_drag_src, _x, _y| {
                                    sender.input(TracksViewIn::RecalcDragSource);
                                    None
                                }
                            }
                        }
                    }
                },
            },

            // filters
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

                                #[name = "new_filter"]
                                gtk::DropDown {
                                    set_model: Some(&Category::tracks()),
                                    set_factory: Some(&Category::factory()),
                                },

                                gtk::Button {
                                    set_icon_name: "list-add-symbolic",
                                    connect_clicked => Self::Input::FilterAdd,
                                }
                            }
                        },
                    }
                }
            }
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
            TracksViewIn::UpdateFavoriteSong(id, state) => {
                (0..self.tracks.len())
                    .filter_map(|i| self.tracks.get(i))
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
                            track.borrow_mut().item_mut().starred = None;
                            if let Some(fav) = &track.borrow().fav_btn() {
                                fav.set_icon_name("non-starred-symbolic");
                            }
                        }
                    });
            }
            TracksViewIn::UpdatePlayCountSong(id, play_count) => (0..self.tracks.len())
                .filter_map(|i| self.tracks.get(i))
                .filter(|t| t.borrow().item().id == id)
                .for_each(|track| track.borrow_mut().set_play_count(play_count)),
            TracksViewIn::SearchChanged => {
                self.tracks.notify_filter_changed(1);
                sender.input(TracksViewIn::UpdateWidgetsSearchFilterChanged);
            }
            TracksViewIn::FilterChanged => {
                self.tracks.notify_filter_changed(0);
                sender.input(TracksViewIn::UpdateWidgetsSearchFilterChanged);
            }
            TracksViewIn::UpdateWidgetsSearchFilterChanged => {
                // recalc shown info
                self.shown_tracks.clear();
                self.shown_albums.clear();
                self.shown_artists.clear();
                (0..self.tracks.len())
                    .filter_map(|i| self.tracks.get_visible(i))
                    .for_each(|track| {
                        let track = track.borrow().item().clone();
                        self.shown_artists.insert(track.artist);
                        self.shown_albums.insert(track.album);
                        self.shown_tracks.push(track.name);
                    });

                // update widgets
                self.calc_sensitivity_of_buttons(widgets);
                self.update_count_labels(
                    &widgets.shown_tracks,
                    &widgets.shown_albums,
                    &widgets.shown_artists,
                );
            }
            TracksViewIn::FilterAdd => {
                use glib::object::Cast;

                let Some(list_item) = widgets.new_filter.selected_item() else {
                    sender
                        .output(TracksViewOut::DisplayToast(
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
                sender.input(TracksViewIn::FilterChanged);
            }
            TracksViewIn::FilterRow(msg) => match msg {
                FilterRowOut::RemoveFilter(index) => {
                    self.filters
                        .borrow_mut()
                        .guard()
                        .remove(index.current_index());
                    sender.input(TracksViewIn::FilterChanged);
                }
                FilterRowOut::ParameterChanged => sender.input(TracksViewIn::FilterChanged),
                FilterRowOut::DisplayToast(msg) => {
                    sender.output(TracksViewOut::DisplayToast(msg)).unwrap()
                }
            },
            TracksViewIn::Cover(msg) => match msg {
                CoverOut::DisplayToast(msg) => {
                    sender.output(TracksViewOut::DisplayToast(msg)).unwrap();
                }
            },
            TracksViewIn::AddToQueue => {
                if self.active_filters() {
                    if self.shown_tracks.is_empty() {
                        return;
                    }
                    let tracks = self
                        .shown_tracks
                        .iter()
                        .filter_map(|id| self.subsonic.borrow().find_track(id))
                        .collect();
                    let drop = Droppable::Queue(tracks);
                    sender.output(TracksViewOut::AddToQueue(drop)).unwrap();
                } else {
                    let tracks = self.subsonic.borrow().tracks().clone();
                    let drop = Droppable::Queue(tracks);
                    sender.output(TracksViewOut::AddToQueue(drop)).unwrap();
                }
            }
            TracksViewIn::AppendToQueue => {
                if self.active_filters() {
                    if self.shown_tracks.is_empty() {
                        return;
                    }
                    let tracks = self
                        .shown_tracks
                        .iter()
                        .filter_map(|id| self.subsonic.borrow().find_track(id))
                        .collect();
                    let drop = Droppable::Queue(tracks);
                    sender.output(TracksViewOut::AppendToQueue(drop)).unwrap();
                } else {
                    let tracks = self.subsonic.borrow().tracks().clone();
                    let drop = Droppable::Queue(tracks);
                    sender.output(TracksViewOut::AppendToQueue(drop)).unwrap();
                }
            }
            TracksViewIn::ReplaceQueue => {
                if self.active_filters() {
                    if self.shown_tracks.is_empty() {
                        return;
                    }
                    let tracks = self
                        .shown_tracks
                        .iter()
                        .filter_map(|id| self.subsonic.borrow().find_track(id))
                        .collect();
                    let drop = Droppable::Queue(tracks);
                    sender.output(TracksViewOut::ReplaceQueue(drop)).unwrap();
                } else {
                    let tracks = self.subsonic.borrow().tracks().clone();
                    let drop = Droppable::Queue(tracks);
                    sender.output(TracksViewOut::ReplaceQueue(drop)).unwrap();
                }
            }
            TracksViewIn::ToggleFilters => {
                sender.input(TracksViewIn::FilterChanged);
                widgets
                    .filters
                    .set_reveal_child(!widgets.filters.reveals_child());
            }
            TracksViewIn::TrackClicked(uid) => {
                let len = self.tracks.selection_model.n_items();
                if let Some(track) = (0..len)
                    .filter_map(|i| self.tracks.get(i))
                    .find(|track| track.borrow().uid() == &uid)
                {
                    self.info_cover
                        .emit(CoverIn::LoadSong(Box::new(track.borrow().item().clone())));
                }
            }
            TracksViewIn::RecalcDragSource => {
                let len = self.tracks.selection_model.n_items();
                let selected_rows: Vec<u32> = (0..len)
                    .filter(|i| self.tracks.view.model().unwrap().is_selected(*i))
                    .collect();

                // remove DragSource of not selected items
                (0..len)
                    .filter(|i| !selected_rows.contains(i))
                    .filter_map(|i| self.tracks.get(i))
                    .for_each(|row| row.borrow_mut().remove_drag_src());

                // get selected children
                let children: Vec<submarine::data::Child> = selected_rows
                    .iter()
                    .filter_map(|i| self.tracks.get(*i))
                    .map(|row| row.borrow().item().clone())
                    .collect();

                // set children as content for DragSource
                let drop = Droppable::Queue(children);
                selected_rows
                    .iter()
                    .filter_map(|i| self.tracks.get(*i))
                    .for_each(|row| row.borrow_mut().set_drag_src(drop.clone()));
            }
            TracksViewIn::CreatePlaylist => {
                if self.shown_tracks.is_empty() {
                    return;
                }
                let tracks: Vec<_> = self
                    .shown_tracks
                    .iter()
                    .filter_map(|id| self.subsonic.borrow().find_track(id))
                    .collect();

                // might show warning
                if tracks.len() >= 2000 {
                    relm4::view! {
                        #[template]
                        warning = WarningDialog {
                            #[template_child]
                            warning_text {
                                set_label: &gettext("You're aboput to create a playlist with over 2000 songs\nDo want to proceed?"),
                            },
                            #[template_child]
                            cancel_btn {
                                set_label: &gettext("Cancel"),
                            },
                            #[template_child]
                            proceed_btn {
                                set_label: &gettext("Create Playlist"),
                            }
                        }
                    }

                    let win = warning.clone();
                    warning.cancel_btn.connect_clicked(move |_btn| {
                        win.close();
                    });
                    let win = warning.clone();
                    warning.proceed_btn.connect_clicked(move |_btn| {
                        sender
                            .output(TracksViewOut::CreatePlaylist(
                                gettext("New playlist from tracks"),
                                tracks.clone(),
                            ))
                            .unwrap();
                        win.close();
                    });

                    warning.dialog.show();
                } else {
                    sender
                        .output(TracksViewOut::CreatePlaylist(
                            gettext("New playlist from tracks"),
                            tracks.clone(),
                        ))
                        .unwrap();
                }
            }
        }
    }
}
