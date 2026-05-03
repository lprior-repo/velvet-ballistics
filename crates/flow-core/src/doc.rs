use crate::ids::*;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use smol_str::SmolStr;

// ---------------------------------------------------------------------------
// Top-level document
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FlowDocument {
    pub schema: SmolStr,
    pub semantic_kind: SmolStr,
    pub graph: FlowGraph,
    pub editor: EditorMetadata,
    pub plugin_state: IndexMap<PluginId, serde_json::Value>,
}

impl Default for FlowDocument {
    fn default() -> Self {
        Self {
            schema: SmolStr::from("flow-core/v1"),
            semantic_kind: SmolStr::from("generic"),
            graph: FlowGraph::default(),
            editor: EditorMetadata::default(),
            plugin_state: IndexMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Graph
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct FlowGraph {
    pub nodes: IndexMap<NodeId, FlowNodeRecord>,
    pub edges: IndexMap<EdgeId, FlowEdgeRecord>,
    pub groups: IndexMap<GroupId, FlowGroupRecord>,
    pub entry_node: Option<NodeId>,
}

// ---------------------------------------------------------------------------
// Node
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FlowNodeRecord {
    pub id: NodeId,
    pub kind: SmolStr,
    pub title: SmolStr,
    pub position: [f64; 2],
    pub size: [f64; 2],
    pub z_index: i32,
    pub parent: Option<GroupId>,
    pub ports: Vec<FlowPortRecord>,
    pub flags: NodeFlags,
    pub data: serde_json::Value,
    pub ui: NodeUiState,
}

// ---------------------------------------------------------------------------
// Node flags
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct NodeFlags {
    pub locked: bool,
    pub hidden: bool,
    pub terminal: bool,
    pub entry: bool,
}

// ---------------------------------------------------------------------------
// Port
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FlowPortRecord {
    pub id: PortId,
    pub side: PortSide,
    pub role: PortRole,
    pub label: SmolStr,
    pub order: u16,
    pub cardinality: Cardinality,
    pub data_type: Option<SmolStr>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PortSide {
    Left,
    Right,
    Top,
    Bottom,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PortRole {
    Source,
    Target,
    Bidirectional,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Cardinality {
    One,
    Many,
}

// ---------------------------------------------------------------------------
// Edge
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FlowEdgeRecord {
    pub id: EdgeId,
    pub source_node: NodeId,
    pub source_port: PortId,
    pub target_node: NodeId,
    pub target_port: PortId,
    pub label: Option<SmolStr>,
    pub style: EdgeStyle,
    pub data: serde_json::Value,
    pub ui: EdgeUiState,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EdgeStyle {
    pub line_style: LineStyle,
    pub width: f32,
    pub animated: bool,
    pub marker: EdgeMarker,
}

impl Default for EdgeStyle {
    fn default() -> Self {
        Self {
            line_style: LineStyle::Solid,
            width: 2.0,
            animated: false,
            marker: EdgeMarker::Arrow,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum LineStyle {
    Solid,
    Dashed,
    Dotted,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum EdgeMarker {
    None,
    Arrow,
    ArrowFilled,
    Circle,
}

// ---------------------------------------------------------------------------
// Group
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FlowGroupRecord {
    pub id: GroupId,
    pub kind: GroupKind,
    pub title: SmolStr,
    pub bounds: [f64; 4],
    pub data: serde_json::Value,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum GroupKind {
    Generic,
    Swimlane,
    Subflow,
    BranchContainer,
}

// ---------------------------------------------------------------------------
// UI state
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct NodeUiState {
    pub collapsed: bool,
    pub color_override: Option<[f32; 4]>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct EdgeUiState {
    pub color_override: Option<[f32; 4]>,
}

// ---------------------------------------------------------------------------
// Editor metadata
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EditorMetadata {
    pub viewport: ViewportState,
    pub selection: SelectionState,
    pub collapsed_groups: Vec<GroupId>,
    pub bookmarks: Vec<NodeId>,
    pub version: u32,
}

impl Default for EditorMetadata {
    fn default() -> Self {
        Self {
            viewport: ViewportState::default(),
            selection: SelectionState::default(),
            collapsed_groups: Vec::new(),
            bookmarks: Vec::new(),
            version: 1,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ViewportState {
    pub pan_x: f64,
    pub pan_y: f64,
    pub zoom: f64,
}

impl Default for ViewportState {
    fn default() -> Self {
        Self {
            pan_x: 0.0,
            pan_y: 0.0,
            zoom: 1.0,
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SelectionState {
    pub selected_nodes: Vec<NodeId>,
    pub selected_edges: Vec<EdgeId>,
    pub selected_groups: Vec<GroupId>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_node(id: &str) -> FlowNodeRecord {
        FlowNodeRecord {
            id: SmolStr::from(id),
            kind: SmolStr::from("test"),
            title: SmolStr::from(id),
            position: [0.0, 0.0],
            size: [100.0, 50.0],
            z_index: 0,
            parent: None,
            ports: Vec::new(),
            flags: NodeFlags::default(),
            data: serde_json::Value::Null,
            ui: NodeUiState::default(),
        }
    }

    fn make_port(id: &str, side: PortSide, role: PortRole) -> FlowPortRecord {
        FlowPortRecord {
            id: SmolStr::from(id),
            side,
            role,
            label: SmolStr::from(id),
            order: 0,
            cardinality: Cardinality::One,
            data_type: None,
        }
    }

    fn make_edge(id: &str, src: &str, src_port: &str, tgt: &str, tgt_port: &str) -> FlowEdgeRecord {
        FlowEdgeRecord {
            id: SmolStr::from(id),
            source_node: SmolStr::from(src),
            source_port: SmolStr::from(src_port),
            target_node: SmolStr::from(tgt),
            target_port: SmolStr::from(tgt_port),
            label: None,
            style: EdgeStyle::default(),
            data: serde_json::Value::Null,
            ui: EdgeUiState::default(),
        }
    }

    fn make_group(id: &str) -> FlowGroupRecord {
        FlowGroupRecord {
            id: SmolStr::from(id),
            kind: GroupKind::Generic,
            title: SmolStr::from(id),
            bounds: [0.0, 0.0, 200.0, 200.0],
            data: serde_json::Value::Null,
        }
    }

    // ---- FlowDocument ----

    #[test]
    fn document_default() {
        let doc = FlowDocument::default();
        assert_eq!(doc.schema.as_str(), "flow-core/v1");
        assert_eq!(doc.semantic_kind.as_str(), "generic");
        assert!(doc.graph.nodes.is_empty());
        assert!(doc.graph.edges.is_empty());
        assert!(doc.graph.groups.is_empty());
        assert!(doc.plugin_state.is_empty());
    }

    #[test]
    fn document_clone_preserves_data() {
        let mut doc = FlowDocument::default();
        let node = make_node("n1");
        doc.graph.nodes.insert(SmolStr::from("n1"), node);
        let cloned = doc.clone();
        assert!(cloned.graph.nodes.contains_key(&SmolStr::from("n1")));
    }

    #[test]
    fn document_serialization_roundtrip() {
        let mut doc = FlowDocument::default();
        doc.graph.nodes.insert(SmolStr::from("n1"), make_node("n1"));
        let json = serde_json::to_string(&doc).expect("serialize should succeed");
        let back: FlowDocument = serde_json::from_str(&json).expect("deserialize should succeed");
        assert!(back.graph.nodes.contains_key(&SmolStr::from("n1")));
    }

    // ---- FlowGraph ----

    #[test]
    fn graph_default_is_empty() {
        let graph = FlowGraph::default();
        assert!(graph.nodes.is_empty());
        assert!(graph.edges.is_empty());
        assert!(graph.groups.is_empty());
        assert!(graph.entry_node.is_none());
    }

    #[test]
    fn graph_insert_and_lookup_node() {
        let mut graph = FlowGraph::default();
        let node = make_node("n1");
        graph.nodes.insert(SmolStr::from("n1"), node);
        assert!(graph.nodes.contains_key(&SmolStr::from("n1")));
        assert_eq!(graph.nodes.len(), 1);
    }

    #[test]
    fn graph_remove_node() {
        let mut graph = FlowGraph::default();
        graph.nodes.insert(SmolStr::from("n1"), make_node("n1"));
        let removed = graph.nodes.shift_remove(&SmolStr::from("n1"));
        assert!(removed.is_some());
        assert!(graph.nodes.is_empty());
    }

    #[test]
    fn graph_insert_and_lookup_edge() {
        let mut graph = FlowGraph::default();
        let edge = make_edge("e1", "n1", "out", "n2", "in");
        graph.edges.insert(SmolStr::from("e1"), edge);
        assert!(graph.edges.contains_key(&SmolStr::from("e1")));
        assert_eq!(graph.edges.len(), 1);
    }

    #[test]
    fn graph_remove_edge() {
        let mut graph = FlowGraph::default();
        graph.edges.insert(
            SmolStr::from("e1"),
            make_edge("e1", "n1", "out", "n2", "in"),
        );
        let removed = graph.edges.shift_remove(&SmolStr::from("e1"));
        assert!(removed.is_some());
        assert!(graph.edges.is_empty());
    }

    #[test]
    fn graph_insert_and_lookup_group() {
        let mut graph = FlowGraph::default();
        let group = make_group("g1");
        graph.groups.insert(SmolStr::from("g1"), group);
        assert!(graph.groups.contains_key(&SmolStr::from("g1")));
    }

    #[test]
    fn graph_entry_node() {
        let mut graph = FlowGraph::default();
        assert!(graph.entry_node.is_none());
        graph.entry_node = Some(SmolStr::from("start"));
        assert_eq!(graph.entry_node.as_ref().map(|s| s.as_str()), Some("start"));
    }

    // ---- FlowNodeRecord ----

    #[test]
    fn node_record_fields() {
        let node = make_node("n42");
        assert_eq!(node.id.as_str(), "n42");
        assert_eq!(node.kind.as_str(), "test");
        assert_eq!(node.title.as_str(), "n42");
        assert_eq!(node.position, [0.0, 0.0]);
        assert_eq!(node.size, [100.0, 50.0]);
        assert_eq!(node.z_index, 0);
        assert!(node.parent.is_none());
        assert!(node.ports.is_empty());
        assert!(!node.flags.locked);
        assert!(!node.flags.hidden);
        assert!(!node.flags.terminal);
        assert!(!node.flags.entry);
    }

    #[test]
    fn node_with_ports() {
        let mut node = make_node("n1");
        node.ports
            .push(make_port("p1", PortSide::Left, PortRole::Target));
        node.ports
            .push(make_port("p2", PortSide::Right, PortRole::Source));
        assert_eq!(node.ports.len(), 2);
    }

    #[test]
    fn node_with_parent_group() {
        let mut node = make_node("n1");
        node.parent = Some(SmolStr::from("g1"));
        assert_eq!(node.parent.as_ref().map(|s| s.as_str()), Some("g1"));
    }

    #[test]
    fn node_clone() {
        let node = make_node("n1");
        let cloned = node.clone();
        assert_eq!(cloned.id, node.id);
    }

    // ---- NodeFlags ----

    #[test]
    fn node_flags_default() {
        let flags = NodeFlags::default();
        assert!(!flags.locked);
        assert!(!flags.hidden);
        assert!(!flags.terminal);
        assert!(!flags.entry);
    }

    #[test]
    fn node_flags_custom() {
        let flags = NodeFlags {
            locked: true,
            hidden: true,
            terminal: true,
            entry: true,
        };
        assert!(flags.locked);
        assert!(flags.hidden);
        assert!(flags.terminal);
        assert!(flags.entry);
    }

    // ---- FlowPortRecord ----

    #[test]
    fn port_record_fields() {
        let port = make_port("p-in-0", PortSide::Left, PortRole::Target);
        assert_eq!(port.id.as_str(), "p-in-0");
        assert_eq!(port.side, PortSide::Left);
        assert_eq!(port.role, PortRole::Target);
        assert_eq!(port.cardinality, Cardinality::One);
        assert!(port.data_type.is_none());
    }

    #[test]
    fn port_with_data_type() {
        let mut port = make_port("p1", PortSide::Right, PortRole::Source);
        port.data_type = Some(SmolStr::from("f32"));
        assert_eq!(port.data_type.as_ref().map(|s| s.as_str()), Some("f32"));
    }

    #[test]
    fn port_sides() {
        assert_ne!(PortSide::Left, PortSide::Right);
        assert_ne!(PortSide::Top, PortSide::Bottom);
    }

    #[test]
    fn port_roles() {
        assert_ne!(PortRole::Source, PortRole::Target);
        assert_ne!(PortRole::Bidirectional, PortRole::Source);
    }

    #[test]
    fn cardinality_variants() {
        assert_ne!(Cardinality::One, Cardinality::Many);
    }

    // ---- FlowEdgeRecord ----

    #[test]
    fn edge_record_fields() {
        let edge = make_edge("e1", "n1", "out-0", "n2", "in-0");
        assert_eq!(edge.id.as_str(), "e1");
        assert_eq!(edge.source_node.as_str(), "n1");
        assert_eq!(edge.source_port.as_str(), "out-0");
        assert_eq!(edge.target_node.as_str(), "n2");
        assert_eq!(edge.target_port.as_str(), "in-0");
        assert!(edge.label.is_none());
    }

    #[test]
    fn edge_with_label() {
        let mut edge = make_edge("e1", "n1", "out", "n2", "in");
        edge.label = Some(SmolStr::from("data flow"));
        assert_eq!(edge.label.as_ref().map(|s| s.as_str()), Some("data flow"));
    }

    // ---- EdgeStyle ----

    #[test]
    fn edge_style_default() {
        let style = EdgeStyle::default();
        assert_eq!(style.line_style, LineStyle::Solid);
        assert!((style.width - 2.0).abs() < f32::EPSILON);
        assert!(!style.animated);
        assert_eq!(style.marker, EdgeMarker::Arrow);
    }

    #[test]
    fn line_style_variants() {
        assert_ne!(LineStyle::Solid, LineStyle::Dashed);
        assert_ne!(LineStyle::Dashed, LineStyle::Dotted);
    }

    #[test]
    fn edge_marker_variants() {
        assert_ne!(EdgeMarker::None, EdgeMarker::Arrow);
        assert_ne!(EdgeMarker::Arrow, EdgeMarker::ArrowFilled);
        assert_ne!(EdgeMarker::ArrowFilled, EdgeMarker::Circle);
    }

    // ---- FlowGroupRecord ----

    #[test]
    fn group_record_fields() {
        let group = make_group("g1");
        assert_eq!(group.id.as_str(), "g1");
        assert_eq!(group.kind, GroupKind::Generic);
        assert_eq!(group.title.as_str(), "g1");
        assert_eq!(group.bounds, [0.0, 0.0, 200.0, 200.0]);
    }

    #[test]
    fn group_kinds() {
        assert_ne!(GroupKind::Generic, GroupKind::Swimlane);
        assert_ne!(GroupKind::Subflow, GroupKind::BranchContainer);
    }

    // ---- NodeUiState ----

    #[test]
    fn node_ui_state_default() {
        let ui = NodeUiState::default();
        assert!(!ui.collapsed);
        assert!(ui.color_override.is_none());
    }

    #[test]
    fn node_ui_state_custom() {
        let ui = NodeUiState {
            collapsed: true,
            color_override: Some([1.0, 0.0, 0.0, 1.0]),
        };
        assert!(ui.collapsed);
        assert!(ui.color_override.is_some());
    }

    // ---- EdgeUiState ----

    #[test]
    fn edge_ui_state_default() {
        let ui = EdgeUiState::default();
        assert!(ui.color_override.is_none());
    }

    // ---- EditorMetadata ----

    #[test]
    fn editor_metadata_default() {
        let meta = EditorMetadata::default();
        assert!((meta.viewport.pan_x).abs() < f64::EPSILON);
        assert!((meta.viewport.pan_y).abs() < f64::EPSILON);
        assert!((meta.viewport.zoom - 1.0).abs() < f64::EPSILON);
        assert!(meta.selection.selected_nodes.is_empty());
        assert!(meta.collapsed_groups.is_empty());
        assert!(meta.bookmarks.is_empty());
        assert_eq!(meta.version, 1);
    }

    // ---- ViewportState ----

    #[test]
    fn viewport_default() {
        let vp = ViewportState::default();
        assert!((vp.pan_x).abs() < f64::EPSILON);
        assert!((vp.pan_y).abs() < f64::EPSILON);
        assert!((vp.zoom - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn viewport_custom() {
        let vp = ViewportState {
            pan_x: 10.0,
            pan_y: -5.0,
            zoom: 2.5,
        };
        assert!((vp.pan_x - 10.0).abs() < f64::EPSILON);
        assert!((vp.pan_y - (-5.0)).abs() < f64::EPSILON);
        assert!((vp.zoom - 2.5).abs() < f64::EPSILON);
    }

    // ---- SelectionState ----

    #[test]
    fn selection_default() {
        let sel = SelectionState::default();
        assert!(sel.selected_nodes.is_empty());
        assert!(sel.selected_edges.is_empty());
        assert!(sel.selected_groups.is_empty());
    }

    #[test]
    fn selection_with_items() {
        let sel = SelectionState {
            selected_nodes: vec![SmolStr::from("n1"), SmolStr::from("n2")],
            selected_edges: vec![SmolStr::from("e1")],
            selected_groups: Vec::new(),
        };
        assert_eq!(sel.selected_nodes.len(), 2);
        assert_eq!(sel.selected_edges.len(), 1);
        assert!(sel.selected_groups.is_empty());
    }

    // ---- Serialization ----

    #[test]
    fn node_serialization_roundtrip() {
        let node = make_node("n1");
        let json = serde_json::to_string(&node).expect("serialize");
        let back: FlowNodeRecord = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.id, node.id);
    }

    #[test]
    fn edge_serialization_roundtrip() {
        let edge = make_edge("e1", "n1", "out", "n2", "in");
        let json = serde_json::to_string(&edge).expect("serialize");
        let back: FlowEdgeRecord = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.id, edge.id);
    }

    #[test]
    fn group_serialization_roundtrip() {
        let group = make_group("g1");
        let json = serde_json::to_string(&group).expect("serialize");
        let back: FlowGroupRecord = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.id, group.id);
    }

    #[test]
    fn full_document_serialization_roundtrip() {
        let mut doc = FlowDocument::default();
        doc.graph.nodes.insert(SmolStr::from("n1"), make_node("n1"));
        doc.graph.nodes.insert(SmolStr::from("n2"), make_node("n2"));
        doc.graph.edges.insert(
            SmolStr::from("e1"),
            make_edge("e1", "n1", "out", "n2", "in"),
        );
        doc.graph
            .groups
            .insert(SmolStr::from("g1"), make_group("g1"));
        doc.graph.entry_node = Some(SmolStr::from("n1"));

        let json = serde_json::to_string(&doc).expect("serialize");
        let back: FlowDocument = serde_json::from_str(&json).expect("deserialize");

        assert_eq!(back.graph.nodes.len(), 2);
        assert_eq!(back.graph.edges.len(), 1);
        assert_eq!(back.graph.groups.len(), 1);
        assert_eq!(
            back.graph.entry_node.as_ref().map(|s| s.as_str()),
            Some("n1")
        );
    }
}
