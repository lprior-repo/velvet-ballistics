use crate::doc::*;
use crate::ids::*;
use serde::{Deserialize, Serialize};
use smol_str::SmolStr;

// ---------------------------------------------------------------------------
// Patches
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum FlowPatch {
    InsertNode { node: FlowNodeRecord },
    UpdateNode { id: NodeId, changes: NodeChangeSet },
    RemoveNode { id: NodeId },
    InsertEdge { edge: FlowEdgeRecord },
    UpdateEdge { id: EdgeId, changes: EdgeChangeSet },
    RemoveEdge { id: EdgeId },
    InsertGroup { group: FlowGroupRecord },
    UpdateGroup { id: GroupId, changes: GroupChangeSet },
    RemoveGroup { id: GroupId },
    SetViewport { viewport: ViewportState },
    SetEntryNode { node: Option<NodeId> },
    ReparentNodes { node_ids: Vec<NodeId>, new_parent: Option<GroupId> },
}

// ---------------------------------------------------------------------------
// Change sets
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct NodeChangeSet {
    pub position: Option<[f64; 2]>,
    pub size: Option<[f64; 2]>,
    pub title: Option<SmolStr>,
    pub kind: Option<SmolStr>,
    pub data: Option<serde_json::Value>,
    pub flags: Option<NodeFlags>,
    pub ui: Option<NodeUiState>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct EdgeChangeSet {
    pub label: Option<Option<SmolStr>>,
    pub style: Option<EdgeStyle>,
    pub data: Option<serde_json::Value>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct GroupChangeSet {
    pub title: Option<SmolStr>,
    pub bounds: Option<[f64; 4]>,
    pub data: Option<serde_json::Value>,
}

// ---------------------------------------------------------------------------
// Transaction
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct FlowTransaction {
    pub id: u64,
    pub label: SmolStr,
    pub patches: Vec<FlowPatch>,
    pub origin: ChangeOrigin,
    pub merge_key: Option<SmolStr>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChangeOrigin {
    User,
    Plugin,
    Import,
    AI,
    Undo,
    Redo,
}

// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub enum FlowCommand {
    ApplyTransaction(FlowTransaction),
    Undo,
    Redo,
    CopySelection,
    CutSelection,
    PasteSelection { anchor: [f64; 2] },
    DeleteSelection,
    SelectAll,
    FitView,
    AutoLayout,
    ValidateNow,
}

// ---------------------------------------------------------------------------
// Events
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub enum FlowEvent {
    TransactionCommitted { summary: TransactionSummary },
    TransactionRejected { reason: String },
    SelectionChanged(SelectionState),
    ViewportChanged(ViewportState),
    DiagnosticsChanged(Vec<Diagnostic>),
    ConnectionProposed {
        source_node: NodeId,
        source_port: PortId,
        target_node: NodeId,
        target_port: PortId,
    },
}

// ---------------------------------------------------------------------------
// Transaction summary
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Default)]
pub struct TransactionSummary {
    pub nodes_added: usize,
    pub nodes_removed: usize,
    pub nodes_updated: usize,
    pub edges_added: usize,
    pub edges_removed: usize,
    pub edges_updated: usize,
}

// ---------------------------------------------------------------------------
// Diagnostics
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct Diagnostic {
    pub severity: DiagnosticSeverity,
    pub code: SmolStr,
    pub message: String,
    pub node: Option<NodeId>,
    pub edge: Option<EdgeId>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiagnosticSeverity {
    Error,
    Warning,
    Info,
}
