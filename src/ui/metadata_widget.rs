#![allow(dead_code)]
use anyhow::Result;
use async_trait::async_trait;
use ratatui::{prelude::*, style::palette::tailwind, widgets::*};

use crate::{
    app::{actions::Action, AppState},
    component::Component,
    models::{Scrollable, StatefulTable},
    tui::Event,
    ui::{centered_rect, Theme, HIGHLIGHT_SYMBOL, PALETTES},
};

const BLOCK_TITLE_SCROLLABLE: &str = " Metadata | <Esc> close | (↑) move up | (↓) move down ";
const BLOCK_TITLE: &str = " Metadata | <Esc> close ";

#[derive(Debug)]
struct TableColors {
    buffer_bg: Color,
    row_fg: Color,
    normal_row_color: Color,
    alt_row_color: Color,
    selected_style_fg: Color,
}

impl TableColors {
    const fn new(color: &tailwind::Palette) -> Self {
        Self {
            buffer_bg: tailwind::SLATE.c950,
            row_fg: tailwind::SLATE.c200,
            normal_row_color: tailwind::SLATE.c950,
            alt_row_color: tailwind::SLATE.c800,
            selected_style_fg: color.c400,
        }
    }
}

#[derive(Debug)]
pub struct MetadataPage {
    /// Action sender that can send actions to all other components
    action_sender: Option<tokio::sync::mpsc::UnboundedSender<Action>>,
    theme: Theme,
    border_style: Style,
    border_type: BorderType,
    title_style: Style,
    metadata: StatefulTable<Vec<String>>,
    scrollbar_state: ScrollbarState,
    object_name: String,
    colors: TableColors,
    color_index: usize,
    is_active: bool,
}

impl MetadataPage {
    /// Helper function to send a [`Action`] to all components
    fn send_app_action(&self, action: Action) -> Result<()> {
        if let Some(handler) = &self.action_sender {
            handler.send(action)?
        }
        Ok(())
    }
}

impl Default for MetadataPage {
    fn default() -> Self {
        Self {
            action_sender: Default::default(),
            theme: Default::default(),
            border_style: Style::new().bold().fg(Color::LightGreen),
            border_type: BorderType::Rounded,
            title_style: Default::default(),
            metadata: StatefulTable::new(),
            scrollbar_state: ScrollbarState::default(),
            object_name: Default::default(),
            color_index: Default::default(),
            colors: TableColors::new(&PALETTES[0]),
            is_active: Default::default(),
        }
    }
}

#[async_trait(?Send)]
impl Component for MetadataPage {
    fn register_component_action_sender(
        &mut self,
        tx: tokio::sync::mpsc::UnboundedSender<Action>,
    ) -> Result<()> {
        self.action_sender = Some(tx);
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
                self.metadata.scroll_up_by(1);
                self.scrollbar_state = self.scrollbar_state.position(self.metadata.selected_item);

                Ok(None)
            }
            crossterm::event::KeyCode::Down => {
                self.metadata.scroll_down_by(1);
                self.scrollbar_state = self.scrollbar_state.position(self.metadata.selected_item);

                Ok(None)
            }
            crossterm::event::KeyCode::Esc => {
                self.is_active = false;
                Ok(Action::CloseMetadata.into())
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
            Action::ShowFileMetadata(file_path, metadata) => {
                self.send_app_action(Action::UpdateAppState(AppState::Done("Done".to_string())))?;
                self.object_name = file_path
                    .file_name()
                    .map_or(file_path.as_os_str().to_string_lossy().to_string(), |f| {
                        f.to_string_lossy().to_string()
                    });
                self.metadata
                    .set_items(metadata.get_metadata_rows(file_path));
                self.metadata.state.select(Some(0));
                self.scrollbar_state = ScrollbarState::new(self.metadata.items.len()).position(0);
                self.is_active = true;
            }
            Action::ShowDirMetadata(metadata) => {
                self.send_app_action(Action::UpdateAppState(AppState::Done("Done".to_string())))?;
                self.metadata.set_items(metadata.get_metadata_rows());
                self.metadata.state.select(Some(0));
                self.object_name = metadata.dir_name.clone();
                self.scrollbar_state = ScrollbarState::new(self.metadata.items.len()).position(0);
                self.is_active = true;
            }
            _ => {}
        }
        Ok(None)
    }

    fn render(&mut self, f: &mut ratatui::Frame<'_>, area: Rect) -> Result<()> {
        if self.should_render() {
            let draw_area = centered_rect(60, 50, area);
            let block = Block::new()
                .title_bottom(Span::styled(
                    format!(" {} ", self.object_name),
                    Style::new().fg(Color::White),
                ))
                .title_alignment(Alignment::Center)
                .title_style(self.title_style)
                .border_type(self.border_type)
                .borders(Borders::ALL)
                .border_style(self.border_style)
                .padding(Padding {
                    left: 1,
                    right: 0,
                    top: 0,
                    bottom: 1,
                });

            let rows_counter: usize = self.metadata.items.iter().enumerate().map(|(i, _)| i).sum();

            let rows = self.metadata.items.iter().enumerate().map(|(i, data)| {
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

            let table_widths = [Constraint::Percentage(35), Constraint::Percentage(70)];

            // clear/reset a certain area to allow overdrawing (e.g. for popups).
            f.render_widget(Clear, draw_area);

            if draw_area.height < rows_counter as u16 {
                let metadata_page_table = Table::new(rows, table_widths)
                    .block(block.title(Line::from(BLOCK_TITLE_SCROLLABLE).left_aligned()))
                    .highlight_symbol(
                        Text::from(vec!["\n".into(), HIGHLIGHT_SYMBOL.into()])
                            .style(Style::new().fg(self.colors.selected_style_fg)),
                    )
                    .bg(self.colors.buffer_bg);

                let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                    .begin_symbol(Some("↑"))
                    .end_symbol(Some("↓"));
                f.render_stateful_widget(metadata_page_table, draw_area, &mut self.metadata.state);
                f.render_stateful_widget(
                    scrollbar,
                    draw_area.inner(Margin {
                        // using an inner vertical margin of 1 unit makes the scrollbar inside the current block
                        vertical: 1,
                        horizontal: 0,
                    }),
                    &mut self.scrollbar_state,
                );
            } else {
                let metadata_page_table = Table::new(rows, table_widths)
                    .block(block.title(Line::from(BLOCK_TITLE).left_aligned()))
                    .bg(self.colors.buffer_bg);

                self.scrollbar_state = ScrollbarState::new(self.metadata.items.len()).position(0);
                self.metadata.state.select(Some(0));
                f.render_stateful_widget(metadata_page_table, draw_area, &mut self.metadata.state);
            }
        }

        Ok(())
    }
}
