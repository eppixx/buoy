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
    use tracing_subscriber::{prelude::*, EnvFilter};
    //take rules from var RUST_LOG
    //example RUST_LOG=warn or RUST_LOG=relm4=info,buoy=info
    let env_filter = EnvFilter::from_default_env();
    //set some default for stdout
    let stdout_log = tracing_subscriber::fmt::layer()
        .compact()
        .with_span_events(tracing_subscriber::fmt::format::FmtSpan::FULL)
        .with_filter(tracing_subscriber::filter::LevelFilter::INFO);
    tracing_subscriber::registry()
        .with(env_filter)
        .with(stdout_log)
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
