#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WindowState {
    Loading,
    LoginForm,
    Main,
}

impl std::fmt::Display for WindowState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Loading => write!(f, "Loading"),
            Self::LoginForm => write!(f, "Login Form"),
            Self::Main => write!(f, "Main"),
        }
    }
}

impl TryFrom<String> for WindowState {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.as_ref() {
            "Loading" => Ok(Self::Loading),
            "Login Form" => Ok(Self::LoginForm),
            "Main" => Ok(Self::Main),
            e => Err(format!("\"{e}\" is not a State")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gtk_helper::stack::test_self;

    #[test]
    fn window_state_conversion() {
        test_self(WindowState::Loading);
        test_self(WindowState::LoginForm);
        test_self(WindowState::Main);
    }
}
