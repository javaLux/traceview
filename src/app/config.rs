use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::{
    ui::Theme,
    utils::{absolute_path_as_string, data_dir, format_path_for_display, user_home_dir},
};

pub const CONFIG_NAME: &str = "config.toml";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppConfig {
    /// The default app theme on start up [Light, Dark, Dracula, Indigo]
    default_theme: Theme,
    /// Directory in which the Explorer should start
    start_dir: PathBuf,
    /// Directory to which the search results are to be exported
    export_dir: PathBuf,
    /// Enable/Disable following symbolic links
    follow_sym_links: bool,
    /// Update Rate per second for the System-Overview
    system_update_rate: u8,
    /// Frames per Second (Rendering)
    fps: u8,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            default_theme: Default::default(),
            start_dir: user_home_dir().map_or(
                {
                    std::env::current_dir()
                        .ok()
                        .unwrap_or(PathBuf::new().join("."))
                },
                |home_dir| home_dir,
            ),
            export_dir: data_dir(),
            follow_sym_links: false,
            system_update_rate: 1,
            fps: 45,
        }
    }
}

impl AppConfig {
    pub fn load_config<P: AsRef<Path>>(p: P) -> AppConfig {
        let config_file = p.as_ref();

        if !config_file.exists() {
            let config = AppConfig::default();
            // If no config found, try to store a new config file at start up
            if let Err(err) = confy::store_path(config_file, config.clone()) {
                log::error!(
                    "Failed to store a new configuration file at [{}]",
                    absolute_path_as_string(config_file)
                );
                AppConfig::log_confy_err(err);
            }
            config
        } else {
            // Try to load the given configuration file
            match confy::load_path::<AppConfig>(config_file) {
                Ok(config) => config.validate_config(),
                Err(err) => {
                    log::error!(
                        "Failed to load configuration file [{}]",
                        absolute_path_as_string(config_file)
                    );
                    AppConfig::log_confy_err(err);
                    AppConfig::default()
                }
            }
        }
    }

    fn log_confy_err<T: std::error::Error>(err: T) {
        log::error!("{:#?}", err);
        log::warn!("Fallback to the default configuration");
    }

    pub fn save_config<P: AsRef<Path>>(&self, p: P) -> Result<()> {
        confy::store_path(p.as_ref(), self)?;
        Ok(())
    }

    fn validate_config(self) -> Self {
        let mut config = self.clone();

        if !config.start_dir.is_dir() {
            let start_dir = user_home_dir().map_or(
                {
                    std::env::current_dir()
                        .ok()
                        .unwrap_or(PathBuf::new().join("."))
                },
                |home_dir| home_dir,
            );
            log::warn!(
                "[Config] Invalid path for option 'start_dir' -> No such directory, fallback to default [{}]",
                absolute_path_as_string(&start_dir)
            );

            config.start_dir = start_dir;
        }

        if !config.export_dir.is_dir() {
            let export_dir = data_dir();
            log::warn!(
                "[Config] Invalid path for option 'export_dir' -> No such directory, fallback to default [{}]",
                absolute_path_as_string(&export_dir)
            );
            config.export_dir = export_dir;
        }

        if config.fps > 60 {
            let default_fps: u8 = 45;
            log::warn!(
                "[Config] Invalid value for option 'fps' -> It cannot be greater than 60, fallback to default [{}]",
                default_fps
            );
            config.fps = default_fps;
        }

        if config.system_update_rate > 5 {
            let default_update_rate: u8 = 1;
            log::warn!(
                "[Config] Invalid value for option 'system_update_rate' -> It cannot be greater than 5, fallback to default [{}]",
                default_update_rate
            );
            config.system_update_rate = default_update_rate;
        }

        config
    }

    pub fn config_docs(&self, with_description: bool) -> Vec<Vec<String>> {
        let start_dir = format_path_for_display(absolute_path_as_string(self.start_dir()));
        let export_dir = format_path_for_display(absolute_path_as_string(self.export_dir()));
        let follow_sym_links = if self.follow_sym_links() { "Yes" } else { "No" };
        let fps = format!("{} / sec", self.fps());
        let update_rate = format!("{} / sec", self.system_update_rate());

        let rows = vec![
            (
                "Default theme",
                self.theme().to_string(),
                "App theme at startup",
            ),
            (
                "Start directory",
                start_dir,
                "Explorer directory at startup",
            ),
            (
                "Export directory",
                export_dir,
                "Directory to which the search results are exported",
            ),
            (
                "Follow symbolic links",
                follow_sym_links.to_string(),
                "Defines whether Explorer follows symbolic links",
            ),
            (
                "Frames per second (render)",
                fps,
                "Frames per second (TUI render)",
            ),
            (
                "Update rate (System-Resources)",
                update_rate,
                "Update rate of system resource usage per second",
            ),
        ];

        rows.into_iter()
            .map(|(key, value, desc)| {
                if with_description {
                    vec![key.into(), value, desc.into()]
                } else {
                    vec![key.into(), value]
                }
            })
            .collect()
    }

    pub fn set_theme(&mut self, t: Theme) {
        self.default_theme = t;
    }

    pub fn set_start_dir<P: AsRef<Path>>(&mut self, p: P) {
        self.start_dir = p.as_ref().to_path_buf();
    }

    pub fn set_export_dir<P: AsRef<Path>>(&mut self, p: P) {
        self.export_dir = p.as_ref().to_path_buf();
    }

    pub fn set_follow_sym_links(&mut self, yes: bool) {
        self.follow_sym_links = yes;
    }

    pub fn set_system_update_rate(&mut self, rate: u8) {
        self.system_update_rate = rate;
    }

    pub fn set_fps(&mut self, fps: u8) {
        self.fps = fps;
    }

    pub fn theme(&self) -> Theme {
        self.default_theme
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

    pub fn system_update_rate(&self) -> u8 {
        self.system_update_rate
    }

    pub fn fps(&self) -> u8 {
        self.fps
    }
}
