use ratatui::{prelude::*, style::palette::tailwind};
use serde::{Deserialize, Serialize};

pub mod about_widget;
pub mod explorer_widget;
pub mod footer_widget;
pub mod help_widget;
pub mod info_widget;
pub mod metadata_widget;
pub mod result_widget;
pub mod search_widget;
pub mod title_widget;

pub const PALETTES: [tailwind::Palette; 4] = [
    tailwind::BLUE,
    tailwind::EMERALD,
    tailwind::INDIGO,
    tailwind::RED,
];

#[cfg(not(target_os = "windows"))]
pub const HIGHLIGHT_SYMBOL: &str = " ➤  ";

#[cfg(target_os = "windows")]
pub const HIGHLIGHT_SYMBOL: &str = " →  ";

#[derive(Debug, Default, Clone)]
pub struct ThemeColor {
    pub main_bg: Color,
    pub alt_bg: Color,
    pub main_fg: Color,
    pub main_text_fg: Color,
    pub alt_fg: Color,
    pub file_color: Color,
    pub dir_color: Color,
    pub header_bg: Color,
    pub header_fg: Color,
    pub normal_row_color: Color,
    pub alt_row_color: Color,
    pub search_highlight_color: Color,
    pub selected_color: Color,
    pub done_state_color: Color,
    pub failure_state_color: Color,
    pub working_state_color: Color,
}

#[derive(Debug, Default, Clone, Copy, Eq, PartialEq, Hash)]
pub struct MainLayout {
    /// Associated terminal area for the App-Title
    pub title_area: Rect,
    /// Associated terminal area for the App-Overview
    pub overview_area: Rect,
    /// Associated terminal area for the App-Main-Screen
    pub main_area: Rect,
    /// Associated terminal area for the Footer (e.g. user hints)
    pub footer_area: Rect,
}

/// Return the application main layout.<br>
/// The main layout is a vertical layout of the following parts:
/// - Title-Area as [`Rect`]
/// - Overview-Area as [`Rect`]
/// - Main-Area as [`Rect`]
/// - Footer-Area as [`Rect`]
pub fn get_main_layout(area: Rect) -> MainLayout {
    let vertical = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(8),
        Constraint::Fill(1),
        Constraint::Length(1),
    ]);

    let [title_area, overview_area, main_area, footer_area] = vertical.areas(area);

    let overview_area = overview_area.inner(Margin {
        horizontal: 0,
        vertical: 0,
    });

    MainLayout {
        title_area,
        overview_area,
        main_area,
        footer_area,
    }
}

/// helper function to create a centered rect using up certain percentage of the available rect `r`
pub fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::vertical([
        Constraint::Percentage((100 - percent_y) / 2),
        Constraint::Percentage(percent_y),
        Constraint::Percentage((100 - percent_y) / 2),
    ])
    .split(r);

    Layout::horizontal([
        Constraint::Percentage((100 - percent_x) / 2),
        Constraint::Percentage(percent_x),
        Constraint::Percentage((100 - percent_x) / 2),
    ])
    .split(popup_layout[1])[1]
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Theme {
    Light,
    #[default]
    Dark,
    Dracula,
    Indigo,
}

impl std::fmt::Display for Theme {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Theme::Light => write!(f, "Light"),
            Theme::Dark => write!(f, "Dark"),
            Theme::Dracula => write!(f, "Dracula"),
            Theme::Indigo => write!(f, "Indigo"),
        }
    }
}

impl Theme {
    pub fn theme_colors(&self) -> ThemeColor {
        match self {
            Theme::Light => ThemeColor {
                main_bg: tailwind::GRAY.c300,
                alt_bg: tailwind::GRAY.c400,
                main_fg: tailwind::BLUE.c800,
                main_text_fg: Color::Black,
                alt_fg: tailwind::BLACK,
                file_color: tailwind::EMERALD.c700,
                dir_color: tailwind::BLUE.c700,
                header_bg: tailwind::BLUE.c900,
                header_fg: tailwind::SLATE.c200,
                normal_row_color: tailwind::GRAY.c300,
                alt_row_color: tailwind::GRAY.c400,
                search_highlight_color: tailwind::RED.c700,
                selected_color: tailwind::BLACK,
                done_state_color: Color::Blue,
                failure_state_color: tailwind::RED.c700,
                working_state_color: Color::Blue,
            },
            Theme::Dark => ThemeColor {
                main_bg: tailwind::SLATE.c800,
                alt_bg: tailwind::SLATE.c900,
                main_fg: tailwind::SKY.c300,
                main_text_fg: tailwind::GRAY.c300,
                alt_fg: tailwind::GRAY.c200,
                file_color: tailwind::TEAL.c400,
                dir_color: tailwind::INDIGO.c400,
                header_bg: tailwind::BLUE.c900,
                header_fg: tailwind::SLATE.c200,
                normal_row_color: tailwind::SLATE.c800,
                alt_row_color: tailwind::SLATE.c900,
                search_highlight_color: tailwind::GREEN.c500,
                selected_color: tailwind::SKY.c400,
                done_state_color: Color::LightCyan,
                failure_state_color: tailwind::RED.c500,
                working_state_color: Color::LightCyan,
            },
            Theme::Dracula => ThemeColor {
                main_bg: tailwind::SLATE.c900,
                alt_bg: tailwind::BLACK,
                main_fg: tailwind::ORANGE.c500,
                main_text_fg: tailwind::GRAY.c300,
                alt_fg: tailwind::YELLOW.c300,
                file_color: tailwind::ZINC.c300,
                dir_color: tailwind::EMERALD.c400,
                header_bg: tailwind::BLUE.c900,
                header_fg: tailwind::SLATE.c200,
                normal_row_color: tailwind::SLATE.c900,
                alt_row_color: tailwind::BLACK,
                search_highlight_color: tailwind::GREEN.c500,
                selected_color: tailwind::SKY.c400,
                done_state_color: tailwind::CYAN.c300,
                failure_state_color: tailwind::RED.c500,
                working_state_color: tailwind::CYAN.c300,
            },
            Theme::Indigo => ThemeColor {
                main_bg: tailwind::INDIGO.c600,
                alt_bg: tailwind::INDIGO.c900,
                main_fg: tailwind::LIME.c300,
                main_text_fg: tailwind::CYAN.c300,
                alt_fg: tailwind::YELLOW.c400,
                file_color: tailwind::LIME.c400,
                dir_color: tailwind::ORANGE.c400,
                header_bg: tailwind::BLUE.c600,
                header_fg: tailwind::SLATE.c200,
                normal_row_color: tailwind::INDIGO.c800,
                alt_row_color: tailwind::INDIGO.c900,
                search_highlight_color: tailwind::GREEN.c500,
                selected_color: tailwind::GREEN.c400,
                done_state_color: tailwind::WHITE,
                failure_state_color: tailwind::RED.c700,
                working_state_color: tailwind::WHITE,
            },
        }
    }

    /// Get the next available app theme
    pub fn toggle_theme(self) -> Self {
        match self {
            Theme::Dark => Theme::Indigo,
            Theme::Indigo => Theme::Light,
            Theme::Light => Theme::Dracula,
            Theme::Dracula => Theme::Dark,
        }
    }
}

/// Highlights occurrences of a specified substring in a given text while ignoring case sensitivity.
///
/// This function searches for all occurrences of `highlight` in `text`, applying a specified
/// `highlight_color` to the background of the matches while keeping the rest of the text in `default_color`.
///
/// # Arguments
///
/// * `text` - The input string where highlighting is applied.
/// * `highlight` - The substring to search for, case-insensitively.
/// * `highlight_color` - The color applied to highlighted portions.
/// * `default_color` - The color applied to non-highlighted portions.
///
/// # Returns
///
/// A `Vec<Span>` containing the input text divided into segments, with highlighted portions styled accordingly.
pub fn highlight_text_part(
    text: String,
    highlight: &str,
    highlight_color: Color,
    default_color: Color,
) -> Vec<Span<'_>> {
    let mut spans = Vec::new();

    // Prevent infinite loop by returning the full string unchanged if highlight is empty
    if highlight.trim().is_empty() {
        return vec![Span::from(text.clone()).fg(default_color)];
    }

    let lower_text = text.to_lowercase();
    let lower_highlight = highlight.to_lowercase();
    let mut start = 0;

    while let Some(pos) = lower_text[start..].find(&lower_highlight) {
        let pos = start + pos;

        // Push the text before the highlight if it exists
        if pos > start {
            spans.push(Span::from(text[start..pos].to_string()).fg(default_color));
        }

        // Get the text that should be highlighted
        let highlighted_part = &text[pos..pos + highlight.len()];
        spans.push(
            Span::from(highlighted_part.to_string())
                .fg(default_color)
                .bg(highlight_color),
        );

        // Move to the remaining part after the highlighted section
        start = pos + highlight.len();
    }

    // Push the remaining text if there is any left
    if start < text.len() {
        spans.push(Span::from(text[start..].to_string()).fg(default_color));
    }

    spans
}

#[cfg(test)]
mod tests {
    use super::highlight_text_part;
    use ratatui::{
        style::{Color, Stylize},
        text::Span,
    };

    #[test]
    fn test_basic_highlight_1() {
        let filename = "important_document.txt";
        let highlight = "important";
        let result = highlight_text_part(filename.into(), highlight, Color::Cyan, Color::White);
        assert_eq!(
            result,
            vec![
                Span::from("important").white().on_cyan(),
                Span::from("_document.txt").white()
            ]
        )
    }

    #[test]
    fn test_basic_highlight_2() {
        let filename = "test_my_doc.rs";
        let highlight = "my";
        let result = highlight_text_part(filename.into(), highlight, Color::Cyan, Color::White);
        assert_eq!(
            result,
            vec![
                Span::from("test_").white(),
                Span::from("my").white().on_cyan(),
                Span::from("_doc.rs").white()
            ]
        )
    }

    #[test]
    fn test_basic_highlight_3() {
        let filename = "test_rust_file.rs";
        let highlight = ".rs";
        let result = highlight_text_part(filename.into(), highlight, Color::Cyan, Color::White);
        assert_eq!(
            result,
            vec![
                Span::from("test_rust_file").white(),
                Span::from(".rs").white().on_cyan()
            ]
        )
    }

    #[test]
    fn test_multiple_highlight() {
        let filename = "test_my_doc_test.rs";
        let highlight = "test";
        let result = highlight_text_part(filename.into(), highlight, Color::Cyan, Color::White);
        assert_eq!(
            result,
            vec![
                Span::from("test").white().on_cyan(),
                Span::from("_my_doc_").white(),
                Span::from("test").white().on_cyan(),
                Span::from(".rs").white()
            ]
        )
    }

    #[test]
    fn test_empty_highlight_given_1() {
        let filename = "document_important.pdf";
        let highlight = "";
        let result = highlight_text_part(filename.into(), highlight, Color::Cyan, Color::White);
        assert_eq!(result, vec![Span::from("document_important.pdf").white()])
    }

    #[test]
    fn test_empty_highlight_given_2() {
        let filename = "document_important.pdf";
        let highlight = "\t      ";
        let result = highlight_text_part(filename.into(), highlight, Color::Cyan, Color::White);
        assert_eq!(result, vec![Span::from("document_important.pdf").white()])
    }

    #[test]
    fn test_highlight_uppercase() {
        let filename = "document_important.pdf";
        let highlight = "POrTanT";
        let result = highlight_text_part(filename.into(), highlight, Color::Cyan, Color::White);
        assert_eq!(
            result,
            vec![
                Span::from("document_im").white(),
                Span::from("portant").white().on_cyan(),
                Span::from(".pdf").white()
            ]
        )
    }

    #[test]
    fn test_highlight_lowercase() {
        let filename = "DOCUMENT.PDF";
        let highlight = ".pdf";
        let result = highlight_text_part(filename.into(), highlight, Color::Cyan, Color::White);
        assert_eq!(
            result,
            vec![
                Span::from("DOCUMENT").white(),
                Span::from(".PDF").white().on_cyan()
            ]
        )
    }
}
