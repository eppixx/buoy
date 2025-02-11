use std::{cell::RefCell, rc::Rc};

use clap::Parser;
use components::main_window::MainWindow;
use config::GETTEXT_PACKAGE;
use relm4::{gtk, RelmApp};

mod app;
pub mod client;
pub mod common;
pub mod components;
pub mod config;
pub mod css;
mod download;
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
pub mod views;

const LOG_PARA: &str = "info,bouy:trace,submarine:info";

#[derive(Parser, Debug, Clone)]
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

    /// logs infos about events while running; useful for debugging bugs
    #[arg(short, long)]
    debug_logs: bool,
}

fn main() -> anyhow::Result<()> {
    let args = Rc::new(RefCell::new(Args::parse()));

    // invert debug logging when compiled with debug mode
    #[cfg(debug_assertions)]
    {
        let mut args = args.borrow_mut();
        args.debug_logs = !args.debug_logs;
    }

    //enable logging
    if args.borrow().debug_logs {
        // use filters from RUST_LOG variable when given, otherwise use default filters
        let filter = match tracing_subscriber::EnvFilter::builder()
            .with_env_var("RUST_LOG")
            .try_from_env()
        {
            Ok(filter) => filter,
            Err(_) => tracing_subscriber::EnvFilter::builder().parse(LOG_PARA)?,
        };
        tracing_subscriber::fmt().with_env_filter(filter).init();
    }

    // setup of gettext translations
    gettextrs::bindtextdomain(GETTEXT_PACKAGE, config::LOCALEDIR)
        .expect("Unable to bind the text domain");
    gettextrs::bind_textdomain_codeset(GETTEXT_PACKAGE, "UTF-8")
        .expect("Unable to set the text domain encoding");
    gettextrs::textdomain(GETTEXT_PACKAGE).expect("Unable to switch to the text domain");

    let app = RelmApp::new(&args.borrow().alternative_id);
    load_css();
    // gtk parses arguments and conclicts with clap
    app.with_args(vec![]).run_async::<MainWindow>(args);

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
