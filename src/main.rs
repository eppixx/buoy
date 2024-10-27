use clap::Parser;
use relm4::{gtk, RelmApp};

use app::App;

mod app;
pub mod client;
pub mod common;
pub mod components;
pub mod config;
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

const LOG_PARA: &str = "info,bouy:trace,submarine:info";

#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// starts and closes the application and shows the startup time
    #[arg(short, long)]
    time_startup: bool,

    /// using a alternative id; a debug option
    #[arg(long, default_value = "com.github.eppixx.buoy")]
    alternative_id: String,

    /// using a alternative title; a debug option
    #[arg(long, default_value = "buoy")]
    alternative_title: String,
}

fn main() -> anyhow::Result<()> {
    //enable logging
    // use filters from RUST_LOG variable when given, otherwise use default filters
    let filter = match tracing_subscriber::EnvFilter::builder()
        .with_env_var("RUST_LOG")
        .try_from_env()
    {
        Ok(filter) => filter,
        Err(_) => tracing_subscriber::EnvFilter::builder().parse(LOG_PARA)?,
    };
    tracing_subscriber::fmt().with_env_filter(filter).init();

    let args = Args::parse();

    let app = RelmApp::new(&args.alternative_id);
    load_css();
    // gtk parses arguments and conclicts with clap
    app.with_args(vec![]).run_async::<App>(args);

    Ok(())
}

fn load_css() {
    use gtk::gdk;
    let display = gdk::Display::default().expect("Could not get default display.");
    let provider = gtk::CssProvider::new();
    let priority = gtk::STYLE_PROVIDER_PRIORITY_APPLICATION;

    provider.load_from_data(include_str!("../data/buoy.css"));
    gtk::style_context_add_provider_for_display(&display, &provider, priority);
}
