use anyhow::Result;
use async_trait::async_trait;
use ratatui::{prelude::*, style::palette::tailwind, widgets::*};

use crate::{
    app::{actions::Action, config::AppConfig, AppContext},
    component::Component,
    tui::Event,
    ui::PALETTES,
    utils::{absolute_path_as_string, config_dir, data_dir, format_path_for_display},
};

const BLOCK_TITLE: &str = " About | <Esc> close ";

#[derive(Debug)]
struct TableColors {
    buffer_bg: Color,
    header_bg: Color,
    header_fg: Color,
    row_fg: Color,
    normal_row_color: Color,
    alt_row_color: Color,
}

impl TableColors {
    const fn new(color: &tailwind::Palette) -> Self {
        Self {
            buffer_bg: tailwind::SLATE.c950,
            header_bg: color.c900,
            header_fg: tailwind::SLATE.c200,
            row_fg: tailwind::SLATE.c200,
            normal_row_color: tailwind::SLATE.c950,
            alt_row_color: tailwind::SLATE.c800,
        }
    }
}

#[derive(Debug)]
pub struct AboutPage {
    caller_context: AppContext,
    config: AppConfig,
    border_style: Style,
    border_type: BorderType,
    title_style: Style,
    colors: TableColors,
    is_active: bool,
}

impl AboutPage {
    fn app_info(&self) -> Vec<Vec<String>> {
        let authors = env!("CARGO_PKG_AUTHORS").replace(":", ", ");
        let version = env!("CARGO_PKG_VERSION");
        let repo = env!("CARGO_PKG_REPOSITORY");

        let config_dir = format_path_for_display(absolute_path_as_string(config_dir()));
        let data_dir = format_path_for_display(absolute_path_as_string(data_dir()));
        vec![
            vec!["Authors".into(), authors],
            vec!["Version".into(), version.into()],
            vec!["Repository".into(), repo.into()],
            vec!["Config-Directory".into(), config_dir],
            vec!["Data-Directory".into(), data_dir],
        ]
    }

    fn config_info(&self) -> Vec<Vec<String>> {
        let default_theme = self.config.theme().to_string();
        let start_dir = format_path_for_display(absolute_path_as_string(self.config.start_dir()));
        let export_dir = format_path_for_display(absolute_path_as_string(self.config.export_dir()));
        let follow_sym_links = match self.config.follow_sym_links() {
            true => "Yes",
            false => "No",
        };

        vec![
            vec!["Default theme".into(), default_theme],
            vec!["Start directory".into(), start_dir],
            vec!["Export directory".into(), export_dir],
            vec!["Follow symbolic links".into(), follow_sym_links.into()],
        ]
    }
}

impl Default for AboutPage {
    fn default() -> Self {
        Self {
            caller_context: AppContext::NotActive,
            config: Default::default(),
            border_style: Style::new().bold().fg(Color::LightGreen),
            border_type: BorderType::Rounded,
            title_style: Default::default(),
            colors: TableColors::new(&PALETTES[0]),
            is_active: Default::default(),
        }
    }
}

#[async_trait(?Send)]
impl Component for AboutPage {
    fn register_config_handler(&mut self, config: AppConfig) -> Result<()> {
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
            crossterm::event::KeyCode::Up => Ok(None),
            crossterm::event::KeyCode::Down => Ok(None),
            crossterm::event::KeyCode::Esc => {
                self.is_active = false;
                Ok(Action::SwitchAppContext(self.caller_context).into())
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

    async fn update(&mut self, action: Action) -> Result<Option<Action>> {
        if let Action::ShowAbout(caller_context) = action {
            self.caller_context = caller_context;
            self.is_active = true;
        }

        Ok(None)
    }

    fn render(&mut self, f: &mut ratatui::Frame<'_>, area: Rect) -> Result<()> {
        if self.should_render() {
            let outer_block = Block::new().bg(self.colors.buffer_bg);

            let [about_block_area] =
                Layout::vertical([Constraint::Fill(1)]).areas(outer_block.inner(area));

            let about_block = Block::new()
                .title(BLOCK_TITLE)
                .title_alignment(Alignment::Left)
                .title_style(self.title_style)
                .border_type(self.border_type)
                .borders(Borders::ALL)
                .border_style(self.border_style)
                .bg(self.colors.buffer_bg);

            let app_info_height = self
                .app_info()
                .iter()
                .map(|item| item.len() as u16)
                .sum::<u16>()
                + 4;
            let config_info_height = self
                .config_info()
                .iter()
                .map(|item| item.len() as u16)
                .sum::<u16>()
                + 1;

            let [app_info_area, config_info_area] = Layout::vertical([
                Constraint::Length(app_info_height),
                Constraint::Length(config_info_height),
            ])
            .areas(about_block.inner(about_block_area));

            let app_info = self.app_info();
            let app_info_rows = app_info.iter().enumerate().map(|(i, data)| {
                let color = match i % 2 {
                    0 => self.colors.normal_row_color,
                    _ => self.colors.alt_row_color,
                };

                data.iter()
                    .map(|content| Cell::from(Text::from(format!("\n{content}\n"))))
                    .collect::<Row>()
                    .style(Style::new().fg(self.colors.row_fg).bg(color))
                    .height(2)
            });

            let config_info = self.config_info();
            let config_info_rows = config_info.iter().enumerate().map(|(i, data)| {
                let color = match i % 2 {
                    0 => self.colors.normal_row_color,
                    _ => self.colors.alt_row_color,
                };

                data.iter()
                    .map(|content| Cell::from(Text::from(format!("\n{content}\n"))))
                    .collect::<Row>()
                    .style(Style::new().fg(self.colors.row_fg).bg(color))
                    .height(2)
            });

            let table_widths = [Constraint::Length(25), Constraint::Fill(1)];

            let header_style = Style::default()
                .fg(self.colors.header_fg)
                .bg(self.colors.header_bg);

            let header_app_info = ["App", " "]
                .into_iter()
                .map(Cell::from)
                .collect::<Row>()
                .style(header_style)
                .height(1);

            let header_config_info = ["Configuration", " "]
                .into_iter()
                .map(Cell::from)
                .collect::<Row>()
                .style(header_style)
                .height(1);

            let app_info_table = Table::new(app_info_rows, table_widths)
                .header(header_app_info)
                .block(Block::new().bg(self.colors.buffer_bg).padding(Padding {
                    left: 1,
                    right: 1,
                    top: 1,
                    bottom: 1,
                }))
                .bg(self.colors.buffer_bg);

            let config_info_table = Table::new(config_info_rows, table_widths)
                .header(header_config_info)
                .block(Block::new().bg(self.colors.buffer_bg).padding(Padding {
                    left: 1,
                    right: 1,
                    top: 0,
                    bottom: 0,
                }))
                .bg(self.colors.buffer_bg);

            // clear/reset a certain area to allow overdrawing (e.g. for popups).
            f.render_widget(Clear, area);

            f.render_widget(about_block, about_block_area);
            f.render_widget(app_info_table, app_info_area);
            f.render_widget(config_info_table, config_info_area);
        }

        Ok(())
    }
}
