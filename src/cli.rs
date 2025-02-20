use std::path::PathBuf;

use crate::{
    app::config::CONFIG_NAME,
    utils::{absolute_path_as_string, config_dir, format_path_for_display, version},
};
use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version = version(), about)]
pub struct Cli {
    #[arg(
        short,
        long,
        value_name = "INTEGER",
        help = "Refresh rate, i.e. ticks per second the system usage should be updated [max. 5]",
        default_value_t = 1,
        value_parser = clap::value_parser!(u8).range(1..=5),
    )]
    pub refresh_rate: u8,

    #[arg(
        short,
        long,
        value_name = "INTEGER",
        help = "Frame rate, i.e. the number of frames rendered per second [max. 60]",
        default_value_t = 45,
        value_parser = clap::value_parser!(u8).range(1..=60),
    )]
    pub frame_rate: u8,

    #[arg(
        short,
        long,
        value_name = "FILE",
        help = format!("Set a specific config file [default: {}]", format_path_for_display(absolute_path_as_string(config_dir().join(CONFIG_NAME)))),
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
