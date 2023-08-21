use components::queue::QueueModel;
use gtk::prelude::{BoxExt, ButtonExt, GtkWindowExt, OrientableExt};
use relm4::{
    gtk::{self, traits::WidgetExt},
    Component, ComponentController, ComponentParts, ComponentSender, Controller, RelmApp,
    RelmWidgetExt, SimpleComponent,
};

mod components;
pub mod css;
mod factory;
mod play_state;
pub mod types;

struct AppModel {
    counter: u8,
    queue: Controller<QueueModel>,
}

#[derive(Debug)]
enum AppMsg {
    Increment,
    Decrement,
}

#[relm4::component]
impl SimpleComponent for AppModel {
    type Input = AppMsg;

    type Output = ();
    type Init = u8;

    // Initialize the UI.
    fn init(
        counter: Self::Init,
        root: &Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let queue: Controller<QueueModel> =
            QueueModel::builder()
                .launch(())
                .forward(sender.input_sender(), |msg| match msg {
                    _ => todo!(),
                });
        let model = AppModel { counter, queue };

        // Insert the macro code generation here
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>) {
        match msg {
            AppMsg::Increment => {
                self.counter = self.counter.wrapping_add(1);
            }
            AppMsg::Decrement => {
                self.counter = self.counter.wrapping_sub(1);
            }
        }
    }

    view! {
        #[root]
        gtk::Window {
            set_title: Some("Simple app"),
            set_default_width: 500,
            set_default_height: 700,

            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_spacing: 5,

                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_spacing: 5,
                    set_margin_top: 5,
                    set_margin_start: 5,
                    set_margin_end: 5,

                    gtk::Button {
                        set_label: "Increment",
                        connect_clicked => AppMsg::Increment,
                    },

                    gtk::Button::with_label("Decrement") {
                        connect_clicked => AppMsg::Decrement,
                    },
                },

                gtk::Label {
                    #[watch]
                    set_label: &format!("Counter: {}", model.counter),
                    set_margin_all: 5,
                },

                append: model.queue.widget(),
            }
        }
    }
}

fn main() {
    let app = RelmApp::new("relm4.test.simple");
    css::setup_css();
    app.run::<AppModel>(0);
}
