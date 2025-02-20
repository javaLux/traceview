use anyhow::{Context, Result};
use console::style;
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::prelude::Rect;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::unbounded_channel;

use crate::{
    app::{actions::Action, config::AppConfig},
    component::Component,
    file_handling::ExplorerTask,
    tui,
    ui::{
        about_widget::AboutPage, explorer_widget::ExplorerWidget, footer_widget::Footer,
        help_widget::HelpPage, info_widget::SystemOverview, metadata_widget::MetadataPage,
        result_widget::ResultWidget, search_widget::SearchWidget, title_widget::TitleBar,
    },
};

pub mod actions;
pub mod config;
pub mod key_bindings;

pub const APP_NAME: &str = env!("CARGO_CRATE_NAME");
pub const GRACEFUL_SHUTDOWN_MSG: &str = "Graceful shutdown... success";
pub const FORCED_SHUTDOWN_MSG: &str = "Forced shutdown - Not all operations could be completed";

/// Represents the possible app context
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AppContext {
    /// The Explorer page to walk through the file system
    #[default]
    Explorer,
    /// The Search page called from Explorer, to search for files or folders
    Search,
    /// Result page called from the Search page
    Results,
    /// Helper context for the Help-Page => describes possible contexts
    All,
    NotActive,
}

impl std::fmt::Display for AppContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppContext::Explorer => write!(f, "Explorer"),
            AppContext::Search => write!(f, "Search"),
            AppContext::Results => write!(f, "Result"),
            AppContext::All => write!(f, "All Contexts"),
            AppContext::NotActive => write!(f, ""),
        }
    }
}

// Represents possible states of the app
#[derive(Debug, PartialEq, PartialOrd, Eq, Serialize, Deserialize, Clone)]
pub enum AppState {
    Done(String),
    Failure(String),
    Working(String),
}

impl AppState {
    pub fn done_empty() -> Self {
        Self::Done("".to_string())
    }
}

impl std::fmt::Display for AppState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppState::Done(msg) => write!(f, "{}", msg),
            AppState::Failure(err) => write!(f, "{}", err),
            AppState::Working(msg) => write!(f, "{}", msg),
        }
    }
}

/// Application
pub struct App {
    config: AppConfig,
    components: Vec<Box<dyn Component>>,
    /// Refresh rate, i.e. ticks per second the system usage should be updated
    tick_rate: f64,
    /// Frame rate, i.e. number of frames per second
    frame_rate: f64,
    should_quit: bool,
    is_forced_shutdown: bool,
}

impl App {
    /// Constructs a new instance of [`App`].
    pub fn new(tick_rate: u8, frame_rate: u8, config: AppConfig) -> Self {
        let title_bar = TitleBar::default();
        let sys_info = SystemOverview::default();
        let file_explorer =
            ExplorerWidget::new(config.start_dir().clone(), config.follow_sym_links());
        let search_widget = SearchWidget::default();
        let result_widget = ResultWidget::default();
        let footer = Footer::default();
        let help_page = HelpPage::default();
        let about_page = AboutPage::default();
        let metadata_page = MetadataPage::default();

        Self {
            config,
            components: vec![
                Box::new(title_bar),
                Box::new(sys_info),
                Box::new(file_explorer),
                Box::new(search_widget),
                Box::new(result_widget),
                Box::new(footer),
                Box::new(help_page),
                Box::new(about_page),
                Box::new(metadata_page),
            ],
            tick_rate: tick_rate as f64,
            frame_rate: frame_rate as f64,
            should_quit: false,
            is_forced_shutdown: false,
        }
    }

    pub async fn run(&mut self) -> Result<()> {
        // We are going introduce a new mpsc::unbounded_channel to communicate between the different app components.
        // The advantage of this is that we can programmatically trigger updates to the state of the app by sending Actions on the channel.
        let (component_tx, mut component_rx) = unbounded_channel::<Action>();

        // Use a separate channel to communicate with the Explorer background task
        let (explorer_tx, explorer_rx) = unbounded_channel::<Action>();

        // build the TUI
        let mut tui = tui::Tui::new()?
            .tick_rate(self.tick_rate)
            .frame_rate(self.frame_rate);

        // init the TUI and starts the TUI-Event-Handler loop
        tui.enter()?;

        let terminal_size = tui.size()?;
        let area = tui.get_frame().area();

        for component in self.components.iter_mut() {
            component.init_terminal_size(terminal_size)?;
        }

        for component in self.components.iter_mut() {
            component.init_area(area)?;
        }

        for component in self.components.iter_mut() {
            component.register_component_action_sender(component_tx.clone())?;
        }

        // Register the Explorer-Action-Sender for each app component that needs it
        for component in self.components.iter_mut() {
            component.register_explorer_action_sender(explorer_tx.clone())?;
        }

        // Register the config handler for each component
        for component in self.components.iter_mut() {
            component.register_config_handler(self.config.clone())?;
        }

        // Init and run the explorer background task
        let mut explorer_task = ExplorerTask::new(component_tx.clone());
        explorer_task.run(explorer_rx);

        // This is the Application main loop
        loop {
            // Try to receive some TUI-Events
            let tui_event = tui.next().await?;

            for component in self.components.iter_mut() {
                if let Some(action) = component.handle_events(Some(tui_event.clone())).await? {
                    component_tx.send(action)?;
                }
            }

            // Map TUI-Events to Application Actions
            match tui_event {
                tui::Event::Error(err) => component_tx.send(Action::Error(err))?,
                tui::Event::AppTick => component_tx.send(Action::Tick)?,
                tui::Event::RenderTick => component_tx.send(Action::Render)?,
                tui::Event::FocusGained => component_tx.send(Action::Resume)?,
                tui::Event::FocusLost => component_tx.send(Action::Suspend)?,
                tui::Event::Key(key_event) => match key_event.code {
                    // Quit the app at any time
                    KeyCode::Char('q') if key_event.modifiers == KeyModifiers::CONTROL => {
                        component_tx.send(Action::Quit)?
                    }
                    // KeyCode::Char('p') => panic!("Testing the panic handler"),
                    // KeyCode::Char('e') => component_tx.send(Action::Error(
                    //     "Testing application error".to_string(),
                    // ))?,
                    _ => component_tx.send(Action::None)?,
                },
                tui::Event::Resize(w, h) => component_tx.send(Action::Resize(w, h))?,
                _ => component_tx.send(Action::None)?,
            }

            // handle application actions
            while let Ok(action) = component_rx.try_recv() {
                match action {
                    Action::ForcedShutdown => self.is_forced_shutdown = true,
                    Action::Quit => self.should_quit = true,
                    // draw to the screen buffer only if Action::Render or Action::Resize will received
                    Action::Render => {
                        tui.draw(|f| {
                            for component in self.components.iter_mut() {
                                let _ = component.render(f, f.area());
                            }
                        })
                        .with_context(|| "Failed to render UI on screen")?;
                    }
                    Action::Resize(w, h) => {
                        tui.resize(Rect::new(0, 0, w, h))?;
                        tui.draw(|f| {
                            for component in self.components.iter_mut() {
                                let _ = component.render(f, f.area());
                            }
                        })
                        .with_context(|| "Failed to draw UI on screen while resizing")?;
                    }
                    Action::Error(err) => {
                        return Err(anyhow::anyhow!(format!(
                            "Internal application error: {}",
                            err
                        )));
                    }
                    _ => {}
                }

                // Update App components dependent on the received Action
                for component in self.components.iter_mut() {
                    if let Some(action) = component.update(action.clone()).await? {
                        component_tx.send(action)?
                    };
                }
            }

            if self.should_quit {
                explorer_task.stop();
                tui.stop();
                break;
            }
        }

        tui.exit()?;

        if explorer_task.is_forced_shutdown() || self.is_forced_shutdown {
            log::warn!("[{}] => {}", APP_NAME, FORCED_SHUTDOWN_MSG);
            println!(
                "{} [{}] => {}",
                style("[WARN]").yellow().bold(),
                APP_NAME,
                FORCED_SHUTDOWN_MSG
            )
        } else {
            log::info!("[{}] => {}", APP_NAME, GRACEFUL_SHUTDOWN_MSG);
            println!(
                "{} [{}] => {}",
                style("[INFO]").color256(40), // 40 = Light green
                APP_NAME,
                GRACEFUL_SHUTDOWN_MSG
            )
        }

        Ok(())
    }
}
