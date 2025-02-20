use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::{
    app::{AppContext, AppState},
    file_handling::{
        metadata::{DirMetadata, FileMetadata},
        Explorer, SearchResult,
    },
    ui::{search_widget::SearchMode, Theme},
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
/// Enum that tracks all the actions that can be carried out by the App
pub enum Action {
    Error(String),
    ExportDone,
    ExportFailure(String),
    HideOrShowSystemOverview,
    Init,
    LoadDir(PathBuf, bool),
    LoadDirDone(Explorer),
    LoadDirMetadata(String, PathBuf, bool),
    LoadDirMetadataDone(Option<DirMetadata>),
    None,
    Quit,
    Render,
    Resize(u16, u16),
    Resume,
    SearchDone(Option<SearchResult>),
    SetCommandDescription(Option<String>),
    ShowAbout(AppContext),
    ShowDirMetadata(DirMetadata),
    ShowFileMetadata(PathBuf, FileMetadata),
    ShowHelp(AppContext),
    CloseMetadata,
    ShowResultsPage(SearchResult, SearchMode),
    ShowSearchPage(PathBuf),
    StartSearch(PathBuf, String, usize, bool),
    Suspend,
    SwitchAppContext(AppContext),
    Tick,
    ForcedShutdown,
    ToggleTheme(Theme),
    UpdateAppState(AppState),
}
