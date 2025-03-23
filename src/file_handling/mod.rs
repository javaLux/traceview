use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use tokio::{
    sync::mpsc::{UnboundedReceiver, UnboundedSender},
    task::JoinHandle,
};
use tokio_util::sync::CancellationToken;
use walkdir::WalkDir;

use crate::{
    app::{actions::Action, AppState},
    file_handling::metadata::{DirMetadata, FileMetadata},
    models::Scrollable,
    utils,
};

pub mod metadata;

#[cfg(not(windows))]
pub const SEPARATOR: &str = "/";

#[cfg(target_os = "windows")]
pub const SEPARATOR: &str = "\\";

/// Represents the parent directory entry in the explorer list depending on the OS and the right separator
pub fn parent_dir_entry() -> String {
    format!("..{}", SEPARATOR)
}

/// Represents a file or directory on disk.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DiskEntry {
    pub name: String,
    pub path: PathBuf,
    pub file_metadata: Option<FileMetadata>,
    // Helper field to partition files and directories
    is_dir: bool,
}

impl DiskEntry {
    pub fn is_dir(&self) -> bool {
        self.is_dir
    }

    pub fn build_as_json(&self) -> serde_json::Value {
        let abs_path = utils::absolute_path_as_string(&self.path);

        if self.is_dir {
            serde_json::json!({
                "path": abs_path,
                "name": self.name,
                "type": "Directory",
            })
        } else {
            let file_format = file_format::FileFormat::from_file(&self.path)
                .ok()
                .map_or("Unknown".to_string(), |file_format| {
                    file_format.name().to_string()
                });

            let size = match &self.file_metadata {
                Some(metadata) => utils::convert_bytes_to_human_readable(metadata.size),
                None => "Unknown".into(),
            };

            serde_json::json!({
                "path": abs_path,
                "name": self.name,
                "type": "File",
                "format": file_format,
                "size": size
            })
        }
    }
}

/// Represents a task that runs the explorer
pub struct ExplorerTask {
    task: JoinHandle<()>,
    cancellation_token: CancellationToken,
    /// This sender is used to send actions back to the main thread
    action_sender: UnboundedSender<Action>,
    is_forced_shutdown: bool,
}

impl ExplorerTask {
    /// Constructs a new instance of [`ExplorerTask`].
    pub fn new(tx: UnboundedSender<Action>) -> Self {
        let cancellation_token = CancellationToken::new();
        let task = tokio::spawn(async {
            std::future::pending::<()>().await;
        });
        Self {
            task,
            cancellation_token,
            action_sender: tx,
            is_forced_shutdown: false,
        }
    }

    /// Runs the explorer task
    pub fn run(&mut self, rx: UnboundedReceiver<Action>) {
        let tx = self.action_sender.clone();
        let mut rx = rx;

        self.cancel();
        self.cancellation_token = CancellationToken::new();
        let _cancellation_token = self.cancellation_token.clone();

        self.task = tokio::task::spawn(async move {
            loop {
                tokio::select! {
                        _ = _cancellation_token.cancelled() => {
                            break;
                          }
                        Some(action) = rx.recv() => {
                            match action {
                                Action::LoadDir(p, follow_sym_links) => {
                                    tx.send(Action::UpdateAppState(AppState::Working("Loading directory...".into())))
                                        .expect("Explorer: Unable to send 'Action::UpdateExplorerState'");
                                    let explorer = Explorer::load_directory(p, follow_sym_links);
                                    tx.send(Action::LoadDirDone(explorer)).expect("Explorer: Unable to send 'Action::LoadDirDone'");
                                }
                                Action::LoadDirMetadata(dir_name, path, follow_sym_links) => {
                                    // handle result, if it was not possible to send a Action over the channel, we don't want to panic
                                    // in this case, instead we log the error
                                    match Explorer::get_dir_metadata(tx.clone(), dir_name, path, follow_sym_links) {
                                        Ok(dir_metadata) => tx.send(Action::LoadDirMetadataDone(dir_metadata)).expect("Explorer: Unable to send 'Action::LoadDirMetadataDone'"),
                                        Err(_) => {
                                            log::error!("Explorer: Unable to send 'Action::UpdateExplorerState' while processing directory metadata. The channel may have been dropped or closed before the sending completed.");
                                        },
                                    }
                                }
                                Action::StartSearch(cwd, search_query, depth, follow_sym_links) => {
                                    match Explorer::find_entries_by_name(tx.clone(), cwd, search_query, depth, follow_sym_links) {
                                        Ok(search_result) => tx.send(Action::SearchDone(search_result)).expect("Explorer: Unable to send 'Action::SearchDone'"),
                                        Err(_) => {
                                            log::error!("Explorer: Unable to send 'Action::UpdateExplorerState' while searching for files/folders. The channel may have been dropped or closed before the sending completed.");
                                        },
                                    }
                                }
                                _ => {}
                            }
                    }
                }
            }
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
                self.is_forced_shutdown = true;
                log::error!("Unable to abort Explorer-Task in 500 milliseconds for unknown reason");
                break;
            }
        }
    }

    pub fn is_forced_shutdown(&self) -> bool {
        self.is_forced_shutdown
    }
}

/// Represents the search results for file/directory names
#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SearchResult {
    // The shorted CWD -> used as Block title
    cwd_display_name: String,
    // The selected item (DirEntry) in the table
    selected: usize,
    // The terminal height is used to determine how many items to display on the screen
    // This is IMPORTANT, as many items have to be drawn in very large tables,
    // which can lead to high CPU utilization and the app freezing as a result
    terminal_height: usize,
    // The index of the first item to display on the screen
    start_index: usize,
    search_query: String,
    items: Vec<DiskEntry>,
}

impl Scrollable for SearchResult {
    /// Scrolls up by a page through the table content until the first element is reached.
    fn page_up_by(&mut self, height: u16) {
        let page_height = height as usize;

        if page_height >= self.items.len() {
            let iterations = self.selected;

            for _ in 0..iterations {
                self.scroll_up();
            }
        } else {
            let iterations = if self.selected >= page_height {
                page_height
            } else {
                self.selected
            };

            for _ in 0..iterations {
                self.scroll_up();
            }
        }
    }

    /// Scrolls down by a page through the table content until the last element is reached.
    fn page_down_by(&mut self, height: u16) {
        let page_height = height as usize;

        if page_height >= self.items.len() {
            let iterations = self
                .items
                .len()
                .saturating_sub(1)
                .saturating_sub(self.selected);

            for _ in 0..iterations {
                self.scroll_down();
            }
        } else {
            let iterations = if self.selected + page_height < self.items.len() {
                page_height
            } else {
                self.items
                    .len()
                    .saturating_sub(1)
                    .saturating_sub(self.selected)
            };

            for _ in 0..iterations {
                self.scroll_down();
            }
        }
    }

    /// Scrolls up through the table content. Adjusts the `start_index`,
    /// and `selected` indices appropriately to reflect the current view and selection.
    fn scroll_up(&mut self) {
        if self.selected == 0 {
            self.start_index = self.items.len().saturating_sub(self.terminal_height);
            self.selected = self.items.len().saturating_sub(1);
        } else if self.start_index > 0 {
            self.start_index = self.start_index.saturating_sub(1);
            self.selected = self.selected.saturating_sub(1);
        } else {
            self.selected = self.selected.saturating_sub(1);
        }
    }

    /// Scrolls down through the table content. Adjusts the `start_index`,
    /// and `selected` indices appropriately to reflect the current view and selection.
    fn scroll_down(&mut self) {
        if self.selected >= self.items.len().saturating_sub(1) {
            self.start_index = 0;
            self.selected = 0;
        } else if self.selected >= self.terminal_height - 1 {
            self.start_index = self.start_index.saturating_add(1);
            self.selected = self.selected.saturating_add(1);
        } else {
            self.selected = self.selected.saturating_add(1);
        }
    }
}

impl SearchResult {
    pub fn set_terminal_height(&mut self, size: u16) {
        self.terminal_height = size as usize;
    }

    pub fn cwd_display_name(&self) -> &str {
        &self.cwd_display_name
    }

    pub fn items(&self) -> &Vec<DiskEntry> {
        &self.items
    }

    pub fn selected(&self) -> usize {
        self.selected
    }

    pub fn reset_state(&mut self) {
        self.selected = 0;
        self.start_index = 0;
    }

    pub fn search_query(&self) -> &String {
        &self.search_query
    }

    /// Returns a vector containing the current page of directory content to display,
    /// based on the `start_index` and `page_size`.
    pub fn get_content_to_draw(&self) -> Vec<DiskEntry> {
        let end = (self.start_index + self.terminal_height).min(self.items.len());
        self.items[self.start_index..end].to_vec()
    }
}

/// Represents the result when searching for an entry by it's initial letter
#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FilteredEntries {
    /// The initial char to search for
    initial_letter: Option<char>,
    /// Only a helper field to show the match position in the user hint
    /// For this reason the field is not 0 based indexed
    user_hint_pos: usize,
    /// The search results
    indices: Vec<usize>,
}

impl FilteredEntries {
    pub fn new(initial_letter: char, indices: Vec<usize>) -> Self {
        Self {
            initial_letter: Some(initial_letter),
            user_hint_pos: 0,
            indices,
        }
    }

    pub fn find_next(&mut self, selected: usize) -> Option<&usize> {
        if self.indices.is_empty() {
            return None;
        }

        if let Some(index) = self.indices.iter().position(|&i| i == selected) {
            let next_index = (index + 1) % self.indices.len();
            self.user_hint_pos = next_index + 1;
            return self.indices.get(next_index);
        }

        if let Some(index) = self.indices.iter().find(|&&i| i > selected) {
            self.user_hint_pos = self
                .indices
                .iter()
                .position(|i| i == index)
                .unwrap_or_default()
                + 1;
            return Some(index);
        }

        self.user_hint_pos = 1;
        // cycle back to the beginning of the matches
        self.indices.first()
    }

    pub fn user_hint_pos(&self) -> usize {
        self.user_hint_pos
    }

    pub fn total_entries(&self) -> usize {
        self.indices.len()
    }

    pub fn matches_letter(&self, character: char) -> bool {
        self.initial_letter
            .unwrap_or_default()
            .eq_ignore_ascii_case(&character)
    }

    pub fn reset(&mut self) {
        self.initial_letter = None;
        self.user_hint_pos = 0;
        self.indices.clear();
    }
}

/// Allows you to navigate through the files and folders in the local file system
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Explorer {
    cwd: PathBuf,
    // The shorted CWD -> used as Block title
    cwd_display_name: String,
    items: Vec<DiskEntry>,
    file_counter: usize,
    dir_counter: usize,
    // The selected item (DirEntry) in the explorer
    selected: usize,
    // The terminal height is used to determine how many items to display on the screen
    // This is IMPORTANT, as many items have to be drawn in very large directories,
    // which can lead to high CPU utilization and the app freezing as a result
    terminal_height: usize,
    // The index of the first item to display on the screen
    start_index: usize,
}

impl Scrollable for Explorer {
    /// Scrolls up by a page through the directory content until the first element is reached.
    fn page_up_by(&mut self, height: u16) {
        let page_height = height as usize;

        if page_height >= self.items.len() {
            let iterations = self.selected;

            for _ in 0..iterations {
                self.scroll_up();
            }
        } else {
            let iterations = if self.selected >= page_height {
                page_height
            } else {
                self.selected
            };

            for _ in 0..iterations {
                self.scroll_up();
            }
        }
    }

    /// Scrolls down by a page through the directory content until the last element is reached.
    fn page_down_by(&mut self, height: u16) {
        let page_height = height as usize;

        if page_height >= self.items.len() {
            let iterations = self
                .items
                .len()
                .saturating_sub(1)
                .saturating_sub(self.selected);

            for _ in 0..iterations {
                self.scroll_down();
            }
        } else {
            let iterations = if self.selected + page_height < self.items.len() {
                page_height
            } else {
                self.items
                    .len()
                    .saturating_sub(1)
                    .saturating_sub(self.selected)
            };

            for _ in 0..iterations {
                self.scroll_down();
            }
        }
    }

    /// Scrolls up through the directory content. Adjusts the `start_index`,
    /// and `selected` indices appropriately to reflect the current view and selection.
    fn scroll_up(&mut self) {
        if self.selected == 0 {
            self.start_index = self.items.len().saturating_sub(self.terminal_height);
            self.selected = self.items.len().saturating_sub(1);
        } else if self.start_index > 0 {
            self.start_index = self.start_index.saturating_sub(1);
            self.selected = self.selected.saturating_sub(1);
        } else {
            self.selected = self.selected.saturating_sub(1);
        }
    }

    /// Scrolls down through the directory content. Adjusts the `start_index`,
    /// and `selected` indices appropriately to reflect the current view and selection.
    fn scroll_down(&mut self) {
        if self.selected >= self.items.len().saturating_sub(1) {
            self.start_index = 0;
            self.selected = 0;
        } else if self.selected >= self.terminal_height - 1 {
            self.start_index = self.start_index.saturating_add(1);
            self.selected = self.selected.saturating_add(1);
        } else {
            self.selected = self.selected.saturating_add(1);
        }
    }
}

impl Explorer {
    /// Load the content of the given path
    pub fn load_directory(p: PathBuf, follow_sym_links: bool) -> Self {
        let cwd = p;
        let cwd_display_name = utils::format_path_for_display(&cwd);

        let parent_dir_entry = parent_dir_entry();

        let (mut dirs, mut files): (Vec<_>, Vec<_>) = WalkDir::new(cwd.clone())
            .max_depth(1)
            .follow_links(follow_sym_links)
            .into_iter()
            .filter_map(Result::ok)
            // exclude the current working directory!!!
            .filter(|entry| entry.path() != cwd)
            .map(|entry| {
                let entry_name = entry.file_name().to_string_lossy().to_string();
                let path = entry.path().to_path_buf();

                let is_dir = entry.file_type().is_dir();

                let mut file_metadata: Option<FileMetadata> = None;

                let name = if is_dir {
                    format!("{}{}", entry_name, SEPARATOR)
                } else {
                    file_metadata = entry.metadata().ok().map(|metadata| FileMetadata {
                        created: metadata.created().ok(),
                        last_access: metadata.accessed().ok(),
                        modified: metadata.modified().ok(),
                        size: metadata.len(),
                        read_only: metadata.permissions().readonly(),
                    });
                    entry_name
                };

                DiskEntry {
                    name,
                    path,
                    file_metadata,
                    is_dir,
                }
            })
            .partition(|file_entry| file_entry.is_dir); // Separate files and folders

        dirs.sort_by(|f1, f2| f1.name.cmp(&f2.name));
        files.sort_by(|f1, f2| f1.name.cmp(&f2.name));

        let file_counter = files.len();
        let dir_counter = dirs.len();

        let dir_content = if let Some(parent) = cwd.parent() {
            let mut disk_items = Vec::with_capacity(1 + dirs.len() + files.len());

            disk_items.push(DiskEntry {
                name: parent_dir_entry,
                path: parent.to_path_buf(),
                file_metadata: None,
                is_dir: true,
            });

            disk_items.extend(dirs);
            disk_items.extend(files);

            disk_items
        } else {
            let mut disk_items = Vec::with_capacity(dirs.len() + files.len());

            disk_items.extend(dirs);
            disk_items.extend(files);

            disk_items
        };

        Self {
            cwd,
            cwd_display_name,
            items: dir_content,
            file_counter,
            dir_counter,
            selected: 0,
            terminal_height: 0,
            start_index: 0,
        }
    }

    fn get_dir_metadata(
        tx: UnboundedSender<Action>,
        dir_name: String,
        p: PathBuf,
        follow_sym_links: bool,
    ) -> Result<Option<DirMetadata>> {
        let mut dir_metadata = p.metadata().ok().map(|metadata| DirMetadata {
            dir_name,
            created: metadata.created().ok(),
            modified: metadata.modified().ok(),
            file_count: 0,
            dir_count: 0,
            total_size: 0,
        });

        if let Some(metadata) = &mut dir_metadata {
            let result: Result<()> = WalkDir::new(p.clone())
                .max_depth(usize::MAX)
                .follow_links(follow_sym_links)
                .into_iter()
                .filter_map(Result::ok)
                // exclude the current working directory!!!
                .filter(|entry| entry.path() != p)
                .try_for_each(|entry| -> Result<()> {
                    let filetype = entry.file_type();
                    let is_dir = filetype.is_dir();

                    if is_dir {
                        metadata.dir_count += 1;
                    } else {
                        metadata.file_count += 1;
                        metadata.total_size += entry.metadata().ok().map_or(0, |m| m.len());
                    }

                    // Don't panic here, because we want to be able to shutdown the app without a panic report
                    tx.send(Action::UpdateAppState(AppState::Working(format!(
                        "Calculate metadata... {} Files, {} Dirs",
                        metadata.file_count, metadata.dir_count
                    ))))?;

                    Ok(())
                });

            return match result {
                Ok(_) => Ok(dir_metadata),
                Err(err) => Err(anyhow::anyhow!(err)),
            };
        }

        Ok(dir_metadata)
    }

    pub fn find_entries_with_initial(&self, initial: char) -> Option<FilteredEntries> {
        let parent_dir_entry = parent_dir_entry();
        let initial_lower = initial.to_lowercase().next(); // Get the first character after lowercasing

        // If initial_lower is None (rare, but possible), return None early
        let initial_lower = initial_lower?;

        let entries: Vec<_> = self
            .items
            .iter()
            .filter(|item| !item.name.starts_with(&parent_dir_entry)) // Remove parent directory entry
            .enumerate() // Attach indices to items
            .filter_map(|(index, item)| {
                item.name
                    .chars()
                    .next() // Get the first character of the name
                    .and_then(|c| c.to_lowercase().next()) // Lowercase and get the first character
                    .filter(|&c| c == initial_lower) // Compare without case sensitivity
                    .map(|_| match self.cwd.parent() {
                        Some(_) => index + 1, // Adjust index if parent exists
                        None => index,
                    })
            })
            .collect();

        if entries.is_empty() {
            None
        } else {
            Some(FilteredEntries::new(initial, entries))
        }
    }

    pub fn find_entries_by_name(
        tx: UnboundedSender<Action>,
        cwd: PathBuf,
        search_query: String,
        depth: usize,
        follow_sym_links: bool,
    ) -> Result<Option<SearchResult>> {
        let lower_case_query = search_query.to_lowercase();
        let mut matches: Vec<DiskEntry> = vec![];
        let mut file_counter: usize = 0;
        let mut dir_counter: usize = 0;

        let search_result: Result<()> = WalkDir::new(cwd.clone())
            .max_depth(depth)
            .follow_links(follow_sym_links)
            .sort_by_file_name()
            .into_iter()
            .filter_map(Result::ok)
            // exclude the current working directory!!!
            .filter(|entry| entry.path() != cwd)
            .try_for_each(|entry| -> Result<()> {
                let entry_name = entry.file_name().to_string_lossy().to_string();
                let is_dir = entry.file_type().is_dir();

                if is_dir {
                    dir_counter += 1;
                } else {
                    file_counter += 1;
                }

                if entry_name.to_lowercase().contains(&lower_case_query) {
                    let path = entry.path().to_path_buf();
                    let disk_entry = if is_dir {
                        DiskEntry {
                            name: entry_name,
                            path,
                            file_metadata: None,
                            is_dir,
                        }
                    } else {
                        let file_metadata = entry.metadata().ok().map(|metadata| FileMetadata {
                            created: metadata.created().ok(),
                            last_access: metadata.accessed().ok(),
                            modified: metadata.modified().ok(),
                            size: metadata.len(),
                            read_only: metadata.permissions().readonly(),
                        });

                        DiskEntry {
                            name: entry_name,
                            path,
                            file_metadata,
                            is_dir,
                        }
                    };

                    matches.push(disk_entry);
                }

                // Don't panic here, because we want to be able to shutdown the app without a panic report
                tx.send(Action::UpdateAppState(AppState::Working(format!(
                    "Search in progress... {} Files, {} Dirs",
                    &file_counter, &dir_counter
                ))))?;

                Ok(())
            });

        match search_result {
            Ok(_) => {
                if !matches.is_empty() {
                    let result = SearchResult {
                        cwd_display_name: utils::format_path_for_display(&cwd),
                        search_query,
                        items: matches,
                        selected: Default::default(),
                        terminal_height: Default::default(),
                        start_index: Default::default(),
                    };
                    Ok(Some(result))
                } else {
                    Ok(None)
                }
            }
            Err(err) => Err(anyhow::anyhow!(err)),
        }
    }

    pub fn go_to_index(&mut self, index: usize) {
        // reset the selected index and start index
        self.reset_state();
        // scroll down to the given index
        for _ in 0..index {
            self.scroll_down();
        }
    }

    pub fn set_terminal_height(&mut self, size: u16) {
        self.terminal_height = size as usize;
    }

    pub fn cwd(&self) -> &PathBuf {
        &self.cwd
    }

    pub fn cwd_display_name(&self) -> &str {
        &self.cwd_display_name
    }

    pub fn items(&self) -> &Vec<DiskEntry> {
        &self.items
    }

    pub fn file_counter(&self) -> usize {
        self.file_counter
    }

    pub fn dir_counter(&self) -> usize {
        self.dir_counter
    }

    pub fn selected(&self) -> usize {
        self.selected
    }

    pub fn reset_state(&mut self) {
        self.selected = 0;
        self.start_index = 0;
    }

    /// Returns a vector containing the current page of directory content to display,
    /// based on the `start_index` and `page_size`.
    pub fn get_content_to_draw(&self) -> Vec<DiskEntry> {
        let end = (self.start_index + self.terminal_height).min(self.items.len());
        self.items[self.start_index..end].to_vec()
    }
}
