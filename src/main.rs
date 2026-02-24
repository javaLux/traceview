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
        App,
        config::{AppConfig, CONFIG_NAME},
    },
    cli::Cli,
    panic_handling::initialize_panic_hook,
    tui::Tui,
    utils::{app_name, config_dir, create_data_dir, initialize_logging},
};

#[tokio::main]
async fn main() -> Result<()> {
    let args = Cli::parse();
    create_data_dir()?;
    initialize_logging()?;
    initialize_panic_hook()?;

    // get the config file path
    let config_path = args.config.unwrap_or(config_dir().join(CONFIG_NAME));

    // load configuration or store a new one
    let config = AppConfig::load_config(&config_path);

    let mut app = App::new(config, config_path);
    if let Err(err) = app.run().await {
        // Reset the terminal before printing the error
        let mut tui = Tui::new()?;
        tui.exit()?;
        log::error!("{err}");
        println!(
            "{} - Something went wrong while running {}",
            style("[ERROR]").bold().red(),
            app_name(),
        );
        println!("\t=> {err}");
        std::process::exit(1);
    }

    Ok(())
}
