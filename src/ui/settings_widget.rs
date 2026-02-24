use std::path::Path;

use anyhow::Result;
use async_trait::async_trait;
use ratatui::{prelude::*, style::palette::tailwind, widgets::*};

use crate::{
    app::{AppContext, actions::Action, config::AppConfig},
    component::Component,
    models::{Scrollable, StatefulTable},
    tui::Event,
    ui::{HIGHLIGHT_SYMBOL, PALETTES, Theme, dropdown::Dropdown, input::SettingsInput},
    utils,
};

/// Track the current selected settings type
#[derive(Debug)]
enum SettingsTypes {
    DefaultTheme,
    StartDirectory,
    ExportDirectory,
    FollowSymLinks,
    Fps,
    SystemUpdateRate,
}

/// The dropdown types for the settings page, to choose the right dropdown depending on the selected setting
#[derive(Debug, Default)]
enum DropDownTypes {
    Theme(Dropdown<Theme>),
    SymLinks(Dropdown<String>),
    Fps(Dropdown<u8>),
    SystemUpdateRate(Dropdown<u8>),
    #[default]
    Undefined,
}

impl DropDownTypes {
    fn theme(current: &Theme) -> Self {
        Self::Theme(Dropdown::new(
            vec![Theme::Dark, Theme::Dracula, Theme::Indigo, Theme::Light],
            current,
        ))
    }

    fn sym_links(current: &str) -> Self {
        Self::SymLinks(Dropdown::new(
            vec!["Yes".into(), "No".into()],
            &current.to_string(),
        ))
    }

    fn fps(current: &u8) -> Self {
        Self::Fps(Dropdown::new((30_u8..=60_u8).collect(), current).with_max_visible(8))
    }

    fn system_update_rate(current: &u8) -> Self {
        Self::SystemUpdateRate(Dropdown::new((1_u8..=5_u8).collect(), current))
    }

    async fn handle_key_events(
        &mut self,
        key: crossterm::event::KeyEvent,
    ) -> Result<Option<Action>> {
        match self {
            Self::Theme(d) => d.handle_key_events(key).await,
            Self::SymLinks(d) => d.handle_key_events(key).await,
            Self::Fps(d) => d.handle_key_events(key).await,
            Self::SystemUpdateRate(d) => d.handle_key_events(key).await,
            _ => Ok(None),
        }
    }

    fn render(&mut self, f: &mut Frame, area: Rect) {
        match self {
            Self::Theme(d) => d.render(f, area, "Theme"),
            Self::SymLinks(d) => d.render(f, area, "Follow symbolic links"),
            Self::Fps(d) => d.render(f, area, "Frames per second"),
            Self::SystemUpdateRate(d) => d.render(f, area, "System update rate per second"),
            _ => {}
        }
    }
}

/// The input types for the settings page, to choose the right input depending on the selected setting
#[derive(Debug, Default)]
enum InputTypes {
    StartDir(SettingsInput),
    ExportDir(SettingsInput),
    #[default]
    Undefined,
}

impl InputTypes {
    fn start_dir<P: AsRef<Path>>(current: P) -> Self {
        Self::StartDir(SettingsInput::new("Edit Start-Directory").with_value(current))
    }

    fn export_dir<P: AsRef<Path>>(current: P) -> Self {
        Self::ExportDir(SettingsInput::new("Edit Export-Directory").with_value(current))
    }

    async fn handle_key_events(
        &mut self,
        key: crossterm::event::KeyEvent,
    ) -> Result<Option<Action>> {
        match self {
            Self::StartDir(d) => d.handle_key_events(key).await,
            Self::ExportDir(d) => d.handle_key_events(key).await,
            _ => Ok(None),
        }
    }

    fn render(&mut self, f: &mut Frame, area: Rect) {
        match self {
            Self::StartDir(d) => d.render(f, area, true),
            Self::ExportDir(d) => d.render(f, area, true),
            _ => {}
        }
    }
}

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
pub struct SettingsPage {
    /// Action sender that can send actions to all other components
    action_sender: Option<tokio::sync::mpsc::UnboundedSender<Action>>,
    caller_context: AppContext,
    config: AppConfig,
    dropdown: DropDownTypes,
    settings_input: InputTypes,
    settings: StatefulTable<Vec<String>>,
    settings_types: StatefulTable<SettingsTypes>, // to track the currently selected setting, to show the right dropdown/input for it
    scrollbar_state: ScrollbarState,
    border_style: Style,
    border_type: BorderType,
    title_style: Style,
    colors: TableColors,
    is_active: bool,
    is_dropdown_active: bool,
    is_input_active: bool,
}

impl SettingsPage {
    /// Helper function to get the right edit mode [Dropdown / Input] for the currently selected setting
    fn init_edit_mode(&mut self) -> Result<Option<Action>> {
        let mut action = None;

        if let Some(setting_type) = self.settings_types.current_item() {
            match setting_type {
                SettingsTypes::DefaultTheme => {
                    self.dropdown = DropDownTypes::theme(&self.config.theme());
                    action = Some(Action::DropDownShowing);
                }
                SettingsTypes::StartDirectory => {
                    self.settings_input = InputTypes::start_dir(self.config.start_dir());
                    action = Some(Action::SettingsInputShowing);
                }
                SettingsTypes::ExportDirectory => {
                    self.settings_input = InputTypes::export_dir(self.config.export_dir());
                    action = Some(Action::SettingsInputShowing);
                }
                SettingsTypes::FollowSymLinks => {
                    let current = if self.config.follow_sym_links() {
                        "Yes"
                    } else {
                        "No"
                    };
                    self.dropdown = DropDownTypes::sym_links(current);
                    action = Some(Action::DropDownShowing);
                }
                SettingsTypes::Fps => {
                    self.dropdown = DropDownTypes::fps(&self.config.fps());
                    action = Some(Action::DropDownShowing);
                }
                SettingsTypes::SystemUpdateRate => {
                    self.dropdown =
                        DropDownTypes::system_update_rate(&self.config.system_update_rate());
                    action = Some(Action::DropDownShowing);
                }
            }
        }
        Ok(action)
    }

    /// Helper function to send a [`Action`] to all components
    fn send_app_action(&self, action: Action) -> Result<()> {
        if let Some(handler) = &self.action_sender {
            handler.send(action)?
        }
        Ok(())
    }

    fn block_title() -> ratatui::prelude::Line<'static> {
        Line::from(vec![
            Span::raw(" Settings | "),
            Span::styled("<Esc> ", Style::default().fg(Color::Yellow)),
            Span::raw("Save and Close  "),
            Span::styled("<Enter> ", Style::default().fg(Color::Yellow)),
            Span::raw("Edit  "),
            Span::styled("<↑↓> ", Style::default().fg(Color::Yellow)),
            Span::raw("Select "),
        ])
    }
}

impl Default for SettingsPage {
    fn default() -> Self {
        Self {
            action_sender: Default::default(),
            caller_context: AppContext::NotActive,
            config: Default::default(),
            dropdown: Default::default(),
            settings_input: InputTypes::default(),
            settings: StatefulTable::new(),
            settings_types: StatefulTable::new(),
            scrollbar_state: ScrollbarState::default(),
            border_style: Style::new().bold().fg(Color::LightGreen),
            border_type: BorderType::Rounded,
            title_style: Default::default(),
            colors: TableColors::new(&PALETTES[3]),
            is_active: Default::default(),
            is_dropdown_active: Default::default(),
            is_input_active: Default::default(),
        }
    }
}

#[async_trait(?Send)]
impl Component for SettingsPage {
    fn register_component_action_sender(
        &mut self,
        tx: tokio::sync::mpsc::UnboundedSender<Action>,
    ) -> Result<()> {
        self.action_sender = Some(tx);
        Ok(())
    }

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
                    if self.is_dropdown_active {
                        return self.dropdown.handle_key_events(key_event).await;
                    }
                    if self.is_input_active {
                        return self.settings_input.handle_key_events(key_event).await;
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
                self.settings.scroll_up_by(1);
                self.settings_types.scroll_up_by(1);
                self.scrollbar_state = self.scrollbar_state.position(self.settings.selected_item);

                Ok(None)
            }
            crossterm::event::KeyCode::Down => {
                self.settings.scroll_down_by(1);
                self.settings_types.scroll_down_by(1);
                self.scrollbar_state = self.scrollbar_state.position(self.settings.selected_item);

                Ok(None)
            }
            crossterm::event::KeyCode::Esc => {
                self.is_active = false;
                self.send_app_action(Action::ApplyAppSettings(self.config.clone()))?;
                Ok(Action::SwitchAppContext(self.caller_context).into())
            }
            crossterm::event::KeyCode::Enter => self.init_edit_mode(),
            _ => Ok(None),
        }
    }

    fn should_handle_events(&self) -> bool {
        self.is_active && !self.is_dropdown_active && !self.is_input_active
    }

    fn should_render(&self) -> bool {
        self.is_active
    }

    async fn update(&mut self, action: &Action) -> Result<Option<Action>> {
        match action {
            Action::ShowSettings(caller_context) => {
                self.caller_context = *caller_context;

                self.settings = StatefulTable::with_items(self.config.config_docs(true));
                self.settings.state.select(Some(0));
                self.scrollbar_state = ScrollbarState::new(self.settings.items.len()).position(0);

                self.settings_types = StatefulTable::with_items(vec![
                    SettingsTypes::DefaultTheme,
                    SettingsTypes::StartDirectory,
                    SettingsTypes::ExportDirectory,
                    SettingsTypes::FollowSymLinks,
                    SettingsTypes::Fps,
                    SettingsTypes::SystemUpdateRate,
                ]);
                self.settings_types.state.select(Some(0));

                self.is_active = true;
            }
            Action::DropDownShowing => {
                self.is_dropdown_active = true;
            }
            Action::DropDownClosed => {
                // update the config with the new settings from the dropdown
                match &mut self.dropdown {
                    DropDownTypes::Theme(d) => {
                        self.config.set_theme(*d.selected());
                    }
                    DropDownTypes::SymLinks(d) => {
                        let follow_sym_links = match d.selected().as_str() {
                            "Yes" => true,
                            "No" => false,
                            _ => self.config.follow_sym_links(), // fallback to current value if something unexpected happens
                        };
                        self.config.set_follow_sym_links(follow_sym_links);
                    }
                    DropDownTypes::Fps(d) => {
                        self.config.set_fps(*d.selected());
                    }
                    DropDownTypes::SystemUpdateRate(d) => {
                        self.config.set_system_update_rate(*d.selected());
                    }
                    _ => {}
                }
                // also update the new settings on the settings page
                self.settings.set_items(self.config.config_docs(true));
                self.is_dropdown_active = false;
            }
            Action::SettingsInputShowing => {
                self.is_input_active = true;
            }
            Action::SettingsInputCanceled => {
                self.is_input_active = false;
            }
            Action::ApplySettingsInput => {
                // update the config with the new settings from the input
                match &mut self.settings_input {
                    InputTypes::StartDir(d) => {
                        let new_start_dir = utils::expand_and_resolve_path(d.value());
                        self.config.set_start_dir(new_start_dir);
                    }
                    InputTypes::ExportDir(d) => {
                        let new_export_dir = utils::expand_and_resolve_path(d.value());
                        self.config.set_export_dir(new_export_dir);
                    }
                    _ => {}
                }
                // also update the new settings on the settings page
                self.settings.set_items(self.config.config_docs(true));
                self.is_input_active = false;
            }
            Action::Quit => {
                // send changed settings also if user quit the app <Ctrl+Q>, to save this
                return Ok(Some(Action::ApplyAppSettings(self.config.clone())));
            }
            _ => {}
        }

        Ok(None)
    }

    fn render(&mut self, f: &mut ratatui::Frame<'_>, area: Rect) -> Result<()> {
        if self.should_render() {
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

            let header = ["Name", "Value", "Description"]
                .into_iter()
                .map(Cell::from)
                .collect::<Row>()
                .style(header_style)
                .height(1);

            let rows = self.settings.items.iter().enumerate().map(|(i, data)| {
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
                Constraint::Length(35),
                Constraint::Percentage(35),
                Constraint::Percentage(45),
            ];

            // clear/reset a certain area to allow overdrawing (e.g. for popups).
            f.render_widget(Clear, area);

            let help_page_table = Table::new(rows, table_widths)
                .header(header)
                .block(
                    help_block
                        .title(SettingsPage::block_title())
                        .padding(Padding {
                            left: 0,
                            right: 0,
                            top: 1,
                            bottom: 0,
                        }),
                )
                .highlight_symbol(
                    Text::from(vec!["\n".into(), HIGHLIGHT_SYMBOL.into()])
                        .style(Style::new().fg(self.colors.selected_style_fg)),
                )
                .bg(self.colors.buffer_bg)
                .highlight_spacing(HighlightSpacing::Always);

            let scrollbar_vertical = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("↑"))
                .end_symbol(Some("↓"));

            f.render_stateful_widget(help_page_table, help_block_area, &mut self.settings.state);
            f.render_stateful_widget(
                scrollbar_vertical,
                help_block_area.inner(Margin {
                    // using an inner vertical margin of 1 unit makes the scrollbar inside the current block
                    vertical: 1,
                    horizontal: 0,
                }),
                &mut self.scrollbar_state,
            );

            if self.is_dropdown_active {
                self.dropdown.render(f, area);
            }

            if self.is_input_active {
                self.settings_input.render(f, area);
            }
        }

        Ok(())
    }
}
