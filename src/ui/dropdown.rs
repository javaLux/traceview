use anyhow::Result;
use ratatui::{
    Frame,
    layout::Rect,
    prelude::*,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, BorderType, Borders, Clear, List, ListItem, ListState, Scrollbar,
        ScrollbarOrientation, ScrollbarState,
    },
};

use crate::{app::actions::Action, ui::centered_rect};
use crossterm::event::{KeyCode, KeyEvent};

#[derive(Debug, Default)]
pub struct Dropdown<T>
where
    T: ToString + Clone,
{
    items: Vec<T>,
    /// last confirmed item
    confirmed_index: usize,
    /// current highlighted item at navigation
    highlighted_index: usize,
    expanded: bool,
    list_state: ListState,
    max_visible_items: usize,
    scroll_offset: usize,
}

impl<T> Dropdown<T>
where
    T: ToString + Clone + PartialEq,
{
    pub fn new(items: Vec<T>, default: &T) -> Self {
        // Search for a default item
        let default_index = items.iter().position(|item| item == default).unwrap_or(0);

        let mut list_state = ListState::default();
        list_state.select(Some(default_index));

        Self {
            items,
            confirmed_index: default_index,
            highlighted_index: default_index,
            expanded: false,
            list_state,
            max_visible_items: 5,
            scroll_offset: default_index,
        }
    }

    pub fn with_max_visible(mut self, max: usize) -> Self {
        self.max_visible_items = max.max(1); // at least 1 Item is visible
        self
    }

    pub fn selected(&self) -> &T {
        &self.items[self.confirmed_index]
    }

    pub fn _selected_index(&self) -> usize {
        self.confirmed_index
    }

    pub fn toggle(&mut self) {
        if self.expanded {
            self.close();
        } else {
            self.open();
        }
    }

    pub fn open(&mut self) {
        self.expanded = true;
        // Navigation starts at the confirmed selection
        self.highlighted_index = self.confirmed_index;
        self.list_state.select(Some(self.confirmed_index));
        // set Scroll-Offset so that confirmed is visible
        self.scroll_offset = self
            .confirmed_index
            .saturating_sub(self.max_visible_items / 2);
        self.adjust_scroll();
    }

    pub fn close(&mut self) {
        self.expanded = false;
        // reset highlighted_index to confirmed
        self.highlighted_index = self.confirmed_index;
    }

    pub async fn handle_key_events(&mut self, key: KeyEvent) -> Result<Option<Action>> {
        match key.code {
            KeyCode::Down => {
                self.next();
            }
            KeyCode::Up => {
                self.previous();
            }
            KeyCode::Enter => {
                if !self.expanded {
                    self.toggle();
                } else {
                    self.confirm();
                }
            }
            KeyCode::Esc => {
                if !self.expanded {
                    self.close();
                    // Return an action to close the dropdown in the parent component
                    return Ok(Some(Action::DropDownClosed));
                } else {
                    // just close the dropdown without applying changes
                    self.close();
                }
            }
            _ => {}
        }
        Ok(None)
    }

    fn next(&mut self) {
        let last = self.items.len().saturating_sub(1);
        let next = self
            .list_state
            .selected()
            .map(|i| (i + 1).min(last))
            .unwrap_or(0);

        self.highlighted_index = next;
        self.list_state.select(Some(next));
        self.adjust_scroll();
    }

    fn previous(&mut self) {
        let prev = self
            .list_state
            .selected()
            .map(|i| i.saturating_sub(1))
            .unwrap_or(0);

        self.highlighted_index = prev;
        self.list_state.select(Some(prev));
        self.adjust_scroll();
    }

    fn confirm(&mut self) {
        self.confirmed_index = self.highlighted_index;
        self.expanded = false;
    }

    fn adjust_scroll(&mut self) {
        if let Some(selected) = self.list_state.selected() {
            if selected < self.scroll_offset {
                self.scroll_offset = selected;
            } else if selected >= self.scroll_offset + self.max_visible_items {
                self.scroll_offset = selected - self.max_visible_items + 1;
            }
        }
    }

    pub fn render(&mut self, f: &mut Frame, area: Rect, title: &str) {
        let draw_area = centered_rect(50, 40, area);
        let collapsed_height: u16 = 4;

        let collapsed_area = Rect {
            x: draw_area.x,
            y: draw_area.y,
            width: draw_area.width,
            height: collapsed_height,
        };

        self.render_collapsed(f, collapsed_area, title);

        if self.expanded {
            self.render_dropdown_list(f, collapsed_area);
        }
    }

    fn render_collapsed(&self, f: &mut Frame, area: Rect, title: &str) {
        let indicator = if self.expanded { "▲" } else { "▼" };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::new().bold().fg(Color::LightGreen))
            .title(format!(" {title} "))
            .title_style(Style::default())
            .title_bottom(self.help_text())
            .title_alignment(Alignment::Center);

        let inner = block.inner(area);

        f.render_widget(Clear, area);
        f.render_widget(block, area);

        let selected_text = Line::from(vec![
            Span::styled(
                format!(" {} ", self.items[self.confirmed_index].to_string()),
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::styled(indicator.to_string(), Style::default().fg(Color::Yellow)),
        ]);

        f.render_widget(
            List::new(vec![
                ListItem::new(Line::raw(" ")), // ← Empty Line for Padding
                ListItem::new(selected_text),  // ← Selected Item
            ]),
            inner,
        );
    }

    fn render_dropdown_list(&mut self, f: &mut Frame, collapsed_area: Rect) {
        let dropdown_height = self.items.len().min(self.max_visible_items) as u16;
        let needed_height = dropdown_height + 2; // + 2 Borders

        // check if there is enough space below the collapsed area to render the dropdown, otherwise render it above
        let space_below = f
            .area()
            .height
            .saturating_sub(collapsed_area.y + collapsed_area.height);

        let dropdown_area = if space_below >= needed_height {
            Rect {
                x: collapsed_area.x,
                y: collapsed_area.y + collapsed_area.height,
                width: collapsed_area.width,
                height: needed_height,
            }
        } else {
            // No space below, render above
            Rect {
                x: collapsed_area.x,
                y: collapsed_area.y.saturating_sub(needed_height),
                width: collapsed_area.width,
                height: needed_height,
            }
        };

        let visible_items: Vec<ListItem> = self
            .items
            .iter()
            .enumerate()
            .skip(self.scroll_offset)
            .take(self.max_visible_items)
            .map(|(i, item)| {
                let label = if i == self.confirmed_index {
                    format!("✓ {}", item.to_string())
                } else {
                    format!("  {}", item.to_string())
                };
                ListItem::new(label)
            })
            .collect();

        let mut list_state = self.list_state;
        if let Some(selected) = self.list_state.selected() {
            list_state.select(Some(selected.saturating_sub(self.scroll_offset)));
        }

        let list = List::new(visible_items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::new().bold().fg(Color::LightGreen)),
            )
            .highlight_style(
                Style::default()
                    .bg(Color::Blue)
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol(" ");

        f.render_widget(Clear, dropdown_area);
        f.render_stateful_widget(list, dropdown_area, &mut list_state);

        if self.items.len() > self.max_visible_items {
            let mut scrollbar_state = ScrollbarState::default()
                .content_length(self.items.len())
                .position(self.highlighted_index);

            f.render_stateful_widget(
                Scrollbar::default().orientation(ScrollbarOrientation::VerticalRight),
                dropdown_area,
                &mut scrollbar_state,
            );
        }
    }

    fn help_text(&self) -> ratatui::prelude::Line<'_> {
        // Help-Text based on state (expanded/collapsed)
        if self.expanded {
            Line::from(vec![
                Span::styled(" <↑↓> ", Style::default().fg(Color::Yellow)),
                Span::raw("Select  "),
                Span::styled("<Enter> ", Style::default().fg(Color::Yellow)),
                Span::raw("Confirm  "),
                Span::styled("<Esc> ", Style::default().fg(Color::Yellow)),
                Span::raw("Cancel "),
            ])
        } else {
            Line::from(vec![
                Span::styled(" <Enter> ", Style::default().fg(Color::Yellow)),
                Span::raw("Toggle  "),
                Span::styled("<Esc> ", Style::default().fg(Color::Yellow)),
                Span::raw("Apply "),
            ])
        }
    }
}
