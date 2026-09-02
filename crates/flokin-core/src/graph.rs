use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
};

use crate::{Collection, Document, RelationIndex, RelationStatus};

pub const GRAPH_MIN_ZOOM: f32 = 0.35;
pub const GRAPH_MAX_ZOOM: f32 = 3.0;
const LAYOUT_NODE_WIDTH: f32 = 168.0;
const LAYOUT_NODE_HEIGHT: f32 = 70.0;

#[derive(Debug, Clone, PartialEq)]
pub struct GraphProjection {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum GraphNodeId {
    Document(PathBuf),
    Unresolved { source: PathBuf, raw: String },
    Ambiguous { source: PathBuf, raw: String },
}

#[derive(Debug, Clone, PartialEq)]
pub struct GraphNode {
    pub id: GraphNodeId,
    pub title: String,
    pub relative_path: Option<PathBuf>,
    pub collection: Option<String>,
    pub document_type: Option<String>,
    pub outgoing_count: usize,
    pub incoming_count: usize,
    pub kind: GraphNodeKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphNodeKind {
    Document,
    Unresolved,
    Ambiguous,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GraphEdge {
    pub source: GraphNodeId,
    pub target: GraphNodeId,
    pub relation_type: String,
    pub target_label: String,
    pub status: GraphEdgeStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphEdgeStatus {
    Resolved,
    Unresolved,
    Ambiguous,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GraphPoint {
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GraphBounds {
    pub min_x: f32,
    pub min_y: f32,
    pub max_x: f32,
    pub max_y: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GraphViewport {
    pub pan_x: f32,
    pub pan_y: f32,
    pub zoom: f32,
}

impl GraphProjection {
    pub fn build(documents: &[Document], relation_index: &RelationIndex) -> Self {
        let collections = BTreeMap::<String, String>::new();
        Self::build_with_collections(documents, &collections, relation_index)
    }

    pub fn build_with_collections(
        documents: &[Document],
        collections: &BTreeMap<String, String>,
        relation_index: &RelationIndex,
    ) -> Self {
        let mut incoming_counts = BTreeMap::<PathBuf, usize>::new();
        let mut outgoing_counts = BTreeMap::<PathBuf, usize>::new();
        for relation in relation_index.all() {
            *outgoing_counts
                .entry(relation.source_document.clone())
                .or_default() += 1;
            if let RelationStatus::Resolved(target) = &relation.status {
                *incoming_counts.entry(target.path.clone()).or_default() += 1;
            }
        }

        let mut nodes = documents
            .iter()
            .map(|document| GraphNode {
                id: GraphNodeId::Document(document.path.clone()),
                title: document.title.clone(),
                relative_path: Some(document.relative_path.clone()),
                collection: collections
                    .get(&document.collection_id)
                    .cloned()
                    .or_else(|| Some(document.collection_id.clone())),
                document_type: document.document_type.clone(),
                outgoing_count: outgoing_counts
                    .get(&document.path)
                    .copied()
                    .unwrap_or_default(),
                incoming_count: incoming_counts
                    .get(&document.path)
                    .copied()
                    .unwrap_or_default(),
                kind: GraphNodeKind::Document,
            })
            .collect::<Vec<_>>();
        nodes.sort_by(|left, right| left.id.cmp(&right.id));

        let document_paths = documents
            .iter()
            .map(|document| document.path.clone())
            .collect::<BTreeSet<_>>();
        let mut edges = Vec::new();
        let mut ghost_nodes = BTreeMap::<GraphNodeId, GraphNode>::new();

        for relation in relation_index.all() {
            if !document_paths.contains(&relation.source_document) {
                continue;
            }

            let source = GraphNodeId::Document(relation.source_document.clone());
            let (target, status, kind, title) = match &relation.status {
                RelationStatus::Resolved(target) if document_paths.contains(&target.path) => (
                    GraphNodeId::Document(target.path.clone()),
                    GraphEdgeStatus::Resolved,
                    GraphNodeKind::Document,
                    target.title.clone(),
                ),
                RelationStatus::Resolved(_) => {
                    continue;
                }
                RelationStatus::Unresolved => (
                    GraphNodeId::Unresolved {
                        source: relation.source_document.clone(),
                        raw: relation.target.raw.clone(),
                    },
                    GraphEdgeStatus::Unresolved,
                    GraphNodeKind::Unresolved,
                    format!("? {}", relation.target.display),
                ),
                RelationStatus::Ambiguous(_) => (
                    GraphNodeId::Ambiguous {
                        source: relation.source_document.clone(),
                        raw: relation.target.raw.clone(),
                    },
                    GraphEdgeStatus::Ambiguous,
                    GraphNodeKind::Ambiguous,
                    format!("? {} - ambiguo", relation.target.display),
                ),
            };

            if kind != GraphNodeKind::Document {
                ghost_nodes.entry(target.clone()).or_insert(GraphNode {
                    id: target.clone(),
                    title,
                    relative_path: None,
                    collection: None,
                    document_type: None,
                    outgoing_count: 0,
                    incoming_count: 0,
                    kind,
                });
            }

            edges.push(GraphEdge {
                source,
                target,
                relation_type: relation.property.clone(),
                target_label: relation.target.display.clone(),
                status,
            });
        }

        nodes.extend(ghost_nodes.into_values());
        nodes.sort_by(|left, right| left.id.cmp(&right.id));
        edges.sort_by(|left, right| {
            left.source
                .cmp(&right.source)
                .then_with(|| left.target.cmp(&right.target))
                .then_with(|| left.relation_type.cmp(&right.relation_type))
                .then_with(|| left.target_label.cmp(&right.target_label))
        });

        Self { nodes, edges }
    }

    pub fn relation_count(&self) -> usize {
        self.edges.len()
    }

    pub fn document_count(&self) -> usize {
        self.nodes
            .iter()
            .filter(|node| node.kind == GraphNodeKind::Document)
            .count()
    }

    pub fn problem_count(&self) -> usize {
        self.nodes
            .iter()
            .filter(|node| node.kind != GraphNodeKind::Document)
            .count()
    }
}

pub fn initial_graph_layout(projection: &GraphProjection) -> BTreeMap<GraphNodeId, GraphPoint> {
    if projection.nodes.is_empty() {
        return BTreeMap::new();
    }

    let mut adjacency = BTreeMap::<GraphNodeId, BTreeSet<GraphNodeId>>::new();
    for node in &projection.nodes {
        adjacency.entry(node.id.clone()).or_default();
    }
    for edge in &projection.edges {
        adjacency
            .entry(edge.source.clone())
            .or_default()
            .insert(edge.target.clone());
        adjacency
            .entry(edge.target.clone())
            .or_default()
            .insert(edge.source.clone());
    }

    let mut visited = BTreeSet::<GraphNodeId>::new();
    let mut components = Vec::<Vec<GraphNodeId>>::new();
    for node in projection.nodes.iter().map(|node| node.id.clone()) {
        if visited.contains(&node) {
            continue;
        }
        let mut stack = vec![node.clone()];
        let mut component = Vec::new();
        visited.insert(node);
        while let Some(current) = stack.pop() {
            component.push(current.clone());
            if let Some(neighbors) = adjacency.get(&current) {
                for neighbor in neighbors.iter().rev() {
                    if visited.insert(neighbor.clone()) {
                        stack.push(neighbor.clone());
                    }
                }
            }
        }
        component.sort();
        components.push(component);
    }
    components.sort_by(|left, right| left.first().cmp(&right.first()));

    let mut positions = BTreeMap::new();
    let node_spacing_x = 178.0;
    let node_spacing_y = 118.0;
    let component_gap_x = 112.0;
    let component_gap_y = 88.0;
    let target_row_width = ((projection.nodes.len() as f32).sqrt().ceil().max(3.0)
        * node_spacing_x
        + LAYOUT_NODE_WIDTH)
        .max(node_spacing_x * 3.0 + LAYOUT_NODE_WIDTH);
    let mut offset_x = 0.0;
    let mut offset_y = 0.0;
    let mut row_height = 0.0;

    for component in components {
        let count = component.len();
        let columns = ((count as f32) * 1.35).sqrt().ceil().max(1.0) as usize;
        let rows = count.div_ceil(columns);
        let used_columns = columns.min(count);
        let component_width =
            used_columns.saturating_sub(1) as f32 * node_spacing_x + LAYOUT_NODE_WIDTH;
        let component_height = rows.saturating_sub(1) as f32 * node_spacing_y + LAYOUT_NODE_HEIGHT;

        if offset_x > 0.0 && offset_x + component_width > target_row_width {
            offset_x = 0.0;
            offset_y += row_height + component_gap_y;
            row_height = 0.0;
        }

        for (index, id) in component.into_iter().enumerate() {
            let col = index % columns;
            let row = index / columns;
            positions.insert(
                id,
                GraphPoint {
                    x: offset_x + col as f32 * node_spacing_x,
                    y: offset_y + row as f32 * node_spacing_y,
                },
            );
        }
        offset_x += component_width + component_gap_x;
        row_height = row_height.max(component_height);
    }

    positions
}

pub fn graph_bounds(
    positions: &BTreeMap<GraphNodeId, GraphPoint>,
    node_width: f32,
    node_height: f32,
) -> Option<GraphBounds> {
    let mut iter = positions.values();
    let first = iter.next()?;
    let mut bounds = GraphBounds {
        min_x: first.x,
        min_y: first.y,
        max_x: first.x + node_width,
        max_y: first.y + node_height,
    };
    for position in iter {
        bounds.min_x = bounds.min_x.min(position.x);
        bounds.min_y = bounds.min_y.min(position.y);
        bounds.max_x = bounds.max_x.max(position.x + node_width);
        bounds.max_y = bounds.max_y.max(position.y + node_height);
    }
    Some(bounds)
}

pub fn fit_graph_viewport(
    bounds: Option<GraphBounds>,
    viewport_width: f32,
    viewport_height: f32,
    padding: f32,
) -> GraphViewport {
    let Some(bounds) = bounds else {
        return GraphViewport {
            pan_x: viewport_width / 2.0,
            pan_y: viewport_height / 2.0,
            zoom: 1.0,
        };
    };
    if viewport_width <= 0.0 || viewport_height <= 0.0 {
        return GraphViewport {
            pan_x: 0.0,
            pan_y: 0.0,
            zoom: 1.0,
        };
    }
    let graph_width = (bounds.max_x - bounds.min_x).max(1.0);
    let graph_height = (bounds.max_y - bounds.min_y).max(1.0);
    let available_width = (viewport_width - padding * 2.0).max(1.0);
    let available_height = (viewport_height - padding * 2.0).max(1.0);
    let fit_zoom = (available_width / graph_width).min(available_height / graph_height);
    let readable_min = readable_fit_min_zoom(graph_width, graph_height);
    let readable_max = readable_fit_max_zoom(graph_width, graph_height);
    let zoom = clamp_graph_zoom(fit_zoom.clamp(readable_min, readable_max));
    let graph_center_x = (bounds.min_x + bounds.max_x) / 2.0;
    let graph_center_y = (bounds.min_y + bounds.max_y) / 2.0;

    GraphViewport {
        pan_x: viewport_width / 2.0 - graph_center_x * zoom,
        pan_y: viewport_height / 2.0 - graph_center_y * zoom,
        zoom,
    }
}

pub fn clamp_graph_zoom(zoom: f32) -> f32 {
    if zoom.is_finite() {
        zoom.clamp(GRAPH_MIN_ZOOM, GRAPH_MAX_ZOOM)
    } else {
        1.0
    }
}

fn readable_fit_min_zoom(graph_width: f32, graph_height: f32) -> f32 {
    let long_side = graph_width.max(graph_height);
    if long_side <= 900.0 {
        1.0
    } else if long_side <= 1_500.0 {
        0.78
    } else if long_side <= 2_400.0 {
        0.55
    } else {
        GRAPH_MIN_ZOOM
    }
}

fn readable_fit_max_zoom(graph_width: f32, graph_height: f32) -> f32 {
    let long_side = graph_width.max(graph_height);
    if long_side <= 220.0 {
        1.65
    } else if long_side <= 900.0 {
        1.45
    } else if long_side <= 1_500.0 {
        1.2
    } else {
        1.0
    }
}

pub fn graph_collections_map(collections: &[Collection]) -> BTreeMap<String, String> {
    collections
        .iter()
        .map(|collection| (collection.id.clone(), collection.display_name.clone()))
        .collect()
}

pub fn document_node_id(path: impl AsRef<Path>) -> GraphNodeId {
    GraphNodeId::Document(path.as_ref().to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{RelationTarget, ScanResult};
    use std::{ffi::OsString, time::SystemTime};

    fn doc(path: &str, title: &str) -> Document {
        Document {
            path: PathBuf::from("/ws").join(path),
            relative_path: PathBuf::from(path),
            file_name: OsString::from(Path::new(path).file_name().unwrap()),
            metadata: crate::DocumentMetadata {
                file_size: None,
                modified: Some(SystemTime::UNIX_EPOCH),
            },
            title: title.to_owned(),
            source_content: Some(String::new()),
            markdown_content: String::new(),
            properties: BTreeMap::new(),
            document_type: None,
            collection_id: String::from("documents"),
            warnings: Vec::new(),
        }
    }

    fn with_relation(mut document: Document, property: &str, target: &str) -> Document {
        document.properties.insert(
            property.to_owned(),
            crate::PropertyValue::String(format!("[[{target}]]")),
        );
        document
    }

    fn projection(documents: Vec<Document>) -> GraphProjection {
        let index = RelationIndex::build(&documents);
        GraphProjection::build(&documents, &index)
    }

    #[test]
    fn one_document_becomes_one_node() {
        let graph = projection(vec![doc("a.md", "A")]);
        assert_eq!(graph.nodes.len(), 1);
        assert_eq!(graph.edges.len(), 0);
    }

    #[test]
    fn resolved_relation_becomes_edge_and_preserves_type() {
        let graph = projection(vec![
            with_relation(doc("a.md", "A"), "project", "B"),
            doc("b.md", "B"),
        ]);
        assert_eq!(graph.edges.len(), 1);
        assert_eq!(graph.edges[0].relation_type, "project");
        assert_eq!(graph.edges[0].status, GraphEdgeStatus::Resolved);
    }

    #[test]
    fn two_relations_between_same_nodes_are_preserved() {
        let mut a = doc("a.md", "A");
        a.properties.insert(
            String::from("owner"),
            crate::PropertyValue::String(String::from("[[B]]")),
        );
        a.properties.insert(
            String::from("participants"),
            crate::PropertyValue::String(String::from("[[B]]")),
        );
        let graph = projection(vec![a, doc("b.md", "B")]);
        assert_eq!(graph.edges.len(), 2);
        assert_eq!(graph.edges[0].relation_type, "owner");
        assert_eq!(graph.edges[1].relation_type, "participants");
    }

    #[test]
    fn incoming_and_outgoing_counts_are_represented() {
        let graph = projection(vec![
            with_relation(doc("a.md", "A"), "related", "B"),
            doc("b.md", "B"),
        ]);
        let a = graph
            .nodes
            .iter()
            .find(|node| node.title == "A")
            .expect("A node");
        let b = graph
            .nodes
            .iter()
            .find(|node| node.title == "B")
            .expect("B node");
        assert_eq!(a.outgoing_count, 1);
        assert_eq!(b.incoming_count, 1);
    }

    #[test]
    fn unresolved_relation_uses_problem_node() {
        let graph = projection(vec![with_relation(doc("a.md", "A"), "owner", "Maria")]);
        assert_eq!(graph.problem_count(), 1);
        assert_eq!(graph.edges[0].status, GraphEdgeStatus::Unresolved);
        assert!(matches!(
            graph.edges[0].target,
            GraphNodeId::Unresolved { .. }
        ));
    }

    #[test]
    fn ambiguous_relation_never_chooses_target() {
        let graph = projection(vec![
            with_relation(doc("a.md", "A"), "project", "CARF"),
            doc("one/carf.md", "CARF"),
            doc("two/carf.md", "CARF"),
        ]);
        assert_eq!(graph.problem_count(), 1);
        assert_eq!(graph.edges[0].status, GraphEdgeStatus::Ambiguous);
        assert!(matches!(
            graph.edges[0].target,
            GraphNodeId::Ambiguous { .. }
        ));
    }

    #[test]
    fn self_relation_and_cycle_are_safe() {
        let graph = projection(vec![
            with_relation(doc("a.md", "A"), "related", "A"),
            with_relation(doc("b.md", "B"), "related", "A"),
        ]);
        assert!(graph.edges.iter().any(|edge| edge.source == edge.target));
        let positions = initial_graph_layout(&graph);
        assert_eq!(positions.len(), graph.nodes.len());
    }

    #[test]
    fn deterministic_projection_and_no_duplicate_nodes() {
        let documents = vec![
            with_relation(doc("b.md", "B"), "related", "A"),
            with_relation(doc("a.md", "A"), "related", "B"),
        ];
        let left = projection(documents.clone());
        let right = projection(documents);
        assert_eq!(left, right);
        let unique = left
            .nodes
            .iter()
            .map(|node| &node.id)
            .collect::<BTreeSet<_>>();
        assert_eq!(unique.len(), left.nodes.len());
    }

    #[test]
    fn no_dangling_resolved_edges() {
        let stale = GraphProjection::build(
            &[doc("a.md", "A")],
            &RelationIndex::from_relations(vec![crate::Relation {
                source_document: PathBuf::from("/ws/a.md"),
                source_title: String::from("A"),
                source_relative_path: PathBuf::from("a.md"),
                property: String::from("related"),
                target: RelationTarget {
                    raw: String::from("B"),
                    display: String::from("B"),
                },
                status: RelationStatus::Resolved(crate::RelationDocument {
                    path: PathBuf::from("/ws/b.md"),
                    relative_path: PathBuf::from("b.md"),
                    title: String::from("B"),
                }),
            }]),
        );
        assert!(stale.edges.is_empty());
    }

    #[test]
    fn watcher_style_rebuild_create_and_remove_target() {
        let unresolved = projection(vec![with_relation(doc("a.md", "A"), "owner", "Maria")]);
        assert_eq!(unresolved.edges[0].status, GraphEdgeStatus::Unresolved);

        let resolved = projection(vec![
            with_relation(doc("a.md", "A"), "owner", "Maria"),
            doc("people/maria.md", "Maria"),
        ]);
        assert_eq!(resolved.edges[0].status, GraphEdgeStatus::Resolved);

        let removed = projection(vec![with_relation(doc("a.md", "A"), "owner", "Maria")]);
        assert_eq!(removed.edges[0].status, GraphEdgeStatus::Unresolved);
    }

    #[test]
    fn title_update_changes_node_label_and_unicode_titles_work() {
        let graph = projection(vec![doc("cafe.md", "Café")]);
        assert_eq!(graph.nodes[0].title, "Café");
        let graph = projection(vec![doc("cafe.md", "Café novo")]);
        assert_eq!(graph.nodes[0].title, "Café novo");
    }

    #[test]
    fn workspace_change_removes_old_graph() {
        let first = projection(vec![doc("a.md", "A")]);
        let second = projection(vec![doc("b.md", "B")]);
        assert_ne!(first.nodes[0].id, second.nodes[0].id);
    }

    #[test]
    fn layout_positions_are_finite_and_deterministic() {
        let mut documents = Vec::new();
        for index in 0..200 {
            let mut document = doc(&format!("{index:03}.md"), &format!("Doc {index:03}"));
            if index < 199 && index % 2 == 0 {
                document.properties.insert(
                    String::from("related"),
                    crate::PropertyValue::String(format!("[[Doc {:03}]]", index + 1)),
                );
            }
            documents.push(document);
        }
        let graph = projection(documents);
        let left = initial_graph_layout(&graph);
        let right = initial_graph_layout(&graph);
        assert_eq!(left, right);
        assert!(left
            .values()
            .all(|point| point.x.is_finite() && point.y.is_finite()));
    }

    #[test]
    fn layout_handles_empty_one_two_disconnected_and_fit() {
        let empty = GraphProjection {
            nodes: Vec::new(),
            edges: Vec::new(),
        };
        assert!(initial_graph_layout(&empty).is_empty());

        for documents in [
            vec![doc("a.md", "A")],
            vec![doc("a.md", "A"), doc("b.md", "B")],
            vec![doc("a.md", "A"), doc("b.md", "B"), doc("c.md", "C")],
        ] {
            let graph = projection(documents);
            let positions = initial_graph_layout(&graph);
            assert_eq!(positions.len(), graph.nodes.len());
            let bounds = graph_bounds(&positions, 150.0, 64.0);
            let viewport = fit_graph_viewport(bounds, 800.0, 600.0, 40.0);
            assert!(viewport.zoom.is_finite());
            assert!(viewport.pan_x.is_finite());
            assert!(viewport.pan_y.is_finite());
        }
    }

    #[test]
    fn layout_packs_disconnected_components_without_one_long_row() {
        let graph = projection(
            (0..12)
                .map(|index| doc(&format!("{index}.md"), &format!("Doc {index}")))
                .collect(),
        );

        let positions = initial_graph_layout(&graph);
        let bounds = graph_bounds(&positions, 168.0, 70.0).unwrap();

        assert!(bounds.max_x - bounds.min_x < 900.0);
        assert!(bounds.max_y - bounds.min_y > 180.0);
    }

    #[test]
    fn fit_graph_viewport_keeps_small_graph_readable_and_centered() {
        let bounds = GraphBounds {
            min_x: 0.0,
            min_y: 0.0,
            max_x: 524.0,
            max_y: 188.0,
        };

        let viewport = fit_graph_viewport(Some(bounds), 900.0, 600.0, 56.0);
        let graph_center_x = (bounds.min_x + bounds.max_x) / 2.0;
        let graph_center_y = (bounds.min_y + bounds.max_y) / 2.0;

        assert!(viewport.zoom >= 1.0);
        assert!(viewport.zoom <= 1.45);
        assert!((viewport.pan_x + graph_center_x * viewport.zoom - 450.0).abs() < 0.01);
        assert!((viewport.pan_y + graph_center_y * viewport.zoom - 300.0).abs() < 0.01);
    }

    #[test]
    fn fit_graph_viewport_clamps_large_graph_without_absurd_zoom_out() {
        let bounds = GraphBounds {
            min_x: 0.0,
            min_y: 0.0,
            max_x: 6_000.0,
            max_y: 4_000.0,
        };

        let viewport = fit_graph_viewport(Some(bounds), 900.0, 600.0, 56.0);

        assert_eq!(viewport.zoom, GRAPH_MIN_ZOOM);
        assert!(viewport.pan_x.is_finite());
        assert!(viewport.pan_y.is_finite());
    }

    #[test]
    fn zoom_clamp_is_bounded() {
        assert_eq!(clamp_graph_zoom(0.01), GRAPH_MIN_ZOOM);
        assert_eq!(clamp_graph_zoom(10.0), GRAPH_MAX_ZOOM);
        assert_eq!(clamp_graph_zoom(f32::NAN), 1.0);
    }

    #[test]
    fn graph_projection_from_scan_result_shape() {
        let result = ScanResult {
            root: PathBuf::from("/ws"),
            documents: vec![doc("a.md", "A")],
            collections: Vec::new(),
            directories: Vec::new(),
            errors: Vec::new(),
            duration: std::time::Duration::ZERO,
        };
        let graph = GraphProjection::build(&result.documents, &RelationIndex::default());
        assert_eq!(graph.document_count(), 1);
    }
}
