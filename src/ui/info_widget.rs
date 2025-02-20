#![allow(dead_code)]
use anyhow::Result;
use async_trait::async_trait;
use ratatui::{prelude::*, style::palette::tailwind, widgets::*};

use crate::{
    app::{actions::Action, config::AppConfig},
    component::Component,
    system::SystemDetails,
    tui::Event,
    ui::{get_main_layout, Theme},
    utils,
};

// Gauge usage colors
const NORMAL_USAGE_COLOR: Color = tailwind::GREEN.c500;
const MEDIUM_USAGE_COLOR: Color = tailwind::YELLOW.c500;
const HIGH_USAGE_COLOR: Color = tailwind::RED.c500;

/// Display the system details like OS version, memory usage etc...
#[derive(Debug)]
pub struct SystemOverview {
    /// Action sender that can send actions to all other components
    action_sender: Option<tokio::sync::mpsc::UnboundedSender<Action>>,
    /// Collection of system information, e.g. system resources, usage and OS specifics
    system_details: SystemDetails,
    /// Default App-Theme is `Dark`
    theme: Theme,
    is_active: bool,
}

impl Default for SystemOverview {
    fn default() -> Self {
        Self {
            action_sender: None,
            system_details: SystemDetails::default(),
            theme: Theme::default(),
            is_active: true,
        }
    }
}

impl SystemOverview {
    /// Helper function to send a [`Action`] to all components
    fn send_app_action(&self, action: Action) -> Result<()> {
        if let Some(handler) = &self.action_sender {
            handler.send(action)?
        }
        Ok(())
    }

    fn refresh_system_details(&mut self) {
        self.system_details.refresh()
    }

    fn get_sys_info_lines(&self) -> (Vec<Line>, Vec<Line>) {
        let theme_colors = self.theme.theme_colors();

        let system_keys: Vec<Line> = vec![
            Line::from(Span::from("OS-Name       :").fg(theme_colors.alt_fg))
                .alignment(Alignment::Left),
            Line::from(Span::from("Kernel-Version:").fg(theme_colors.alt_fg))
                .alignment(Alignment::Left),
            Line::from(Span::from("OS-Version    :").fg(theme_colors.alt_fg))
                .alignment(Alignment::Left),
            Line::from(Span::from("Hostname      :").fg(theme_colors.alt_fg))
                .alignment(Alignment::Left),
            Line::from(Span::from("CPU-Arch      :").fg(theme_colors.alt_fg))
                .alignment(Alignment::Left),
        ];

        let system_values = vec![
            Line::from(
                Span::default()
                    .content(self.system_details.system_name.to_owned())
                    .fg(theme_colors.alt_fg),
            )
            .alignment(Alignment::Left),
            Line::from(
                Span::default()
                    .content(&self.system_details.kernel_version)
                    .fg(theme_colors.alt_fg),
            )
            .alignment(Alignment::Left),
            Line::from(
                Span::default()
                    .content(&self.system_details.os_version)
                    .fg(theme_colors.alt_fg),
            )
            .alignment(Alignment::Left),
            Line::from(
                Span::default()
                    .content(&self.system_details.hostname)
                    .fg(theme_colors.alt_fg),
            )
            .alignment(Alignment::Left),
            Line::from(
                Span::default()
                    .content(&self.system_details.cpu_arch)
                    .fg(theme_colors.alt_fg),
            )
            .alignment(Alignment::Left),
        ];

        (system_keys, system_values)
    }

    fn get_resource_info_lines(&self) -> (Vec<Line>, Vec<Line>) {
        let theme_colors = self.theme.theme_colors();

        let resource_keys = vec![
            Line::from(Span::from("CPU-Cores   :").fg(theme_colors.alt_fg))
                .alignment(Alignment::Left),
            Line::from(Span::from("Total Space :").fg(theme_colors.alt_fg))
                .alignment(Alignment::Left),
            Line::from(Span::from("Total Memory:").fg(theme_colors.alt_fg))
                .alignment(Alignment::Left),
            Line::from(Span::from("Total Swap  :").fg(theme_colors.alt_fg))
                .alignment(Alignment::Left),
        ];

        let resource_values = vec![
            Line::from(
                Span::default()
                    .content(self.system_details.cpu_cores.to_string())
                    .fg(theme_colors.alt_fg),
            )
            .alignment(Alignment::Left),
            Line::from(
                Span::default()
                    .content(utils::convert_bytes_to_human_readable(
                        self.system_details.total_space,
                    ))
                    .fg(theme_colors.alt_fg),
            )
            .alignment(Alignment::Left),
            Line::from(
                Span::default()
                    .content(utils::convert_bytes_to_human_readable(
                        self.system_details.total_memory,
                    ))
                    .fg(theme_colors.alt_fg),
            )
            .alignment(Alignment::Left),
            Line::from(
                Span::default()
                    .content(utils::convert_bytes_to_human_readable(
                        self.system_details.total_swap,
                    ))
                    .fg(theme_colors.alt_fg),
            )
            .alignment(Alignment::Left),
        ];

        (resource_keys, resource_values)
    }

    fn draw_resource_info(&self, f: &mut ratatui::Frame<'_>, area: Rect) {
        let theme_colors = self.theme.theme_colors();

        let info_block = Block::default()
            .title(" Resources ")
            .title_style(Style::new().fg(theme_colors.main_fg))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::new().fg(theme_colors.main_fg))
            .title_alignment(Alignment::Left)
            .padding(Padding::proportional(1))
            .style(Style::new().bg(theme_colors.main_bg));

        let inner_block_area = info_block.inner(area);

        let [keys_area, values_area] =
            Layout::horizontal([Constraint::Length(14), Constraint::Fill(1)])
                .areas(inner_block_area);

        let keys = Paragraph::new(self.get_resource_info_lines().0);

        let values = Paragraph::new(self.get_resource_info_lines().1);

        f.render_widget(info_block, area);
        f.render_widget(keys, keys_area);
        f.render_widget(values, values_area);
    }

    fn draw_sys_info(&self, f: &mut ratatui::Frame<'_>, area: Rect) {
        let theme_colors = self.theme.theme_colors();

        let info_block = Block::default()
            .title(" System ")
            .title_style(Style::new().fg(theme_colors.main_fg))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::new().fg(theme_colors.main_fg))
            .title_alignment(Alignment::Left)
            .padding(Padding::left(1))
            .style(Style::new().bg(theme_colors.main_bg));

        let inner_block_area = info_block.inner(area);

        let [keys_area, values_area] =
            Layout::horizontal([Constraint::Length(16), Constraint::Fill(1)])
                .areas(inner_block_area);

        let keys = Paragraph::new(self.get_sys_info_lines().0);

        let values = Paragraph::new(self.get_sys_info_lines().1);

        f.render_widget(info_block, area);
        f.render_widget(keys, keys_area);
        f.render_widget(values, values_area);
    }

    fn draw_usage_info(&self, f: &mut ratatui::Frame<'_>, area: Rect) {
        let theme_colors = self.theme.theme_colors();

        let memory_block = Block::default()
            .title(" Usage ")
            .title_style(Style::new().fg(theme_colors.main_fg))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::new().fg(theme_colors.main_fg))
            .title_alignment(Alignment::Left)
            .padding(Padding::proportional(1))
            .style(Style::new().bg(theme_colors.main_bg));

        let inner_block = memory_block.inner(area);

        let [cpu_gauge_area, disk_gauge_area, memory_gauge_area, swap_gauge_area] =
            Layout::vertical([
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
            ])
            .areas(inner_block);

        let cpu_label = format!("CPU     {:.1}%", self.system_details.cpu_usage);
        let cpu_gauge = LineGauge::default()
            .line_set(symbols::line::THICK)
            .filled_style(get_gauge_color(self.system_details.cpu_usage as f64))
            .ratio(self.system_details.cpu_usage as f64 / 100.0)
            .fg(theme_colors.alt_bg)
            .label(Line::from(Span::default().content(cpu_label)).fg(theme_colors.alt_fg));

        let disk_usage_value = utils::calculate_percentage_f64(
            self.system_details.used_space as f64,
            self.system_details.total_space as f64,
        );
        let disk_label = format!("Disk    {:.1}%", disk_usage_value);
        let disk_gauge = LineGauge::default()
            .line_set(symbols::line::THICK)
            .filled_style(get_gauge_color(disk_usage_value))
            .ratio(disk_usage_value / 100.0)
            .fg(theme_colors.alt_bg)
            .label(Line::from(Span::default().content(disk_label)).fg(theme_colors.alt_fg));

        let memory_usage_value = utils::calculate_percentage_f64(
            self.system_details.used_memory as f64,
            self.system_details.total_memory as f64,
        );

        let memory_label = format!("Memory  {:.1}%", memory_usage_value);

        let memory_gauge = LineGauge::default()
            .line_set(symbols::line::THICK)
            .filled_style(get_gauge_color(memory_usage_value))
            .ratio(memory_usage_value / 100.0)
            .fg(theme_colors.alt_bg)
            .label(Line::from(Span::default().content(memory_label)).fg(theme_colors.alt_fg));

        let swap_usage_value = utils::calculate_percentage_f64(
            self.system_details.used_swap as f64,
            self.system_details.total_swap as f64,
        );

        let swap_label = format!("Swap    {:.1}%", swap_usage_value);

        let swap_gauge = LineGauge::default()
            .line_set(symbols::line::THICK)
            .filled_style(get_gauge_color(swap_usage_value))
            .ratio(swap_usage_value / 100.0)
            .fg(theme_colors.alt_bg)
            .label(Line::from(Span::default().content(swap_label)).fg(theme_colors.alt_fg));

        f.render_widget(memory_block, area);
        f.render_widget(cpu_gauge, cpu_gauge_area);
        f.render_widget(disk_gauge, disk_gauge_area);
        f.render_widget(memory_gauge, memory_gauge_area);
        f.render_widget(swap_gauge, swap_gauge_area);
    }
}

#[async_trait(?Send)]
impl Component for SystemOverview {
    fn register_component_action_sender(
        &mut self,
        tx: tokio::sync::mpsc::UnboundedSender<Action>,
    ) -> Result<()> {
        self.action_sender = Some(tx);
        Ok(())
    }

    fn register_config_handler(&mut self, config: AppConfig) -> Result<()> {
        self.theme = config.theme();
        Ok(())
    }

    async fn handle_events(&mut self, event: Option<crate::tui::Event>) -> Result<Option<Action>> {
        if let Some(event) = event {
            match event {
                Event::Key(_) => {
                    return Ok(None);
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
        self.is_active
    }

    async fn update(&mut self, action: Action) -> Result<Option<Action>> {
        match action {
            Action::Tick => {
                if self.is_active {
                    self.refresh_system_details()
                }
            }
            Action::ToggleTheme(theme) => {
                self.theme = theme;
            }
            Action::HideOrShowSystemOverview => {
                self.is_active = !self.is_active;
            }
            _ => {}
        }

        Ok(None)
    }

    fn render(&mut self, f: &mut ratatui::Frame<'_>, area: Rect) -> Result<()> {
        if self.should_render() {
            let draw_area = get_main_layout(area).overview_area;

            let overview_layout = Layout::horizontal([
                Constraint::Length(42),
                Constraint::Length(30),
                Constraint::Fill(1),
            ]);

            let [sys_info_area, resource_info_area, usage_info_area] =
                overview_layout.areas(draw_area);

            self.draw_sys_info(f, sys_info_area);
            self.draw_resource_info(f, resource_info_area);
            self.draw_usage_info(f, usage_info_area);
        }

        Ok(())
    }
}

// Get the gauge color depending on utilization
fn get_gauge_color(usage: f64) -> Color {
    if usage <= 40.0 {
        NORMAL_USAGE_COLOR
    } else if usage > 40.0 && usage <= 75.0 {
        MEDIUM_USAGE_COLOR
    } else {
        HIGH_USAGE_COLOR
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_gauge_color_green() {
        let input = 20.99845637_f64;
        assert_eq!(get_gauge_color(input), NORMAL_USAGE_COLOR);
    }

    #[test]
    fn test_gauge_color_yellow() {
        let input = 66.0000_f64;
        assert_eq!(get_gauge_color(input), MEDIUM_USAGE_COLOR);
    }

    #[test]
    fn test_gauge_color_red() {
        let input = 76.267735_f64;
        assert_eq!(get_gauge_color(input), HIGH_USAGE_COLOR);
    }
}
