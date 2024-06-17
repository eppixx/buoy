use relm4::{
    component::AsyncComponentController,
    gtk::{self, prelude::GtkWindowExt, prelude::WidgetExt},
};

use crate::{
    components::login_form::{LoginForm, LoginFormOut},
    settings::Settings,
};

#[derive(Debug)]
pub struct LoginWindow {
    window: gtk::Window,
    login_form: relm4::component::AsyncController<LoginForm>,
}

#[derive(Debug)]
pub enum LoginWindowIn {
    LoginForm(LoginFormOut),
}

#[derive(Debug)]
pub enum LoginWindowOut {
    LoggedIn,
}

#[relm4::component(async, pub)]
impl relm4::component::AsyncComponent for LoginWindow {
    type Init = ();
    type Input = LoginWindowIn;
    type Output = LoginWindowOut;
    type CommandOutput = ();

    async fn init(
        _init: Self::Init,
        root: Self::Root,
        sender: relm4::AsyncComponentSender<Self>,
    ) -> relm4::component::AsyncComponentParts<Self> {
        let login_form: relm4::component::AsyncController<LoginForm> = LoginForm::builder()
            .launch(())
            .forward(sender.input_sender(), LoginWindowIn::LoginForm);

        let model = LoginWindow {
            window: gtk::Window::default(),
            login_form,
        };
        let widgets = view_output!();
        relm4::component::AsyncComponentParts { model, widgets }
    }

    view! {
        gtk::Window {
            model.login_form.widget() {
                set_hexpand: true,
                set_vexpand: true,
                set_halign: gtk::Align::Center,
                set_valign: gtk::Align::Center,
            }
        }
    }

    async fn update(
        &mut self,
        msg: Self::Input,
        sender: relm4::AsyncComponentSender<Self>,
        root: &Self::Root,
    ) {
        match msg {
            LoginWindowIn::LoginForm(LoginFormOut::LoggedIn) => {
                tracing::error!("login accepted");
                //TODO go to main window
                Settings::get().lock().unwrap().save();
                root.close();
                let _ = sender.output(LoginWindowOut::LoggedIn);
            }
        }
    }
}
