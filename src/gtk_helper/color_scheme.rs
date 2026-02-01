use ashpd::desktop::settings::{ColorScheme, Settings};
use futures::StreamExt;
use relm4::gtk;

pub fn listen_to_color_scheme() {
    gtk::glib::spawn_future_local(async move {
        let gtk_settings = gtk::Settings::default().expect("Unable to get the GtkSettings object");
        let settings = Settings::new().await.unwrap();

        // set color scheme
        let scheme = settings.color_scheme().await.unwrap();
        let is_dark = match scheme {
            ColorScheme::PreferDark => true,
            ColorScheme::NoPreference | ColorScheme::PreferLight => false,
        };
        gtk_settings.set_gtk_application_prefer_dark_theme(is_dark);

        // watch changes in color scheme
        loop {
            let color_scheme_stream = settings.receive_color_scheme_changed();
            match color_scheme_stream.await {
                Ok(mut scheme) => {
                    while let Some(scheme) = scheme.next().await {
                        let is_dark = match scheme {
                            ColorScheme::PreferDark => true,
                            ColorScheme::NoPreference | ColorScheme::PreferLight => false,
                        };
                        gtk_settings.set_gtk_application_prefer_dark_theme(is_dark);
                    }
                }
                Err(e) => tracing::error!("error listening to color scheme: {e}"),
            }
        }
    });
}
