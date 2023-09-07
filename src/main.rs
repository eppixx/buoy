use components::queue::QueueModel;
use gtk::prelude::{BoxExt, GtkWindowExt, OrientableExt};
use relm4::{
    gtk::{
        self,
        gio::SimpleAction,
        prelude::{ActionMapExt, ApplicationExt},
        traits::GtkApplicationExt,
    },
    Component, ComponentController, ComponentParts, ComponentSender, Controller, RelmApp,
    SimpleComponent,
};

mod components;
pub mod css;
mod factory;
mod play_state;
pub mod types;

struct AppModel {
    queue: Controller<QueueModel>,
}

#[derive(Debug)]
enum AppMsg {}

#[relm4::component]
impl SimpleComponent for AppModel {
    type Input = AppMsg;

    type Output = ();
    type Init = ();

    // Initialize the UI.
    fn init(
        _init: Self::Init,
        root: &Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let queue: Controller<QueueModel> = QueueModel::builder()
            .launch(())
            .forward(sender.input_sender(), |_msg| todo!());
        let model = AppModel { queue };

        // Insert the macro code generation here
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>) {
        match msg {}
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

                append: model.queue.widget(),
            }
        }
    }
}

fn main() -> anyhow::Result<()> {
    let application = relm4::main_application();

    // quit action
    let quit = SimpleAction::new("quit", None);
    let app = application.clone();
    quit.connect_activate(move |_action, _parameter| {
        app.quit();
    });
    application.set_accels_for_action("app.quit", &["<Primary>Q"]);
    application.add_action(&quit);

    //relaod css action
    let reload_css = SimpleAction::new("reload_css", None);
    reload_css.connect_activate(move |_action, _parameter| {
        css::setup_css().unwrap();
    });
    application.set_accels_for_action("app.reload_css", &["<Primary><Shift>C"]);
    application.add_action(&reload_css);

    let app = RelmApp::new("relm4.test.simple");
    css::setup_css()?;
    app.run::<AppModel>(());
    Ok(())
}
