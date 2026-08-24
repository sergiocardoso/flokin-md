use std::{path::PathBuf, time::Instant};

use flokin_core::{
    ExplorerNodeId, GraphNodeId, ScanResult, SqlCatalog, SqlError, SqlQueryResult, WorkspaceUpdate,
};
use iced::{keyboard, widget::text_editor, window};

use crate::services::file_watcher::WatcherMessage;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuId {
    File,
    View,
    Navigate,
    Data,
    Help,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuAction {
    OpenFolder,
    Reindex,
    ToggleTheme,
    ToggleLeftSidebar,
    ToggleRightSidebar,
    Explorer,
    Data,
    Graph,
    SqlExplorer,
    Settings,
    Search,
    ExecuteSql,
    About,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppMode {
    Files,
    Data,
    Graph,
    Sql,
    Settings,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplitterKind {
    LeftSidebar,
    Inspector,
    SqlSchema,
    SqlEditor,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Message {
    AppModeSelected(AppMode),
    #[allow(dead_code)]
    ExplorerNodeToggled(ExplorerNodeId),
    OpenFolder,
    FolderSelected(Option<PathBuf>),
    ScanCompleted(u64, PathBuf, Result<ScanResult, String>),
    ReindexWorkspace,
    WorkspaceWatcher(WatcherMessage),
    WorkspaceUpdateCompleted(u64, PathBuf, Result<WorkspaceUpdate, String>),
    CollectionSelected(String),
    TableHeaderSelected(String),
    MarkdownSelected(PathBuf),
    GraphFitRequested,
    GraphFocusSelected,
    GraphZoomIn,
    GraphZoomOut,
    GraphZoomReset,
    GraphViewportChanged(f32, f32),
    GraphNodeSelected(GraphNodeId),
    GraphNodeOpened(GraphNodeId),
    GraphPanBy(f32, f32),
    GraphZoomAt {
        x: f32,
        y: f32,
        delta: f32,
    },
    GraphNodeDragged {
        node: GraphNodeId,
        dx: f32,
        dy: f32,
    },
    EditorTabSelected(PathBuf),
    EditorTabCloseRequested(PathBuf),
    MarkdownEditorAction(text_editor::Action),
    EditorSaveRequested,
    EditorSaveCompleted(PathBuf, String, Result<(), String>),
    EditorCloseActiveRequested,
    EditorDialogCancel,
    EditorDialogDiscard,
    EditorDialogSave,
    EditorExternalReload,
    EditorExternalKeep,
    WindowCloseRequested(window::Id),
    SearchOpened,
    SearchClosed,
    SearchQueryChanged(String),
    SearchDebounceElapsed(Instant),
    SearchNext,
    SearchPrevious,
    SearchActivated,
    SearchResultSelected(PathBuf),
    SqlExplorerOpened,
    SqlSchemaTableToggled(String),
    SqlEditorAction(text_editor::Action),
    SqlCompletionRequested,
    SqlCompletionNext,
    SqlCompletionPrevious,
    SqlCompletionAccepted,
    SqlCompletionSelected(usize),
    SqlCompletionClosed,
    SqlExecute,
    SqlProjectionCompleted(u64, PathBuf, Result<SqlCatalog, SqlError>),
    SqlQueryCompleted(Result<SqlQueryResult, SqlError>),
    KeyboardEvent(keyboard::Event),
    ThemeToggled,
    ThemeSelected(bool),
    MenuToggled(MenuId),
    MenuTriggerMoved(MenuId, f32),
    MenuAction(MenuAction),
    MenuClosed,
    AboutClosed,
    SplitterPressed(SplitterKind, f32),
    SplitterMoved(f32, f32),
    SplitterReleased,
    ToggleLeftSidebar,
    ToggleRightSidebar,
    ResetLayout,
    MockAction,
}
