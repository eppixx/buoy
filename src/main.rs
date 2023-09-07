use components::{
    play_controls::{PlayControlModel, PlayControlOutput},
    queue::{QueueModel, QueueInput},
};
use gtk::prelude::{BoxExt, GtkWindowExt, OrientableExt};
use relm4::{
    gtk::{
        self,
        gio::SimpleAction,
        prelude::{ActionMapExt, ApplicationExt},
        traits::{GtkApplicationExt, WidgetExt},
    },
    Component, ComponentController, ComponentParts, ComponentSender, Controller, RelmApp,
    SimpleComponent,
};

use crate::play_state::PlayState;

mod components;
pub mod css;
mod factory;
mod play_state;
pub mod types;

struct AppModel {
    queue: Controller<QueueModel>,
    play_controls: Controller<PlayControlModel>,
}

#[derive(Debug)]
enum AppMsg {
    PlayControlOutput(PlayControlOutput),
}

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
        let play_controls = PlayControlModel::builder()
            .launch(PlayState::Pause) // TODO change to previous state
            .forward(sender.input_sender(), |msg| AppMsg::PlayControlOutput(msg));
        let model = AppModel {
            queue,
            play_controls,
        };

        // Insert the macro code generation here
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>) {
        match msg {
            AppMsg::PlayControlOutput(PlayControlOutput::Next) => {
                _ = self.queue.sender().send(QueueInput::PlayNext);
            }
            AppMsg::PlayControlOutput(PlayControlOutput::Previous) => {
                _ = self.queue.sender().send(QueueInput::PlayPrevious);
            }
            AppMsg::PlayControlOutput(PlayControlOutput::Status(status)) => {
                _ = self.queue.sender().send(QueueInput::NewState(status));
            }
        }
    }

    view! {
        #[root]
        gtk::Window {
            add_css_class: "main-window",
            set_title: Some("Bouy"),
            set_default_width: 500,
            set_default_height: 700,

            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_spacing: 5,

                append: model.play_controls.widget(),

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

    let app = RelmApp::new("com.github.eppixx.bouy");
    css::setup_css()?;
    app.run::<AppModel>(());
    Ok(())
}
