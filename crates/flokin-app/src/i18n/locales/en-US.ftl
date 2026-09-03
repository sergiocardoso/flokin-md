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
action-back = Back
action-previous = Previous
action-next = Next
action-yes = Yes
action-no = No
state-on = ON
state-off = OFF

# Menus
menu-file = File
menu-view = View
menu-navigate = Navigate
menu-data = Data
menu-help = Help
menu-new-file = New file
menu-open-folder = Open folder
menu-close-folder = Close folder
menu-reindex = Reindex
menu-toggle-theme = Toggle theme
menu-left-sidebar = Left sidebar
menu-right-sidebar = Right sidebar
menu-files = Files
menu-context = Context
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
activity-context = Context
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
about-version = Version { $version }
about-tagline = A local-first database for your Markdown files.
about-description = FlokinMD is an open-source project built to turn Markdown folders into structured data workspaces without taking ownership of the user's files.
about-source-truth = Markdown remains the source of truth.
about-open-source = Open-source project
about-created-by = Created by Sérgio Cardoso.
about-motivation-title = Why FlokinMD exists
about-motivation-paragraph-1 = Markdown is no longer just documentation.
about-motivation-paragraph-2 =
    With AI, agents, SDD, skills and context-driven workflows, .md files are increasingly becoming the memory and infrastructure of software projects.
about-motivation-paragraph-3 =
    Specs, instructions, prompts, decisions, context and operational knowledge are increasingly living in Markdown.
about-motivation-paragraph-4 =
    FlokinMD was created to make this knowledge base easy to query, edit, visualize and, most importantly, understand how the documents relate to each other.
about-context-highlight = Markdown became context infrastructure.
about-creator-title = About the creator
about-creator-role = Software Engineer · Creator of FlokinMD
about-creator-paragraph-1 =
    I created FlokinMD from a problem I started experiencing in my own work with software, agents and AI: as more context moved into Markdown files, it became increasingly difficult to understand that knowledge base as a system.
about-creator-paragraph-2 =
    I wanted a fast and focused tool to open a folder, query, edit, visualize and understand the relationships between documents — without adopting a full knowledge-management suite or migrating my files to another format.
about-flokin-project = FlokinMD is an open-source project by Flokin.
about-flokin-project-short = A Flokin project.
about-built-with = Built with Rust + Iced.
about-linkedin = LinkedIn
about-website = Website
about-email = Email
about-manifesto = Markdown is the database.
about-principle = Markdown is the database.

# Search
search-type-to-search = Type to search documents.
search-no-results = No documents found for "{ $query }".
search-results =
    { $count ->
        [one] 1 result
       *[other] { $count } results
    }
search-results-limited = { $count }+ results

# Explorer
explorer-title = EXPLORER
explorer-new-file = New file
explorer-file-name = File name
explorer-create-file = Create file
explorer-file-exists = A file with this name already exists.
explorer-file-create-failed = Could not create the file: { $message }
explorer-invalid-file-name = Enter a valid Markdown file name.
explorer-expand-all = Expand all
explorer-collapse-all = Collapse all
explorer-open = Open
explorer-reindex = Reindex
explorer-no-workspace = No folder open
explorer-scanning = Scanning documents...
explorer-scan-failed = Failed to scan workspace
explorer-no-markdown = No Markdown files found.
explorer-files = FILES
explorer-data = DATA
explorer-collections = COLLECTIONS
explorer-filters = FILTERS
explorer-filters-empty = Available after indexing
explorer-database = DATABASE
semantic-agent = Agent
semantic-agent-instructions = Agent instructions
semantic-skill = Skill
semantic-spec = Spec
semantic-ice = ICE
semantic-context = Context
semantic-prompt = Prompt
semantic-rules = Rules
semantic-memory = Memory
semantic-mcp = MCP
explorer-documents-found =
    { $count ->
        [one] 1 document found
       *[other] { $count } documents found
    }
explorer-access-errors =
    { $count ->
        [one] 1 item could not be accessed
       *[other] { $count } items could not be accessed
    }
explorer-warnings =
    { $count ->
        [one] 1 warning
       *[other] { $count } warnings
    }
sql-schema-empty = No Collection available.
sql-schema-building = Building schema...
sql-schema-open-folder = Open a folder to generate the schema.

# Status Bar
status-no-workspace = No folder open
status-scanning = Scanning documents...
status-updating = Updating...
status-scan-failed = Failed to scan workspace
status-workspace-watched = Workspace monitored
status-read-only = Read only
status-sqlite-memory = SQLite :memory:
status-markdown = Markdown
status-documents =
    { $count ->
        [one] 1 document
       *[other] { $count } documents
    }
status-collections =
    { $count ->
        [one] 1 collection
       *[other] { $count } collections
    }
status-warnings =
    { $count ->
        [one] 1 warning
       *[other] { $count } warnings
    }

# Editor
editor-empty-workspace-hint = Open a folder containing .md or .markdown files.
editor-scanned-folder = Scanned folder: { $path }
editor-mode-edit = Edit
editor-mode-split = Split
editor-mode-preview = Preview
editor-save-tooltip = Save (Ctrl+S)
editor-conflict-modified = The file was changed externally.
editor-conflict-deleted = The file was removed externally.
editor-reload-disk = Reload from disk
editor-keep-local = Keep my changes
editor-empty-preview = Empty preview.
editor-empty-file = Empty file.
editor-select-document = Select a Markdown document to view its contents.

# SQL
sql-reviewing = Reviewing...
sql-running = Running...
sql-review-update = Review update
sql-run = Run
sql-mode-query = Query
sql-mode-update = Update
sql-update-context = SQL Updates are converted into Markdown changes and always require preview.
sql-query-context = Query mode is read-only.
sql-results = RESULTS
sql-error = Error:
sql-no-results = No results
sql-preview-status = { $matched } documents match • { $changed } will change
sql-result-status = { $rows } rows • { $ms } ms
sql-results-limited = Results limited to 1,000 rows.
sql-empty-update-preview = Review an UPDATE statement to see the preview.
sql-empty-query-results = Run a SELECT query to see the grid.
sql-update-no-matches = No documents match this update.
sql-update-no-changes =
    { $count ->
        [one] 1 document matches, but no change is needed.
       *[other] { $count } documents match, but no change is needed.
    }
sql-documents-match =
    { $count ->
        [one] 1 document matches
       *[other] { $count } documents match
    }
sql-no-result-columns = Query executed with no result columns.
sql-schema-table-name = SQL: { $table }
sql-column-type-text = TEXT
sql-column-type-integer = INTEGER
sql-column-type-real = REAL
sql-column-type-boolean = BOOLEAN
sql-column-type-json = JSON
sql-column-type-null = NULL

# Data / Schema / Bulk
data-properties =
    { $count ->
        [one] 1 property
       *[other] { $count } properties
    }
data-panel-data = Data
data-panel-schema = Schema
data-empty-collection = No documents in this Collection.
data-empty-schema = No schema available for this Collection.
schema-source-inferred = Inferred schema
schema-source-explicit = Explicit schema + inferred observations
schema-inferred-title = Inferred schema
schema-inferred-description = FlokinMD detected this structure automatically from your documents. Create an explicit schema to define types and required fields.
schema-explicit-title = Explicit schema
schema-explicit-invalid = Invalid explicit schema
schema-field = FIELD
schema-type = TYPE
schema-required = REQUIRED
schema-present = PRESENT
schema-present-in = Present in
schema-present-ratio = { $observed } / { $total } documents
schema-null-values = Null values
schema-declared = Declared
schema-not-declared = Not declared
schema-observed-types = Observed types: { $types }
schema-unknown = Unknown
schema-structural-suffix =  · structural/derived field
bulk-edit-title = Bulk edit
bulk-clear-selection = Clear selection
bulk-selected-count =
    { $count ->
        [one] 1 selected
       *[other] { $count } selected
    }
bulk-selected-documents =
    { $count ->
        [one] 1 document selected
       *[other] { $count } documents selected
    }
bulk-step-configure = 1. Configure
bulk-step-review = 2. Review
bulk-preview-unavailable = Preview unavailable. Go back and review the configuration.
bulk-review-changes = Review changes
bulk-operation = Operation
bulk-operation-set = Set property
bulk-operation-remove = Remove property
bulk-new-property-option = + New property...
bulk-property-placeholder = Choose a property
bulk-property = Property
bulk-property-name = Property name
bulk-property-name-placeholder = e.g. reviewed
bulk-type = Type
bulk-value = Value
bulk-target = Target
bulk-target-placeholder = Target
bulk-value-placeholder = Value
bulk-null-value = Value: null
value-true = True
value-false = False
value-type-string = Text
value-type-integer = Integer
value-type-float = Float
value-type-boolean = Boolean
value-type-array = Array
value-type-object = Object
value-type-mixed = Mixed
value-type-null = Null
value-type-relation = Relation
change-status-changed = Changed
change-status-no-change = No change
change-status-blocked = Blocked
change-status-unsupported = Unsupported
apply-changes =
    { $count ->
        [one] Apply 1 change
       *[other] Apply { $count } changes
    }
changes-will-change =
    { $count ->
        [one] 1 will change
       *[other] { $count } will change
    }
changes-no-change =
    { $count ->
        [one] 1 no change
       *[other] { $count } no change
    }
changes-blocked =
    { $count ->
        [one] 1 blocked
       *[other] { $count } blocked
    }
pagination-status = { $start }-{ $end } of { $total }  ·  Page { $page } of { $pages }

# Settings
settings-section-interface = INTERFACE
settings-language = Language
settings-section-appearance = APPEARANCE
settings-theme = Theme
theme-light = Light
theme-dark = Dark
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

# History
history-title = History
history-description = Bulk Edit, SQL Update, and Undo operations recorded locally.
history-no-workspace-title = Open a workspace to view history.
history-no-workspace-subtitle = History is local and isolated by workspace.
history-empty-title = No changes recorded yet.
history-empty-subtitle = Bulk Edit and SQL Update operations will appear here.
history-select-title = Select an operation.
history-select-subtitle = Details will appear here.
history-clear = Clear history
history-clear-confirm-title = Clear history
history-clear-confirm-description = This will remove operation history for this workspace. Markdown files will not be changed.
history-today = Today
history-yesterday = Yesterday
history-source-bulk = Bulk Edit
history-source-sql = SQL Update
history-source-undo = Undo
history-undo-available = undo available
history-undo-unavailable = undo unavailable
history-undone = undone
history-undo-button = Undo change
history-original-operation = Original operation: { $id }
history-full-content-recorded = Full content recorded.
history-full-content-restore = Full content will be restored.
undo-review-title = Review undo
undo-files-restore =
    { $count ->
        [one] 1 file will be restored
       *[other] { $count } files will be restored
    }
undo-apply =
    { $count ->
        [one] Undo 1 change
       *[other] Undo { $count } changes
    }

# Health
health-title = Database Health
health-total-documents =
    { $count ->
        [one] 1 document
       *[other] { $count } documents
    }
health-errors = Errors
health-warnings = Warnings
health-healthy = Healthy
health-filter-all = All
health-filter-errors = Errors
health-filter-warnings = Warnings
health-filter-placeholder = Filter issues...
health-schema-absent = Explicit schema is not configured.
health-schema-inferred = The database is using only inferred structure.
health-schema-invalid = There is a problem in flokin.schema.yaml.
health-no-issues = No issues found.
health-severity = SEVERITY
health-category = CATEGORY
health-document = DOCUMENT
health-property = PROPERTY
health-problem = PROBLEM
health-expected = EXPECTED
health-found = FOUND
health-workspace = workspace
health-severity-error = Error
health-severity-warning = Warning
health-severity-info = Info
health-category-parsing = Parsing
health-category-schema = Schema
health-category-relations = Relations
health-category-workspace = Workspace
health-issue-invalid-frontmatter = Invalid YAML frontmatter.
health-issue-file-read-error = Could not read the file.
health-issue-workspace-scan-error = Error while processing the workspace.
health-issue-explicit-schema-invalid = Explicit schema is invalid.
health-issue-required-field-missing = Required field is missing.
health-issue-type-mismatch = Expected { $expected }, found { $found }.
health-issue-undeclared-field = Field is not declared in the explicit schema.
health-issue-mixed-observed-types = Inconsistent types detected.
health-issue-relation-unresolved = Unresolved relation.
health-issue-relation-ambiguous =
    { $count ->
        [one] Ambiguous relation: 1 document matches.
       *[other] Ambiguous relation: { $count } documents match.
    }

# Inspector
inspector-properties = PROPERTIES
inspector-relations = RELATIONS
inspector-referenced-by = REFERENCED BY
inspector-tags = TAGS
inspector-warnings = WARNINGS
inspector-metadata = METADATA
inspector-issue = ISSUE
inspector-details = DETAILS
inspector-open-document = Open document
relation-unresolved = Unresolved
relation-ambiguous-count =
    { $count ->
        [one] Ambiguous — 1 document matches
       *[other] Ambiguous — { $count } documents match
    }
relation-structured-reference = structured reference

# Graph
graph-title = Graph
graph-section = GRAPH
graph-documents = Documents
graph-relations = Relations
graph-problems = Problems
graph-summary = { $documents } documents • { $relations } relations
graph-zoom-out = Zoom out
graph-zoom-in = Zoom in
graph-zoom-reset = Reset zoom
graph-focus-selected = Center selected
graph-fit = Fit to screen

# Context
context-title = Context
context-sidebar-title = CONTEXT
context-overview = Overview
context-agents = Agents
context-skills = Skills
context-specs = SDD / Specs
context-ice = ICE
context-contexts = Context
context-prompts = Prompts
context-rules = Rules / Instructions
context-memory = Memory
context-mcp = MCP
context-artifact-count =
    { $count ->
        [one] 1 artifact
       *[other] { $count } artifacts
    }
context-artifacts = ARTIFACTS
context-name = Name
context-kind = Kind
context-path = Path
context-relations = Relations
context-referenced-by = Referenced by
context-references = References
context-unconnected = Unconnected
context-unconnected-count =
    { $count ->
        [one] 1 artifact
       *[other] { $count } artifacts
    }
context-inspector-title = CONTEXT
context-select-artifact = Select an artifact to view details.
context-open-editor = Open in Editor
context-show-graph = Show in Graph
context-metadata = Metadata
context-unresolved = unresolved
context-ambiguous = ambiguous
context-empty = No context artifacts found in this workspace.
context-no-agents = No Agents found in this workspace.
context-no-skills = No Skills found in this workspace.
context-no-specs = No Specs found in this workspace.
context-no-ice = No ICE artifacts found in this workspace.
context-no-contexts = No Context artifacts found in this workspace.
context-no-prompts = No Prompts found in this workspace.
context-no-rules = No Rules or Instructions found in this workspace.
context-no-memory = No Memory artifacts found in this workspace.
context-no-mcp = No MCP artifacts found in this workspace.

# Welcome
welcome-title = Open a Markdown folder to get started
welcome-open-folder = Open folder
workspace-restoring = Opening workspace...
workspace-previous-unavailable = The previously used folder is not available.
