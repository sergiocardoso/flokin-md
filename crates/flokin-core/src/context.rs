use std::{
    cmp::Ordering,
    collections::BTreeMap,
    path::{Path, PathBuf},
};

use crate::{
    classify_semantic_entry, Document, ExplorerNodeKind, PropertyValue, RelationIndex, SemanticKind,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ContextSection {
    Overview,
    Agents,
    Skills,
    Specs,
    Ice,
    Contexts,
    Prompts,
    Rules,
    Memory,
    Mcp,
}

impl ContextSection {
    pub const ALL: [Self; 10] = [
        Self::Overview,
        Self::Agents,
        Self::Skills,
        Self::Specs,
        Self::Ice,
        Self::Contexts,
        Self::Prompts,
        Self::Rules,
        Self::Memory,
        Self::Mcp,
    ];

    pub const fn from_semantic_kind(kind: SemanticKind) -> Self {
        match kind {
            SemanticKind::Agent => Self::Agents,
            SemanticKind::Skill => Self::Skills,
            SemanticKind::Spec => Self::Specs,
            SemanticKind::Ice => Self::Ice,
            SemanticKind::Context => Self::Contexts,
            SemanticKind::Prompt => Self::Prompts,
            SemanticKind::Rules | SemanticKind::AgentInstructions => Self::Rules,
            SemanticKind::Memory => Self::Memory,
            SemanticKind::Mcp => Self::Mcp,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextArtifact {
    pub document_path: PathBuf,
    pub relative_path: PathBuf,
    pub title: String,
    pub semantic_kind: SemanticKind,
    pub incoming_count: usize,
    pub outgoing_count: usize,
    pub properties: BTreeMap<String, PropertyValue>,
}

impl ContextArtifact {
    pub const fn section(&self) -> ContextSection {
        ContextSection::from_semantic_kind(self.semantic_kind)
    }

    pub const fn relations_count(&self) -> usize {
        self.incoming_count + self.outgoing_count
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ContextProjection {
    pub artifacts: Vec<ContextArtifact>,
}

impl ContextProjection {
    pub fn count_for_section(&self, section: ContextSection) -> usize {
        self.artifacts_for_section(section).count()
    }

    pub fn artifacts_for_section(
        &self,
        section: ContextSection,
    ) -> impl Iterator<Item = &ContextArtifact> {
        self.artifacts.iter().filter(move |artifact| {
            section == ContextSection::Overview || artifact.section() == section
        })
    }

    pub fn artifact_for_path(&self, path: &Path) -> Option<&ContextArtifact> {
        self.artifacts
            .iter()
            .find(|artifact| artifact.document_path == path)
    }

    pub fn unconnected_count(&self) -> usize {
        self.artifacts
            .iter()
            .filter(|artifact| artifact.relations_count() == 0)
            .count()
    }
}

pub fn build_context_projection(
    documents: &[Document],
    relation_index: &RelationIndex,
) -> ContextProjection {
    let mut artifacts = documents
        .iter()
        .filter_map(|document| {
            let semantic_kind = classify_context_document(document)?;
            Some(ContextArtifact {
                document_path: document.path.clone(),
                relative_path: document.relative_path.clone(),
                title: document.title.clone(),
                semantic_kind,
                incoming_count: relation_index.incoming(&document.path).len(),
                outgoing_count: relation_index.outgoing(&document.path).len(),
                properties: document.properties.clone(),
            })
        })
        .collect::<Vec<_>>();

    artifacts.sort_by(compare_artifacts);
    artifacts.dedup_by(|left, right| left.document_path == right.document_path);
    ContextProjection { artifacts }
}

pub fn classify_context_document(document: &Document) -> Option<SemanticKind> {
    document
        .file_name
        .to_str()
        .and_then(|name| classify_semantic_entry(name, ExplorerNodeKind::File, &[]))
        .or_else(|| classify_context_path(&document.relative_path))
}

fn classify_context_path(path: &Path) -> Option<SemanticKind> {
    path.parent()?.components().find_map(|component| {
        let name = component.as_os_str().to_str()?;
        classify_semantic_entry(name, ExplorerNodeKind::Folder, &[])
    })
}

fn compare_artifacts(left: &ContextArtifact, right: &ContextArtifact) -> Ordering {
    left.section()
        .cmp(&right.section())
        .then_with(|| left.title.to_lowercase().cmp(&right.title.to_lowercase()))
        .then_with(|| {
            left.relative_path
                .to_string_lossy()
                .to_lowercase()
                .cmp(&right.relative_path.to_string_lossy().to_lowercase())
        })
        .then_with(|| left.relative_path.cmp(&right.relative_path))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DocumentMetadata, Relation, RelationStatus, RelationTarget};

    fn document(
        relative_path: &str,
        title: &str,
        properties: BTreeMap<String, PropertyValue>,
    ) -> Document {
        let path = PathBuf::from("/workspace").join(relative_path);
        Document {
            path,
            relative_path: PathBuf::from(relative_path),
            file_name: Path::new(relative_path).file_name().unwrap().to_os_string(),
            metadata: DocumentMetadata {
                file_size: None,
                modified: None,
            },
            title: title.to_owned(),
            source_content: None,
            markdown_content: String::new(),
            properties,
            document_type: None,
            collection_id: String::from("docs"),
            warnings: Vec::new(),
        }
    }

    #[test]
    fn classifies_context_conventions() {
        let cases = [
            ("skills/foo/SKILL.md", "Skill", Some(SemanticKind::Skill)),
            ("sdd/SDD-0001-auth.md", "SDD", Some(SemanticKind::Spec)),
            ("sdd/SDD_TEMPLATE.md", "Template", Some(SemanticKind::Spec)),
            ("ice/ICE_TEMPLATE.md", "ICE", Some(SemanticKind::Ice)),
            ("CONTEXT.md", "Context", Some(SemanticKind::Context)),
            ("PROMPT.md", "Prompt", Some(SemanticKind::Prompt)),
            ("RULES.md", "Rules", Some(SemanticKind::Rules)),
            ("MEMORY.md", "Memory", Some(SemanticKind::Memory)),
            ("normal.md", "Normal", None),
        ];

        for (path, title, expected) in cases {
            let document = document(path, title, BTreeMap::new());
            assert_eq!(classify_context_document(&document), expected, "{path}");
        }
    }

    #[test]
    fn projection_counts_relations_unconnected_and_order() {
        let skill = document("skills/deploy/SKILL.md", "Deploy", BTreeMap::new());
        let spec = document("sdd/SDD-0001-auth.md", "Auth", BTreeMap::new());
        let prompt = document("prompts/release.md", "Release", BTreeMap::new());
        let note = document("notes.md", "Notes", BTreeMap::new());
        let relations = RelationIndex::from_relations(vec![Relation {
            source_document: spec.path.clone(),
            source_title: spec.title.clone(),
            source_relative_path: spec.relative_path.clone(),
            property: String::from("skill"),
            target: RelationTarget {
                raw: String::from("Deploy"),
                display: String::from("Deploy"),
            },
            status: RelationStatus::Resolved(crate::RelationDocument {
                path: skill.path.clone(),
                relative_path: skill.relative_path.clone(),
                title: skill.title.clone(),
            }),
        }]);

        let projection = build_context_projection(&[prompt, skill, note, spec], &relations);

        assert_eq!(projection.artifacts.len(), 3);
        assert_eq!(projection.count_for_section(ContextSection::Skills), 1);
        assert_eq!(projection.count_for_section(ContextSection::Specs), 1);
        assert_eq!(projection.count_for_section(ContextSection::Prompts), 1);
        assert_eq!(projection.unconnected_count(), 1);

        let skill = projection
            .artifacts
            .iter()
            .find(|artifact| artifact.title == "Deploy")
            .unwrap();
        assert_eq!(skill.incoming_count, 1);
        assert_eq!(skill.outgoing_count, 0);
        assert_eq!(
            projection
                .artifacts_for_section(ContextSection::Overview)
                .map(|artifact| artifact.section())
                .collect::<Vec<_>>(),
            vec![
                ContextSection::Skills,
                ContextSection::Specs,
                ContextSection::Prompts
            ]
        );
    }

    #[test]
    fn projection_refresh_reflects_changed_documents() {
        let relations = RelationIndex::default();
        let first = build_context_projection(
            &[document(
                "skills/deploy/SKILL.md",
                "Deploy",
                BTreeMap::new(),
            )],
            &relations,
        );
        let second = build_context_projection(
            &[document(
                "skills/deploy/README.md",
                "Deploy",
                BTreeMap::new(),
            )],
            &relations,
        );

        assert_eq!(first.count_for_section(ContextSection::Skills), 1);
        assert_eq!(second.count_for_section(ContextSection::Skills), 1);
    }
}
