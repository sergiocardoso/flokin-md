use std::{path::PathBuf, time::Instant};

use flokin_core::{
    CollectionPanel, EditorViewMode, ExplorerNodeId, GraphNodeId, HealthFilter, ScanResult,
    SqlCatalog, SqlError, SqlQueryResult, WorkspaceUpdate,
};
use iced::{
    keyboard,
    widget::{markdown, text_editor},
    window,
};

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
    Health,
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
    Health,
    Sql,
    Settings,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplitterKind {
    LeftSidebar,
    Inspector,
    SqlSchema,
    SqlEditor,
    MarkdownPreview,
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
    CollectionPanelSelected(CollectionPanel),
    SchemaFieldSelected {
        collection_id: String,
        field_name: String,
    },
    HealthFilterSelected(HealthFilter),
    HealthQueryChanged(String),
    HealthIssueSelected(String),
    HealthIssueOpened(String),
    SchemaCreateRequested,
    SchemaCreateCanceled,
    SchemaCreateConfirmed,
    SchemaCreateCompleted(Result<PathBuf, String>),
    SchemaOpenRequested,
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
    EditorViewModeSelected(EditorViewMode),
    MarkdownLinkClicked(markdown::Uri),
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
