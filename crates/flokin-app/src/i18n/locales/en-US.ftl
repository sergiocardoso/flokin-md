# Common
app-name = FlokinMD
action-apply = Apply
action-cancel = Cancel
action-close = Close
action-discard = Discard changes
action-open = Open
action-save = Save
action-save-all = Save all
action-do-not-save = Don't save
action-reset = Reset
state-on = ON
state-off = OFF

# Menus
menu-file = File
menu-view = View
menu-navigate = Navigate
menu-data = Data
menu-help = Help
menu-open-folder = Open folder
menu-reindex = Reindex
menu-toggle-theme = Toggle theme
menu-left-sidebar = Left sidebar
menu-right-sidebar = Right sidebar
menu-files = Files
menu-graph = Graph
menu-health = Database Health
menu-sql-explorer = SQL Explorer
menu-history = History
menu-settings = Settings
menu-search = Search
menu-open-data = Open Data
menu-open-graph = Open Graph
menu-run-query = Run query
menu-about = About FlokinMD

# Activity
activity-files = Files
activity-data = Data
activity-graph = Graph
activity-health = Database Health
activity-sql = SQL Explorer
activity-history = History
activity-settings = Settings

# Top Shell
search-placeholder = Search documents...
tooltip-hide-left-sidebar = Hide left sidebar
tooltip-show-left-sidebar = Show left sidebar
tooltip-hide-right-sidebar = Hide right sidebar
tooltip-show-right-sidebar = Show right sidebar
tooltip-toggle-theme = Toggle theme
about-description = Markdown workspace with a disposable SQL projection.

# Search
search-type-to-search = Type to search documents.
search-no-results = No documents found for "{ $query }".
search-results =
    { $count ->
        [one] 1 result
       *[other] { $count } results
    }
search-results-limited = { $count }+ results

# Settings
settings-section-interface = INTERFACE
settings-language = Language
settings-section-appearance = APPEARANCE
settings-theme = Theme
settings-section-layout = LAYOUT
settings-hide-left-sidebar = Hide left sidebar
settings-show-left-sidebar = Show left sidebar
settings-hide-right-sidebar = Hide right sidebar
settings-show-right-sidebar = Show right sidebar
settings-reset-layout = Reset layout

# Dialogs
dirty-close-title = Save changes to { $file }?
dirty-close-description = Your changes have not been written to Markdown yet.
dirty-workspace-title =
    { $count ->
        [one] There is 1 file with unsaved changes.
       *[other] There are { $count } files with unsaved changes.
    }
dirty-workspace-description = Choose how to handle dirty tabs before continuing.
schema-create-title = Create explicit schema
schema-create-description = FlokinMD will create { $file } at the root of this workspace.
schema-exists-warning = A flokin.schema.yaml file already exists in this workspace.
schema-none-available = No Collection is available for schema generation.
schema-from-inferred = The file will be generated from the currently inferred Schema.
schema-detected-collections = Detected Collections:
schema-collection-count =
    { $count ->
        [one] { $name } (1 document)
       *[other] { $name } ({ $count } documents)
    }
schema-mixed-fields-omitted = Mixed fields will be omitted: { $fields }.
schema-open = Open schema
schema-create = Create schema

# Feedback
files-restored =
    { $count ->
        [one] 1 file restored.
       *[other] { $count } files restored.
    }
error-no-workspace-schema = No workspace is open.
error-schema-empty = No Collection is available for schema generation.
error-schema-generate = Could not generate the schema: { $error }
error-stale-preview = The workspace changed since the preview was generated. Review the changes again.
error-save-file = Could not save { $path }: { $error }
error-history-clear-no-workspace = Open a workspace to clear history.
