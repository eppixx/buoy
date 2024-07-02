use gtk::prelude::OrientableExt;
use relm4::gtk::{
    self, glib,
    prelude::{BoxExt, ButtonExt, WidgetExt},
};

use super::filter_row::{Category, Filter, FilterRowOut};
use crate::components::filter_row::FilterRow;

#[derive(Debug)]
pub struct FilterBox {
    possible_categories: Vec<Category>,
    filters: relm4::factory::FactoryVecDeque<FilterRow>,
    favorite_switch: gtk::Switch,
}

impl FilterBox {
    pub fn get_filters(&self) -> Vec<Filter> {
        let mut filters: Vec<Filter> = self
            .filters
            .iter()
            .filter_map(|row| row.filter().as_ref())
            .cloned()
            .collect();

        if self.favorite_switch.is_active() {
            filters.push(Filter::Favorite(true));
        }
        filters
    }
}

#[derive(Debug)]
pub enum FilterBoxIn {
    ClearFilters,
    Favorited(bool),
    AddNewFilter,
    FilterRow(FilterRowOut),
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
        let model = Self {
            possible_categories: init,
            filters: relm4::factory::FactoryVecDeque::builder()
                .launch(gtk::ListBox::default())
                .forward(sender.input_sender(), Self::Input::FilterRow),
            favorite_switch: gtk::Switch::default(),
        };

        let widgets = view_output!();

        relm4::ComponentParts { model, widgets }
    }

    view! {
        #[root]
        gtk::Box {
            add_css_class: "filter-box",
            set_orientation: gtk::Orientation::Vertical,
            set_spacing: 10,

            gtk::CenterBox {
                #[wrap(Some)]
                set_center_widget = &gtk::Label {
                    add_css_class: granite::STYLE_CLASS_H3_LABEL,
                    set_hexpand: true,
                    set_halign: gtk::Align::Center,
                    set_text: "Active Filters",
                },
                #[wrap(Some)]
                set_end_widget = &gtk::Image {
                    set_icon_name: Some("dialog-information-symbolic"),
                    set_tooltip_text: Some("Elements will be shown when every condition is true.\nText input allows for regular expressions"),
                    //TODO show examples for regular expression
                }
            },

            gtk::CenterBox {
                #[wrap(Some)]
                set_start_widget = &gtk::Label {
                    set_margin_end: 15,
                    set_text: "Show Favorites",
                },
                #[wrap(Some)]
                set_end_widget = &model.favorite_switch.clone() -> gtk::Switch {
                    set_margin_start: 15,
                    connect_state_set[sender] => move |_btn, state| {
                        sender.input(Self::Input::Favorited(state));
                        glib::signal::Propagation::Proceed
                    }
                }
            },

            model.filters.widget().clone() -> gtk::ListBox {},

            gtk::Button {
                set_label: "Add new filter",
                connect_clicked => Self::Input::AddNewFilter,
            }
        }
    }

    fn update(&mut self, msg: Self::Input, sender: relm4::ComponentSender<Self>) {
        match msg {
            Self::Input::ClearFilters => {
                self.filters.guard().clear();
                sender
                    .output(Self::Output::FiltersChanged)
                    .expect("sending failed");
            }
            Self::Input::Favorited(value) => {
                sender
                    .output(Self::Output::FiltersChanged)
                    .expect("sending failed");
            }
            Self::Input::AddNewFilter => {
                self.filters.guard().push_back(());
                sender
                    .output(Self::Output::FiltersChanged)
                    .expect("sending failed");
            }
            Self::Input::FilterRow(msg) => match msg {
                FilterRowOut::RemoveFilter(index) => {
                    self.filters.guard().remove(index.current_index());
                    println!("current filters {:?}", self.get_filters());
                    sender
                        .output(Self::Output::FiltersChanged)
                        .expect("sending failed");
                }
                FilterRowOut::ParameterChanged => {
                    sender
                        .output(FilterBoxOut::FiltersChanged)
                        .expect("sending failed");
                    println!("current filters {:?}", self.get_filters());
                }
            },
        }
    }
}
