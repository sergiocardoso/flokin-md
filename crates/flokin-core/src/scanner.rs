use std::{
    collections::{BTreeMap, BTreeSet},
    ffi::OsString,
    fs, io,
    path::{Path, PathBuf},
    time::{Duration, Instant, SystemTime},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScanResult {
    pub root: PathBuf,
    pub documents: Vec<Document>,
    pub collections: Vec<Collection>,
    pub directories: Vec<PathBuf>,
    pub errors: Vec<ScanError>,
    pub duration: Duration,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Document {
    pub path: PathBuf,
    pub relative_path: PathBuf,
    pub file_name: OsString,
    pub metadata: DocumentMetadata,
    pub title: String,
    pub source_content: Option<String>,
    pub markdown_content: String,
    pub properties: BTreeMap<String, PropertyValue>,
    pub document_type: Option<String>,
    pub collection_id: String,
    pub warnings: Vec<DocumentWarning>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentMetadata {
    pub file_size: Option<u64>,
    pub modified: Option<SystemTime>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Collection {
    pub id: String,
    pub display_name: String,
    pub document_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PropertyValue {
    Null,
    Bool(bool),
    Number(String),
    String(String),
    Array(Vec<PropertyValue>),
    Object(BTreeMap<String, PropertyValue>),
}

impl PropertyValue {
    pub fn as_non_empty_string(&self) -> Option<&str> {
        match self {
            Self::String(value) if !value.trim().is_empty() => Some(value.trim()),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentWarning {
    pub path: PathBuf,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScanError {
    pub path: PathBuf,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkspaceEvent {
    Upsert(PathBuf),
    Remove(PathBuf),
    Rename { from: PathBuf, to: PathBuf },
    Rescan,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceUpdate {
    pub root: PathBuf,
    pub upserts: Vec<Document>,
    pub removals: Vec<PathBuf>,
    pub errors: Vec<ScanError>,
    pub duration: Duration,
    pub needs_rescan: bool,
    pub schema_changed: bool,
}

impl WorkspaceUpdate {
    pub fn changed_paths(&self) -> Vec<PathBuf> {
        self.upserts
            .iter()
            .map(|document| document.path.clone())
            .chain(self.removals.iter().cloned())
            .collect()
    }
}

pub fn scan_workspace(root: &Path) -> io::Result<ScanResult> {
    let started_at = Instant::now();
    let root = root.to_path_buf();
    let mut documents = Vec::new();
    let mut directories = BTreeSet::new();
    let mut errors = Vec::new();

    scan_dir(
        &root,
        Path::new(""),
        &root,
        &mut documents,
        &mut directories,
        &mut errors,
    )?;

    documents.sort_by(|left, right| compare_paths(&left.relative_path, &right.relative_path));
    let collections = build_collections(&documents);

    Ok(ScanResult {
        root,
        documents,
        collections,
        directories: directories.into_iter().collect(),
        errors,
        duration: started_at.elapsed(),
    })
}

pub fn workspace_update_from_events(
    root: &Path,
    events: &[WorkspaceEvent],
) -> io::Result<WorkspaceUpdate> {
    let started_at = Instant::now();
    let mut upsert_paths = BTreeSet::<PathBuf>::new();
    let mut removals = BTreeSet::<PathBuf>::new();
    let mut errors = Vec::new();
    let mut needs_rescan = false;
    let mut schema_changed = false;

    for event in events {
        match event {
            WorkspaceEvent::Upsert(path) => {
                if is_workspace_markdown_path(root, path) {
                    upsert_paths.insert(path.clone());
                } else if crate::is_workspace_schema_path(root, path) {
                    schema_changed = true;
                } else if should_rescan_path(root, path) {
                    needs_rescan = true;
                }
            }
            WorkspaceEvent::Remove(path) => {
                if is_workspace_markdown_path(root, path) {
                    removals.insert(path.clone());
                } else if crate::is_workspace_schema_path(root, path) {
                    schema_changed = true;
                } else if should_rescan_path(root, path) {
                    needs_rescan = true;
                }
            }
            WorkspaceEvent::Rename { from, to } => {
                if is_workspace_markdown_path(root, from) {
                    removals.insert(from.clone());
                } else if crate::is_workspace_schema_path(root, from) {
                    schema_changed = true;
                } else if should_rescan_path(root, from) {
                    needs_rescan = true;
                }

                if is_workspace_markdown_path(root, to) {
                    upsert_paths.insert(to.clone());
                } else if crate::is_workspace_schema_path(root, to) {
                    schema_changed = true;
                } else if should_rescan_path(root, to) {
                    needs_rescan = true;
                }
            }
            WorkspaceEvent::Rescan => needs_rescan = true,
        }
    }

    let event_paths = upsert_paths
        .union(&removals)
        .cloned()
        .collect::<BTreeSet<_>>();
    removals.clear();

    let mut upserts = Vec::new();
    for path in event_paths {
        match fs::symlink_metadata(&path) {
            Ok(metadata) if metadata.file_type().is_symlink() => continue,
            Ok(metadata) if metadata.is_file() => {
                let relative_path = path.strip_prefix(root).unwrap_or(&path).to_path_buf();
                let file_name = path
                    .file_name()
                    .map(ToOwned::to_owned)
                    .unwrap_or_else(|| OsString::from("document.md"));
                upserts.push(parse_document(path, relative_path, file_name));
            }
            Ok(metadata) if metadata.is_dir() => needs_rescan = true,
            Ok(_) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                if is_workspace_markdown_path(root, &path) {
                    removals.insert(path);
                }
            }
            Err(error) => errors.push(ScanError {
                path,
                message: error.to_string(),
            }),
        }
    }

    upserts.sort_by(|left, right| compare_paths(&left.relative_path, &right.relative_path));

    Ok(WorkspaceUpdate {
        root: root.to_path_buf(),
        upserts,
        removals: removals.into_iter().collect(),
        errors,
        duration: started_at.elapsed(),
        needs_rescan,
        schema_changed,
    })
}

pub fn is_workspace_markdown_path(root: &Path, path: &Path) -> bool {
    path.strip_prefix(root)
        .ok()
        .filter(|relative_path| !has_ignored_component(relative_path))
        .map(is_markdown_path)
        .unwrap_or(false)
}

pub fn should_ignore_workspace_path(root: &Path, path: &Path) -> bool {
    path.strip_prefix(root)
        .map(has_ignored_component)
        .unwrap_or(true)
}

fn scan_dir(
    dir: &Path,
    relative_dir: &Path,
    root: &Path,
    documents: &mut Vec<Document>,
    directories: &mut BTreeSet<PathBuf>,
    errors: &mut Vec<ScanError>,
) -> io::Result<()> {
    let entries = fs::read_dir(dir)?;

    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => {
                errors.push(ScanError {
                    path: dir.to_path_buf(),
                    message: error.to_string(),
                });
                continue;
            }
        };

        let path = entry.path();
        let file_type = match entry.file_type() {
            Ok(file_type) => file_type,
            Err(error) => {
                errors.push(ScanError {
                    path,
                    message: error.to_string(),
                });
                continue;
            }
        };

        if file_type.is_symlink() {
            continue;
        }

        if file_type.is_dir() {
            if should_ignore_dir(&entry.file_name()) {
                continue;
            }

            let relative_path = path.strip_prefix(root).unwrap_or(&path).to_path_buf();
            if let Err(error) =
                scan_dir(&path, &relative_path, root, documents, directories, errors)
            {
                errors.push(ScanError {
                    path,
                    message: error.to_string(),
                });
            }
            continue;
        }

        if file_type.is_file() && is_markdown_path(&path) {
            if !relative_dir.as_os_str().is_empty() {
                directories.insert(relative_dir.to_path_buf());
                for ancestor in relative_dir.ancestors().skip(1) {
                    if !ancestor.as_os_str().is_empty() {
                        directories.insert(ancestor.to_path_buf());
                    }
                }
            }

            let relative_path = path.strip_prefix(root).unwrap_or(&path).to_path_buf();
            documents.push(parse_document(path, relative_path, entry.file_name()));
        }
    }

    Ok(())
}

fn parse_document(path: PathBuf, relative_path: PathBuf, file_name: OsString) -> Document {
    let mut warnings = Vec::new();
    let mut properties = BTreeMap::new();
    let mut body = String::new();
    let mut source_content = None;
    let metadata = document_metadata(&path);

    match fs::read_to_string(&path) {
        Ok(content) => {
            let parsed = parse_frontmatter(&path, &content);
            body = parsed.body.to_string();
            properties = parsed.properties;
            warnings.extend(parsed.warnings);
            source_content = Some(content);
        }
        Err(error) => warnings.push(DocumentWarning {
            path: path.clone(),
            message: error.to_string(),
        }),
    }

    let title = title_from_properties(&properties)
        .or_else(|| first_h1(&body))
        .unwrap_or_else(|| title_from_file_name(&file_name));
    let document_type = document_type_from_properties(&properties)
        .or_else(|| infer_type_from_parent(&relative_path));
    let collection_id = collection_id(document_type.as_deref());

    Document {
        path,
        relative_path,
        file_name,
        metadata,
        title,
        source_content,
        markdown_content: body,
        properties,
        document_type,
        collection_id,
        warnings,
    }
}

fn document_metadata(path: &Path) -> DocumentMetadata {
    match fs::metadata(path) {
        Ok(metadata) => DocumentMetadata {
            file_size: Some(metadata.len()),
            modified: metadata.modified().ok(),
        },
        Err(_) => DocumentMetadata {
            file_size: None,
            modified: None,
        },
    }
}

struct ParsedFrontmatter<'a> {
    properties: BTreeMap<String, PropertyValue>,
    body: &'a str,
    warnings: Vec<DocumentWarning>,
}

pub fn markdown_body_without_frontmatter(content: &str) -> &str {
    parse_frontmatter(Path::new(""), content).body
}

fn parse_frontmatter<'a>(path: &Path, content: &'a str) -> ParsedFrontmatter<'a> {
    let Some(after_opening) = content
        .strip_prefix("---\n")
        .or_else(|| content.strip_prefix("---\r\n"))
    else {
        return ParsedFrontmatter {
            properties: BTreeMap::new(),
            body: content,
            warnings: Vec::new(),
        };
    };

    let mut yaml = String::new();
    let mut body_start = content.len();
    let mut found_closing = false;
    let mut offset = content.len() - after_opening.len();

    for line in after_opening.split_inclusive('\n') {
        let trimmed = line.trim_end_matches(['\r', '\n']);
        if trimmed == "---" {
            body_start = offset + line.len();
            found_closing = true;
            break;
        }
        yaml.push_str(line);
        offset += line.len();
    }

    if !found_closing {
        return ParsedFrontmatter {
            properties: BTreeMap::new(),
            body: content,
            warnings: Vec::new(),
        };
    }

    let mut warnings = Vec::new();
    let properties = match serde_yaml::from_str::<serde_yaml::Value>(&yaml) {
        Ok(value) => property_map_from_yaml(value),
        Err(error) => {
            warnings.push(DocumentWarning {
                path: path.to_path_buf(),
                message: format!("YAML frontmatter inválido: {error}"),
            });
            BTreeMap::new()
        }
    };

    ParsedFrontmatter {
        properties,
        body: &content[body_start..],
        warnings,
    }
}

fn property_map_from_yaml(value: serde_yaml::Value) -> BTreeMap<String, PropertyValue> {
    match value {
        serde_yaml::Value::Mapping(mapping) => mapping
            .into_iter()
            .filter_map(|(key, value)| match key {
                serde_yaml::Value::String(key) => Some((key, property_value_from_yaml(value))),
                _ => None,
            })
            .collect(),
        _ => BTreeMap::new(),
    }
}

fn property_value_from_yaml(value: serde_yaml::Value) -> PropertyValue {
    match value {
        serde_yaml::Value::Null => PropertyValue::Null,
        serde_yaml::Value::Bool(value) => PropertyValue::Bool(value),
        serde_yaml::Value::Number(value) => PropertyValue::Number(value.to_string()),
        serde_yaml::Value::String(value) => PropertyValue::String(value),
        serde_yaml::Value::Sequence(values) => {
            PropertyValue::Array(values.into_iter().map(property_value_from_yaml).collect())
        }
        serde_yaml::Value::Mapping(mapping) => PropertyValue::Object(
            mapping
                .into_iter()
                .filter_map(|(key, value)| match key {
                    serde_yaml::Value::String(key) => Some((key, property_value_from_yaml(value))),
                    _ => None,
                })
                .collect(),
        ),
        serde_yaml::Value::Tagged(tagged) => property_value_from_yaml(tagged.value),
    }
}

fn title_from_properties(properties: &BTreeMap<String, PropertyValue>) -> Option<String> {
    properties
        .get("title")
        .and_then(PropertyValue::as_non_empty_string)
        .map(ToOwned::to_owned)
}

fn document_type_from_properties(properties: &BTreeMap<String, PropertyValue>) -> Option<String> {
    properties
        .get("type")
        .and_then(PropertyValue::as_non_empty_string)
        .map(normalize_document_type)
}

fn first_h1(content: &str) -> Option<String> {
    content.lines().find_map(|line| {
        let line = line.trim_start();
        line.strip_prefix("# ")
            .map(str::trim)
            .filter(|title| !title.is_empty())
            .map(ToOwned::to_owned)
    })
}

fn title_from_file_name(file_name: &OsString) -> String {
    Path::new(file_name)
        .file_stem()
        .filter(|stem| !stem.is_empty())
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_else(|| file_name.to_string_lossy().into_owned())
}

fn infer_type_from_parent(relative_path: &Path) -> Option<String> {
    let parent = relative_path.parent()?.file_name()?.to_str()?;
    match normalize_document_type(parent).as_str() {
        "project" => Some(String::from("project")),
        "person" => Some(String::from("person")),
        "meeting" => Some(String::from("meeting")),
        _ => None,
    }
}

fn normalize_document_type(value: &str) -> String {
    let value = value.trim().to_ascii_lowercase().replace(['_', ' '], "-");
    match value.as_str() {
        "projects" => String::from("project"),
        "people" | "persons" => String::from("person"),
        "meetings" => String::from("meeting"),
        "documents" | "docs" => String::from("document"),
        _ => value.trim_end_matches('s').to_owned(),
    }
}

fn collection_id(document_type: Option<&str>) -> String {
    document_type
        .filter(|document_type| !document_type.trim().is_empty())
        .map(normalize_document_type)
        .unwrap_or_else(|| String::from("documents"))
}

fn build_collections(documents: &[Document]) -> Vec<Collection> {
    let mut counts = BTreeMap::<String, usize>::new();
    for document in documents {
        *counts.entry(document.collection_id.clone()).or_default() += 1;
    }

    let mut collections = counts
        .into_iter()
        .map(|(id, document_count)| Collection {
            display_name: collection_display_name(&id),
            id,
            document_count,
        })
        .collect::<Vec<_>>();
    collections.sort_by(|left, right| {
        left.display_name
            .to_lowercase()
            .cmp(&right.display_name.to_lowercase())
    });
    collections
}

fn collection_display_name(id: &str) -> String {
    match id {
        "project" => String::from("Projects"),
        "person" => String::from("People"),
        "meeting" => String::from("Meetings"),
        "document" | "documents" => String::from("Documents"),
        value => {
            let mut chars = value.chars();
            match chars.next() {
                Some(first) => format!(
                    "{}{}s",
                    first.to_uppercase().collect::<String>(),
                    chars.collect::<String>()
                ),
                None => String::from("Documents"),
            }
        }
    }
}

fn should_ignore_dir(name: &OsString) -> bool {
    matches!(
        name.to_string_lossy().as_ref(),
        ".git" | "target" | "node_modules"
    )
}

pub fn is_markdown_path(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| {
            extension.eq_ignore_ascii_case("md") || extension.eq_ignore_ascii_case("markdown")
        })
        .unwrap_or(false)
}

fn should_rescan_path(root: &Path, path: &Path) -> bool {
    path.strip_prefix(root)
        .ok()
        .filter(|relative_path| !has_ignored_component(relative_path))
        .map(|relative_path| relative_path.extension().is_none())
        .unwrap_or(false)
}

fn has_ignored_component(path: &Path) -> bool {
    path.components().any(|component| match component {
        std::path::Component::Normal(name) => should_ignore_dir(&name.to_os_string()),
        _ => false,
    })
}

fn compare_paths(left: &Path, right: &Path) -> std::cmp::Ordering {
    left.to_string_lossy()
        .to_lowercase()
        .cmp(&right.to_string_lossy().to_lowercase())
}

#[cfg(test)]
mod tests {
    use super::{
        markdown_body_without_frontmatter, scan_workspace, workspace_update_from_events,
        PropertyValue, WorkspaceEvent,
    };

    use std::{
        fs,
        path::{Path, PathBuf},
        time::{SystemTime, UNIX_EPOCH},
    };

    #[test]
    fn finds_md_files() {
        let workspace = TempWorkspace::new();
        workspace.write("README.md", "");

        let result = scan_workspace(workspace.path()).unwrap();

        assert_eq!(result.documents.len(), 1);
        assert_eq!(
            result.documents[0].relative_path,
            PathBuf::from("README.md")
        );
    }

    #[test]
    fn finds_markdown_files() {
        let workspace = TempWorkspace::new();
        workspace.write("notes.markdown", "");

        let result = scan_workspace(workspace.path()).unwrap();

        assert_eq!(result.documents.len(), 1);
        assert_eq!(
            result.documents[0].relative_path,
            PathBuf::from("notes.markdown")
        );
    }

    #[test]
    fn scans_recursively() {
        let workspace = TempWorkspace::new();
        workspace.write("docs/product/ROADMAP.md", "");

        let result = scan_workspace(workspace.path()).unwrap();

        assert_eq!(result.documents.len(), 1);
        assert_eq!(
            result.documents[0].relative_path,
            PathBuf::from("docs/product/ROADMAP.md")
        );
    }

    #[test]
    fn ignores_non_markdown_files() {
        let workspace = TempWorkspace::new();
        workspace.write("notes.txt", "");
        workspace.write("README.md", "");

        let result = scan_workspace(workspace.path()).unwrap();

        assert_eq!(result.documents.len(), 1);
        assert_eq!(
            result.documents[0].relative_path,
            PathBuf::from("README.md")
        );
    }

    #[test]
    fn schema_file_event_sets_schema_changed_without_scanning_yaml_files() {
        let workspace = TempWorkspace::new();
        workspace.write("flokin.schema.yaml", "version: 1\ncollections: {}\n");
        workspace.write("other.yaml", "ignored: true\n");

        let schema_update = workspace_update_from_events(
            workspace.path(),
            &[WorkspaceEvent::Upsert(
                workspace.path().join("flokin.schema.yaml"),
            )],
        )
        .unwrap();
        assert!(schema_update.schema_changed);
        assert!(schema_update.upserts.is_empty());

        let other_update = workspace_update_from_events(
            workspace.path(),
            &[WorkspaceEvent::Upsert(workspace.path().join("other.yaml"))],
        )
        .unwrap();
        assert!(!other_update.schema_changed);
        assert!(other_update.upserts.is_empty());
    }

    #[test]
    fn ignores_git_directory() {
        let workspace = TempWorkspace::new();
        workspace.write(".git/ignored.md", "");
        workspace.write("kept.md", "");

        let result = scan_workspace(workspace.path()).unwrap();

        assert_eq!(relative_paths(&result), vec![PathBuf::from("kept.md")]);
    }

    #[test]
    fn ignores_target_directory() {
        let workspace = TempWorkspace::new();
        workspace.write("target/ignored.md", "");
        workspace.write("kept.md", "");

        let result = scan_workspace(workspace.path()).unwrap();

        assert_eq!(relative_paths(&result), vec![PathBuf::from("kept.md")]);
    }

    #[test]
    fn ignores_node_modules_directory() {
        let workspace = TempWorkspace::new();
        workspace.write("node_modules/ignored.md", "");
        workspace.write("kept.md", "");

        let result = scan_workspace(workspace.path()).unwrap();

        assert_eq!(relative_paths(&result), vec![PathBuf::from("kept.md")]);
    }

    #[test]
    fn empty_workspace_returns_zero_documents() {
        let workspace = TempWorkspace::new();

        let result = scan_workspace(workspace.path()).unwrap();

        assert!(result.documents.is_empty());
        assert!(result.directories.is_empty());
        assert!(result.collections.is_empty());
    }

    #[test]
    fn keeps_relative_path() {
        let workspace = TempWorkspace::new();
        workspace.write("docs/ARCHITECTURE.md", "");

        let result = scan_workspace(workspace.path()).unwrap();

        assert_eq!(
            result.documents[0].relative_path,
            PathBuf::from("docs/ARCHITECTURE.md")
        );
    }

    #[test]
    fn supports_unicode_paths() {
        let workspace = TempWorkspace::new();
        workspace.write("ações/visão.md", "");

        let result = scan_workspace(workspace.path()).unwrap();

        assert_eq!(
            result.documents[0].relative_path,
            PathBuf::from("ações/visão.md")
        );
    }

    #[test]
    fn detects_extensions_case_insensitively() {
        let workspace = TempWorkspace::new();
        workspace.write("notes.MD", "");

        let result = scan_workspace(workspace.path()).unwrap();

        assert_eq!(result.documents.len(), 1);
    }

    #[test]
    #[cfg(unix)]
    fn does_not_follow_symlink_by_default() {
        use std::os::unix::fs::symlink;

        let workspace = TempWorkspace::new();
        workspace.write("outside/linked.md", "");
        symlink(
            workspace.path().join("outside"),
            workspace.path().join("link-to-outside"),
        )
        .unwrap();

        let result = scan_workspace(workspace.path()).unwrap();

        assert_eq!(
            relative_paths(&result),
            vec![PathBuf::from("outside/linked.md")]
        );
    }

    #[test]
    fn returns_deterministic_ordering() {
        let workspace = TempWorkspace::new();
        workspace.write("b.md", "");
        workspace.write("A.md", "");
        workspace.write("docs/c.md", "");
        workspace.write("docs/B.md", "");

        let result = scan_workspace(workspace.path()).unwrap();

        assert_eq!(
            relative_paths(&result),
            vec![
                PathBuf::from("A.md"),
                PathBuf::from("b.md"),
                PathBuf::from("docs/B.md"),
                PathBuf::from("docs/c.md"),
            ]
        );
    }

    #[test]
    fn markdown_without_frontmatter_is_valid_document() {
        let workspace = TempWorkspace::new();
        workspace.write("note.md", "# Note\n");

        let document = only_document(&workspace);

        assert!(document.properties.is_empty());
        assert_eq!(document.title, "Note");
    }

    #[test]
    fn markdown_preview_body_keeps_markdown_without_frontmatter() {
        assert_eq!(
            markdown_body_without_frontmatter("# Title\nBody\n"),
            "# Title\nBody\n"
        );
    }

    #[test]
    fn markdown_preview_body_removes_frontmatter() {
        let source = "---\ntitle: CARF\ntype: project\n---\n\n# CARF\n";
        assert_eq!(markdown_body_without_frontmatter(source), "\n# CARF\n");
    }

    #[test]
    fn markdown_preview_body_removes_invalid_frontmatter_when_delimited() {
        let source = "---\ntitle: [broken\n---\n# Fallback\n";
        assert_eq!(markdown_body_without_frontmatter(source), "# Fallback\n");
    }

    #[test]
    fn markdown_preview_body_supports_markdown_shapes_unicode_and_empty_body() {
        let source = "---\ntitle: Teste\n---\n## Olá\n- item\n\n```rust\nfn main() {}\n```\n\n| Nome | Status |\n| --- | --- |\n| Ação | ativa |\n";
        let body = markdown_body_without_frontmatter(source);

        assert!(body.contains("## Olá"));
        assert!(body.contains("- item"));
        assert!(body.contains("```rust"));
        assert!(body.contains("| Nome | Status |"));
        assert_eq!(
            markdown_body_without_frontmatter("---\ntitle: Empty\n---\n"),
            ""
        );
    }

    #[test]
    fn parses_valid_yaml_frontmatter() {
        let workspace = TempWorkspace::new();
        workspace.write(
            "project.md",
            "---\ntitle: CARF\ntype: project\nstatus: active\n---\n# Ignored\n",
        );

        let document = only_document(&workspace);

        assert_eq!(document.title, "CARF");
        assert_eq!(
            document.properties.get("status"),
            Some(&PropertyValue::String(String::from("active")))
        );
    }

    #[test]
    fn invalid_yaml_does_not_fail_workspace() {
        let workspace = TempWorkspace::new();
        workspace.write("broken.md", "---\ntitle: [broken\n---\n# Fallback\n");

        let document = only_document(&workspace);

        assert_eq!(document.title, "Fallback");
        assert_eq!(document.warnings.len(), 1);
    }

    #[test]
    fn title_prefers_frontmatter_title() {
        let workspace = TempWorkspace::new();
        workspace.write(
            "project.md",
            "---\ntitle: Frontmatter Title\n---\n# H1 Title\n",
        );

        let document = only_document(&workspace);

        assert_eq!(document.title, "Frontmatter Title");
    }

    #[test]
    fn title_uses_first_h1() {
        let workspace = TempWorkspace::new();
        workspace.write("project.md", "Intro\n# H1 Title\n");

        let document = only_document(&workspace);

        assert_eq!(document.title, "H1 Title");
    }

    #[test]
    fn title_uses_filename() {
        let workspace = TempWorkspace::new();
        workspace.write("fallback-name.md", "No title\n");

        let document = only_document(&workspace);

        assert_eq!(document.title, "fallback-name");
    }

    #[test]
    fn type_uses_frontmatter() {
        let workspace = TempWorkspace::new();
        workspace.write("notes/carf.md", "---\ntype: Project\n---\n");

        let document = only_document(&workspace);

        assert_eq!(document.document_type.as_deref(), Some("project"));
        assert_eq!(document.collection_id, "project");
    }

    #[test]
    fn type_uses_parent_folder_fallback() {
        let workspace = TempWorkspace::new();
        workspace.write("people/sergio.md", "# Sergio\n");

        let document = only_document(&workspace);

        assert_eq!(document.document_type.as_deref(), Some("person"));
        assert_eq!(document.collection_id, "person");
    }

    #[test]
    fn document_without_type_goes_to_documents_collection() {
        let workspace = TempWorkspace::new();
        workspace.write("notes/random.md", "# Random\n");

        let document = only_document(&workspace);

        assert_eq!(document.document_type, None);
        assert_eq!(document.collection_id, "documents");
    }

    #[test]
    fn preserves_string_property() {
        let workspace = TempWorkspace::new();
        workspace.write("doc.md", "---\nowner: sergio\n---\n");

        let document = only_document(&workspace);

        assert_eq!(
            document.properties.get("owner"),
            Some(&PropertyValue::String(String::from("sergio")))
        );
    }

    #[test]
    fn preserves_number_property() {
        let workspace = TempWorkspace::new();
        workspace.write("doc.md", "---\nscore: 42\n---\n");

        let document = only_document(&workspace);

        assert_eq!(
            document.properties.get("score"),
            Some(&PropertyValue::Number(String::from("42")))
        );
    }

    #[test]
    fn preserves_bool_property() {
        let workspace = TempWorkspace::new();
        workspace.write("doc.md", "---\nactive: true\n---\n");

        let document = only_document(&workspace);

        assert_eq!(
            document.properties.get("active"),
            Some(&PropertyValue::Bool(true))
        );
    }

    #[test]
    fn preserves_array_property() {
        let workspace = TempWorkspace::new();
        workspace.write("doc.md", "---\ntags:\n  - jota\n  - python\n---\n");

        let document = only_document(&workspace);

        assert_eq!(
            document.properties.get("tags"),
            Some(&PropertyValue::Array(vec![
                PropertyValue::String(String::from("jota")),
                PropertyValue::String(String::from("python")),
            ]))
        );
    }

    #[test]
    fn documents_with_same_type_form_collection() {
        let workspace = TempWorkspace::new();
        workspace.write("projects/carf.md", "---\ntype: project\n---\n");
        workspace.write("projects/cvm.md", "---\ntype: projects\n---\n");

        let result = scan_workspace(workspace.path()).unwrap();

        assert_eq!(result.collections.len(), 1);
        assert_eq!(result.collections[0].id, "project");
        assert_eq!(result.collections[0].display_name, "Projects");
        assert_eq!(result.collections[0].document_count, 2);
    }

    #[test]
    fn collection_ordering_is_deterministic() {
        let workspace = TempWorkspace::new();
        workspace.write("projects/carf.md", "---\ntype: project\n---\n");
        workspace.write("people/sergio.md", "---\ntype: person\n---\n");
        workspace.write("meetings/daily.md", "---\ntype: meeting\n---\n");

        let result = scan_workspace(workspace.path()).unwrap();

        assert_eq!(
            result
                .collections
                .iter()
                .map(|collection| collection.display_name.as_str())
                .collect::<Vec<_>>(),
            vec!["Meetings", "People", "Projects"]
        );
    }

    #[test]
    fn supports_unicode_title_and_path() {
        let workspace = TempWorkspace::new();
        workspace.write("ações/visão.md", "# Visão Geral\n");

        let document = only_document(&workspace);

        assert_eq!(document.title, "Visão Geral");
        assert_eq!(document.relative_path, PathBuf::from("ações/visão.md"));
    }

    #[test]
    fn empty_file_is_valid_document() {
        let workspace = TempWorkspace::new();
        workspace.write("empty.md", "");

        let document = only_document(&workspace);

        assert_eq!(document.title, "empty");
        assert!(document.properties.is_empty());
        assert_eq!(document.source_content.as_deref(), Some(""));
    }

    #[test]
    fn source_content_preserves_full_markdown_with_frontmatter() {
        let workspace = TempWorkspace::new();
        let content = "---\ntitle: CARF Daily\ntype: meeting\n---\n\n# CARF Daily\n";
        workspace.write("meetings/carf.md", content);

        let document = only_document(&workspace);

        assert_eq!(document.source_content.as_deref(), Some(content));
        assert_eq!(document.markdown_content, "\n# CARF Daily\n");
    }

    #[test]
    fn source_content_preserves_unicode() {
        let workspace = TempWorkspace::new();
        let content = "---\ntitle: Ação\n---\n\n# Visão\nConteúdo com acento.\n";
        workspace.write("ações/visão.md", content);

        let document = only_document(&workspace);

        assert_eq!(document.source_content.as_deref(), Some(content));
    }

    #[test]
    fn records_filesystem_metadata_for_document() {
        let workspace = TempWorkspace::new();
        workspace.write("note.md", "12345");

        let document = only_document(&workspace);

        assert_eq!(document.metadata.file_size, Some(5));
        assert!(document.metadata.modified.is_some());
    }

    #[test]
    fn remove_then_upsert_existing_file_converges_to_upsert() {
        let workspace = TempWorkspace::new();
        workspace.write("doc.md", "# Final\n");
        let path = workspace.path().join("doc.md");

        let update = workspace_update_from_events(
            workspace.path(),
            &[
                WorkspaceEvent::Remove(path.clone()),
                WorkspaceEvent::Upsert(path.clone()),
            ],
        )
        .unwrap();

        assert!(update.removals.is_empty());
        assert_eq!(update.upserts.len(), 1);
        assert_eq!(
            update.upserts[0].source_content.as_deref(),
            Some("# Final\n")
        );
    }

    #[test]
    fn upsert_then_remove_existing_file_converges_to_upsert() {
        let workspace = TempWorkspace::new();
        workspace.write("doc.md", "# Final\n");
        let path = workspace.path().join("doc.md");

        let update = workspace_update_from_events(
            workspace.path(),
            &[
                WorkspaceEvent::Upsert(path.clone()),
                WorkspaceEvent::Remove(path.clone()),
            ],
        )
        .unwrap();

        assert!(update.removals.is_empty());
        assert_eq!(update.upserts.len(), 1);
        assert_eq!(
            update.upserts[0].source_content.as_deref(),
            Some("# Final\n")
        );
    }

    #[test]
    fn real_delete_converges_to_removal() {
        let workspace = TempWorkspace::new();
        workspace.write("doc.md", "# Removed\n");
        let path = workspace.path().join("doc.md");
        fs::remove_file(&path).unwrap();

        let update =
            workspace_update_from_events(workspace.path(), &[WorkspaceEvent::Remove(path.clone())])
                .unwrap();

        assert!(update.upserts.is_empty());
        assert_eq!(update.removals, vec![path]);
    }

    #[test]
    fn event_storm_converges_to_final_existing_file_once() {
        let workspace = TempWorkspace::new();
        workspace.write("doc.md", "# Final\n");
        let path = workspace.path().join("doc.md");

        let update = workspace_update_from_events(
            workspace.path(),
            &[
                WorkspaceEvent::Upsert(path.clone()),
                WorkspaceEvent::Upsert(path.clone()),
                WorkspaceEvent::Remove(path.clone()),
                WorkspaceEvent::Upsert(path.clone()),
            ],
        )
        .unwrap();

        assert!(update.removals.is_empty());
        assert_eq!(update.upserts.len(), 1);
        assert_eq!(
            update.upserts[0].source_content.as_deref(),
            Some("# Final\n")
        );
    }

    fn relative_paths(result: &super::ScanResult) -> Vec<PathBuf> {
        result
            .documents
            .iter()
            .map(|document| document.relative_path.clone())
            .collect()
    }

    fn only_document(workspace: &TempWorkspace) -> super::Document {
        let result = scan_workspace(workspace.path()).unwrap();
        assert_eq!(result.documents.len(), 1);
        result.documents.into_iter().next().unwrap()
    }

    struct TempWorkspace {
        path: PathBuf,
    }

    impl TempWorkspace {
        fn new() -> Self {
            let mut path = std::env::temp_dir();
            let unique = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            path.push(format!("flokin-md-scanner-{}-{unique}", std::process::id()));
            fs::create_dir(&path).unwrap();
            Self { path }
        }

        fn path(&self) -> &Path {
            &self.path
        }

        fn write(&self, relative_path: &str, content: &str) {
            let path = self.path.join(relative_path);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(path, content).unwrap();
        }
    }

    impl Drop for TempWorkspace {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }
}
