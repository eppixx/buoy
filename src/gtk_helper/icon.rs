use std::sync::OnceLock;

use relm4::gtk;

static ICON: OnceLock<Icon> = OnceLock::new();

pub struct Icon<'a> {
    fallbacks: std::collections::HashMap<&'a str, String>,
}

impl<'a> Default for Icon<'a> {
    fn default() -> Self {
        let fallbacks = [
            #[cfg(test)]
            ("nonsense-name", String::from("list-remove-symbolic")),
            #[cfg(test)]
            ("test", String::from("testi")),
            // real icons
            ("playlist-symbolic", String::from("document-open"))
        ];
        Self {
            fallbacks: fallbacks.into(),
        }
    }
}

impl<'a> Icon<'a> {
    pub fn from_str(name: &str) -> &str {
        let theme = gtk::IconTheme::default();
        if theme.has_icon(name) {
            return name;
        }

        // icon is not in theme, lookup possible backup
        let icon_map = ICON.get_or_init(|| Icon::default());
        if let Some(icon_name) = icon_map.fallbacks.get(name) {
            icon_name
        } else {
            name
        }
    }
}

#[cfg(test)]

mod tests {
    use super::Icon;

    #[test]
    fn test() {
        relm4::gtk::init().unwrap();

        // no match in map
        assert_eq!(Icon::from_str("list-add"), "list-add");
        assert_eq!(Icon::from_str("starred"), "starred");
        // matches with no icon in theme
        assert_eq!(Icon::from_str("nonsense-name"), "list-remove-symbolic");
        assert_eq!(Icon::from_str("test"), "testi");
    }
}
