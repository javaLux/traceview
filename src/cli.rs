use std::path::PathBuf;

use crate::{
    app::config::CONFIG_NAME,
    utils::{absolute_path_as_string, config_dir, data_dir, format_path_for_display},
};
use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version = version(), about)]
pub struct Cli {
    #[arg(
        short,
        long,
        value_name = "FILE",
        help = format!("Set a custom config file [default: {}]", format_path_for_display(absolute_path_as_string(config_dir().join(CONFIG_NAME)))),
        value_parser = validate_config_file,
    )]
    pub config: Option<PathBuf>,
}

/// Helper function to validate the config file option [-c, -config]
fn validate_config_file(config: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(config);
    if !path.is_file() {
        Err("The specified configuration file does not exist".into())
    } else {
        Ok(path)
    }
}

/// Extends the default ``clap --version`` with a custom version message
pub fn version() -> String {
    let authors = env!("CARGO_PKG_AUTHORS").replace(":", ", ");
    let version = env!("CARGO_PKG_VERSION");
    let repo = env!("CARGO_PKG_REPOSITORY");

    let config_dir = format_path_for_display(absolute_path_as_string(config_dir()));
    let data_dir = format_path_for_display(absolute_path_as_string(data_dir()));

    println!();
    format!(
        "\
    --- developed with ♥ in Rust
    Authors          : {authors}
    Version          : {version}
    Repository       : {repo}

    Config directory : {config_dir}
    Data directory   : {data_dir}
    "
    )
}
