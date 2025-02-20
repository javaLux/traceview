use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::{ui::Theme, utils};

pub const CONFIG_NAME: &str = "config.toml";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// Define the app theme [Light, Dark, Dracula, Indigo]
    theme: Theme,
    /// Directory in which the Explorer should start
    start_dir: PathBuf,
    /// Directory to which the search results are to be exported
    export_dir: PathBuf,
    /// Enable/Disable following symbolic links
    follow_sym_links: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            theme: Default::default(),
            start_dir: utils::user_home_dir().map_or(
                {
                    std::env::current_dir()
                        .ok()
                        .unwrap_or(PathBuf::new().join("."))
                },
                |home_dir| home_dir,
            ),
            export_dir: utils::data_dir(),
            follow_sym_links: false,
        }
    }
}

impl AppConfig {
    fn ignore_given_init_dir(&mut self) {
        self.start_dir = utils::user_home_dir().map_or(
            {
                std::env::current_dir()
                    .ok()
                    .unwrap_or(PathBuf::new().join("."))
            },
            |home_dir| home_dir,
        );
    }

    fn ignore_given_export_dir(&mut self) {
        self.export_dir = utils::data_dir();
    }

    pub fn load_config<P: AsRef<Path>>(p: P) -> AppConfig {
        let config_file = p.as_ref();

        if !config_file.exists() {
            let config = AppConfig::default();
            // If no config found, try to store a new config file at start up
            if let Err(config_err) = confy::store_path(config_file, config.clone()) {
                log::error!(
                    "Failed to store a new configuration file at '{}'",
                    utils::absolute_path_as_string(config_file)
                );
                log::error!("Config error: {:#?}", config_err);
                log::error!("Fallback to the default configuration");
            }
            config
        } else {
            // Try to load the given configuration file
            match confy::load_path::<AppConfig>(config_file) {
                Ok(config) => config.validate_config(),
                Err(config_err) => {
                    log::error!(
                        "Failed to load configuration file '{}'",
                        utils::absolute_path_as_string(config_file)
                    );
                    log::error!("Config error: {:#?}", config_err);
                    log::error!("Fallback to the default configuration");
                    AppConfig::default()
                }
            }
        }
    }

    fn validate_config(self) -> Self {
        let mut config = self.clone();

        if !self.start_dir.is_dir() {
            log::error!("Invalid path found for config option 'start_dir' -> path will be ignored, fallback to default");
            config.ignore_given_init_dir();
        }

        if !self.export_dir.is_dir() {
            log::error!(
                "Invalid path found for config option 'export_dir' -> path will be ignored, fallback to default"
            );
            config.ignore_given_export_dir();
        }

        config
    }

    pub fn theme(&self) -> Theme {
        self.theme
    }

    pub fn start_dir(&self) -> PathBuf {
        self.start_dir.clone()
    }

    pub fn export_dir(&self) -> PathBuf {
        self.export_dir.clone()
    }

    pub fn follow_sym_links(&self) -> bool {
        self.follow_sym_links
    }
}
