use relm4::gtk::{
    self,
    traits::{BoxExt, ButtonExt, WidgetExt},
};

pub trait Sequence: std::fmt::Debug + 'static {
    fn current(&self) -> &str;
    fn next(&mut self);
    fn tooltip(&self) -> &str;
}

pub struct SequenceButtonModel<T: Sequence> {
    btn: gtk::Button,
    sequence: T,
}

impl<T: Sequence> SequenceButtonModel<T> {
    pub fn current(&self) -> &T {
        &self.sequence
    }
}

#[derive(Debug)]
pub enum SequenceButtonInput {
    Toggle,
}

#[derive(Debug)]
pub enum SequenceButtonOutput {
    Status(Box<dyn Sequence>),
}

#[relm4::component(pub)]
impl<T: Sequence> relm4::SimpleComponent for SequenceButtonModel<T> {
    type Init = T;
    type Input = SequenceButtonInput;
    type Output = SequenceButtonOutput;

    fn init(
        params: Self::Init,
        root: &Self::Root,
        sender: relm4::ComponentSender<Self>,
    ) -> relm4::ComponentParts<Self> {
        let model = SequenceButtonModel {
            btn: gtk::Button::new(),
            sequence: params,
        };

        let widgets = view_output!();
        relm4::ComponentParts { model, widgets }
    }

    view! {
        gtk::Box {
            append = &model.btn.clone() {
                set_icon_name: model.sequence.current(),
                set_tooltip_text: Some(model.sequence.tooltip()),
                connect_clicked => SequenceButtonInput::Toggle,
            }
        }
    }

    fn update(&mut self, msg: Self::Input, _sender: relm4::ComponentSender<Self>) {
        match msg {
            SequenceButtonInput::Toggle => {
                self.sequence.next();
                self.btn.set_icon_name(self.sequence.current());
                self.btn.set_tooltip_text(Some(self.sequence.tooltip()));
            }
        }
    }
}
