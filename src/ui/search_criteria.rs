#![allow(dead_code)]
use anyhow::Result;
use async_trait::async_trait;
use ratatui::{prelude::*, widgets::*};
use serde::{Deserialize, Serialize};
use tui_checkbox::{Checkbox, symbols};

use crate::{
    app::{actions::Action, config::AppConfig},
    component::Component,
    tui::Event,
    ui::centered_rect,
};

// Describes the configurable search criteria
// only_files -> means the search for the pattern applies only to files
// only_folders -> same as 'only_files' but for directories
// case_sensitive -> means if 'true' the pattern is case-sensitive
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Criteria {
    only_files: bool,
    only_folders: bool,
    case_sensitive: bool,
}

impl Criteria {
    pub fn only_files(&self) -> bool {
        self.only_files
    }

    pub fn only_folders(&self) -> bool {
        self.only_folders
    }

    pub fn case_sensitive(&self) -> bool {
        self.case_sensitive
    }
}

#[derive(Debug)]
pub struct SearchCriteriaPage {
    /// Action sender that can send actions to all other components
    action_sender: Option<tokio::sync::mpsc::UnboundedSender<Action>>,
    border_style: Style,
    border_type: BorderType,
    config: AppConfig,
    title_style: Style,
    is_active: bool,
    criteria: Criteria,
    // Index of the current selected checkbox
    selected: u8,
}

const CHECKBOX_COUNT: u8 = 3;

impl SearchCriteriaPage {
    /// Helper function to send a [`Action`] to all components
    fn send_app_action(&self, action: Action) -> Result<()> {
        if let Some(handler) = &self.action_sender {
            handler.send(action)?
        }
        Ok(())
    }

    fn block_title() -> ratatui::prelude::Line<'static> {
        Line::from(vec![
            Span::raw(" Search-Criteria | "),
            Span::styled("<Esc> ", Style::default().fg(Color::Yellow)),
            Span::raw("Close "),
            Span::styled(" <↑↓> ", Style::default().fg(Color::Yellow)),
            Span::raw("Choose "),
            Span::styled(" <Space> ", Style::default().fg(Color::Yellow)),
            Span::raw("Toggle "),
        ])
    }

    /// Moves the selection one up (wrap around to the end)
    fn select_previous(&mut self) {
        if self.selected == 0 {
            self.selected = CHECKBOX_COUNT - 1;
        } else {
            self.selected -= 1;
        }
    }

    /// Moves the selection one down (wrap around to the start)
    fn select_next(&mut self) {
        self.selected = (self.selected + 1) % CHECKBOX_COUNT;
    }

    /// Toggle the value of the current selected checkbox
    fn toggle_selected(&mut self) {
        match self.selected {
            0 => {
                self.criteria.only_files = !self.criteria.only_files;
                if self.criteria.only_files {
                    self.criteria.only_folders = false;
                }
            }
            1 => {
                self.criteria.only_folders = !self.criteria.only_folders;
                if self.criteria.only_folders {
                    self.criteria.only_files = false;
                }
            }
            2 => self.criteria.case_sensitive = !self.criteria.case_sensitive,
            _ => {}
        }
    }
}

impl Default for SearchCriteriaPage {
    fn default() -> Self {
        Self {
            action_sender: Default::default(),
            border_style: Style::new().bold().fg(Color::LightGreen),
            border_type: BorderType::Rounded,
            config: Default::default(),
            title_style: Default::default(),
            is_active: Default::default(),
            criteria: Default::default(),
            selected: Default::default(),
        }
    }
}

#[async_trait(?Send)]
impl Component for SearchCriteriaPage {
    fn register_component_action_sender(
        &mut self,
        tx: tokio::sync::mpsc::UnboundedSender<Action>,
    ) -> Result<()> {
        self.action_sender = Some(tx);
        Ok(())
    }

    fn register_config_handler(&mut self, config: AppConfig) -> Result<()> {
        self.criteria = *config.search_criteria();
        self.config = config;
        Ok(())
    }

    async fn handle_events(&mut self, event: Option<crate::tui::Event>) -> Result<Option<Action>> {
        if let Some(event) = event {
            match event {
                Event::Key(key_event) => {
                    if self.should_handle_events() {
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
            crossterm::event::KeyCode::Up => {
                self.select_previous();
                Ok(None)
            }
            crossterm::event::KeyCode::Down => {
                self.select_next();
                Ok(None)
            }
            crossterm::event::KeyCode::Char(' ') => {
                self.toggle_selected();
                Ok(None)
            }
            crossterm::event::KeyCode::Esc => {
                self.is_active = false;
                // store changed search criteria in app settings, if necessary
                if self.criteria != *self.config.search_criteria() {
                    self.config.set_search_criteria(self.criteria);
                    self.send_app_action(Action::ApplyAppSettings(self.config.clone()))?
                }
                Ok(Action::SearchCriteriaClosed(self.criteria).into())
            }
            _ => Ok(None),
        }
    }

    fn should_handle_events(&self) -> bool {
        self.is_active
    }

    fn should_render(&self) -> bool {
        self.is_active
    }

    async fn update(&mut self, action: &Action) -> Result<Option<Action>> {
        match action {
            Action::ShowSearchCriteria => {
                self.is_active = true;
                // reset selection
                self.selected = 0;
            }
            // we have to handle the case, if in settings page, the settings are changed -> these changes must also stored here, because if
            // we change the app settings in the settings page and than we changed the search criteria here, the older changes will be lost
            Action::ApplyAppSettings(c) => {
                self.config = c.clone();
            }
            _ => {}
        }
        Ok(None)
    }

    fn render(&mut self, f: &mut ratatui::Frame<'_>, area: Rect) -> Result<()> {
        if self.should_render() {
            let draw_area = centered_rect(50, 30, area);

            let block = Block::new()
                .title(SearchCriteriaPage::block_title())
                .title_alignment(Alignment::Left)
                .title_style(self.title_style)
                .border_type(self.border_type)
                .borders(Borders::ALL)
                .border_style(self.border_style)
                .padding(Padding {
                    left: 2,
                    right: 2,
                    top: 1,
                    bottom: 1,
                });

            let inner = block.inner(draw_area);

            // vertical layout: 3 rows of equal height for the checkboxes,
            // centered mit Flex::Center, each row has exactly a height of 1
            let rows = Layout::vertical([
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
            ])
            .flex(layout::Flex::Center) // uniform vertical spacing
            .split(inner);

            // horizontal centering: Each checkbox is assigned a fixed width and is centered in its respective row
            let centered_row = |row: Rect, width: u16| -> Rect {
                Layout::horizontal([Constraint::Length(width)])
                    .flex(layout::Flex::Center)
                    .split(row)[0]
            };

            // Style-Helper: active row will be highlighted
            let row_style = |idx: u8| -> Style {
                if self.selected == idx {
                    Style::default().fg(Color::Yellow).bold()
                } else {
                    Style::default()
                }
            };

            let chkbox_only_files = Checkbox::new("Only Files", self.criteria.only_files)
                .checked_symbol(symbols::CHECKED_X)
                .unchecked_symbol(symbols::UNCHECKED_SPACE)
                .style(row_style(0));

            let chkbox_only_folders = Checkbox::new("Only Folders", self.criteria.only_folders)
                .checked_symbol(symbols::CHECKED_X)
                .unchecked_symbol(symbols::UNCHECKED_SPACE)
                .style(row_style(1));

            let chkbox_case_sensitive =
                Checkbox::new("Case-Sensitive", self.criteria.case_sensitive)
                    .checked_symbol(symbols::CHECKED_X)
                    .unchecked_symbol(symbols::UNCHECKED_SPACE)
                    .style(row_style(2));

            f.render_widget(Clear, draw_area);
            f.render_widget(block, draw_area);

            f.render_widget(
                chkbox_only_files,
                centered_row(rows[0], "            Only Files".len() as u16),
            );
            f.render_widget(
                chkbox_only_folders,
                centered_row(rows[1], "          Only Folders".len() as u16),
            );
            f.render_widget(
                chkbox_case_sensitive,
                centered_row(rows[2], "       Case-Sensitive".len() as u16),
            );
        }

        Ok(())
    }
}
