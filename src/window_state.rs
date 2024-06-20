pub enum WindowState {
    Loading,
    LoginForm,
    Main,
}

impl WindowState {
    pub fn to_str(&self) -> &str {
        match self {
            Self::Loading => "Loading",
            Self::LoginForm => "Login Form",
            Self::Main => "Main",
        }
    }
}

#[derive(Debug)]
pub enum NavigationMode {
    Normal,
    Search,
}

impl NavigationMode {
    pub fn to_str(&self) -> &str {
        match self {
            Self::Normal => "Normal",
            Self::Search => "Search",
        }
    }
}
