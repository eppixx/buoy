use app::App;
use components::login_window::LoginWindow;
use relm4::RelmApp;

mod app;
pub mod client;
pub mod common;
pub mod components;
pub mod css;
mod factory;
mod play_state;
mod playback;
pub mod settings;
pub mod subsonic;
pub mod subsonic_cover;
pub mod types;

fn main() -> anyhow::Result<()> {
    //enable logging
    tracing_subscriber::fmt()
        .with_span_events(tracing_subscriber::fmt::format::FmtSpan::FULL)
        .with_max_level(tracing::Level::INFO)
        .init();

    //load css file
    let data = std::fs::read_to_string("data/bouy.css")?;

    let app = RelmApp::new("com.github.eppixx.bouy");
    app.set_global_css(&data);
    // app.run_async::<LoginWindow>(());
    app.run_async::<App>(());
    tracing::error!("past login");

    Ok(())
}
