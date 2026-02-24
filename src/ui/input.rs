use std::path::{Path, PathBuf};

use anyhow::Result;
use crossterm::event::KeyModifiers;
use ratatui::{prelude::*, widgets::*};

use crate::{app::actions::Action, ui::centered_rect_fixed_height, utils};

/// A reusable, standalone struct that handles text input logic.
/// It supports cursor movement, character insertion/deletion and clipboard paste.
#[derive(Debug, Default)]
pub struct TextInput {
    /// Current value of the input
    value: String,
    /// Position of cursor in the editor area
    character_index: usize,
    /// To control how many characters the input field can hold
    input_field_width: u16,
}

impl TextInput {
    /// Returns the current input value
    pub fn value(&self) -> &str {
        &self.value
    }

    /// Clears the current input and resets the cursor
    pub fn _clear(&mut self) {
        self.value.clear();
        self.reset_cursor();
    }

    /// Returns `true` if the current input is not empty or blank
    pub fn is_empty(&self) -> bool {
        self.value.trim().is_empty()
    }

    /// Sets the width of the input field.
    /// Should be called every render cycle with the actual available width.
    fn set_width(&mut self, width: u16) {
        self.input_field_width = width;
    }

    /// Returns the current cursor index
    fn _cursor_index(&self) -> usize {
        self.character_index
    }

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    fn move_cursor_left(&mut self) {
        let new_pos = self.character_index.saturating_sub(1);
        self.character_index = self.clamp_cursor(new_pos);
    }

    fn move_cursor_right(&mut self) {
        let new_pos = self.character_index.saturating_add(1);
        self.character_index = self.clamp_cursor(new_pos);
    }

    fn enter_string(&mut self, v: &str) {
        v.chars()
            .filter(|c| !c.is_whitespace())
            .for_each(|c| self.enter_char(c));
    }

    fn enter_char(&mut self, new_char: char) {
        // Only insert if we still have room in the visible input field
        if self.input_field_width > 2
            && self.character_index <= (self.input_field_width - 3) as usize
            && !new_char.is_whitespace()
        {
            let byte_idx = self.byte_index();
            self.value.insert(byte_idx, new_char);
            self.move_cursor_right();
        }
    }

    /// Returns the byte index for the current character cursor position.
    fn byte_index(&self) -> usize {
        self.value
            .char_indices()
            .map(|(i, _)| i)
            .nth(self.character_index)
            .unwrap_or(self.value.len())
    }

    fn delete_char(&mut self, key_code: crossterm::event::KeyCode) {
        match key_code {
            // DELETE  →  remove character to the right of the cursor
            crossterm::event::KeyCode::Delete => {
                if self.character_index < self.value.len() {
                    let before = self.value.chars().take(self.character_index);
                    let after = self.value.chars().skip(self.character_index + 1);
                    self.value = before.chain(after).collect();
                }
            }
            // BACKSPACE  →  remove character to the left of the cursor
            crossterm::event::KeyCode::Backspace => {
                if self.character_index != 0 {
                    let before = self.value.chars().take(self.character_index - 1);
                    let after = self.value.chars().skip(self.character_index);
                    self.value = before.chain(after).collect();
                    self.move_cursor_left();
                }
            }
            _ => {}
        }
    }

    fn clamp_cursor(&self, pos: usize) -> usize {
        pos.clamp(0, self.value.chars().count())
    }

    fn reset_cursor(&mut self) {
        self.character_index = 0;
    }

    fn handle_paste(&mut self) -> Result<()> {
        match utils::paste_from_clipboard() {
            Ok(content) => {
                if content.trim().is_empty() {
                    return Err(anyhow::anyhow!("Nothing to paste from clipboard"));
                }
                self.enter_string(&content);
            }
            Err(e) => {
                log::error!("Clipboard paste failed: {:?}", e);
                return Err(e);
            }
        }
        Ok(())
    }
}

#[derive(Debug, Default)]
pub struct SearchInput {
    /// History of the inputs
    history: Vec<String>,
    /// Current index in the history
    history_index: Option<usize>,
    /// Handles the users input
    pub text_input: TextInput,
}

impl SearchInput {
    /// Navigate backwards (older) through the history
    fn history_backward(&mut self) {
        if self.history.is_empty() {
            return;
        }
        self.text_input.value.clear();
        self.text_input.reset_cursor();

        self.history_index = Some(match self.history_index {
            Some(i) if i > 0 => i - 1,
            // wrap around to the most recent entry
            _ => self.history.len() - 1,
        });

        let entry = self.history[self.history_index.unwrap()].clone();
        entry.chars().for_each(|c| self.text_input.enter_char(c));
    }

    /// Navigate forwards (newer) through the history
    fn history_forward(&mut self) {
        if self.history.is_empty() {
            return;
        }
        self.text_input.value.clear();
        self.text_input.reset_cursor();

        self.history_index = Some(match self.history_index {
            Some(i) if i < self.history.len() - 1 => i + 1,
            // wrap around to the oldest entry
            _ => 0,
        });

        let entry = self.history[self.history_index.unwrap()].clone();
        entry.chars().for_each(|c| self.text_input.enter_char(c));
    }

    /// Saves the current input into the history (if not already present)
    pub fn submit(&mut self) {
        if !self.text_input.value.trim().is_empty()
            && !self.history.contains(&self.text_input.value)
        {
            self.history.push(self.text_input.value.clone());
        }
        self.history_index = None;
    }

    pub async fn handle_key_events(&mut self, key: crossterm::event::KeyEvent) -> Result<()> {
        match key.code {
            crossterm::event::KeyCode::Char(c) => {
                match key.modifiers {
                    // Ctrl + V  →  paste from clipboard
                    KeyModifiers::CONTROL if c.eq_ignore_ascii_case(&'v') => {
                        self.text_input.handle_paste()?;
                    }

                    // Allow printable characters with NONE / SHIFT / ALT / CTRL+ALT
                    modifiers
                        if modifiers.contains(KeyModifiers::SHIFT)
                            || modifiers.contains(KeyModifiers::ALT)
                            || modifiers.contains(KeyModifiers::CONTROL | KeyModifiers::ALT)
                            || modifiers.is_empty() =>
                    {
                        if !c.is_whitespace() {
                            self.text_input.enter_char(c);
                        }
                    }

                    // Ignore everything else
                    _ => return Ok(()),
                }
            }

            crossterm::event::KeyCode::Backspace => self.text_input.delete_char(key.code),
            crossterm::event::KeyCode::Delete => self.text_input.delete_char(key.code),
            crossterm::event::KeyCode::Left => self.text_input.move_cursor_left(),
            crossterm::event::KeyCode::Right => self.text_input.move_cursor_right(),
            crossterm::event::KeyCode::Up => self.history_backward(),
            crossterm::event::KeyCode::Down => self.history_forward(),

            _ => return Ok(()),
        }

        Ok(())
    }

    /// Renders the input field content directly into the given area without drawing a surrounding block.
    ///
    /// This method is intended for use cases where the caller manages the block/border rendering
    /// itself and only needs the raw text and cursor to be drawn. The available width is
    /// automatically derived from `area`, so no manual call to [`set_width`] is required.
    ///
    /// # Arguments
    /// * `f` - The ratatui frame to render into
    /// * `area` - The area to render the input text into
    /// * `bg` - Background color for the input text
    /// * `text_fg` - Foreground color for the input text
    /// * `show_cursor` - Whether to display the cursor at the current character position
    pub fn render(
        &mut self,
        f: &mut ratatui::Frame<'_>,
        area: Rect,
        bg: Color,
        text_fg: Color,
        show_cursor: bool,
    ) {
        // Store the actual available width so enter_char can enforce the limit
        self.text_input.set_width(area.width); // <- derived from provided draw area

        let paragraph =
            Paragraph::new(self.text_input.value()).style(Style::new().bg(bg).fg(text_fg));

        let text_area = Rect {
            x: area.x + 1,
            width: area.width.saturating_sub(1),
            ..area
        };

        f.render_widget(paragraph, text_area);

        if show_cursor {
            f.set_cursor_position(Position::new(
                area.x + self.text_input.character_index as u16 + 1,
                area.y,
            ));
        }
    }
}

#[derive(Debug, Default)]
pub struct SettingsInput {
    title: String,
    text_input: TextInput,
    /// Indicates whether the directory path is valid or not
    is_valid_path: bool,
}

impl SettingsInput {
    pub fn new(title: &str) -> Self {
        Self {
            title: title.to_string(),
            text_input: TextInput::default(),
            is_valid_path: true,
        }
    }

    /// Sets the input value to the given path string.
    /// The path is expanded and resolved to an absolute path before being set as the input value.
    pub fn with_value<P: AsRef<Path>>(mut self, v: P) -> Self {
        self.text_input.set_width(u16::MAX); // temporarily set to max to allow entering the full string without truncation
        self.text_input
            .enter_string(&utils::absolute_path_as_string(v));
        self
    }

    pub fn value(&self) -> &str {
        self.text_input.value()
    }

    pub async fn handle_key_events(
        &mut self,
        key: crossterm::event::KeyEvent,
    ) -> Result<Option<Action>> {
        match key.code {
            crossterm::event::KeyCode::Char(c) => {
                match key.modifiers {
                    // Ctrl + V  →  paste from clipboard
                    KeyModifiers::CONTROL if c.eq_ignore_ascii_case(&'v') => {
                        self.is_valid_path = true; // Reset the valid path state on new input, to hide error message while validating the new path
                        self.text_input.handle_paste()?;
                    }

                    // Allow printable characters with NONE / SHIFT / ALT / CTRL+ALT
                    modifiers
                        if modifiers.contains(KeyModifiers::SHIFT)
                            || modifiers.contains(KeyModifiers::ALT)
                            || modifiers.contains(KeyModifiers::CONTROL | KeyModifiers::ALT)
                            || modifiers.is_empty() =>
                    {
                        if !c.is_whitespace() {
                            self.is_valid_path = true; // Reset the valid path state on new input, to hide error message while validating the new path
                            self.text_input.enter_char(c);
                        }
                    }

                    // Ignore everything else
                    _ => return Ok(None),
                }
            }
            crossterm::event::KeyCode::Enter => {
                self.is_valid_path = self.is_valid_input();

                if self.is_valid_path {
                    return Ok(Some(Action::ApplySettingsInput));
                }
            }
            crossterm::event::KeyCode::Esc => return Ok(Some(Action::SettingsInputCanceled)),
            crossterm::event::KeyCode::Backspace => self.text_input.delete_char(key.code),
            crossterm::event::KeyCode::Delete => self.text_input.delete_char(key.code),
            crossterm::event::KeyCode::Left => self.text_input.move_cursor_left(),
            crossterm::event::KeyCode::Right => self.text_input.move_cursor_right(),

            _ => return Ok(None),
        }

        Ok(None)
    }

    /// Renders the input widget inside the given `area` with a surrounding block.
    ///
    /// # Parameters
    /// - `f`            – the ratatui frame
    /// - `area`         – the area to render into
    /// - `show_cursor`  – whether the terminal cursor should be placed
    pub fn render(&mut self, f: &mut ratatui::Frame<'_>, area: Rect, show_cursor: bool) {
        let block = Block::default()
            .title_top(format!(" {} ", self.title))
            .title_bottom(SettingsInput::help_text())
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::new().bold().fg(Color::LightGreen))
            .style(Style::new().bg(Color::default()));

        let centered_area = centered_rect_fixed_height(65, 5, area);

        // inner layout: one spacer line + one text line
        let [spacer_area_top, input_area, spacer_area_bottom] = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .areas(block.inner(centered_area));

        // Store the actual available width so enter_char can enforce the limit
        self.text_input.set_width(input_area.width);

        f.render_widget(Clear, centered_area);
        f.render_widget(block, centered_area);
        f.render_widget(Line::from(" ").bg(Color::default()), spacer_area_top);

        let paragraph = Paragraph::new(self.text_input.value())
            .style(Style::new().bg(Color::default()).fg(Color::White))
            .block(Block::default().padding(Padding {
                left: 1,
                right: 0,
                top: 0,
                bottom: 0,
            }));

        f.render_widget(paragraph, input_area);

        if self.is_valid_path {
            f.render_widget(Line::from(" ").bg(Color::default()), spacer_area_bottom);
        } else {
            f.render_widget(
                Line::from(" Invalid path - No such directory ")
                    .fg(Color::Red)
                    .bg(Color::default()),
                spacer_area_bottom,
            );
        }

        if show_cursor {
            f.set_cursor_position(Position::new(
                input_area.x + self.text_input.character_index as u16 + 1,
                input_area.y,
            ));
        }
    }

    /// Validates the current input value as a directory path. If the path starts with '~',
    /// it is expanded to the user's home directory before validation.
    /// Returns `true` if the expanded path is a valid directory, otherwise returns `false`.
    fn is_valid_input(&self) -> bool {
        let expanded_path = utils::expand_and_resolve_path(self.text_input.value());
        PathBuf::from(expanded_path).is_dir()
    }

    fn help_text() -> ratatui::prelude::Line<'static> {
        Line::from(vec![
            Span::styled(" <Enter> ", Style::default().fg(Color::Yellow)),
            Span::raw("Apply  "),
            Span::styled("<Esc> ", Style::default().fg(Color::Yellow)),
            Span::raw("Cancel "),
        ])
    }
}
