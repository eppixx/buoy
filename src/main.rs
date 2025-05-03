use std::{cell::RefCell, rc::Rc};

use clap::Parser;
use components::main_window::MainWindow;
use config::GETTEXT_PACKAGE;
use relm4::{gtk, RelmApp};
use tracing_subscriber::layer::SubscriberExt;

pub mod client;
pub mod common;
pub mod components;
pub mod config;
pub mod css;
mod download;
mod factory;
pub mod gtk_helper;
mod mpris;
mod playback;
pub mod settings;
pub mod subsonic;
pub mod subsonic_cover;
pub mod types;
pub mod views;

const DEFAULT_LOG_ENV_PARA: &str = "info,bouy:trace,submarine:info";
const LOG_PREFIX: &str = "Buoy";
const LOG_FILE_NAME: &str = "log.txt";

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

    // setup folder cache and config
    let cache_path = dirs::cache_dir()
        .expect("cant create cache dir")
        .join(LOG_PREFIX);
    std::fs::create_dir_all(&cache_path).expect("could not create cache folder");
    let config_path = dirs::config_dir()
        .expect("cant create config dir")
        .join(LOG_PREFIX);
    std::fs::create_dir_all(config_path).expect("could not create config folder");

    // invert debug logging when compiled with debug mode
    #[cfg(debug_assertions)]
    {
        let mut args = args.borrow_mut();
        args.debug_logs = !args.debug_logs;
    }

    //enable logging
    if args.borrow().debug_logs {
        // use filters from RUST_LOG variable when given, otherwise use default filters
        let default_filter = tracing_subscriber::EnvFilter::builder()
            .parse(DEFAULT_LOG_ENV_PARA)
            .expect("cant parse default parameter");
        let env_filter = tracing_subscriber::EnvFilter::builder()
            .with_env_var("RUST_LOG")
            .try_from_env()
            .unwrap_or(default_filter);

        // setup log file
        let cache_path = cache_path.join(LOG_FILE_NAME);
        let log_file = std::fs::File::create(cache_path).expect("cant create log file");

        // create file layer
        let file_layer = tracing_subscriber::fmt::layer()
            .with_writer(log_file) // set file to write to
            .with_ansi(false); // disable color for file output

        // create subscriber
        let subscriber = tracing_subscriber::Registry::default()
            .with(env_filter)
            .with(file_layer)
            .with(tracing_subscriber::fmt::layer().with_target(false));

        // set subscriber as default output
        tracing::subscriber::set_global_default(subscriber).expect("failed to set subscriber");
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
