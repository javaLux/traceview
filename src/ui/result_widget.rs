use std::io::{BufWriter, Write};
use std::path::PathBuf;

use anyhow::Result;
use async_trait::async_trait;
use ratatui::{prelude::*, widgets::*};
use tokio::{sync::mpsc, task::JoinHandle};
use tokio_util::sync::CancellationToken;

use crate::{
    app::{actions::Action, config::AppConfig, key_bindings, AppContext, AppState},
    component::Component,
    file_handling::SearchResult,
    models::Scrollable,
    tui::Event,
    ui::{
        get_main_layout, highlight_text_part, search_widget::SearchMode, Theme, HIGHLIGHT_SYMBOL,
    },
    utils,
};

/// Represents the Export-Task for exporting search results as JSON
pub struct ExportTask {
    task: JoinHandle<()>,
    cancellation_token: CancellationToken,
}

impl Default for ExportTask {
    fn default() -> Self {
        let cancellation_token = CancellationToken::new();
        let task = tokio::spawn(async {
            std::future::ready(()).await;
        });
        Self {
            task,
            cancellation_token,
        }
    }
}

impl ExportTask {
    pub fn export_as_json(
        &mut self,
        search_query: String,
        mut json_rx: mpsc::Receiver<serde_json::Value>,
        action_sender: mpsc::UnboundedSender<Action>,
        export_dir: PathBuf,
    ) {
        self.cancel();
        self.cancellation_token = CancellationToken::new();
        let cancellation_token = self.cancellation_token.clone();

        let export_path = export_dir.join(format!(
            "search_results_{}.json",
            chrono::Local::now().format("%Y-%m-%dT%H_%M_%S")
        ));

        self.task = tokio::task::spawn(async move {
            let file = match std::fs::File::create(export_path) {
                Ok(file) => file,
                Err(err) => {
                    log::error!("Failed to create export file - Details {:?}", err);
                    let _ = action_sender
                        .send(Action::ExportFailure("Failed to create export file".into()));
                    return;
                }
            };

            let mut writer = BufWriter::new(file);

            // Write the opening JSON structure
            let open_json = format!(
                "{{\n  \"search_query\": \"{}\",\n  \"results\": [\n",
                search_query
            );

            if let Err(err) = writer.write_all(open_json.as_bytes()) {
                log::error!(
                    "Failed to write open JSON string to export file - Details {:?}",
                    err
                );
                let _ = action_sender.send(Action::ExportFailure(
                    "Failed to write to export file".into(),
                ));
                return;
            };

            let mut first = true;

            // Write each JSON entry
            while let Some(entry) = json_rx.recv().await {
                if cancellation_token.is_cancelled() {
                    let _ = action_sender.send(Action::ForcedShutdown);
                    break;
                }
                if !first {
                    if let Err(err) = writer.write_all(b",\n") {
                        log::error!(
                            "Failed to write indentation to export file - Details {:?}",
                            err
                        );
                        let _ = action_sender.send(Action::ExportFailure(
                            "Failed to write to export file".into(),
                        ));
                        return;
                    }
                }
                first = false;

                // Indent each entry with 4 spaces
                if let Err(err) = writer.write_all(b"    ") {
                    log::error!("Failed to write indentation - Details {:?}", err);
                    let _ = action_sender.send(Action::ExportFailure(format!(
                        "Failed to write indentation: {}",
                        err
                    )));
                    return;
                }

                if let Err(err) = serde_json::to_writer(&mut writer, &entry) {
                    log::error!("Failed to search result to export file - Details {:?}", err);
                    let _ = action_sender.send(Action::ExportFailure(
                        "Failed to write to export file".into(),
                    ));
                    return;
                };
            }

            // Write the closing JSON structure
            let close_json = "\n  ]\n}".to_string();
            if let Err(err) = writer.write_all(close_json.as_bytes()) {
                log::error!(
                    "Failed to write closing JSON string to export file - Details {:?}",
                    err
                );
                let _ = action_sender.send(Action::ExportFailure(
                    "Failed to write to export file".into(),
                ));
                return;
            }
            let _ = writer.flush();

            let _ = action_sender.send(Action::ExportDone);
        });
    }

    pub fn cancel(&self) {
        self.cancellation_token.cancel();
    }

    pub fn stop(&mut self) {
        self.cancel();
        let mut counter = 0;

        while !self.task.is_finished() {
            counter += 1;
            std::thread::sleep(std::time::Duration::from_millis(1));
            if counter > 50 {
                self.task.abort();
            }
            if counter >= 500 {
                panic!("Unable to abort Export-Task in 500 milliseconds for unknown reason");
            }
        }
    }
}

pub struct ResultWidget {
    /// The actually context of this widget
    app_context: AppContext,
    /// The context of the previous active widget
    previous_context: AppContext,
    /// Action sender that can send actions to all other components
    action_sender: Option<tokio::sync::mpsc::UnboundedSender<Action>>,
    /// Associated Explorer operation sender, that can send actions to the [`Explorer`]
    explorer_action_sender: Option<tokio::sync::mpsc::UnboundedSender<Action>>,
    /// Flag to control the available draw area for the [`SearchWidget`]
    /// If the [`crate::ui::info_widget::SystemOverview`] is not visible, than use the whole draw area
    use_whole_draw_area: bool,
    search_result: SearchResult,
    /// Terminal height used to control the number of items to display on the screen
    terminal_height: u16,
    /// Page height used to control the PageUp and PageDown operations
    page_height: u16,
    /// Flag to control the receiving of the key events for the search widget
    /// If the widget is working, then incoming key events are ignored
    is_working: bool,
    /// Indicates if the Metadata PopUp widget is showing
    is_metadata_pop_up: bool,
    theme: Theme,
    applied_search_mode: SearchMode,
    table_state: TableState,
    selected_hint: String,
    export_task: ExportTask,
    // Directory in which the search results should be exported
    export_dir: PathBuf,
    follow_sym_links: bool,
}

impl Default for ResultWidget {
    fn default() -> Self {
        Self {
            app_context: AppContext::NotActive,
            previous_context: AppContext::Search,
            action_sender: Default::default(),
            explorer_action_sender: Default::default(),
            use_whole_draw_area: Default::default(),
            search_result: Default::default(),
            terminal_height: Default::default(),
            page_height: Default::default(),
            is_working: Default::default(),
            is_metadata_pop_up: Default::default(),
            theme: Default::default(),
            applied_search_mode: Default::default(),
            table_state: Default::default(),
            selected_hint: Default::default(),
            export_task: Default::default(),
            export_dir: Default::default(),
            follow_sym_links: Default::default(),
        }
    }
}

impl ResultWidget {
    /// Helper function to send a [`Action`] to the [`crate::file_handling::Explorer`]
    /// Set the `is_working` flag to true
    fn send_explorer_action(&mut self, action: Action) -> Result<()> {
        if let Some(handler) = &self.explorer_action_sender {
            self.is_working = true;
            handler.send(action)?
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

    fn build_selected_hint(&mut self) {
        self.selected_hint = format!(
            " {}/{} ",
            self.search_result.selected() + 1,
            self.search_result.items().len()
        );
    }
}

#[async_trait(?Send)]
impl Component for ResultWidget {
    fn init_area(&mut self, area: Rect) -> Result<()> {
        self.page_height = area.height;
        self.table_state.select(Some(0));
        Ok(())
    }

    fn init_terminal_size(&mut self, terminal_size: Size) -> Result<()> {
        self.terminal_height = terminal_size.height;
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
        tx: tokio::sync::mpsc::UnboundedSender<Action>,
    ) -> Result<()> {
        self.explorer_action_sender = Some(tx);
        Ok(())
    }

    fn register_config_handler(&mut self, config: AppConfig) -> Result<()> {
        self.theme = config.theme();
        self.export_dir = config.export_dir();
        self.follow_sym_links = config.follow_sym_links();
        Ok(())
    }

    fn should_handle_events(&self) -> bool {
        self.app_context == AppContext::Results && !self.is_working && !self.is_metadata_pop_up
    }

    fn should_render(&self) -> bool {
        self.app_context == AppContext::Results
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
                self.search_result.scroll_up();
                self.table_state
                    .select(self.search_result.selected().into());
                self.build_selected_hint();
            }
            // Down arrow key -> move one file or folder down -> we cycle back to the beginning when we reach the end
            crossterm::event::KeyCode::Down
                if key.modifiers == crossterm::event::KeyModifiers::NONE =>
            {
                self.search_result.scroll_down();
                self.table_state
                    .select(self.search_result.selected().into());
                self.build_selected_hint();
            }
            crossterm::event::KeyCode::PageUp
                if key.modifiers == crossterm::event::KeyModifiers::NONE =>
            {
                self.send_app_action(Action::UpdateAppState(AppState::done_empty()))?;
                if self.search_result.selected() == 0 {
                    return Ok(Action::UpdateAppState(AppState::Done(
                        "First item reached".to_string(),
                    ))
                    .into());
                }
                self.search_result.page_up_by(self.page_height);
                self.table_state
                    .select(self.search_result.selected().into());
                self.build_selected_hint();
            }
            crossterm::event::KeyCode::PageDown
                if key.modifiers == crossterm::event::KeyModifiers::NONE =>
            {
                self.send_app_action(Action::UpdateAppState(AppState::done_empty()))?;
                if self.search_result.selected()
                    >= self.search_result.items().len().saturating_sub(1)
                {
                    return Ok(Action::UpdateAppState(AppState::Done(
                        "Last item reached".to_string(),
                    ))
                    .into());
                }
                self.search_result.page_down_by(self.page_height);
                self.table_state
                    .select(self.search_result.selected().into());
                self.build_selected_hint();
            }
            // Ctrl + a -> Display metadata for the selected object, if any
            crossterm::event::KeyCode::Char('a')
                if key.modifiers == crossterm::event::KeyModifiers::CONTROL =>
            {
                let selected_entry = &self.search_result.items()[self.search_result.selected()];

                if !selected_entry.path.exists() {
                    return Ok(Some(Action::UpdateAppState(AppState::Failure(
                        "The selected path no longer exists".to_string(),
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
                    self.send_explorer_action(Action::LoadDirMetadata(
                        selected_entry.name.clone(),
                        selected_entry.path.clone(),
                        self.follow_sym_links,
                    ))?;
                }
            }
            // Ctrl + c -> Copy absolute path to clipboard
            crossterm::event::KeyCode::Char('c')
                if key.modifiers == crossterm::event::KeyModifiers::CONTROL =>
            {
                let selected_entry = &self.search_result.items()[self.search_result.selected()];

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
            crossterm::event::KeyCode::Char('o')
                if key.modifiers == crossterm::event::KeyModifiers::CONTROL =>
            {
                return Ok(Action::HideOrShowSystemOverview.into());
            }
            crossterm::event::KeyCode::Char('t')
                if key.modifiers == crossterm::event::KeyModifiers::CONTROL =>
            {
                self.theme = self.theme.toggle_theme();
                return Ok(Action::ToggleTheme(self.theme).into());
            }
            crossterm::event::KeyCode::F(1)
                if key.modifiers == crossterm::event::KeyModifiers::NONE =>
            {
                self.app_context = AppContext::NotActive;
                return Ok(Action::ShowHelp(AppContext::Results).into());
            }
            crossterm::event::KeyCode::F(2)
                if key.modifiers == crossterm::event::KeyModifiers::NONE =>
            {
                self.app_context = AppContext::NotActive;
                return Ok(Action::ShowAbout(AppContext::Results).into());
            }
            // Export search results as JSON
            crossterm::event::KeyCode::F(12)
                if key.modifiers == crossterm::event::KeyModifiers::NONE =>
            {
                self.is_working = true;

                self.send_app_action(Action::UpdateAppState(AppState::Working(
                    "Exporting results...".into(),
                )))?;

                let (tx, rx) = mpsc::channel(100);
                let search_query = self.search_result.search_query().to_string();
                let export_dir = self.export_dir.clone();
                let action_sender = self.action_sender.clone().unwrap();
                let items = self.search_result.items().to_vec();

                self.export_task
                    .export_as_json(search_query, rx, action_sender, export_dir);

                let tx_clone = tx.clone();
                tokio::spawn(async move {
                    for entry in items {
                        let json_value = entry.build_as_json();
                        // Send to writer
                        if tx_clone.send(json_value).await.is_err() {
                            println!("Writer task dropped, stopping producer");
                            break;
                        }
                    }
                });

                // Close the channel to indicate that no more values will be sent
                drop(tx);
            }
            crossterm::event::KeyCode::Esc => {
                self.app_context = AppContext::NotActive;
                self.search_result = SearchResult::default();
                self.table_state
                    .select(self.search_result.selected().into());
                return Ok(Action::SwitchAppContext(self.previous_context).into());
            }
            _ => {}
        }

        Ok(None)
    }

    async fn update(&mut self, action: Action) -> Result<Option<Action>> {
        match action {
            Action::SwitchAppContext(context) => {
                self.app_context = context;
            }
            Action::ShowResultsPage(result, mode) => {
                self.applied_search_mode = mode;
                self.search_result = result;
                self.search_result.set_terminal_height(self.terminal_height);
                self.table_state
                    .select(self.search_result.selected().into());
                self.build_selected_hint();

                return Ok(Action::UpdateAppState(AppState::Done("Done".into())).into());
            }
            Action::LoadDirMetadataDone(metadata) => {
                self.is_working = false;
                match metadata {
                    Some(metadata) => {
                        self.is_metadata_pop_up = true;
                        return Ok(Action::ShowDirMetadata(metadata).into());
                    }
                    None => {
                        self.send_app_action(Action::UpdateAppState(AppState::Failure(
                            "No metadata available".to_string(),
                        )))?;
                    }
                }
            }
            Action::ExportDone => {
                self.is_working = false;
                return Ok(
                    Action::UpdateAppState(AppState::Done("Export completed".into())).into(),
                );
            }
            Action::ExportFailure(msg) => {
                self.is_working = false;
                return Ok(Action::UpdateAppState(AppState::Failure(msg)).into());
            }
            Action::CloseMetadata => self.is_metadata_pop_up = false,
            Action::Resize(_, h) => {
                // update the terminal height
                self.terminal_height = h;
                self.search_result.set_terminal_height(self.terminal_height);
                // reset the start index and selected index to ensure that the selected object is no longer in the field of view
                self.search_result.reset_state();
                self.table_state
                    .select(self.search_result.selected().into());
                self.build_selected_hint();

                if self.app_context == AppContext::Results {
                    // clear the explorer state
                    self.send_app_action(Action::UpdateAppState(AppState::done_empty()))?;
                }
            }
            Action::ToggleTheme(theme) => {
                self.theme = theme;
            }
            Action::HideOrShowSystemOverview => {
                self.use_whole_draw_area = !self.use_whole_draw_area;
            }
            Action::Quit => self.export_task.stop(),
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

            // the main draw area, include the spacer and the first block (CWD)
            let [top_spacer_area, draw_area] =
                Layout::vertical([Constraint::Length(1), Constraint::Fill(1)]).areas(draw_area);

            let theme_colors = self.theme.theme_colors();

            let main_block_title = format!(" Cwd: [{}] ", self.search_result.cwd_display_name());

            let matches_str = if self.search_result.items().len() == 1 {
                "match"
            } else {
                "matches"
            };

            let inner_block_title = format!(
                " Summary → [ Applied Mode: {}, {} {matches_str} ]  ",
                self.applied_search_mode,
                self.search_result.items().len()
            );

            let help_msg = vec![
                " <Esc>".fg(theme_colors.main_text_fg),
                " back to search ".fg(theme_colors.main_fg),
            ];

            let header_style = Style::default()
                .fg(self.theme.theme_colors().header_fg)
                .bg(self.theme.theme_colors().header_bg);

            let header = ["Path", "Type", "Size"]
                .into_iter()
                .map(Cell::from)
                .collect::<Row>()
                .style(header_style)
                .height(1);

            let table_widths = [
                Constraint::Fill(1),
                Constraint::Length(7),
                Constraint::Length(12),
            ];

            let rows = self
                .search_result
                .get_content_to_draw()
                .iter()
                .enumerate()
                .map(|(i, entry)| {
                    let color = match i % 2 {
                        0 => self.theme.theme_colors().alt_row_color,
                        _ => self.theme.theme_colors().normal_row_color,
                    };

                    // FIRST: Shorten the path e.g. => /home/user/test => ~/test
                    let shorten_path = utils::format_path_for_display(&entry.path);

                    // SECOND: extract the containing file/dir name from the shorten path
                    let extract = utils::extract_part(&shorten_path, &entry.name);

                    // THIRD: highlight the search query
                    let path_spans = match extract {
                        Some(name) => {
                            let p = shorten_path.replace(&name, "");
                            let mut highlighted = highlight_text_part(
                                name,
                                self.search_result.search_query(),
                                self.theme.theme_colors().highlight_color,
                                self.theme.theme_colors().alt_fg,
                            );
                            highlighted
                                .insert(0, Span::from(p).fg(self.theme.theme_colors().alt_fg));
                            highlighted
                        }
                        None => highlight_text_part(
                            shorten_path,
                            self.search_result.search_query(),
                            self.theme.theme_colors().highlight_color,
                            self.theme.theme_colors().alt_fg,
                        ),
                    };

                    let object_type = if entry.is_dir() {
                        "Dir".to_string()
                    } else {
                        "File".to_string()
                    };
                    let size = if let Some(metadata) = &entry.file_metadata {
                        utils::convert_bytes_to_human_readable(metadata.size)
                    } else {
                        " - ".to_string()
                    };

                    let path_line = Line::from(path_spans);

                    let path_cell = Cell::from(Text::from(vec![Line::from(" "), path_line]));
                    let object_type_cell = Cell::from(Text::from(vec![
                        Line::from(" "),
                        Line::from(Span::styled(
                            object_type,
                            Style::new().fg(self.theme.theme_colors().alt_fg),
                        )),
                    ]));
                    let size_cell = Cell::from(Text::from(vec![
                        Line::from(" "),
                        Line::from(Span::styled(
                            size,
                            Style::new().fg(self.theme.theme_colors().alt_fg),
                        )),
                    ]));

                    Row::new(vec![path_cell, object_type_cell, size_cell])
                        .height(2)
                        .style(Style::new().bg(color))
                })
                .collect::<Vec<Row>>();

            let results_table = Table::new(rows, table_widths)
                .header(header)
                .block(Block::new().padding(Padding {
                    left: 0,
                    right: 0,
                    top: 1,
                    bottom: 1,
                }))
                .highlight_symbol(
                    Text::from(vec!["\n".into(), HIGHLIGHT_SYMBOL.into()])
                        .style(Style::new().fg(self.theme.theme_colors().selected_color)),
                )
                .bg(self.theme.theme_colors().alt_bg)
                .highlight_spacing(HighlightSpacing::Always);

            // CWD block
            let first_block = Block::default()
                .title_top(
                    Line::from(main_block_title)
                        .style(Style::new().fg(theme_colors.alt_fg))
                        .left_aligned(),
                )
                .title_alignment(Alignment::Center)
                .borders(Borders::TOP | Borders::BOTTOM)
                .border_type(BorderType::QuadrantInside)
                .border_style(Style::new().fg(theme_colors.alt_bg))
                .style(Style::new().bg(theme_colors.alt_bg));

            // Help msg block
            let second_block = Block::default()
                .title_top(Line::from(inner_block_title))
                .title_top(Line::from(self.selected_hint.as_str()).right_aligned())
                .title_bottom(Line::from(help_msg))
                .title_alignment(Alignment::Center)
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::new().fg(theme_colors.main_fg))
                .style(Style::new().bg(theme_colors.alt_bg));

            let [second_block_area] = Layout::vertical([Constraint::Fill(1)])
                .margin(1)
                .areas(first_block.inner(draw_area));

            let [table_area] = Layout::vertical([Constraint::Fill(1)])
                .areas(second_block.inner(second_block_area));

            f.render_widget(Line::from(" ").bg(theme_colors.alt_bg), top_spacer_area);
            f.render_widget(first_block, draw_area);
            f.render_widget(second_block, second_block_area);
            f.render_stateful_widget(results_table, table_area, &mut self.table_state);
        }
        Ok(())
    }
}
