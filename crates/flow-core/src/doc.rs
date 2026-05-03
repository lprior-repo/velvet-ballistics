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
