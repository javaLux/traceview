mod app;
mod cli;
mod component;
mod file_handling;
mod models;
mod panic_handling;
mod system;
mod tui;
mod ui;
mod utils;

use anyhow::Result;
use clap::Parser;
use console::style;

use crate::{
    app::{
        config::{AppConfig, CONFIG_NAME},
        App,
    },
    cli::Cli,
    panic_handling::initialize_panic_hook,
    tui::Tui,
    utils::{config_dir, create_data_dir, initialize_logging},
};

#[tokio::main]
async fn main() -> Result<()> {
    let args = Cli::parse();
    create_data_dir()?;
    initialize_logging()?;
    initialize_panic_hook()?;

    // get the config file path
    let config_file = args.config.unwrap_or(config_dir().join(CONFIG_NAME));

    // load a given configuration or store a new one
    let config = AppConfig::load_config(config_file);

    let mut app = App::new(args.refresh_rate, args.frame_rate, config);
    if let Err(err) = app.run().await {
        // Reset the terminal before printing the error
        let mut tui = Tui::new()?;
        tui.exit()?;
        log::error!("{err}");
        println!(
            "{} - Something went wrong while running the app",
            style("[ERROR]").bold().red()
        );
        eprintln!("\t=> {err}");
        std::process::exit(1);
    }

    Ok(())
}
