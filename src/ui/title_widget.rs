use anyhow::Result;
use async_trait::async_trait;
use ratatui::{prelude::*, widgets::*};
use std::time::Instant;

use crate::{app::actions::Action, component::Component, tui::Event, ui::get_main_layout};

#[derive(Debug, Clone, PartialEq)]
pub struct TitleBar {
    app_name: String,
    help_hint: String,
    app_start_time: Instant,
    app_frames: u32,
    app_fps: f64,
    bg_color: Color,
    render_start_time: Instant,
    render_frames: u32,
    render_fps: f64,
    is_system_overview_showing: bool,
}

impl Default for TitleBar {
    fn default() -> Self {
        Self::new()
    }
}

impl TitleBar {
    fn new() -> Self {
        let app_name = {
            let name = env!("CARGO_PKG_NAME").trim().to_string();
            if !name.is_empty() {
                name
            } else {
                "Unknown".to_string()
            }
        };

        #[cfg(target_os = "windows")]
        let bg_color = Color::Cyan;

        #[cfg(target_os = "linux")]
        let bg_color = Color::LightBlue;

        #[cfg(target_os = "macos")]
        let bg_color = Color::Cyan;

        Self {
            app_name,
            help_hint: String::from("Press <F1> for help"),
            app_start_time: Instant::now(),
            app_frames: 0,
            app_fps: 0.0,
            bg_color,
            render_start_time: Instant::now(),
            render_frames: 0,
            render_fps: 0.0,
            is_system_overview_showing: true,
        }
    }

    fn app_tick(&mut self) -> Result<()> {
        self.app_frames += 1;
        let now = Instant::now();
        let elapsed = (now - self.app_start_time).as_secs_f64();
        if elapsed >= 1.0 {
            self.app_fps = self.app_frames as f64 / elapsed;
            self.app_start_time = now;
            self.app_frames = 0;
        }
        Ok(())
    }

    fn render_tick(&mut self) -> Result<()> {
        self.render_frames += 1;
        let now = Instant::now();
        let elapsed = (now - self.render_start_time).as_secs_f64();
        if elapsed >= 1.0 {
            self.render_fps = self.render_frames as f64 / elapsed;
            self.render_start_time = now;
            self.render_frames = 0;
        }
        Ok(())
    }
}

#[async_trait(?Send)]
impl Component for TitleBar {
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

    async fn handle_key_events(&mut self, _: crossterm::event::KeyEvent) -> Result<Option<Action>> {
        Ok(None)
    }

    fn should_handle_events(&self) -> bool {
        false
    }

    fn should_render(&self) -> bool {
        true
    }

    async fn update(&mut self, action: Action) -> Result<Option<Action>> {
        match action {
            Action::Tick => {
                if self.is_system_overview_showing {
                    self.app_tick()?
                } else {
                    self.app_fps = 0.0;
                }
            }
            Action::Render => self.render_tick()?,
            Action::HideOrShowSystemOverview => {
                self.is_system_overview_showing = !self.is_system_overview_showing;
            }
            _ => {}
        }

        Ok(None)
    }

    fn render(&mut self, f: &mut Frame<'_>, area: Rect) -> Result<()> {
        if self.should_render() {
            let title_area = get_main_layout(area).title_area;

            let [spacer_area, app_name_area, help_hint_area, meta_data_area] =
                Layout::horizontal([
                    Constraint::Length(1),
                    Constraint::Length(20),
                    Constraint::Length(20),
                    Constraint::Fill(1),
                ])
                .areas(title_area);

            // the app and render tick rate formatted with two decimal places
            let rate_meta_data = format!(
                "{:.2} refresh per sec (system-overview) {:.2} fps (render)",
                self.app_fps, self.render_fps
            );

            let app_name = Paragraph::new(Span::styled(
                self.app_name.clone(),
                Style::default()
                    .fg(Color::Black)
                    .add_modifier(Modifier::BOLD),
            ))
            .style(Style::default().bg(self.bg_color))
            .alignment(Alignment::Left);
            f.render_widget(app_name, app_name_area);

            let help_hint = Paragraph::new(Span::styled(
                self.help_hint.clone(),
                Style::default()
                    .fg(Color::LightYellow)
                    .add_modifier(Modifier::BOLD),
            ))
            .style(Style::default().bg(self.bg_color));

            f.render_widget(Span::from(" ").bg(self.bg_color), spacer_area);
            f.render_widget(help_hint, help_hint_area);

            let meta_data = Paragraph::new(Span::styled(
                rate_meta_data,
                Style::default().fg(Color::Black),
            ))
            .style(Style::default().bg(self.bg_color))
            .alignment(Alignment::Right);
            f.render_widget(meta_data, meta_data_area);
        }

        Ok(())
    }
}
