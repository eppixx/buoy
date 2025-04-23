#[derive(Debug, PartialEq)]
pub enum LoadingWidgetState {
    Empty,
    NotEmpty,
    Loading,
}

impl std::fmt::Display for LoadingWidgetState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Empty => write!(f, "Empty"),
            Self::NotEmpty => write!(f, "NotEmpty"),
            Self::Loading => write!(f, "Loading"),
        }
    }
}

impl TryFrom<String> for LoadingWidgetState {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.as_ref() {
            "Empty" => Ok(Self::Empty),
            "NotEmpty" => Ok(Self::NotEmpty),
            "Loading" => Ok(Self::Loading),
            e => Err(format!("\"{e}\" is not a State")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gtk_helper::stack::test_self;

    #[test]
    fn test_loading_played_state_conversion() {
        test_self(LoadingWidgetState::Empty);
        test_self(LoadingWidgetState::NotEmpty);
        test_self(LoadingWidgetState::Loading);
    }
}
