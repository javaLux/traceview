use anyhow::Result;
use async_trait::async_trait;
use ratatui::prelude::*;

use crate::{
    app::{actions::Action, config::AppConfig, AppContext, AppState},
    component::Component,
    ui::{self, Theme},
    utils,
};

const APP_CONTEXT_TITLE: &str = "Context: ";
const APP_CONTEXT_LENGTH: u16 = 17;
const KEYSTROKE_TITLE: &str = "Keystroke: ";
const THEME_HINT_TITLE: &str = "Theme: ";
const THEME_HINT_LENGTH: u16 = 14;
const SPACER_LENGTH: u16 = 2;

#[derive(Debug)]
pub struct Footer {
    /// Track the current active app context => default is `Explorer`
    app_context: AppContext,
    // Track the command description
    command_description: String,
    command_desc_length: u16,
    // To track the app state of the applied operation
    app_state: AppState,
    app_state_hint_length: u16,
    // Track the user input
    key_event: Option<String>,
    key_event_length: u16,
    theme: Theme,
}

impl Default for Footer {
    fn default() -> Self {
        Self {
            app_context: AppContext::default(),
            command_description: Default::default(),
            command_desc_length: Default::default(),
            theme: Default::default(),
            key_event: Default::default(),
            app_state: AppState::done_empty(),
            app_state_hint_length: utils::compute_text_length(&AppState::done_empty().to_string())
                + 2,
            key_event_length: utils::compute_text_length(KEYSTROKE_TITLE) + 7,
        }
    }
}

#[async_trait(?Send)]
impl Component for Footer {
    fn register_config_handler(&mut self, config: AppConfig) -> Result<()> {
        self.theme = config.theme();
        Ok(())
    }

    async fn handle_events(&mut self, event: Option<crate::tui::Event>) -> Result<Option<Action>> {
        if let Some(event) = event {
            match event {
                crate::tui::Event::Key(key_event) => {
                    return self.handle_key_events(key_event).await;
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
        // log all key events independent of the current app context
        self.key_event = Some(utils::key_event_to_string(key));
        self.key_event_length = utils::compute_text_length(&format!(
            "{} {}",
            KEYSTROKE_TITLE,
            &utils::key_event_to_string(key)
        )) + 2;

        // clear the command description
        self.command_description = " ".into();
        self.command_desc_length = utils::compute_text_length(&self.command_description);
        // clear the app state
        self.app_state = AppState::done_empty();

        Ok(None)
    }

    fn should_handle_events(&self) -> bool {
        matches!(
            self.app_context,
            AppContext::Explorer | AppContext::Search | AppContext::Results
        )
    }

    fn should_render(&self) -> bool {
        true
    }

    async fn update(&mut self, action: &Action) -> Result<Option<Action>> {
        match action {
            Action::UpdateAppState(state) => {
                self.app_state = state.clone();
                self.app_state_hint_length =
                    utils::compute_text_length(&self.app_state.to_string()) + 2;
            }
            Action::SetCommandDescription(desc) => {
                self.command_description = match desc {
                    Some(desc) => {
                        if !desc.trim().is_empty() {
                            format!("→ {}", desc)
                        } else {
                            " ".into()
                        }
                    }
                    None => " ".into(),
                };
                self.command_desc_length = utils::compute_text_length(&self.command_description);
            }
            Action::SwitchAppContext(context) => {
                self.app_context = *context;
            }
            Action::ToggleTheme(theme) => {
                self.theme = *theme;
            }
            _ => {}
        }

        Ok(None)
    }

    fn render(&mut self, f: &mut ratatui::Frame<'_>, area: Rect) -> Result<()> {
        if self.should_render() {
            let draw_area = ui::get_main_layout(area).footer_area;

            let [first_spacer, context_hint_area, second_spacer, theme_hint_area, third_spacer, key_hint_area, fourth_spacer, command_desc_area, fifth_spacer, app_state_area] =
                Layout::horizontal([
                    Constraint::Length(1),
                    Constraint::Length(APP_CONTEXT_LENGTH),
                    Constraint::Length(SPACER_LENGTH),
                    Constraint::Length(THEME_HINT_LENGTH),
                    Constraint::Length(SPACER_LENGTH),
                    Constraint::Length(self.key_event_length),
                    Constraint::Length(1),
                    Constraint::Length(self.command_desc_length),
                    Constraint::Length(SPACER_LENGTH),
                    Constraint::Fill(1),
                ])
                .areas(draw_area);

            let context_hint = Line::from(vec![
                Span::styled(APP_CONTEXT_TITLE, self.theme.theme_colors().main_fg),
                Span::styled(
                    format!("{}", self.app_context),
                    self.theme.theme_colors().alt_fg,
                ),
            ])
            .style(Style::new().bg(self.theme.theme_colors().main_bg));

            let theme_hint = Line::from(vec![
                Span::styled(THEME_HINT_TITLE, self.theme.theme_colors().main_fg),
                Span::styled(format!("{}", self.theme), self.theme.theme_colors().alt_fg),
            ])
            .style(Style::new().bg(self.theme.theme_colors().main_bg));

            let key_hint_msg = self.key_event.clone().unwrap_or("None".to_string());

            let key_hint = Line::from(vec![
                Span::styled(KEYSTROKE_TITLE, self.theme.theme_colors().main_fg),
                Span::styled(
                    format!("|{}|", key_hint_msg),
                    self.theme.theme_colors().alt_fg,
                ),
            ])
            .style(Style::new().bg(self.theme.theme_colors().main_bg));

            f.render_widget(
                Line::from(Span::from("  ")).bg(self.theme.theme_colors().main_bg),
                first_spacer,
            );

            f.render_widget(context_hint, context_hint_area);
            f.render_widget(
                Line::from(Span::from("  ")).bg(self.theme.theme_colors().main_bg),
                second_spacer,
            );

            f.render_widget(theme_hint, theme_hint_area);
            f.render_widget(
                Line::from(Span::from("  ")).bg(self.theme.theme_colors().main_bg),
                third_spacer,
            );
            f.render_widget(key_hint, key_hint_area);
            f.render_widget(
                Line::from(Span::from("  ")).bg(self.theme.theme_colors().main_bg),
                fourth_spacer,
            );

            let command_desc_hint = Line::from(Span::styled(
                &self.command_description,
                self.theme.theme_colors().alt_fg,
            ))
            .bg(self.theme.theme_colors().main_bg);

            f.render_widget(command_desc_hint, command_desc_area);
            f.render_widget(
                Line::from(Span::from("  ")).bg(self.theme.theme_colors().main_bg),
                fifth_spacer,
            );
            f.render_widget(self.build_app_state_hint(), app_state_area);
        }

        Ok(())
    }
}

impl Footer {
    fn build_app_state_hint(&self) -> Line<'_> {
        match &self.app_state {
            AppState::Working(msg) => Line::from(Span::styled(
                msg.to_string(),
                Style::new().fg(self.theme.theme_colors().working_state_color),
            ))
            .bg(self.theme.theme_colors().main_bg),
            AppState::Done(msg) => {
                if msg.trim().is_empty() {
                    // if the message is empty, just return a space and display nothing
                    Line::from(" ").bg(self.theme.theme_colors().main_bg)
                } else {
                    Line::from(Span::styled(
                        msg.to_string(),
                        Style::new().fg(self.theme.theme_colors().done_state_color),
                    ))
                    .bg(self.theme.theme_colors().main_bg)
                }
            }
            AppState::Failure(err) => Line::from(Span::styled(
                err.to_string(),
                Style::new().fg(self.theme.theme_colors().failure_state_color),
            ))
            .bg(self.theme.theme_colors().main_bg),
        }
    }
}
