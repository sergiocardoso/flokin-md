use std::{
    collections::{BTreeMap, BTreeSet},
    fs, io,
    path::{Path, PathBuf},
};

use crate::{Collection, Document, PropertyValue, RelationIndex};

pub const SCHEMA_FILE_NAME: &str = "flokin.schema.yaml";

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SchemaCatalog {
    pub collections: Vec<CollectionSchema>,
    pub explicit_schema: ExplicitSchemaState,
    pub warnings: Vec<SchemaWarning>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CollectionSchema {
    pub collection_id: String,
    pub display_name: String,
    pub document_count: usize,
    pub source: SchemaSource,
    pub fields: Vec<SchemaField>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchemaSource {
    Inferred,
    Explicit,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaField {
    pub name: String,
    pub field_type: SchemaType,
    pub inferred_type: SchemaType,
    pub declared_type: Option<SchemaType>,
    pub required: bool,
    pub nullable: bool,
    pub observed_count: usize,
    pub null_count: usize,
    pub total_documents: usize,
    pub observed_types: Vec<ObservedSchemaType>,
    pub structural: bool,
    pub declared: bool,
    pub divergent: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SchemaType {
    String,
    Integer,
    Float,
    Boolean,
    Array,
    Object,
    Relation,
    Mixed,
    Null,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservedSchemaType {
    pub field_type: SchemaType,
    pub count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ExplicitSchemaState {
    #[default]
    Absent,
    Loaded(ExplicitSchema),
    Invalid(SchemaWarning),
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ExplicitSchema {
    pub version: u32,
    pub collections: BTreeMap<String, ExplicitCollectionSchema>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ExplicitCollectionSchema {
    pub fields: BTreeMap<String, ExplicitFieldSchema>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExplicitFieldSchema {
    pub field_type: SchemaType,
    pub required: bool,
    pub target: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaWarning {
    pub path: PathBuf,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneratedExplicitSchema {
    pub yaml: String,
    pub omitted_fields: Vec<GeneratedSchemaOmittedField>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneratedSchemaOmittedField {
    pub collection_id: String,
    pub field_name: String,
    pub field_type: SchemaType,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SchemaGenerationError {
    Empty,
    Serialize(String),
}

impl SchemaCatalog {
    pub fn build(
        documents: &[Document],
        collections: &[Collection],
        relation_index: &RelationIndex,
        explicit_schema: ExplicitSchemaState,
    ) -> Self {
        let loaded = match &explicit_schema {
            ExplicitSchemaState::Loaded(schema) => Some(schema),
            _ => None,
        };
        let mut collection_ids = collections
            .iter()
            .map(|collection| collection.id.clone())
            .collect::<BTreeSet<_>>();
        if let Some(schema) = loaded {
            collection_ids.extend(schema.collections.keys().cloned());
        }

        let collections = collection_ids
            .into_iter()
            .map(|collection_id| {
                let collection = collections
                    .iter()
                    .find(|collection| collection.id == collection_id);
                let display_name = collection
                    .map(|collection| collection.display_name.clone())
                    .unwrap_or_else(|| collection_display_name(&collection_id));
                let document_count = collection
                    .map(|collection| collection.document_count)
                    .unwrap_or(0);
                let explicit_collection =
                    loaded.and_then(|schema| schema.collections.get(&collection_id));
                let fields = infer_collection_fields(
                    &collection_id,
                    documents,
                    relation_index,
                    explicit_collection,
                    document_count,
                );
                CollectionSchema {
                    collection_id,
                    display_name,
                    document_count,
                    source: if explicit_collection.is_some() {
                        SchemaSource::Explicit
                    } else {
                        SchemaSource::Inferred
                    },
                    fields,
                }
            })
            .collect();

        let warnings = match &explicit_schema {
            ExplicitSchemaState::Invalid(warning) => vec![warning.clone()],
            ExplicitSchemaState::Absent | ExplicitSchemaState::Loaded(_) => Vec::new(),
        };

        Self {
            collections,
            explicit_schema,
            warnings,
        }
    }

    pub fn collection(&self, collection_id: &str) -> Option<&CollectionSchema> {
        self.collections
            .iter()
            .find(|collection| collection.collection_id == collection_id)
    }
}

impl SchemaType {
    pub const fn label(self) -> &'static str {
        match self {
            Self::String => "String",
            Self::Integer => "Integer",
            Self::Float => "Float",
            Self::Boolean => "Boolean",
            Self::Array => "Array",
            Self::Object => "Object",
            Self::Relation => "Relation",
            Self::Mixed => "Mixed",
            Self::Null => "Null",
            Self::Unknown => "Unknown",
        }
    }

    fn explicit_from_str(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "string" => Some(Self::String),
            "integer" => Some(Self::Integer),
            "float" => Some(Self::Float),
            "boolean" => Some(Self::Boolean),
            "array" => Some(Self::Array),
            "object" => Some(Self::Object),
            "relation" => Some(Self::Relation),
            "mixed" => Some(Self::Mixed),
            _ => None,
        }
    }
}

pub fn load_explicit_schema(root: &Path) -> ExplicitSchemaState {
    let path = schema_path(root);
    match fs::read_to_string(&path) {
        Ok(content) => parse_explicit_schema(&path, &content)
            .map(ExplicitSchemaState::Loaded)
            .unwrap_or_else(ExplicitSchemaState::Invalid),
        Err(error) if error.kind() == io::ErrorKind::NotFound => ExplicitSchemaState::Absent,
        Err(error) => ExplicitSchemaState::Invalid(SchemaWarning {
            path,
            message: format!("Não foi possível carregar flokin.schema.yaml: {error}"),
        }),
    }
}

pub fn is_workspace_schema_path(root: &Path, path: &Path) -> bool {
    path.strip_prefix(root)
        .map(|relative| relative == Path::new(SCHEMA_FILE_NAME))
        .unwrap_or(false)
}

pub fn schema_path(root: &Path) -> PathBuf {
    root.join(SCHEMA_FILE_NAME)
}

pub fn generate_explicit_schema(
    catalog: &SchemaCatalog,
) -> Result<GeneratedExplicitSchema, SchemaGenerationError> {
    let collections = catalog
        .collections
        .iter()
        .filter(|collection| collection.document_count > 0)
        .collect::<Vec<_>>();
    if collections.is_empty() {
        return Err(SchemaGenerationError::Empty);
    }

    let mut omitted_fields = Vec::new();
    let mut root = serde_yaml::Mapping::new();
    root.insert(
        serde_yaml::Value::String(String::from("version")),
        serde_yaml::Value::Number(1.into()),
    );

    let mut collections_mapping = serde_yaml::Mapping::new();
    for collection in collections {
        let mut fields_mapping = serde_yaml::Mapping::new();
        for field in &collection.fields {
            let Some(type_name) = generated_schema_type(field.field_type) else {
                omitted_fields.push(GeneratedSchemaOmittedField {
                    collection_id: collection.collection_id.clone(),
                    field_name: field.name.clone(),
                    field_type: field.field_type,
                });
                continue;
            };

            let mut field_mapping = serde_yaml::Mapping::new();
            field_mapping.insert(
                serde_yaml::Value::String(String::from("type")),
                serde_yaml::Value::String(type_name.to_owned()),
            );
            field_mapping.insert(
                serde_yaml::Value::String(String::from("required")),
                serde_yaml::Value::Bool(field.required),
            );
            fields_mapping.insert(
                serde_yaml::Value::String(field.name.clone()),
                serde_yaml::Value::Mapping(field_mapping),
            );
        }

        let mut collection_mapping = serde_yaml::Mapping::new();
        collection_mapping.insert(
            serde_yaml::Value::String(String::from("fields")),
            serde_yaml::Value::Mapping(fields_mapping),
        );
        collections_mapping.insert(
            serde_yaml::Value::String(collection.collection_id.clone()),
            serde_yaml::Value::Mapping(collection_mapping),
        );
    }

    root.insert(
        serde_yaml::Value::String(String::from("collections")),
        serde_yaml::Value::Mapping(collections_mapping),
    );
    let mut yaml = serde_yaml::to_string(&serde_yaml::Value::Mapping(root))
        .map_err(|error| SchemaGenerationError::Serialize(error.to_string()))?;
    if !yaml.ends_with('\n') {
        yaml.push('\n');
    }
    Ok(GeneratedExplicitSchema {
        yaml,
        omitted_fields,
    })
}

fn generated_schema_type(field_type: SchemaType) -> Option<&'static str> {
    match field_type {
        SchemaType::String => Some("string"),
        SchemaType::Integer => Some("integer"),
        SchemaType::Float => Some("float"),
        SchemaType::Boolean => Some("boolean"),
        SchemaType::Array => Some("array"),
        SchemaType::Object => Some("object"),
        SchemaType::Relation => Some("relation"),
        SchemaType::Mixed | SchemaType::Null | SchemaType::Unknown => None,
    }
}

fn parse_explicit_schema(path: &Path, content: &str) -> Result<ExplicitSchema, SchemaWarning> {
    let yaml =
        serde_yaml::from_str::<serde_yaml::Value>(content).map_err(|error| SchemaWarning {
            path: path.to_path_buf(),
            message: format!("Não foi possível carregar flokin.schema.yaml: {error}"),
        })?;
    let mapping = yaml.as_mapping().ok_or_else(|| SchemaWarning {
        path: path.to_path_buf(),
        message: String::from("Não foi possível carregar flokin.schema.yaml: raiz inválida."),
    })?;

    let version = mapping
        .get(serde_yaml::Value::String(String::from("version")))
        .and_then(serde_yaml::Value::as_u64)
        .unwrap_or(1);
    if version != 1 {
        return Err(SchemaWarning {
            path: path.to_path_buf(),
            message: format!(
                "Não foi possível carregar flokin.schema.yaml: versão {version} incompatível."
            ),
        });
    }

    let mut schema = ExplicitSchema {
        version: version as u32,
        collections: BTreeMap::new(),
    };
    let Some(collections) = mapping
        .get(serde_yaml::Value::String(String::from("collections")))
        .and_then(serde_yaml::Value::as_mapping)
    else {
        return Ok(schema);
    };

    for (collection_key, collection_value) in collections {
        let Some(collection_id) = collection_key.as_str() else {
            continue;
        };
        let Some(collection_map) = collection_value.as_mapping() else {
            continue;
        };
        let mut collection = ExplicitCollectionSchema::default();
        if let Some(fields) = collection_map
            .get(serde_yaml::Value::String(String::from("fields")))
            .and_then(serde_yaml::Value::as_mapping)
        {
            for (field_key, field_value) in fields {
                let Some(field_name) = field_key.as_str() else {
                    continue;
                };
                let field_map = field_value.as_mapping().ok_or_else(|| SchemaWarning {
                    path: path.to_path_buf(),
                    message: format!(
                        "Não foi possível carregar flokin.schema.yaml: campo {field_name} inválido."
                    ),
                })?;
                let field_type = field_map
                    .get(serde_yaml::Value::String(String::from("type")))
                    .and_then(serde_yaml::Value::as_str)
                    .and_then(SchemaType::explicit_from_str)
                    .ok_or_else(|| SchemaWarning {
                        path: path.to_path_buf(),
                        message: format!(
                            "Não foi possível carregar flokin.schema.yaml: tipo inválido em {field_name}."
                        ),
                    })?;
                let required = field_map
                    .get(serde_yaml::Value::String(String::from("required")))
                    .and_then(serde_yaml::Value::as_bool)
                    .unwrap_or(false);
                let target = field_map
                    .get(serde_yaml::Value::String(String::from("target")))
                    .and_then(serde_yaml::Value::as_str)
                    .map(ToOwned::to_owned);
                collection.fields.insert(
                    field_name.to_owned(),
                    ExplicitFieldSchema {
                        field_type,
                        required,
                        target,
                    },
                );
            }
        }
        schema
            .collections
            .insert(normalize_collection_id(collection_id), collection);
    }

    Ok(schema)
}

fn infer_collection_fields(
    collection_id: &str,
    documents: &[Document],
    relation_index: &RelationIndex,
    explicit_collection: Option<&ExplicitCollectionSchema>,
    document_count: usize,
) -> Vec<SchemaField> {
    let documents = documents
        .iter()
        .filter(|document| document.collection_id == collection_id)
        .collect::<Vec<_>>();
    let mut stats = BTreeMap::<String, FieldStats>::new();

    let title_stats = stats.entry(String::from("title")).or_default();
    title_stats.present = documents.len();
    title_stats
        .observed
        .insert(SchemaType::String, documents.len());

    for document in &documents {
        for (property, value) in &document.properties {
            if property == "type" {
                continue;
            }
            if property == "title" {
                continue;
            }
            let field_type = infer_value_type(document, property, value, relation_index);
            let field_stats = stats.entry(property.clone()).or_default();
            field_stats.present += 1;
            if field_type == SchemaType::Null {
                field_stats.null_count += 1;
            }
            *field_stats.observed.entry(field_type).or_default() += 1;
        }
    }

    if let Some(explicit_collection) = explicit_collection {
        for field in explicit_collection.fields.keys() {
            stats.entry(field.clone()).or_default();
        }
    }

    let mut fields = stats
        .into_iter()
        .map(|(name, field_stats)| {
            let declared = explicit_collection.and_then(|collection| collection.fields.get(&name));
            let inferred_type = field_stats.inferred_type();
            let field_type = declared
                .map(|field| field.field_type)
                .unwrap_or(inferred_type);
            let mut observed_types = field_stats
                .observed
                .into_iter()
                .map(|(field_type, count)| ObservedSchemaType { field_type, count })
                .collect::<Vec<_>>();
            observed_types.sort_by(|left, right| {
                left.field_type
                    .label()
                    .cmp(right.field_type.label())
                    .then_with(|| left.field_type.cmp(&right.field_type))
            });
            let divergent = declared
                .map(|field| has_divergence(field.field_type, &observed_types))
                .unwrap_or(inferred_type == SchemaType::Mixed);
            let structural = name == "title" && field_stats.present == documents.len();
            SchemaField {
                required: if structural {
                    true
                } else {
                    declared
                        .map(|field| field.required)
                        .unwrap_or(document_count > 0 && field_stats.present == document_count)
                },
                nullable: !structural && field_stats.null_count > 0,
                observed_count: field_stats.present,
                null_count: if structural {
                    0
                } else {
                    field_stats.null_count
                },
                total_documents: document_count,
                structural,
                declared: declared.is_some(),
                declared_type: declared.map(|field| field.field_type),
                name,
                field_type: if structural {
                    SchemaType::String
                } else {
                    field_type
                },
                inferred_type: if structural {
                    SchemaType::String
                } else {
                    inferred_type
                },
                observed_types,
                divergent,
            }
        })
        .collect::<Vec<_>>();

    fields.sort_by(|left, right| {
        (left.name != "title")
            .cmp(&(right.name != "title"))
            .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
            .then_with(|| left.name.cmp(&right.name))
    });
    fields
}

#[derive(Default)]
struct FieldStats {
    present: usize,
    null_count: usize,
    observed: BTreeMap<SchemaType, usize>,
}

impl FieldStats {
    fn inferred_type(&self) -> SchemaType {
        let non_null = self
            .observed
            .keys()
            .copied()
            .filter(|field_type| *field_type != SchemaType::Null)
            .collect::<Vec<_>>();
        match non_null.as_slice() {
            [] if self.observed.contains_key(&SchemaType::Null) => SchemaType::Null,
            [] => SchemaType::Unknown,
            [field_type] => *field_type,
            _ => SchemaType::Mixed,
        }
    }
}

fn infer_value_type(
    document: &Document,
    property: &str,
    value: &PropertyValue,
    relation_index: &RelationIndex,
) -> SchemaType {
    schema_type_for_property_value(document, property, value, relation_index)
}

pub fn schema_type_for_property_value(
    document: &Document,
    property: &str,
    value: &PropertyValue,
    relation_index: &RelationIndex,
) -> SchemaType {
    if relation_index
        .outgoing(&document.path)
        .iter()
        .any(|relation| relation.property == property)
    {
        return SchemaType::Relation;
    }
    match value {
        PropertyValue::Null => SchemaType::Null,
        PropertyValue::Bool(_) => SchemaType::Boolean,
        PropertyValue::Number(value) if value.parse::<i64>().is_ok() => SchemaType::Integer,
        PropertyValue::Number(value) if value.parse::<f64>().is_ok() => SchemaType::Float,
        PropertyValue::Number(_) | PropertyValue::String(_) => SchemaType::String,
        PropertyValue::Array(_) => SchemaType::Array,
        PropertyValue::Object(_) => SchemaType::Object,
    }
}

fn has_divergence(declared: SchemaType, observed_types: &[ObservedSchemaType]) -> bool {
    observed_types
        .iter()
        .filter(|observed| observed.field_type != SchemaType::Null)
        .any(|observed| observed.field_type != declared)
}

fn normalize_collection_id(value: &str) -> String {
    let value = value.trim().to_ascii_lowercase().replace(['_', ' '], "-");
    match value.as_str() {
        "projects" => String::from("project"),
        "people" | "persons" => String::from("person"),
        "meetings" => String::from("meeting"),
        "documents" | "docs" => String::from("document"),
        _ => value.trim_end_matches('s').to_owned(),
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn doc(collection_id: &str, title: &str, properties: &[(&str, PropertyValue)]) -> Document {
        Document {
            path: PathBuf::from(format!("{collection_id}/{title}.md")),
            relative_path: PathBuf::from(format!("{collection_id}/{title}.md")),
            file_name: format!("{title}.md").into(),
            metadata: crate::DocumentMetadata {
                file_size: None,
                modified: None,
            },
            title: title.to_owned(),
            source_content: None,
            markdown_content: String::new(),
            properties: properties
                .iter()
                .map(|(key, value)| ((*key).to_owned(), value.clone()))
                .collect(),
            document_type: Some(collection_id.to_owned()),
            collection_id: collection_id.to_owned(),
            warnings: Vec::new(),
        }
    }

    fn collection(id: &str, count: usize) -> Collection {
        Collection {
            id: id.to_owned(),
            display_name: collection_display_name(id),
            document_count: count,
        }
    }

    fn catalog(documents: &[Document], collections: &[Collection]) -> SchemaCatalog {
        let index = RelationIndex::build(documents);
        SchemaCatalog::build(documents, collections, &index, ExplicitSchemaState::Absent)
    }

    fn field<'a>(schema: &'a CollectionSchema, name: &str) -> &'a SchemaField {
        schema
            .fields
            .iter()
            .find(|field| field.name == name)
            .unwrap()
    }

    #[test]
    fn infers_basic_types_relation_null_and_missing() {
        let docs = vec![
            doc(
                "project",
                "A",
                &[
                    ("status", PropertyValue::String(String::from("active"))),
                    ("priority", PropertyValue::Number(String::from("10"))),
                    ("budget", PropertyValue::Number(String::from("10.5"))),
                    ("published", PropertyValue::Bool(true)),
                    (
                        "tags",
                        PropertyValue::Array(vec![PropertyValue::String(String::from("rust"))]),
                    ),
                    (
                        "metadata",
                        PropertyValue::Object(BTreeMap::from([(
                            String::from("foo"),
                            PropertyValue::String(String::from("bar")),
                        )])),
                    ),
                    ("owner", PropertyValue::String(String::from("[[Sergio]]"))),
                    ("plain_owner", PropertyValue::String(String::from("Sergio"))),
                    ("nullable", PropertyValue::Null),
                ],
            ),
            doc(
                "project",
                "B",
                &[
                    ("status", PropertyValue::String(String::from("active"))),
                    ("priority", PropertyValue::Number(String::from("11"))),
                    ("nullable", PropertyValue::String(String::from("ok"))),
                ],
            ),
            doc(
                "project",
                "Sergio",
                &[("status", PropertyValue::String(String::from("active")))],
            ),
        ];
        let catalog = catalog(&docs, &[collection("project", 3)]);
        let schema = catalog.collection("project").unwrap();

        assert_eq!(field(schema, "title").field_type, SchemaType::String);
        assert!(field(schema, "title").required);
        assert!(field(schema, "title").structural);
        assert_eq!(field(schema, "status").field_type, SchemaType::String);
        assert!(field(schema, "status").required);
        assert_eq!(field(schema, "priority").field_type, SchemaType::Integer);
        assert!(!field(schema, "priority").required);
        assert_eq!(field(schema, "priority").observed_count, 2);
        assert_eq!(field(schema, "budget").field_type, SchemaType::Float);
        assert_eq!(field(schema, "published").field_type, SchemaType::Boolean);
        assert_eq!(field(schema, "tags").field_type, SchemaType::Array);
        assert_eq!(field(schema, "metadata").field_type, SchemaType::Object);
        assert_eq!(field(schema, "owner").field_type, SchemaType::Relation);
        assert_eq!(field(schema, "plain_owner").field_type, SchemaType::String);
        assert!(field(schema, "nullable").nullable);
        assert_eq!(field(schema, "nullable").null_count, 1);
    }

    #[test]
    fn distinguishes_yaml_scalar_types_without_field_name_special_cases() {
        let docs = vec![doc(
            "project",
            "A",
            &[
                ("integer_value", PropertyValue::Number(String::from("10"))),
                ("float_value", PropertyValue::Number(String::from("10.5"))),
                ("quoted_number", PropertyValue::String(String::from("10"))),
                ("plain_string", PropertyValue::String(String::from("high"))),
                ("boolean_value", PropertyValue::Bool(true)),
            ],
        )];
        let catalog = catalog(&docs, &[collection("project", 1)]);
        let schema = catalog.collection("project").unwrap();

        assert_eq!(
            field(schema, "integer_value").field_type,
            SchemaType::Integer
        );
        assert_eq!(field(schema, "float_value").field_type, SchemaType::Float);
        assert_eq!(
            field(schema, "quoted_number").field_type,
            SchemaType::String
        );
        assert_eq!(field(schema, "plain_string").field_type, SchemaType::String);
        assert_eq!(
            field(schema, "boolean_value").field_type,
            SchemaType::Boolean
        );
    }

    #[test]
    fn detects_mixed_types_and_deterministic_field_order() {
        let docs = vec![
            doc(
                "project",
                "B",
                &[("priority", PropertyValue::Number(String::from("10")))],
            ),
            doc(
                "project",
                "A",
                &[("priority", PropertyValue::String(String::from("high")))],
            ),
        ];
        let first_catalog = catalog(&docs, &[collection("project", 2)]);
        let schema = first_catalog.collection("project").unwrap();

        assert_eq!(
            schema
                .fields
                .iter()
                .map(|field| field.name.as_str())
                .collect::<Vec<_>>(),
            vec!["title", "priority"]
        );
        let priority = field(schema, "priority");
        assert_eq!(priority.field_type, SchemaType::Mixed);
        assert!(priority.divergent);
        assert_eq!(
            priority
                .observed_types
                .iter()
                .map(|observed| (observed.field_type, observed.count))
                .collect::<Vec<_>>(),
            vec![(SchemaType::Integer, 1), (SchemaType::String, 1)]
        );
    }

    #[test]
    fn null_counts_as_present_but_missing_does_not() {
        let docs = vec![
            doc(
                "project",
                "A",
                &[("status", PropertyValue::String(String::from("active")))],
            ),
            doc("project", "B", &[("status", PropertyValue::Null)]),
        ];
        let second_catalog = catalog(&docs, &[collection("project", 2)]);
        let schema = second_catalog.collection("project").unwrap();
        let status = field(schema, "status");

        assert!(status.required);
        assert!(status.nullable);
        assert_eq!(status.observed_count, 2);
        assert_eq!(status.null_count, 1);

        let docs = vec![
            doc(
                "project",
                "A",
                &[("status", PropertyValue::String(String::from("active")))],
            ),
            doc("project", "B", &[]),
        ];
        let catalog = catalog(&docs, &[collection("project", 2)]);
        let schema = catalog.collection("project").unwrap();
        let status = field(schema, "status");

        assert!(!status.required);
        assert!(!status.nullable);
        assert_eq!(status.observed_count, 1);
        assert_eq!(status.null_count, 0);
    }

    #[test]
    fn relation_inference_comes_from_relation_index_for_strings_and_arrays() {
        let docs = vec![
            doc(
                "project",
                "A",
                &[
                    ("owner", PropertyValue::String(String::from("[[Sergio]]"))),
                    ("plain_owner", PropertyValue::String(String::from("Sergio"))),
                    (
                        "participants",
                        PropertyValue::Array(vec![
                            PropertyValue::String(String::from("[[Sergio]]")),
                            PropertyValue::String(String::from("visitante")),
                        ]),
                    ),
                ],
            ),
            doc("project", "Sergio", &[]),
        ];
        let catalog = catalog(&docs, &[collection("project", 2)]);
        let schema = catalog.collection("project").unwrap();

        assert_eq!(field(schema, "owner").field_type, SchemaType::Relation);
        assert_eq!(field(schema, "plain_owner").field_type, SchemaType::String);
        assert_eq!(
            field(schema, "participants").field_type,
            SchemaType::Relation
        );
    }

    #[test]
    fn keeps_collection_schemas_independent_and_unicode_values() {
        let docs = vec![
            doc(
                "project",
                "A",
                &[("ação", PropertyValue::String(String::from("café")))],
            ),
            doc("person", "Sergio", &[("ação", PropertyValue::Bool(true))]),
        ];
        let catalog = catalog(&docs, &[collection("person", 1), collection("project", 1)]);

        assert_eq!(
            field(catalog.collection("project").unwrap(), "ação").field_type,
            SchemaType::String
        );
        assert_eq!(
            field(catalog.collection("person").unwrap(), "ação").field_type,
            SchemaType::Boolean
        );
    }

    #[test]
    fn explicit_schema_loads_and_preserves_extra_inferred_properties_and_divergence() {
        let docs = vec![doc(
            "project",
            "A",
            &[
                ("priority", PropertyValue::String(String::from("high"))),
                ("extra", PropertyValue::Bool(true)),
            ],
        )];
        let explicit = ExplicitSchema {
            version: 1,
            collections: BTreeMap::from([(
                String::from("project"),
                ExplicitCollectionSchema {
                    fields: BTreeMap::from([(
                        String::from("priority"),
                        ExplicitFieldSchema {
                            field_type: SchemaType::Integer,
                            required: false,
                            target: None,
                        },
                    )]),
                },
            )]),
        };
        let index = RelationIndex::build(&docs);
        let catalog = SchemaCatalog::build(
            &docs,
            &[collection("project", 1)],
            &index,
            ExplicitSchemaState::Loaded(explicit),
        );
        let schema = catalog.collection("project").unwrap();

        assert_eq!(schema.source, SchemaSource::Explicit);
        assert_eq!(field(schema, "priority").field_type, SchemaType::Integer);
        assert_eq!(field(schema, "priority").inferred_type, SchemaType::String);
        assert!(field(schema, "priority").divergent);
        assert!(!field(schema, "extra").declared);
    }

    #[test]
    fn parses_explicit_schema_file_and_errors() {
        let valid = parse_explicit_schema(
            Path::new("flokin.schema.yaml"),
            "version: 1\ncollections:\n  projects:\n    fields:\n      status:\n        type: string\n        required: true\n      priority:\n        type: integer\n        required: false\n      budget:\n        type: float\n      published:\n        type: boolean\n      tags:\n        type: array\n      metadata:\n        type: object\n      owner:\n        type: relation\n        target: people\n",
        )
        .unwrap();

        let project = valid.collections.get("project").unwrap();
        assert_eq!(valid.version, 1);
        assert!(project.fields["status"].required);
        assert_eq!(project.fields["status"].field_type, SchemaType::String);
        assert_eq!(project.fields["priority"].field_type, SchemaType::Integer);
        assert_eq!(project.fields["budget"].field_type, SchemaType::Float);
        assert_eq!(project.fields["published"].field_type, SchemaType::Boolean);
        assert_eq!(project.fields["tags"].field_type, SchemaType::Array);
        assert_eq!(project.fields["metadata"].field_type, SchemaType::Object);
        assert_eq!(project.fields["owner"].field_type, SchemaType::Relation);
        assert_eq!(project.fields["owner"].target.as_deref(), Some("people"));

        assert!(parse_explicit_schema(Path::new("x"), "not: [valid").is_err());
        assert!(parse_explicit_schema(Path::new("x"), "version: 2\ncollections: {}\n").is_err());
        assert!(parse_explicit_schema(
            Path::new("x"),
            "version: 1\ncollections:\n  projects:\n    fields:\n      bad:\n        type: mystery\n"
        )
            .is_err());
    }

    #[test]
    fn generates_explicit_schema_from_inferred_schema_and_roundtrips() {
        let docs = vec![
            doc(
                "project",
                "A",
                &[
                    ("status", PropertyValue::String(String::from("active"))),
                    ("priority", PropertyValue::Number(String::from("10"))),
                    ("budget", PropertyValue::Number(String::from("10.5"))),
                    ("published", PropertyValue::Bool(true)),
                    (
                        "tags",
                        PropertyValue::Array(vec![PropertyValue::String(String::from("rust"))]),
                    ),
                    (
                        "metadata",
                        PropertyValue::Object(BTreeMap::from([(
                            String::from("foo"),
                            PropertyValue::String(String::from("bar")),
                        )])),
                    ),
                    ("owner", PropertyValue::String(String::from("[[Sergio]]"))),
                    ("mixed", PropertyValue::Number(String::from("1"))),
                ],
            ),
            doc(
                "project",
                "B",
                &[
                    ("status", PropertyValue::String(String::from("paused"))),
                    ("priority", PropertyValue::Number(String::from("20"))),
                    ("mixed", PropertyValue::String(String::from("high"))),
                ],
            ),
            doc("person", "Sergio", &[("active", PropertyValue::Bool(true))]),
        ];
        let catalog = catalog(&docs, &[collection("project", 2), collection("person", 1)]);
        let generated = generate_explicit_schema(&catalog).unwrap();

        assert!(generated.yaml.starts_with("version: 1\ncollections:\n"));
        assert_eq!(
            generated.omitted_fields,
            vec![GeneratedSchemaOmittedField {
                collection_id: String::from("project"),
                field_name: String::from("mixed"),
                field_type: SchemaType::Mixed,
            }]
        );
        assert!(!generated.yaml.contains("mixed:"));

        let parsed = parse_explicit_schema(Path::new("flokin.schema.yaml"), &generated.yaml)
            .expect("generated schema must parse");
        assert_eq!(
            parsed.collections["project"].fields["title"].field_type,
            SchemaType::String
        );
        assert!(parsed.collections["project"].fields["title"].required);
        assert!(parsed.collections["project"].fields["status"].required);
        assert!(!parsed.collections["project"].fields["budget"].required);
        assert_eq!(
            parsed.collections["project"].fields["priority"].field_type,
            SchemaType::Integer
        );
        assert_eq!(
            parsed.collections["project"].fields["budget"].field_type,
            SchemaType::Float
        );
        assert_eq!(
            parsed.collections["project"].fields["published"].field_type,
            SchemaType::Boolean
        );
        assert_eq!(
            parsed.collections["project"].fields["tags"].field_type,
            SchemaType::Array
        );
        assert_eq!(
            parsed.collections["project"].fields["metadata"].field_type,
            SchemaType::Object
        );
        assert_eq!(
            parsed.collections["project"].fields["owner"].field_type,
            SchemaType::Relation
        );
        assert_eq!(
            parsed.collections["person"].fields["active"].field_type,
            SchemaType::Boolean
        );
    }

    #[test]
    fn generated_schema_is_deterministic_and_empty_catalog_is_rejected() {
        let docs = vec![doc(
            "project",
            "A",
            &[
                ("zeta", PropertyValue::String(String::from("z"))),
                ("alpha", PropertyValue::Number(String::from("1"))),
            ],
        )];
        let catalog = catalog(&docs, &[collection("project", 1)]);
        let first = generate_explicit_schema(&catalog).unwrap();
        let second = generate_explicit_schema(&catalog).unwrap();
        assert_eq!(first, second);
        assert!(first.yaml.find("title:").unwrap() < first.yaml.find("alpha:").unwrap());
        assert!(first.yaml.find("alpha:").unwrap() < first.yaml.find("zeta:").unwrap());

        let empty = SchemaCatalog::build(
            &[],
            &[],
            &RelationIndex::default(),
            ExplicitSchemaState::Absent,
        );
        assert_eq!(
            generate_explicit_schema(&empty),
            Err(SchemaGenerationError::Empty)
        );
    }

    #[test]
    fn handles_absent_schema_file_and_large_inference() {
        let root = std::env::temp_dir().join(format!(
            "flokinmd-schema-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&root).unwrap();
        assert_eq!(load_explicit_schema(&root), ExplicitSchemaState::Absent);
        fs::remove_dir_all(&root).unwrap();

        let documents = (0..1_000)
            .map(|index| {
                doc(
                    "project",
                    &format!("Doc {index}"),
                    &[("rank", PropertyValue::Number(index.to_string()))],
                )
            })
            .collect::<Vec<_>>();
        let catalog = catalog(&documents, &[collection("project", 1_000)]);
        let schema = catalog.collection("project").unwrap();
        assert_eq!(field(schema, "rank").observed_count, 1_000);
        assert_eq!(field(schema, "rank").field_type, SchemaType::Integer);
    }
}
