use anyhow::Result;
use async_trait::async_trait;
use ratatui::{prelude::*, style::palette::tailwind, widgets::*};

use crate::{
    app::{actions::Action, key_bindings, AppContext},
    component::Component,
    models::{Scrollable, StatefulTable},
    tui::Event,
    ui::{HIGHLIGHT_SYMBOL, PALETTES},
};

// const INFO_TEXT: &str =
//     " Help | <Esc> close | (↑) move up | (↓) move down | (→) next color | (←) previous color ";

const BLOCK_TITLE_SCROLLABLE: &str = " Help | <Esc> close | (↑) move up | (↓) move down ";
const BLOCK_TITLE: &str = " Help | <Esc> close ";

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
pub struct HelpPage {
    caller_context: AppContext,
    border_style: Style,
    border_type: BorderType,
    title_style: Style,
    help_docs: StatefulTable<Vec<String>>,
    scrollbar_vertical_state: ScrollbarState,
    colors: TableColors,
    color_index: usize,
    is_active: bool,
}

impl HelpPage {
    pub fn _next_color(&mut self) {
        self.color_index = (self.color_index + 1) % PALETTES.len();
    }

    pub fn _previous_color(&mut self) {
        let count = PALETTES.len();
        self.color_index = (self.color_index + count - 1) % count;
    }

    pub fn set_colors(&mut self) {
        self.colors = TableColors::new(&PALETTES[self.color_index]);
    }
}

impl Default for HelpPage {
    fn default() -> Self {
        Self {
            caller_context: AppContext::NotActive,
            border_style: Style::new().bold().fg(Color::LightGreen),
            border_type: BorderType::Rounded,
            title_style: Default::default(),
            help_docs: StatefulTable::with_items(key_bindings::get_help_docs()),
            scrollbar_vertical_state: ScrollbarState::new(key_bindings::get_help_docs().len())
                .position(0),
            colors: TableColors::new(&PALETTES[0]),
            color_index: Default::default(),
            is_active: Default::default(),
        }
    }
}

#[async_trait(?Send)]
impl Component for HelpPage {
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
                self.help_docs.scroll_up_by(1);
                self.scrollbar_vertical_state = self
                    .scrollbar_vertical_state
                    .position(self.help_docs.selected_item);
                Ok(None)
            }
            crossterm::event::KeyCode::Down => {
                self.help_docs.scroll_down_by(1);
                self.scrollbar_vertical_state = self
                    .scrollbar_vertical_state
                    .position(self.help_docs.selected_item);
                Ok(None)
            }
            crossterm::event::KeyCode::Esc => {
                self.help_docs.state.select(Some(0));
                self.scrollbar_vertical_state = self.scrollbar_vertical_state.position(0);
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
        if let Action::ShowHelp(caller_context) = action {
            self.caller_context = caller_context;
            self.is_active = true;
        }

        Ok(None)
    }

    fn render(&mut self, f: &mut ratatui::Frame<'_>, area: Rect) -> Result<()> {
        if self.should_render() {
            self.set_colors();

            let header_style = Style::default()
                .fg(self.colors.header_fg)
                .bg(self.colors.header_bg);

            let outer_block = Block::new().bg(self.colors.buffer_bg);

            let [help_block_area] =
                Layout::vertical([Constraint::Fill(1)]).areas(outer_block.inner(area));

            let help_block = Block::new()
                .title_style(self.title_style)
                .border_type(self.border_type)
                .borders(Borders::ALL)
                .border_style(self.border_style)
                .bg(self.colors.buffer_bg);

            let header = ["Key", "Context", "Description"]
                .into_iter()
                .map(Cell::from)
                .collect::<Row>()
                .style(header_style)
                .height(1);

            let rows_counter: usize = self
                .help_docs
                .items
                .iter()
                .map(|item| item.len())
                .sum::<usize>();

            let rows = self.help_docs.items.iter().enumerate().map(|(i, data)| {
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

            let table_widths = [
                Constraint::Length(18),
                Constraint::Length(30),
                Constraint::Fill(1),
            ];

            // clear/reset a certain area to allow overdrawing (e.g. for popups).
            f.render_widget(Clear, area);

            if help_block_area.height < (rows_counter + 4) as u16 {
                let help_page_table = Table::new(rows, table_widths)
                    .header(header)
                    .block(help_block.title(BLOCK_TITLE_SCROLLABLE).padding(Padding {
                        left: 0,
                        right: 0,
                        top: 1,
                        bottom: 0,
                    }))
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
                    help_block_area,
                    &mut self.help_docs.state,
                );
                f.render_stateful_widget(
                    scrollbar_vertical,
                    help_block_area.inner(Margin {
                        // using an inner vertical margin of 1 unit makes the scrollbar inside the current block
                        vertical: 1,
                        horizontal: 0,
                    }),
                    &mut self.scrollbar_vertical_state,
                );
            } else {
                let help_page_table = Table::new(rows, table_widths)
                    .header(header)
                    .block(help_block.title(BLOCK_TITLE).padding(Padding {
                        left: 1,
                        right: 0,
                        top: 1,
                        bottom: 0,
                    }))
                    .bg(self.colors.buffer_bg);

                self.scrollbar_vertical_state =
                    ScrollbarState::new(self.help_docs.items.len()).position(0);
                self.help_docs.state.select(Some(0));

                f.render_stateful_widget(
                    help_page_table,
                    help_block_area,
                    &mut self.help_docs.state,
                );
            }
        }

        Ok(())
    }
}
