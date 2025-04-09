use anyhow::Result;
use async_trait::async_trait;
use crossterm::event::{KeyEvent, MouseEvent};
use ratatui::layout::{Rect, Size};

use crate::{
    app::{actions::Action, config::AppConfig},
    tui::Event,
};

#[async_trait(?Send)]
/// `Component` is a trait that represents a visual and interactive element of the user interface.
/// Implementors of this trait can be registered with the main application loop and will be able to receive events,
/// update state, and be rendered on the screen.
pub trait Component {
    /// Register an separate action sender that can send actions for processing if necessary.
    /// This is useful if the component has to perform work that must,
    /// be decoupled from the rest of the communication between the components.
    ///
    /// # Arguments
    ///
    /// * `tx` - An broadcast sender that can send actions.
    ///
    /// # Returns
    ///
    /// * `Result<()>` - An Ok result or an error.
    #[allow(unused_variables)]
    fn register_component_action_sender(
        &mut self,
        tx: tokio::sync::mpsc::UnboundedSender<Action>,
    ) -> Result<()> {
        Ok(())
    }

    #[allow(unused_variables)]
    fn register_explorer_action_sender(
        &mut self,
        tx: tokio::sync::mpsc::Sender<Action>,
    ) -> Result<()> {
        Ok(())
    }

    // Register a configuration handler that provides configuration settings if necessary.
    //
    // # Arguments
    //
    // * `config` - Configuration settings.
    //
    // # Returns
    //
    // * `Result<()>` - An Ok result or an error.
    #[allow(unused_variables)]
    fn register_config_handler(&mut self, config: AppConfig) -> Result<()> {
        Ok(())
    }

    /// Initialize the component with a specified size of the terminal backend, if necessary.
    ///
    /// # Arguments
    ///
    /// * `area` - Rectangular area to initialize the component within.
    ///
    /// # Returns
    ///
    /// * `Result<()>` - An Ok result or an error.
    #[allow(unused_variables)]
    fn init_terminal_size(&mut self, size: Size) -> Result<()> {
        Ok(())
    }

    /// Initialize the component with a specified area, if necessary.
    #[allow(unused_variables)]
    fn init_area(&mut self, area: Rect) -> Result<()> {
        Ok(())
    }

    /// Handle incoming events and produce actions if necessary.
    ///
    /// # Arguments
    ///
    /// * `event` - An optional event to be processed.
    ///
    /// # Returns
    ///
    /// * `Result<Option<Action>>` - An action to be processed or none.
    async fn handle_events(&mut self, event: Option<Event>) -> Result<Option<Action>> {
        let r = match event {
            Some(Event::Key(key_event)) => self.handle_key_events(key_event).await?,
            Some(Event::Mouse(mouse_event)) => self.handle_mouse_events(mouse_event)?,
            _ => None,
        };
        Ok(r)
    }

    /// Handle key events and produce actions if necessary.
    ///
    /// # Arguments
    ///
    /// * `key` - A key event to be processed.
    ///
    /// # Returns
    ///
    /// * `Result<Option<Action>>` - An action to be processed or none.
    #[allow(unused_variables)]
    async fn handle_key_events(&mut self, key: KeyEvent) -> Result<Option<Action>> {
        Ok(None)
    }

    /// Handle mouse events and produce actions if necessary.
    ///
    /// # Arguments
    ///
    /// * `mouse` - A mouse event to be processed.
    ///
    /// # Returns
    ///
    /// * `Result<Option<Action>>` - An action to be processed or none.
    #[allow(unused_variables)]
    fn handle_mouse_events(&mut self, mouse: MouseEvent) -> Result<Option<Action>> {
        Ok(None)
    }

    /// Update the state of the component based on a received action.
    ///
    /// # Arguments
    ///
    /// * `action` - An action that may modify the state of the component.
    ///
    /// # Returns
    ///
    /// * `Result<Option<Action>>` - An action to be processed or none.
    #[allow(unused_variables)]
    async fn update(&mut self, action: Action) -> Result<Option<Action>> {
        Ok(None)
    }

    /// Render the component on the screen. (REQUIRED)
    ///
    /// # Arguments
    ///
    /// * `f` - A frame used for rendering.
    /// * `area` - The area in which the component should be drawn.
    ///
    /// # Returns
    ///
    /// * `Result<()>` - An Ok result or an error.
    fn render(&mut self, f: &mut ratatui::Frame<'_>, area: Rect) -> Result<()>;

    /// Controls when a component should handle incoming events (REQUIRED)
    fn should_handle_events(&self) -> bool;

    /// Controls when a component should render
    fn should_render(&self) -> bool;
}
