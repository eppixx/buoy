use app::App;
use relm4::RelmApp;

mod app;
pub mod client;
pub mod common;
pub mod components;
pub mod css;
mod factory;
pub mod gtk_helper;
mod mpris;
mod play_state;
mod playback;
pub mod player;
pub mod settings;
pub mod subsonic;
pub mod subsonic_cover;
pub mod types;
pub mod window_state;

fn main() -> anyhow::Result<()> {
    //enable logging
    // use filters from RUST_LOG variable when given, otherwise use default filters
    let filter = match tracing_subscriber::EnvFilter::builder()
        .with_env_var("RUST_LOG")
        .try_from_env()
    {
        Ok(filter) => filter,
        Err(_) => {
            tracing_subscriber::EnvFilter::builder().parse("info,buoy:trace,submarine:info")?
        }
    };
    tracing_subscriber::fmt().with_env_filter(filter).init();

    //load css file
    let data = std::fs::read_to_string("data/buoy.css").expect("css file not found");
    let app = RelmApp::new("com.github.eppixx.buoy");
    app.set_global_css(&data);
    app.run_async::<App>(());

    Ok(())
}
