use anyhow::Result;
use async_trait::async_trait;
use crossterm::event::KeyModifiers;
use ratatui::{prelude::*, widgets::*};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::{
    app::{AppContext, AppState, actions::Action, config::AppConfig, key_bindings},
    component::Component,
    tui::Event,
    ui::{Theme, centered_rect_fixed_height, get_main_layout, input::SearchInput},
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
    /// Current working directory, in which to search for
    cwd: PathBuf,
    // The shortened CWD -> used as Block title
    cwd_display_name: String,
    /// Flag to control the receiving of the key events for the search widget
    /// If the widget is working, then incoming key events are ignored
    is_working: bool,
    theme: Theme,
    mode: SearchMode,
    follow_sym_links: bool,
    /// Handles all text input logic
    search_input: SearchInput,
}

impl Default for SearchWidget {
    fn default() -> Self {
        Self {
            app_context: AppContext::NotActive,
            previous_context: AppContext::Explorer,
            action_sender: Default::default(),
            explorer_action_sender: Default::default(),
            use_whole_draw_area: Default::default(),
            cwd: Default::default(),
            cwd_display_name: Default::default(),
            is_working: Default::default(),
            theme: Default::default(),
            mode: Default::default(),
            follow_sym_links: Default::default(),
            search_input: SearchInput::default(),
        }
    }
}

impl SearchWidget {
    async fn submit_search(&mut self) -> Result<()> {
        // Saves the current query into the history (if not already present)
        self.search_input.submit();

        self.send_explorer_action(Action::StartSearch(
            self.cwd.clone(),
            self.search_input.text_input.value().to_string(),
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
                if !self.search_input.text_input.is_empty() {
                    self.submit_search().await?;
                } else {
                    return Ok(Action::UpdateAppState(AppState::Failure(
                        "Search query must not be empty".to_string(),
                    ))
                    .into());
                }
            }
            crossterm::event::KeyCode::Char('o') if key.modifiers == KeyModifiers::CONTROL => {
                return Ok(Action::HideOrShowSystemOverview.into());
            }
            crossterm::event::KeyCode::Char('t') if key.modifiers == KeyModifiers::CONTROL => {
                self.theme = self.theme.toggle_theme();
                return Ok(Action::ToggleTheme(self.theme).into());
            }
            crossterm::event::KeyCode::Tab if key.modifiers == KeyModifiers::NONE => {
                self.switch_search_mode();
            }
            crossterm::event::KeyCode::F(1) if key.modifiers == KeyModifiers::NONE => {
                self.app_context = AppContext::NotActive;
                return Ok(Action::ShowHelp(AppContext::Search).into());
            }
            crossterm::event::KeyCode::F(2) if key.modifiers == KeyModifiers::NONE => {
                self.app_context = AppContext::NotActive;
                return Ok(Action::ShowAbout(AppContext::Search).into());
            }
            crossterm::event::KeyCode::F(3) if key.modifiers == KeyModifiers::NONE => {
                self.app_context = AppContext::NotActive;
                return Ok(Action::ShowSettings(AppContext::Search).into());
            }
            crossterm::event::KeyCode::Esc => {
                self.app_context = AppContext::NotActive;
                return Ok(Action::SwitchAppContext(self.previous_context).into());
            }
            // ----------------------------------------------------------------
            // Everything else is delegated to the InputWidget
            // ----------------------------------------------------------------
            _ => {
                // The InputWidget handles:
                // - Char input (with modifier awareness)
                // - Backspace / Delete
                // - Left / Right cursor movement
                // - Up / Down history navigation
                // - Ctrl+V clipboard paste
                if let Err(err) = self.search_input.handle_key_events(key).await {
                    return Ok(Action::UpdateAppState(AppState::Failure(err.to_string())).into());
                }
            }
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
            Action::ApplyAppSettings(c) => {
                self.follow_sym_links = c.follow_sym_links();
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

            let [top_spacer_area, draw_area] =
                Layout::vertical([Constraint::Length(1), Constraint::Fill(1)]).areas(draw_area);

            let theme_colors = self.theme.theme_colors();

            let main_block_title = format!(" Cwd: [{}] ", self.cwd_display_name);
            let inner_block_title = match self.mode {
                SearchMode::Flat => " Search for file/directory names in the current directory ",
                SearchMode::Deep => {
                    " Search for file/directory names in the current directory and all subdirectories "
                }
            };
            let input_block_title = format!(" Type search query [Mode: {}] ", self.mode);

            let help_msg = vec![
                " <Esc>".fg(theme_colors.main_text_fg),
                " Back to Explorer ".fg(theme_colors.main_fg),
                "|".fg(theme_colors.main_fg),
                " <Enter>".fg(theme_colors.main_text_fg),
                " Submit search ".fg(theme_colors.main_fg),
                "|".fg(theme_colors.main_fg),
                " <Tab>".fg(theme_colors.main_text_fg),
                " Switch search mode ".fg(theme_colors.main_fg),
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

            let border_fg = match self.mode {
                SearchMode::Flat => theme_colors.main_fg,
                SearchMode::Deep => theme_colors.alt_fg,
            };

            let input_block = Block::default()
                .title_top(Line::from(input_block_title))
                .title_alignment(Alignment::Left)
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::new().fg(border_fg))
                .style(Style::new().bg(theme_colors.alt_bg));

            let [spacer_line_area, input_block_area] =
                Layout::vertical([Constraint::Length(1), Constraint::Length(1)])
                    .areas(input_block.inner(third_block_area));

            f.render_widget(input_block, third_block_area);
            f.render_widget(Line::from(" ").bg(theme_colors.alt_bg), spacer_line_area);

            let input_centered_area = centered_rect_fixed_height(100, 4, input_block_area);
            // Render input
            self.search_input.render(
                f,
                input_centered_area,
                theme_colors.alt_bg,
                theme_colors.main_text_fg,
                !self.is_working, // show_cursor only when no search is running
            );
        }

        Ok(())
    }
}
