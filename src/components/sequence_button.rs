use relm4::gtk::{
    self,
    prelude::{BoxExt, ButtonExt, WidgetExt},
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
#[derive(Debug)]
pub struct SequenceButton<T: Sequence> {
    btn: gtk::Button,
    sequence: T,
}

impl<T: Sequence> SequenceButton<T> {
    pub fn current(&self) -> &T {
        &self.sequence
    }
}

#[derive(Debug)]
pub enum SequenceButtonIn {
    Toggle,
}

#[derive(Debug)]
pub enum SequenceButtonOut {
    Clicked,
}

#[relm4::component(pub)]
impl<T: Sequence> relm4::SimpleComponent for SequenceButton<T> {
    type Init = T;
    type Input = SequenceButtonIn;
    type Output = SequenceButtonOut;

    fn init(
        params: Self::Init,
        root: Self::Root,
        sender: relm4::ComponentSender<Self>,
    ) -> relm4::ComponentParts<Self> {
        let model = SequenceButton {
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
                connect_clicked => SequenceButtonIn::Toggle,
            }
        }
    }

    fn update(&mut self, msg: Self::Input, sender: relm4::ComponentSender<Self>) {
        match msg {
            SequenceButtonIn::Toggle => {
                self.sequence.next();
                self.btn.set_icon_name(self.sequence.current());
                self.btn.set_tooltip_text(self.sequence.tooltip());
            }
        }
        _ = sender.output(SequenceButtonOut::Clicked);
    }
}
