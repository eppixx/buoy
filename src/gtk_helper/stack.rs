use std::fmt::Display;

use relm4::gtk;

pub trait StackExt {
    fn add_enumed<T: Display + TryFrom<String>>(
        &self,
        child: &impl gtk::prelude::IsA<gtk::Widget>,
        state: T,
    ) -> gtk::StackPage;
    fn set_visible_child_enum<T: Display + TryFrom<String>>(&self, state: &T);
    fn visible_child_enum<T: TryFrom<String>>(&self) -> Option<T>;
}

impl StackExt for gtk::Stack {
    fn add_enumed<T: Display>(
        &self,
        child: &impl gtk::prelude::IsA<gtk::Widget>,
        state: T,
    ) -> gtk::StackPage {
        self.add_named(child, Some(&state.to_string()))
    }

    fn set_visible_child_enum<T: Display + TryFrom<String>>(&self, state: &T) {
        self.set_visible_child_name(&state.to_string());
    }

    fn visible_child_enum<T>(&self) -> Option<T>
    where
        T: TryFrom<String>,
    {
        let name = self.visible_child_name()?;
        T::try_from(name.to_string()).ok()
    }
}

#[cfg(test)]
pub fn test_self<T>(state: T)
where
    T: Display + TryFrom<String> + PartialEq + std::fmt::Debug,
    <T as TryFrom<std::string::String>>::Error: PartialEq,
    <T as TryFrom<std::string::String>>::Error: std::fmt::Debug,
{
    assert_eq!(Ok(&state), T::try_from(state.to_string()).as_ref());
}
