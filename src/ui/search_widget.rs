use anyhow::Result;
use async_trait::async_trait;
use crossterm::event::KeyModifiers;
use ratatui::{prelude::*, widgets::*};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::{
    app::{actions::Action, config::AppConfig, key_bindings, AppContext, AppState},
    component::Component,
    tui::Event,
    ui::{get_main_layout, Theme},
    utils,
};

#[derive(Default, Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SearchMode {
    #[default]
    Flat,
    Deep,
}

impl std::fmt::Display for SearchMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SearchMode::Flat => write!(f, "Flat"),
            SearchMode::Deep => write!(f, "Deep"),
        }
    }
}

impl SearchMode {
    fn depth(&self) -> usize {
        match self {
            SearchMode::Flat => 1,
            SearchMode::Deep => usize::MAX,
        }
    }
}

#[derive(Debug)]
pub struct SearchWidget {
    /// The actually context of this widget
    app_context: AppContext,
    /// The context of the previous active widget
    previous_context: AppContext,
    /// Action sender that can send actions to all other components
    action_sender: Option<tokio::sync::mpsc::UnboundedSender<Action>>,
    /// Associated Explorer operation sender, that can send actions to the [`Explorer`]
    explorer_action_sender: Option<tokio::sync::mpsc::Sender<Action>>,
    /// Flag to control the available draw area for the [`SearchWidget`]
    /// If the [`crate::ui::info_widget::SystemOverview`] is not visible, than use the whole draw area
    use_whole_draw_area: bool,
    /// Position of cursor in the editor area.
    character_index: usize,
    /// Current working directory, in which to search for
    cwd: PathBuf,
    /// Current value of the search_query input box
    search_query: String,
    // The shorted CWD -> used as Block title
    cwd_display_name: String,
    /// Flag to control the receiving of the key events for the search widget
    /// If the widget is working, then incoming key events are ignored
    is_working: bool,
    theme: Theme,
    mode: SearchMode,
    /// To control how many characters the input field can hold
    input_field_width: u16,
    /// History of the input
    history: Vec<String>,
    history_index: Option<usize>,
    follow_sym_links: bool,
}

impl Default for SearchWidget {
    fn default() -> Self {
        Self {
            app_context: AppContext::NotActive,
            previous_context: AppContext::Explorer,
            action_sender: Default::default(),
            explorer_action_sender: Default::default(),
            use_whole_draw_area: Default::default(),
            character_index: Default::default(),
            cwd: Default::default(),
            search_query: Default::default(),
            cwd_display_name: Default::default(),
            is_working: Default::default(),
            theme: Default::default(),
            mode: Default::default(),
            input_field_width: Default::default(),
            history: Default::default(),
            history_index: Default::default(),
            follow_sym_links: Default::default(),
        }
    }
}

impl SearchWidget {
    fn move_cursor_left(&mut self) {
        let cursor_moved_left = self.character_index.saturating_sub(1);
        self.character_index = self.clamp_cursor(cursor_moved_left);
    }

    fn move_cursor_right(&mut self) {
        let cursor_moved_right = self.character_index.saturating_add(1);
        self.character_index = self.clamp_cursor(cursor_moved_right);
    }

    fn enter_char(&mut self, new_char: char) {
        // add a new char to the input only if it is no whitespace and fits into the input field
        if self.character_index <= (self.input_field_width - 3) as usize
            && !new_char.is_whitespace()
        {
            let index = self.byte_index();
            self.search_query.insert(index, new_char);
            self.move_cursor_right();
        }
    }

    /// Returns the byte index based on the character position.
    ///
    /// Since each character in a string can be contain multiple bytes, it's necessary to calculate
    /// the byte index based on the index of the character.
    fn byte_index(&self) -> usize {
        self.search_query
            .char_indices()
            .map(|(i, _)| i)
            .nth(self.character_index)
            .unwrap_or(self.search_query.len())
    }

    fn delete_char(&mut self, key_code: crossterm::event::KeyCode) {
        match key_code {
            crossterm::event::KeyCode::Delete => {
                // DELETE key functionality: Remove the nearest right character
                let is_not_cursor_rightmost = self.character_index < self.search_query.len();
                if is_not_cursor_rightmost {
                    let current_index = self.character_index;

                    // Getting all characters before the character to delete.
                    let before_char_to_delete = self.search_query.chars().take(current_index);
                    // Getting all characters after the character to delete.
                    let after_char_to_delete = self.search_query.chars().skip(current_index + 1);

                    // Reconstruct the string without the deleted character.
                    self.search_query = before_char_to_delete.chain(after_char_to_delete).collect();
                }
            }
            crossterm::event::KeyCode::Backspace => {
                // BACKSPACE key functionality: Remove the nearest left character
                let is_not_cursor_leftmost = self.character_index != 0;
                if is_not_cursor_leftmost {
                    let current_index = self.character_index;
                    let from_left_to_current_index = current_index - 1;

                    let before_char_to_delete =
                        self.search_query.chars().take(from_left_to_current_index);
                    let after_char_to_delete = self.search_query.chars().skip(current_index);

                    self.search_query = before_char_to_delete.chain(after_char_to_delete).collect();
                    self.move_cursor_left();
                }
            }
            _ => {}
        }
    }

    fn clamp_cursor(&self, new_cursor_pos: usize) -> usize {
        new_cursor_pos.clamp(0, self.search_query.chars().count())
    }

    fn reset_cursor(&mut self) {
        self.character_index = 0;
    }

    fn reset(&mut self) {
        self.reset_cursor();
        self.history_index = None;
        self.search_query.clear();
    }

    async fn submit_search(&mut self) -> Result<()> {
        // only if the search query does not yet exist, add it to the history
        if !self.history.contains(&self.search_query) {
            self.history.push(self.search_query.clone());
        }
        self.history_index = None;
        self.send_explorer_action(Action::StartSearch(
            self.cwd.clone(),
            self.search_query.clone(),
            self.mode.depth(),
            self.follow_sym_links,
        ))
        .await?;
        Ok(())
    }

    /// Helper function to send a [`Action`] to the [`crate::file_handling::Explorer`]
    /// Set the `is_working` flag to true
    async fn send_explorer_action(&mut self, action: Action) -> Result<()> {
        if let Some(sender) = &self.explorer_action_sender {
            self.is_working = true;
            sender.send(action).await?;
        }
        Ok(())
    }

    /// Helper function to send a [`Action`] to all components
    fn send_app_action(&self, action: Action) -> Result<()> {
        if let Some(handler) = &self.action_sender {
            handler.send(action)?
        }
        Ok(())
    }

    fn switch_search_mode(&mut self) {
        match self.mode {
            SearchMode::Flat => self.mode = SearchMode::Deep,
            SearchMode::Deep => self.mode = SearchMode::Flat,
        }
    }
}

#[async_trait(?Send)]
impl Component for SearchWidget {
    fn register_component_action_sender(
        &mut self,
        tx: tokio::sync::mpsc::UnboundedSender<Action>,
    ) -> Result<()> {
        self.action_sender = Some(tx);
        Ok(())
    }

    fn register_explorer_action_sender(
        &mut self,
        tx: tokio::sync::mpsc::Sender<Action>,
    ) -> Result<()> {
        self.explorer_action_sender = Some(tx);
        Ok(())
    }

    fn register_config_handler(&mut self, config: AppConfig) -> Result<()> {
        self.theme = config.theme();
        self.follow_sym_links = config.follow_sym_links();
        Ok(())
    }

    async fn handle_events(&mut self, event: Option<crate::tui::Event>) -> Result<Option<Action>> {
        if let Some(event) = event {
            match event {
                Event::Key(key_event) => {
                    if self.should_handle_events() {
                        let cmd_desc =
                            key_bindings::get_command_description(&key_event, &self.app_context)
                                .to_owned();
                        self.send_app_action(Action::SetCommandDescription(cmd_desc))?;
                        return self.handle_key_events(key_event).await;
                    }
                }
                _ => {
                    return Ok(None);
                }
            }
        }

        Ok(None)
    }

    async fn handle_key_events(
        &mut self,
        key: crossterm::event::KeyEvent,
    ) -> Result<Option<Action>> {
        match key.code {
            // Submit search
            crossterm::event::KeyCode::Enter => {
                if !self.search_query.trim().is_empty() {
                    self.submit_search().await?;
                } else {
                    return Ok(Action::UpdateAppState(AppState::Failure(
                        "Search query must not be empty".to_string(),
                    ))
                    .into());
                }
            }
            crossterm::event::KeyCode::Char('o')
                if key.modifiers == crossterm::event::KeyModifiers::CONTROL =>
            {
                return Ok(Action::HideOrShowSystemOverview.into());
            }
            crossterm::event::KeyCode::Char('t')
                if key.modifiers == crossterm::event::KeyModifiers::CONTROL =>
            {
                self.theme = self.theme.toggle_theme();
                return Ok(Action::ToggleTheme(self.theme).into());
            }
            crossterm::event::KeyCode::Char(to_insert) => {
                match key.modifiers {
                    // Handle `Ctrl + v` for clipboard paste
                    KeyModifiers::CONTROL if to_insert.eq_ignore_ascii_case(&'v') => {
                        match utils::paste_from_clipboard() {
                            Ok(content) => {
                                if content.trim().is_empty() {
                                    return Ok(Some(Action::UpdateAppState(AppState::Failure(
                                        "Nothing to paste from clipboard".to_string(),
                                    ))));
                                }
                                content.chars().for_each(|c| self.enter_char(c));
                                return Ok(Some(Action::UpdateAppState(AppState::Done(
                                    "Done".to_string(),
                                ))));
                            }
                            Err(err) => {
                                log::error!("{:?}", err);
                                return Ok(Some(Action::UpdateAppState(AppState::Failure(
                                    "Failed to paste content from clipboard".to_string(),
                                ))));
                            }
                        }
                    }

                    // Allow all printable characters with these modifiers: NONE, SHIFT, ALT, CTRL + ALT
                    modifiers
                        if modifiers.contains(KeyModifiers::SHIFT)
                            || modifiers.contains(KeyModifiers::ALT)
                            || modifiers.contains(KeyModifiers::CONTROL | KeyModifiers::ALT)
                            || modifiers.is_empty() =>
                    {
                        if !to_insert.is_whitespace() {
                            self.enter_char(to_insert);
                        }
                    }

                    // Ignore other modifiers
                    _ => {}
                }
            }
            //Moves backward through input history, if any
            crossterm::event::KeyCode::Up => {
                if !self.history.is_empty() {
                    self.search_query.clear();
                    self.reset_cursor();
                    match self.history_index {
                        Some(index) => {
                            if index > 0 {
                                self.history_index = Some(index - 1);
                            } else {
                                // Cycle to the most recent entry
                                self.history_index = Some(self.history.len() - 1);
                            }
                        }
                        None => {
                            // Start cycling from the latest entry
                            self.history_index = Some(self.history.len() - 1);
                        }
                    }
                    let previous_input = self.history[self.history_index.unwrap()].clone();
                    previous_input.chars().for_each(|c| self.enter_char(c));
                }
            }
            // Moves forward through input history, if any
            crossterm::event::KeyCode::Down => {
                if !self.history.is_empty() {
                    self.search_query.clear();
                    self.reset_cursor();
                    match self.history_index {
                        Some(index) => {
                            if index < self.history.len() - 1 {
                                self.history_index = Some(index + 1);
                            } else {
                                // Cycle back to the oldest entry
                                self.history_index = Some(0);
                            }
                        }
                        None => {
                            // Start cycling from the oldest entry
                            self.history_index = Some(0);
                        }
                    }
                    let previous_input = self.history[self.history_index.unwrap()].clone();
                    previous_input.chars().for_each(|c| self.enter_char(c));
                }
            }
            crossterm::event::KeyCode::Delete => self.delete_char(key.code),
            crossterm::event::KeyCode::Backspace => self.delete_char(key.code),
            crossterm::event::KeyCode::Left => self.move_cursor_left(),
            crossterm::event::KeyCode::Right => self.move_cursor_right(),
            crossterm::event::KeyCode::Tab
                if key.modifiers == crossterm::event::KeyModifiers::NONE =>
            {
                self.switch_search_mode()
            }
            crossterm::event::KeyCode::F(1)
                if key.modifiers == crossterm::event::KeyModifiers::NONE =>
            {
                self.app_context = AppContext::NotActive;
                return Ok(Action::ShowHelp(AppContext::Search).into());
            }
            crossterm::event::KeyCode::F(2)
                if key.modifiers == crossterm::event::KeyModifiers::NONE =>
            {
                self.app_context = AppContext::NotActive;
                return Ok(Action::ShowAbout(AppContext::Search).into());
            }
            crossterm::event::KeyCode::Esc => {
                self.reset();
                self.app_context = AppContext::NotActive;
                return Ok(Action::SwitchAppContext(self.previous_context).into());
            }
            _ => {}
        }

        Ok(None)
    }

    fn should_handle_events(&self) -> bool {
        self.app_context == AppContext::Search && !self.is_working
    }

    fn should_render(&self) -> bool {
        self.app_context == AppContext::Search
    }

    async fn update(&mut self, action: &Action) -> Result<Option<Action>> {
        match action {
            Action::SwitchAppContext(context) => {
                self.app_context = *context;
            }
            Action::ShowSearchPage(cwd) => {
                self.cwd = cwd.to_path_buf();
                self.cwd_display_name = utils::format_path_for_display(&self.cwd);
            }
            Action::SearchDone(search_result) => {
                self.is_working = false;
                if let Some(result) = search_result {
                    self.reset();
                    self.send_app_action(Action::ShowResultsPage(result.clone(), self.mode))?;

                    return Ok(Action::SwitchAppContext(AppContext::Results).into());
                } else {
                    return Ok(Action::UpdateAppState(AppState::Failure(
                        "No matches found".to_string(),
                    ))
                    .into());
                }
            }
            Action::ToggleTheme(theme) => {
                self.theme = *theme;
            }
            Action::HideOrShowSystemOverview => {
                self.use_whole_draw_area = !self.use_whole_draw_area;
            }
            _ => {}
        }
        Ok(None)
    }

    fn render(&mut self, f: &mut ratatui::Frame<'_>, area: Rect) -> Result<()> {
        if self.should_render() {
            // Control the draw area dependent if the InfoWidget is showing or not
            let draw_area = if self.use_whole_draw_area {
                let overview_area = get_main_layout(area).overview_area;
                overview_area.union(get_main_layout(area).main_area)
            } else {
                get_main_layout(area).main_area
            };

            // the main draw area, include the spacer and the first block (CWD)
            let [top_spacer_area, draw_area] =
                Layout::vertical([Constraint::Length(1), Constraint::Fill(1)]).areas(draw_area);

            let theme_colors = self.theme.theme_colors();

            let main_block_title = format!(" Cwd: [{}] ", self.cwd_display_name);
            let inner_block_title = match self.mode {
                SearchMode::Flat => " Search for file/directory names in the current directory ",
                SearchMode::Deep => " Search for file/directory names in the current directory and all subdirectories ",
            };
            let input_block_title = format!(" Type search query [Mode: {}] ", self.mode);

            let help_msg = vec![
                " <Esc>".fg(theme_colors.main_text_fg),
                " back to Explorer ".fg(theme_colors.main_fg),
                "|".fg(theme_colors.main_fg),
                " <Enter>".fg(theme_colors.main_text_fg),
                " submit search ".fg(theme_colors.main_fg),
                "|".fg(theme_colors.main_fg),
                " <Tab>".fg(theme_colors.main_text_fg),
                " switch search mode ".fg(theme_colors.main_fg),
            ];

            // CWD block
            let first_block = Block::default()
                .title_top(
                    Line::from(main_block_title)
                        .style(Style::new().fg(theme_colors.alt_fg))
                        .left_aligned(),
                )
                .title_alignment(Alignment::Center)
                .borders(Borders::TOP | Borders::BOTTOM)
                .border_type(BorderType::QuadrantInside)
                .border_style(Style::new().fg(theme_colors.alt_bg))
                .style(Style::new().bg(theme_colors.alt_bg));

            // Help msg block
            let second_block = Block::default()
                .title_top(Line::from(inner_block_title))
                .title_bottom(Line::from(help_msg))
                .title_alignment(Alignment::Center)
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                // .border_style(Style::new().fg(theme_colors.main_fg))
                .border_style(Style::new().fg(match self.mode {
                    SearchMode::Flat => theme_colors.main_fg,
                    SearchMode::Deep => theme_colors.alt_fg,
                }))
                .style(Style::new().bg(theme_colors.alt_bg));

            let [second_block_area] = Layout::vertical([Constraint::Fill(1)])
                .margin(1)
                .areas(first_block.inner(draw_area));

            f.render_widget(Line::from(" ").bg(theme_colors.alt_bg), top_spacer_area);
            f.render_widget(first_block, draw_area);
            f.render_widget(second_block, second_block_area);

            let [third_block_area] = Layout::vertical([Constraint::Length(4)])
                .vertical_margin(2)
                .horizontal_margin(10)
                .areas(second_block_area);

            let input_block = Block::default()
                .title_top(Line::from(input_block_title))
                .title_alignment(Alignment::Left)
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::new().fg(match self.mode {
                    SearchMode::Flat => theme_colors.main_fg,
                    SearchMode::Deep => theme_colors.alt_fg,
                }))
                .style(Style::new().bg(theme_colors.alt_bg));

            let [spacer_line_area, input_block_area] =
                Layout::vertical([Constraint::Length(1), Constraint::Length(1)])
                    .areas(input_block.inner(third_block_area));

            self.input_field_width = input_block_area.width;

            f.render_widget(input_block, third_block_area);

            let input = Paragraph::new(self.search_query.as_str())
                .style(
                    Style::new()
                        .bg(theme_colors.alt_bg)
                        .fg(theme_colors.main_text_fg),
                )
                .block(Block::default().padding(Padding {
                    left: 1,
                    right: 0,
                    top: 0,
                    bottom: 0,
                }));

            f.render_widget(Line::from(" ").bg(theme_colors.alt_bg), spacer_line_area);
            f.render_widget(input, input_block_area);

            // IMPORTANT:
            // Only display the cursor if no search is running to prevent the cursor from flickering
            if !self.is_working {
                f.set_cursor_position(Position::new(
                    // Draw the cursor at the current position in the input field.
                    // This position can be controlled using the left and right arrow keys
                    input_block_area.x + self.character_index as u16 + 1,
                    // Move one line down, from the border to the input line if needed
                    input_block_area.y,
                ))
            }
        }

        Ok(())
    }
}
