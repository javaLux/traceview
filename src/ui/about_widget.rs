use anyhow::{Context, Result};
use async_trait::async_trait;
use ratatui::{prelude::*, style::palette::tailwind, widgets::*};
use serde_json::{Map, Value, json};

use crate::{
    app::{AppContext, actions::Action},
    component::Component,
    models::{Scrollable, StatefulTable},
    tui::Event,
    ui::{HIGHLIGHT_SYMBOL, PALETTES},
    utils::{
        absolute_path_as_string, app_name, config_dir, copy_to_clipboard, data_dir,
        expand_and_resolve_path, format_path_for_display,
    },
};

#[derive(Debug)]
struct TableColors {
    buffer_bg: Color,
    header_bg: Color,
    header_fg: Color,
    row_fg: Color,
    selected_style_fg: Color,
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
            selected_style_fg: color.c400,
            normal_row_color: tailwind::SLATE.c950,
            alt_row_color: tailwind::SLATE.c800,
        }
    }
}

#[derive(Debug)]
pub struct AboutPage {
    caller_context: AppContext,
    border_style: Style,
    border_type: BorderType,
    title_style: Style,
    colors: TableColors,
    about_docs: StatefulTable<Vec<String>>,
    scrollbar_state: ScrollbarState,
    is_active: bool,
}

impl AboutPage {
    fn about() -> Vec<Vec<String>> {
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

    fn copy_about(&self) -> Result<()> {
        let app_info = AboutPage::about();
        let mut map = Map::new();
        for pair in app_info {
            if let [key, value] = &pair[..] {
                if value.starts_with("~") {
                    map.insert(key.clone(), Value::String(expand_and_resolve_path(value)));
                } else {
                    map.insert(key.clone(), Value::String(value.clone()));
                }
            }
        }
        let app_info_json = json!(map);

        let about_json = serde_json::to_string_pretty(&serde_json::json!(
            {
                app_name(): app_info_json
            }
        ))
        .context("Failed to serialize about message")?;

        copy_to_clipboard(&about_json).context("Failed to copy about message to clipboard")?;
        Ok(())
    }

    fn block_title_scroll() -> ratatui::prelude::Line<'static> {
        Line::from(vec![
            Span::raw(" About | "),
            Span::styled("<Esc> ", Style::default().fg(Color::Yellow)),
            Span::raw("Close  "),
            Span::styled("<Ctrl+C> ", Style::default().fg(Color::Yellow)),
            Span::raw("Copy to Clipboard "),
            Span::styled(" <↑↓> ", Style::default().fg(Color::Yellow)),
            Span::raw("Scroll "),
        ])
    }

    fn block_title() -> ratatui::prelude::Line<'static> {
        Line::from(vec![
            Span::raw(" About | "),
            Span::styled("<Esc> ", Style::default().fg(Color::Yellow)),
            Span::raw("Close  "),
            Span::styled("<Ctrl+C> ", Style::default().fg(Color::Yellow)),
            Span::raw("Copy to Clipboard "),
        ])
    }
}

impl Default for AboutPage {
    fn default() -> Self {
        Self {
            caller_context: AppContext::NotActive,
            border_style: Style::new().bold().fg(Color::LightGreen),
            border_type: BorderType::Rounded,
            title_style: Default::default(),
            colors: TableColors::new(&PALETTES[0]),
            about_docs: StatefulTable::with_items(AboutPage::about()),
            scrollbar_state: ScrollbarState::new(AboutPage::about().len()).position(0),
            is_active: Default::default(),
        }
    }
}

#[async_trait(?Send)]
impl Component for AboutPage {
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
                self.about_docs.scroll_up_by(1);
                self.scrollbar_state = self.scrollbar_state.position(self.about_docs.selected_item);
                Ok(None)
            }
            crossterm::event::KeyCode::Down => {
                self.about_docs.scroll_down_by(1);
                self.scrollbar_state = self.scrollbar_state.position(self.about_docs.selected_item);
                Ok(None)
            }
            crossterm::event::KeyCode::Esc => {
                self.is_active = false;
                Ok(Action::SwitchAppContext(self.caller_context).into())
            }
            crossterm::event::KeyCode::Char('c')
                if key.modifiers == crossterm::event::KeyModifiers::CONTROL =>
            {
                if let Err(copy_err) = self.copy_about() {
                    log::error!("Failed to copy about message: {copy_err}");
                }
                Ok(None)
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
        if let Action::ShowAbout(caller_context) = action {
            self.caller_context = *caller_context;
            self.is_active = true;
        }
        Ok(None)
    }

    fn render(&mut self, f: &mut ratatui::Frame<'_>, area: Rect) -> Result<()> {
        if self.should_render() {
            let header_style = Style::default()
                .fg(self.colors.header_fg)
                .bg(self.colors.header_bg);

            let outer_block = Block::new().bg(self.colors.buffer_bg);

            let [about_block_area] =
                Layout::vertical([Constraint::Fill(1)]).areas(outer_block.inner(area));

            let about_block = Block::new()
                .title_style(self.title_style)
                .border_type(self.border_type)
                .borders(Borders::ALL)
                .border_style(self.border_style)
                .bg(self.colors.buffer_bg);

            let header = [app_name(), "".into()]
                .into_iter()
                .map(Cell::from)
                .collect::<Row>()
                .style(header_style)
                .height(1);

            let rows_counter = self.about_docs.items.len() * 2;

            let rows = self.about_docs.items.iter().enumerate().map(|(i, data)| {
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

            // clear/reset a certain area to allow overdrawing (e.g. for popups).
            f.render_widget(Clear, area);

            if area.height <= (rows_counter + 3) as u16 {
                let help_page_table =
                    Table::new(rows, table_widths)
                        .header(header)
                        .block(about_block.title(AboutPage::block_title_scroll()).padding(
                            Padding {
                                left: 0,
                                right: 0,
                                top: 1,
                                bottom: 0,
                            },
                        ))
                        .highlight_symbol(
                            Text::from(vec!["\n".into(), HIGHLIGHT_SYMBOL.into()])
                                .style(Style::new().fg(self.colors.selected_style_fg)),
                        )
                        .bg(self.colors.buffer_bg)
                        .highlight_spacing(HighlightSpacing::Always);

                let scrollbar_vertical = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                    .begin_symbol(Some("↑"))
                    .end_symbol(Some("↓"));

                f.render_stateful_widget(
                    help_page_table,
                    about_block_area,
                    &mut self.about_docs.state,
                );
                f.render_stateful_widget(
                    scrollbar_vertical,
                    about_block_area.inner(Margin {
                        // using an inner vertical margin of 1 unit makes the scrollbar inside the current block
                        vertical: 1,
                        horizontal: 0,
                    }),
                    &mut self.scrollbar_state,
                );
            } else {
                let help_page_table = Table::new(rows, table_widths)
                    .header(header)
                    .block(
                        about_block
                            .title(AboutPage::block_title())
                            .padding(Padding {
                                left: 1,
                                right: 0,
                                top: 1,
                                bottom: 0,
                            }),
                    )
                    .highlight_symbol(
                        Text::from(vec!["\n".into(), "   ".into()])
                            .style(Style::new().fg(self.colors.selected_style_fg)),
                    )
                    .bg(self.colors.buffer_bg)
                    .highlight_spacing(HighlightSpacing::Always);

                self.scrollbar_state = ScrollbarState::new(self.about_docs.items.len()).position(0);
                self.about_docs.state.select(Some(0));

                f.render_stateful_widget(
                    help_page_table,
                    about_block_area,
                    &mut self.about_docs.state,
                );
            }
        }

        Ok(())
    }
}
