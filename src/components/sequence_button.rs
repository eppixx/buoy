use relm4::gtk::{
    self,
    traits::{BoxExt, ButtonExt, WidgetExt},
};

/// Intended to be use with a SequenceButton
pub trait Sequence: std::fmt::Debug + 'static {
    /// returns icon name
    fn current(&self) -> &str;
    /// switches to next enum type
    fn next(&mut self);
    /// returns a tooltip to display for widget
    fn tooltip(&self) -> Option<&str>;
}

/// A button that changes its icon when pressed
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
    Clicked,
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
                set_tooltip_text: model.sequence.tooltip(),
                connect_clicked => SequenceButtonInput::Toggle,
            }
        }
    }

    fn update(&mut self, msg: Self::Input, sender: relm4::ComponentSender<Self>) {
        match msg {
            SequenceButtonInput::Toggle => {
                self.sequence.next();
                self.btn.set_icon_name(self.sequence.current());
                self.btn.set_tooltip_text(self.sequence.tooltip());
            }
        }
        _ = sender.output(SequenceButtonOutput::Clicked);
    }
}
