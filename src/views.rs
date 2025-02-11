use crate::components::browser;

#[derive(Debug)]
pub enum ClickableViews {
    Dashboard,
    Artists,
    Albums,
    Tracks,
    Playlists,
}

#[derive(Debug)]
pub enum Views {
    Clickable(ClickableViews),
    Artist,
    Album,
}

// cant implement From because of the arguments of browser::Views
#[allow(clippy::from_over_into)]
impl Into<Views> for &browser::Views {
    fn into(self) -> Views {
        match self {
            browser::Views::Dashboard(_) => Views::Clickable(ClickableViews::Dashboard),
            browser::Views::Artists(_) => Views::Clickable(ClickableViews::Artists),
            browser::Views::Albums(_) => Views::Clickable(ClickableViews::Albums),
            browser::Views::Tracks(_) => Views::Clickable(ClickableViews::Tracks),
            browser::Views::Playlists(_) => Views::Clickable(ClickableViews::Playlists),
            browser::Views::Artist(_) => Views::Artist,
            browser::Views::Album(_) => Views::Album,
        }
    }
}
