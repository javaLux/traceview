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

use std::process::ExitCode;

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
async fn main() -> ExitCode {
    match run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            report_error(&err);
            ExitCode::FAILURE
        }
    }
}

async fn run() -> Result<()> {
    let args = Cli::parse();
    create_data_dir()?;
    initialize_logging()?;
    initialize_panic_hook()?;

    // use the given config path or fall back to the default location
    let config_path = args
        .config
        .unwrap_or_else(|| config_dir().join(CONFIG_NAME));

    // load configuration or store a new one
    let config = AppConfig::load_config(&config_path);

    let app_result = App::new(config, config_path)?.run().await;

    if app_result.is_err() {
        // Reset the terminal before the error gets printed
        if let Err(reset_err) = reset_terminal() {
            log::warn!("failed to reset terminal: {reset_err:#}");
        }
    }

    app_result
}

fn reset_terminal() -> Result<()> {
    let mut tui = Tui::new()?;
    tui.exit()?;
    Ok(())
}

fn report_error(err: &anyhow::Error) {
    log::error!("{err:#}");
    eprintln!(
        "{} - Something went wrong while running {}",
        style("[ERROR]").bold().red(),
        app_name(),
    );
    eprintln!("\t=> {err:#}");
}
