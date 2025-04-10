use std::path::PathBuf;

use crate::{
    app::{actions::Action, config::AppConfig, key_bindings, AppContext, AppState},
    component::Component,
    file_handling::{parent_dir_entry, Explorer, FilteredEntries},
    models::Scrollable,
    tui::Event,
    ui::{get_main_layout, Theme, HIGHLIGHT_SYMBOL},
    utils,
};
use anyhow::Result;
use async_trait::async_trait;
use ratatui::{prelude::*, widgets::*};

#[derive(Debug)]
/// The [`ExplorerWidget`] struct represents a terminal based file explorer widget,<br>
/// that can be used to navigate through the filesystem.
pub struct ExplorerWidget {
    /// default is `Explorer`
    app_context: AppContext,
    /// File-Explorer instance that contains the logic for traversing files and folders
    explorer: Explorer,
    /// Default App-Theme is `Dark`
    theme: Theme,
    /// Flag to control the available draw area for the [`ExplorerWidget`]
    /// If the [`crate::ui::info_widget::SystemOverview`] is not visible, than use the whole draw area
    use_whole_draw_area: bool,
    /// Action sender that can send actions to all other components
    action_sender: Option<tokio::sync::mpsc::UnboundedSender<Action>>,
    /// Associated Explorer operation sender, that can send actions to the [`Explorer`]
    explorer_action_sender: Option<tokio::sync::mpsc::Sender<Action>>,
    /// Terminal height used to control the number of items to display on the screen
    terminal_height: u16,
    /// Page height used to control the PageUp and PageDown operations
    page_height: u16,
    /// Filtered entries after searching for a item by it's initial letter
    filtered_entries: FilteredEntries,
    /// Flag to control the receiving of the key events for the explorer widget
    /// If the widget is working, then incoming key events are ignored
    is_working: bool,
    /// Indicates if the Metadata PopUp widget is showing, if it is the case the `ExplorerWidget` still drawn
    is_metadata_pop_up: bool,
    list_state: ListState,
    follow_sym_links: bool,
}

impl ExplorerWidget {
    pub fn new(p: PathBuf, follow_sym_links: bool) -> Self {
        Self {
            app_context: Default::default(),
            explorer: Explorer::load_directory(p, follow_sym_links),
            theme: Default::default(),
            use_whole_draw_area: Default::default(),
            action_sender: Default::default(),
            explorer_action_sender: Default::default(),
            terminal_height: Default::default(),
            page_height: Default::default(),
            filtered_entries: Default::default(),
            is_working: Default::default(),
            is_metadata_pop_up: Default::default(),
            list_state: Default::default(),
            follow_sym_links,
        }
    }
    /// Helper function to send a [`Action`] to the [`Explorer`]
    /// Set the `is_working` flag to true
    async fn send_explorer_action(&mut self, action: Action) -> Result<()> {
        if let Some(sender) = &self.explorer_action_sender {
            self.is_working = true;
            sender.send(action).await?;
        }
        Ok(())
    }
    /// Helper function to send a [`Action`] to all components
    fn send_app_action(&self, action: Action) -> Result<()> {
        if let Some(handler) = &self.action_sender {
            handler.send(action)?
        }
        Ok(())
    }

    fn get_entries_by_initial_letter(&mut self, c: char) -> AppState {
        if self.filtered_entries.matches_letter(c) {
            // If the letter matches, find the next entry
            if let Some(&index) = self.filtered_entries.find_next(self.explorer.selected()) {
                self.explorer.go_to_index(index);
                self.list_state.select(self.explorer.selected().into());
            }
        } else {
            // Find new matches
            match self.explorer.find_entries_with_initial(c) {
                Some(match_result) => {
                    self.filtered_entries = match_result;
                    if let Some(&index) = self.filtered_entries.find_next(self.explorer.selected())
                    {
                        self.explorer.go_to_index(index);
                        self.list_state.select(self.explorer.selected().into());
                    }
                }
                None => return AppState::Failure("No matches found".to_string()),
            }
        }

        // Construct message only once
        let msg = format!(
            "Match {}/{}",
            self.filtered_entries.user_hint_pos(),
            self.filtered_entries.total_entries()
        );

        AppState::Done(msg)
    }
}

#[async_trait(?Send)]
impl Component for ExplorerWidget {
    fn init_area(&mut self, area: Rect) -> Result<()> {
        self.page_height = area.height;
        self.list_state.select(Some(0));
        Ok(())
    }

    fn init_terminal_size(&mut self, terminal_size: Size) -> Result<()> {
        self.terminal_height = terminal_size.height;
        self.explorer.set_terminal_height(self.terminal_height);
        Ok(())
    }

    fn register_component_action_sender(
        &mut self,
        tx: tokio::sync::mpsc::UnboundedSender<Action>,
    ) -> Result<()> {
        self.action_sender = Some(tx);
        Ok(())
    }

    fn register_explorer_action_sender(
        &mut self,
        tx: tokio::sync::mpsc::Sender<Action>,
    ) -> Result<()> {
        self.explorer_action_sender = Some(tx);
        Ok(())
    }

    fn register_config_handler(&mut self, config: AppConfig) -> Result<()> {
        self.theme = config.theme();
        Ok(())
    }

    fn should_handle_events(&self) -> bool {
        self.app_context == AppContext::Explorer && !self.is_working && !self.is_metadata_pop_up
    }

    fn should_render(&self) -> bool {
        self.app_context == AppContext::Explorer
    }

    async fn handle_events(&mut self, event: Option<crate::tui::Event>) -> Result<Option<Action>> {
        if let Some(event) = event {
            match event {
                Event::Key(key_event) => {
                    if self.should_handle_events() {
                        let cmd_desc =
                            key_bindings::get_command_description(&key_event, &self.app_context)
                                .to_owned();
                        self.send_app_action(Action::SetCommandDescription(cmd_desc))?;
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
            // Up arrow key -> move one file or folder up -> we cycle back to the end when we reach the beginning
            crossterm::event::KeyCode::Up
                if key.modifiers == crossterm::event::KeyModifiers::NONE =>
            {
                self.explorer.scroll_up();
                self.list_state.select(self.explorer.selected().into());
                Ok(None)
            }
            // Down arrow key -> move one file or folder down -> we cycle back to the beginning when we reach the end
            crossterm::event::KeyCode::Down
                if key.modifiers == crossterm::event::KeyModifiers::NONE =>
            {
                self.explorer.scroll_down();
                self.list_state.select(self.explorer.selected().into());
                Ok(None)
            }
            crossterm::event::KeyCode::PageUp
                if key.modifiers == crossterm::event::KeyModifiers::NONE =>
            {
                self.send_app_action(Action::UpdateAppState(AppState::done_empty()))?;
                if self.explorer.selected() == 0 {
                    return Ok(Action::UpdateAppState(AppState::Done(
                        "First item reached".to_string(),
                    ))
                    .into());
                }
                self.explorer.page_up_by(self.page_height);
                self.list_state.select(self.explorer.selected().into());
                Ok(None)
            }
            crossterm::event::KeyCode::PageDown
                if key.modifiers == crossterm::event::KeyModifiers::NONE =>
            {
                self.send_app_action(Action::UpdateAppState(AppState::done_empty()))?;
                if self.explorer.selected() >= self.explorer.items().len().saturating_sub(1) {
                    return Ok(Action::UpdateAppState(AppState::Done(
                        "Last item reached".to_string(),
                    ))
                    .into());
                }
                self.explorer.page_down_by(self.page_height);
                self.list_state.select(self.explorer.selected().into());
                Ok(None)
            }
            // Refresh the CWD
            crossterm::event::KeyCode::F(5) => {
                if self.explorer.cwd().is_dir() {
                    self.send_explorer_action(Action::LoadDir(
                        self.explorer.cwd().clone(),
                        self.follow_sym_links,
                    ))
                    .await?;
                } else {
                    return Ok(Action::UpdateAppState(AppState::Failure(
                        "The current directory no longer exists".to_string(),
                    ))
                    .into());
                }
                Ok(None)
            }
            // Enter key -> Go into a directory, if any
            crossterm::event::KeyCode::Enter => {
                let selected_entry = &self.explorer.items()[self.explorer.selected()];

                if !selected_entry.path.is_file() {
                    if selected_entry.path.is_dir() {
                        let new_dir = selected_entry.path.clone();

                        // send the explorer operation to change the directory
                        self.send_explorer_action(Action::LoadDir(new_dir, self.follow_sym_links))
                            .await?;
                    } else {
                        return Ok(Action::UpdateAppState(AppState::Failure(
                            "The selected directory no longer exists".to_string(),
                        ))
                        .into());
                    }
                }

                Ok(None)
            }
            // Backspace key -> Go to parent directory, if any
            crossterm::event::KeyCode::Backspace => {
                // check if the current working directory has a parent directory
                match self.explorer.cwd().parent() {
                    Some(parent_dir) => {
                        self.send_explorer_action(Action::LoadDir(
                            parent_dir.to_path_buf(),
                            self.follow_sym_links,
                        ))
                        .await?;
                    }
                    None => {
                        self.send_app_action(Action::UpdateAppState(AppState::Failure(
                            "No parent directory available".to_string(),
                        )))?;
                    }
                }

                Ok(None)
            }
            // Ctrl + c -> Copy absolute path to clipboard
            crossterm::event::KeyCode::Char('c')
                if key.modifiers == crossterm::event::KeyModifiers::CONTROL =>
            {
                let selected_entry = &self.explorer.items()[self.explorer.selected()];

                if !selected_entry.name.starts_with(&parent_dir_entry()) {
                    let path_to_copy = utils::absolute_path_as_string(&selected_entry.path);
                    match utils::copy_to_clipboard(&path_to_copy) {
                        Ok(_) => {
                            return Ok(Some(Action::UpdateAppState(AppState::Done(
                                "Done".to_string(),
                            ))));
                        }
                        Err(err) => {
                            log::error!("{:?}", err);
                            return Ok(Some(Action::UpdateAppState(AppState::Failure(
                                "Failed to copy path to clipboard".to_string(),
                            ))));
                        }
                    }
                }

                Ok(None)
            }
            // Ctrl + f -> Open search page
            crossterm::event::KeyCode::Char('f')
                if key.modifiers == crossterm::event::KeyModifiers::CONTROL =>
            {
                if self.explorer.cwd().is_dir() {
                    self.send_app_action(Action::SwitchAppContext(AppContext::Search))?;
                    return Ok(Action::ShowSearchPage(self.explorer.cwd().clone()).into());
                } else {
                    return Ok(Action::UpdateAppState(AppState::Failure(
                        "The current directory no longer exists".to_string(),
                    ))
                    .into());
                }
            }
            // Ctrl + u -> Go to home directory
            crossterm::event::KeyCode::Char('u')
                if key.modifiers == crossterm::event::KeyModifiers::CONTROL =>
            {
                // try to get the user's home directory
                match utils::user_home_dir() {
                    Some(home_dir) => {
                        // switch to home dir, only if the CWD is not the home dir
                        if *self.explorer.cwd() != home_dir {
                            self.send_explorer_action(Action::LoadDir(
                                home_dir,
                                self.follow_sym_links,
                            ))
                            .await?;
                        } else {
                            self.send_app_action(Action::UpdateAppState(AppState::Done(
                                "Already in home directory".to_string(),
                            )))?;
                        }
                    }
                    None => {
                        self.send_app_action(Action::UpdateAppState(AppState::Failure(
                            "Unable to determine home dir".to_string(),
                        )))?;
                    }
                }

                Ok(None)
            }
            // Ctrl + a -> Display metadata for the selected object, if any
            crossterm::event::KeyCode::Char('a')
                if key.modifiers == crossterm::event::KeyModifiers::CONTROL =>
            {
                let selected_entry = &self.explorer.items()[self.explorer.selected()];

                // IMPORTANT: exclude the entry to go to the parent directory [e.g. ..\, ../]
                if !selected_entry.name.starts_with(&parent_dir_entry()) {
                    if !selected_entry.path.exists() {
                        return Ok(Some(Action::UpdateAppState(AppState::Failure(
                            "The selected object no longer exists".to_string(),
                        ))));
                    } else if selected_entry.path.is_file() {
                        match selected_entry.file_metadata.as_ref() {
                            Some(metadata) => {
                                self.is_metadata_pop_up = true;
                                return Ok(Action::ShowFileMetadata(
                                    selected_entry.path.clone(),
                                    metadata.to_owned(),
                                )
                                .into());
                            }
                            None => {
                                return Ok(Action::UpdateAppState(AppState::Failure(
                                    "No metadata available".to_string(),
                                ))
                                .into());
                            }
                        }
                    } else if selected_entry.path.is_dir() {
                        // send the explorer operation to change the directory
                        self.send_explorer_action(Action::LoadDirMetadata(
                            selected_entry.name.clone(),
                            selected_entry.path.clone(),
                            self.follow_sym_links,
                        ))
                        .await?;
                    }
                }

                Ok(None)
            }
            crossterm::event::KeyCode::Char(c)
                if key.modifiers == crossterm::event::KeyModifiers::NONE
                    || key.modifiers == crossterm::event::KeyModifiers::SHIFT =>
            {
                let result = self.get_entries_by_initial_letter(c);

                Ok(Action::UpdateAppState(result).into())
            }
            crossterm::event::KeyCode::F(1)
                if key.modifiers == crossterm::event::KeyModifiers::NONE =>
            {
                self.app_context = AppContext::NotActive;
                Ok(Action::ShowHelp(AppContext::Explorer).into())
            }
            crossterm::event::KeyCode::F(2)
                if key.modifiers == crossterm::event::KeyModifiers::NONE =>
            {
                self.app_context = AppContext::NotActive;
                Ok(Action::ShowAbout(AppContext::Explorer).into())
            }
            crossterm::event::KeyCode::Char('o')
                if key.modifiers == crossterm::event::KeyModifiers::CONTROL =>
            {
                Ok(Action::HideOrShowSystemOverview.into())
            }
            crossterm::event::KeyCode::Char('t')
                if key.modifiers == crossterm::event::KeyModifiers::CONTROL =>
            {
                self.theme = self.theme.toggle_theme();
                return Ok(Action::ToggleTheme(self.theme).into());
            }
            _ => Ok(None),
        }
    }

    async fn update(&mut self, action: &Action) -> Result<Option<Action>> {
        match action {
            Action::SwitchAppContext(context) => {
                self.app_context = *context;
            }
            Action::LoadDirDone(explorer) => {
                self.is_working = false;
                self.explorer = explorer.clone();
                self.filtered_entries.reset();
                self.list_state.select(self.explorer.selected().into());
                self.explorer.set_terminal_height(self.terminal_height);
                self.send_app_action(Action::UpdateAppState(AppState::Done("Done".to_string())))?;
            }
            Action::LoadDirMetadataDone(metadata) => {
                self.is_working = false;
                match metadata {
                    Some(metadata) => {
                        self.is_metadata_pop_up = true;
                        return Ok(Action::ShowDirMetadata(metadata.clone()).into());
                    }
                    None => {
                        self.send_app_action(Action::UpdateAppState(AppState::Failure(
                            "No metadata available".to_string(),
                        )))?;
                    }
                }
            }
            Action::CloseMetadata => self.is_metadata_pop_up = false,
            Action::Resize(_, h) => {
                // Clear possible match entries results
                self.filtered_entries.reset();

                // update the terminal height
                self.terminal_height = *h;
                self.explorer.set_terminal_height(self.terminal_height);
                // reset the start index and selected index to ensure that the selected object is no longer in the field of view
                self.explorer.reset_state();
                self.list_state.select(self.explorer.selected().into());

                if self.app_context == AppContext::Explorer {
                    // clear the explorer state
                    self.send_app_action(Action::UpdateAppState(AppState::done_empty()))?;
                }
            }
            Action::ToggleTheme(theme) => {
                self.theme = *theme;
            }
            Action::HideOrShowSystemOverview => {
                self.use_whole_draw_area = !self.use_whole_draw_area;
            }
            _ => {}
        }
        Ok(None)
    }

    fn render(&mut self, f: &mut ratatui::Frame<'_>, area: Rect) -> Result<()> {
        if self.should_render() {
            // Control the draw area dependent if the InfoWidget is showing or not
            let draw_area = if self.use_whole_draw_area {
                let overview_area = get_main_layout(area).overview_area;
                overview_area.union(get_main_layout(area).main_area)
            } else {
                get_main_layout(area).main_area
            };
            self.page_height = draw_area.height;

            let [spacer_area, draw_area] =
                Layout::vertical([Constraint::Length(1), Constraint::Fill(1)]).areas(draw_area);

            // self.send_app_action(Action::UpdateAppState(AppState::Working(
            //     format!(
            //         "Selected: {}, Match Result - Pos: {}, Entries: Count: {}",
            //         self.explorer.selected(),
            //         self.filtered_entries.user_hint_pos(),
            //         self.filtered_entries.total_entries(),
            //     ),
            // )))?;

            let theme_colors = self.theme.theme_colors();

            let block_title_top = format!(" Cwd: [{}] ", self.explorer.cwd_display_name());

            let block_title_bottom = format!(
                " Dirs: {} - Files: {} ",
                self.explorer.dir_counter(),
                self.explorer.file_counter(),
            );

            let list = List::new(
                self.explorer
                    .get_content_to_draw()
                    .iter()
                    .map(|file_entry| {
                        let item_color = if file_entry.path.is_dir() {
                            theme_colors.dir_color
                        } else {
                            theme_colors.file_color
                        };
                        Text::from(file_entry.name.clone()).fg(item_color)
                    }),
            )
            .highlight_spacing(HighlightSpacing::Always)
            .highlight_style(Style::new().fg(theme_colors.alt_fg))
            .highlight_symbol(HIGHLIGHT_SYMBOL)
            .block(
                Block::default()
                    .title_top(
                        Line::from(block_title_top)
                            .style(Style::new().fg(theme_colors.alt_fg))
                            .left_aligned(),
                    )
                    .title_bottom(
                        Line::from(block_title_bottom).style(Style::new().fg(theme_colors.alt_fg)),
                    )
                    .title_alignment(Alignment::Center)
                    .borders(Borders::TOP | Borders::BOTTOM)
                    .border_type(BorderType::QuadrantInside)
                    .border_style(Style::new().fg(theme_colors.alt_bg))
                    .style(Style::new().bg(theme_colors.alt_bg))
                    .padding(Padding {
                        left: 0,
                        right: 0,
                        top: 1,
                        bottom: 0,
                    }),
            );

            f.render_widget(Line::from(" ").bg(theme_colors.alt_bg), spacer_area);
            f.render_stateful_widget(list, draw_area, &mut self.list_state);
        }

        Ok(())
    }
}
