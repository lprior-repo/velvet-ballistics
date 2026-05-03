use crate::doc::*;
use crate::ids::*;
use serde::{Deserialize, Serialize};
use smol_str::SmolStr;

// ---------------------------------------------------------------------------
// Patches
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum FlowPatch {
    InsertNode {
        node: FlowNodeRecord,
    },
    UpdateNode {
        id: NodeId,
        changes: NodeChangeSet,
    },
    RemoveNode {
        id: NodeId,
    },
    InsertEdge {
        edge: FlowEdgeRecord,
    },
    UpdateEdge {
        id: EdgeId,
        changes: EdgeChangeSet,
    },
    RemoveEdge {
        id: EdgeId,
    },
    InsertGroup {
        group: FlowGroupRecord,
    },
    UpdateGroup {
        id: GroupId,
        changes: GroupChangeSet,
    },
    RemoveGroup {
        id: GroupId,
    },
    SetViewport {
        viewport: ViewportState,
    },
    SetEntryNode {
        node: Option<NodeId>,
    },
    ReparentNodes {
        node_ids: Vec<NodeId>,
        new_parent: Option<GroupId>,
    },
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
    TransactionCommitted {
        summary: TransactionSummary,
    },
    TransactionRejected {
        reason: String,
    },
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::doc::{
        EdgeStyle, FlowEdgeRecord, FlowGroupRecord, FlowNodeRecord, GroupKind, NodeFlags,
        NodeUiState, SelectionState, ViewportState,
    };
    use crate::ids::{EdgeId, GroupId, NodeId};
    use smol_str::SmolStr;

    fn nid(s: &str) -> NodeId {
        SmolStr::from(s)
    }

    fn eid(s: &str) -> EdgeId {
        SmolStr::from(s)
    }

    fn gid(s: &str) -> GroupId {
        SmolStr::from(s)
    }

    fn make_node(id: &str) -> FlowNodeRecord {
        FlowNodeRecord {
            id: nid(id),
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

    fn make_edge(id: &str, src: &str, tgt: &str) -> FlowEdgeRecord {
        FlowEdgeRecord {
            id: eid(id),
            source_node: nid(src),
            source_port: SmolStr::from("out"),
            target_node: nid(tgt),
            target_port: SmolStr::from("in"),
            label: None,
            style: EdgeStyle::default(),
            data: serde_json::Value::Null,
            ui: EdgeUiState::default(),
        }
    }

    fn make_group(id: &str) -> FlowGroupRecord {
        FlowGroupRecord {
            id: gid(id),
            kind: GroupKind::Generic,
            title: SmolStr::from(id),
            bounds: [0.0, 0.0, 200.0, 200.0],
            data: serde_json::Value::Null,
        }
    }

    // ---- FlowPatch variants ----

    #[test]
    fn patch_insert_node() {
        let node = make_node("n1");
        let patch = FlowPatch::InsertNode { node };
        if let FlowPatch::InsertNode { node: ref n } = patch {
            assert_eq!(n.id, nid("n1"));
        } else {
            panic!("expected InsertNode variant");
        }
    }

    #[test]
    fn patch_update_node() {
        let changes = NodeChangeSet {
            position: Some([10.0, 20.0]),
            ..NodeChangeSet::default()
        };
        let patch = FlowPatch::UpdateNode {
            id: nid("n1"),
            changes,
        };
        if let FlowPatch::UpdateNode { id, changes } = patch {
            assert_eq!(id, nid("n1"));
            assert!(changes.position.is_some());
        } else {
            panic!("expected UpdateNode variant");
        }
    }

    #[test]
    fn patch_remove_node() {
        let patch = FlowPatch::RemoveNode { id: nid("n1") };
        if let FlowPatch::RemoveNode { id } = patch {
            assert_eq!(id, nid("n1"));
        } else {
            panic!("expected RemoveNode variant");
        }
    }

    #[test]
    fn patch_insert_edge() {
        let edge = make_edge("e1", "n1", "n2");
        let patch = FlowPatch::InsertEdge { edge };
        if let FlowPatch::InsertEdge { edge: ref e } = patch {
            assert_eq!(e.id, eid("e1"));
        } else {
            panic!("expected InsertEdge variant");
        }
    }

    #[test]
    fn patch_update_edge() {
        let changes = EdgeChangeSet {
            label: Some(Some(SmolStr::from("new-label"))),
            ..EdgeChangeSet::default()
        };
        let patch = FlowPatch::UpdateEdge {
            id: eid("e1"),
            changes,
        };
        if let FlowPatch::UpdateEdge { id, changes } = patch {
            assert_eq!(id, eid("e1"));
            assert!(changes.label.is_some());
        } else {
            panic!("expected UpdateEdge variant");
        }
    }

    #[test]
    fn patch_remove_edge() {
        let patch = FlowPatch::RemoveEdge { id: eid("e1") };
        if let FlowPatch::RemoveEdge { id } = patch {
            assert_eq!(id, eid("e1"));
        } else {
            panic!("expected RemoveEdge variant");
        }
    }

    #[test]
    fn patch_insert_group() {
        let group = make_group("g1");
        let patch = FlowPatch::InsertGroup { group };
        if let FlowPatch::InsertGroup { group: ref g } = patch {
            assert_eq!(g.id, gid("g1"));
        } else {
            panic!("expected InsertGroup variant");
        }
    }

    #[test]
    fn patch_update_group() {
        let changes = GroupChangeSet {
            title: Some(SmolStr::from("renamed")),
            ..GroupChangeSet::default()
        };
        let patch = FlowPatch::UpdateGroup {
            id: gid("g1"),
            changes,
        };
        if let FlowPatch::UpdateGroup { id, changes } = patch {
            assert_eq!(id, gid("g1"));
            assert!(changes.title.is_some());
        } else {
            panic!("expected UpdateGroup variant");
        }
    }

    #[test]
    fn patch_remove_group() {
        let patch = FlowPatch::RemoveGroup { id: gid("g1") };
        if let FlowPatch::RemoveGroup { id } = patch {
            assert_eq!(id, gid("g1"));
        } else {
            panic!("expected RemoveGroup variant");
        }
    }

    #[test]
    fn patch_set_viewport() {
        let vp = ViewportState {
            pan_x: 10.0,
            pan_y: -5.0,
            zoom: 2.0,
        };
        let patch = FlowPatch::SetViewport { viewport: vp };
        if let FlowPatch::SetViewport { viewport } = patch {
            assert!((viewport.pan_x - 10.0).abs() < f64::EPSILON);
            assert!((viewport.zoom - 2.0).abs() < f64::EPSILON);
        } else {
            panic!("expected SetViewport variant");
        }
    }

    #[test]
    fn patch_set_entry_node_some() {
        let patch = FlowPatch::SetEntryNode {
            node: Some(nid("n1")),
        };
        if let FlowPatch::SetEntryNode { node } = patch {
            assert!(node.is_some());
        } else {
            panic!("expected SetEntryNode variant");
        }
    }

    #[test]
    fn patch_set_entry_node_none() {
        let patch = FlowPatch::SetEntryNode { node: None };
        if let FlowPatch::SetEntryNode { node } = patch {
            assert!(node.is_none());
        } else {
            panic!("expected SetEntryNode variant");
        }
    }

    #[test]
    fn patch_reparent_nodes() {
        let patch = FlowPatch::ReparentNodes {
            node_ids: vec![nid("n1"), nid("n2")],
            new_parent: Some(gid("g1")),
        };
        if let FlowPatch::ReparentNodes {
            node_ids,
            new_parent,
        } = patch
        {
            assert_eq!(node_ids.len(), 2);
            assert!(new_parent.is_some());
        } else {
            panic!("expected ReparentNodes variant");
        }
    }

    #[test]
    fn patch_reparent_nodes_remove_parent() {
        let patch = FlowPatch::ReparentNodes {
            node_ids: vec![nid("n1")],
            new_parent: None,
        };
        if let FlowPatch::ReparentNodes {
            node_ids,
            new_parent,
        } = patch
        {
            assert_eq!(node_ids.len(), 1);
            assert!(new_parent.is_none());
        } else {
            panic!("expected ReparentNodes variant");
        }
    }

    // ---- ChangeSet defaults ----

    #[test]
    fn node_change_set_default() {
        let cs = NodeChangeSet::default();
        assert!(cs.position.is_none());
        assert!(cs.size.is_none());
        assert!(cs.title.is_none());
        assert!(cs.kind.is_none());
        assert!(cs.data.is_none());
        assert!(cs.flags.is_none());
        assert!(cs.ui.is_none());
    }

    #[test]
    fn edge_change_set_default() {
        let cs = EdgeChangeSet::default();
        assert!(cs.label.is_none());
        assert!(cs.style.is_none());
        assert!(cs.data.is_none());
    }

    #[test]
    fn group_change_set_default() {
        let cs = GroupChangeSet::default();
        assert!(cs.title.is_none());
        assert!(cs.bounds.is_none());
        assert!(cs.data.is_none());
    }

    // ---- FlowTransaction ----

    #[test]
    fn transaction_construction() {
        let txn = FlowTransaction {
            id: 1,
            label: SmolStr::from("add-node"),
            patches: vec![FlowPatch::InsertNode {
                node: make_node("n1"),
            }],
            origin: ChangeOrigin::User,
            merge_key: None,
        };
        assert_eq!(txn.id, 1);
        assert_eq!(txn.label.as_str(), "add-node");
        assert_eq!(txn.patches.len(), 1);
        assert_eq!(txn.origin, ChangeOrigin::User);
        assert!(txn.merge_key.is_none());
    }

    #[test]
    fn transaction_with_merge_key() {
        let txn = FlowTransaction {
            id: 2,
            label: SmolStr::from("move"),
            patches: Vec::new(),
            origin: ChangeOrigin::User,
            merge_key: Some(SmolStr::from("drag-session-1")),
        };
        assert!(txn.merge_key.is_some());
    }

    #[test]
    fn transaction_clone() {
        let txn = FlowTransaction {
            id: 3,
            label: SmolStr::from("test"),
            patches: vec![FlowPatch::RemoveNode { id: nid("n1") }],
            origin: ChangeOrigin::Plugin,
            merge_key: None,
        };
        let cloned = txn.clone();
        assert_eq!(cloned.id, txn.id);
        assert_eq!(cloned.patches.len(), txn.patches.len());
    }

    // ---- ChangeOrigin ----

    #[test]
    fn change_origin_variants() {
        assert_ne!(ChangeOrigin::User, ChangeOrigin::Plugin);
        assert_ne!(ChangeOrigin::Import, ChangeOrigin::AI);
        assert_ne!(ChangeOrigin::Undo, ChangeOrigin::Redo);
    }

    // ---- FlowCommand ----

    #[test]
    fn command_apply_transaction() {
        let txn = FlowTransaction {
            id: 1,
            label: SmolStr::from("test"),
            patches: Vec::new(),
            origin: ChangeOrigin::User,
            merge_key: None,
        };
        let cmd = FlowCommand::ApplyTransaction(txn);
        if let FlowCommand::ApplyTransaction(t) = cmd {
            assert_eq!(t.id, 1);
        } else {
            panic!("expected ApplyTransaction");
        }
    }

    #[test]
    fn command_variants_exist() {
        // Just verify all command variants compile
        let _ = FlowCommand::Undo;
        let _ = FlowCommand::Redo;
        let _ = FlowCommand::CopySelection;
        let _ = FlowCommand::CutSelection;
        let _ = FlowCommand::PasteSelection { anchor: [0.0, 0.0] };
        let _ = FlowCommand::DeleteSelection;
        let _ = FlowCommand::SelectAll;
        let _ = FlowCommand::FitView;
        let _ = FlowCommand::AutoLayout;
        let _ = FlowCommand::ValidateNow;
    }

    // ---- FlowEvent ----

    #[test]
    fn event_transaction_committed() {
        let summary = TransactionSummary::default();
        let event = FlowEvent::TransactionCommitted { summary };
        if let FlowEvent::TransactionCommitted { summary } = event {
            assert_eq!(summary.nodes_added, 0);
        } else {
            panic!("expected TransactionCommitted");
        }
    }

    #[test]
    fn event_transaction_rejected() {
        let event = FlowEvent::TransactionRejected {
            reason: String::from("validation failed"),
        };
        if let FlowEvent::TransactionRejected { reason } = event {
            assert_eq!(reason, "validation failed");
        } else {
            panic!("expected TransactionRejected");
        }
    }

    #[test]
    fn event_selection_changed() {
        let sel = SelectionState::default();
        let event = FlowEvent::SelectionChanged(sel);
        if let FlowEvent::SelectionChanged(s) = event {
            assert!(s.selected_nodes.is_empty());
        } else {
            panic!("expected SelectionChanged");
        }
    }

    #[test]
    fn event_viewport_changed() {
        let vp = ViewportState::default();
        let event = FlowEvent::ViewportChanged(vp);
        if let FlowEvent::ViewportChanged(v) = event {
            assert!((v.zoom - 1.0).abs() < f64::EPSILON);
        } else {
            panic!("expected ViewportChanged");
        }
    }

    #[test]
    fn event_diagnostics_changed() {
        let event = FlowEvent::DiagnosticsChanged(Vec::new());
        if let FlowEvent::DiagnosticsChanged(d) = event {
            assert!(d.is_empty());
        } else {
            panic!("expected DiagnosticsChanged");
        }
    }

    #[test]
    fn event_connection_proposed() {
        let event = FlowEvent::ConnectionProposed {
            source_node: nid("n1"),
            source_port: SmolStr::from("out"),
            target_node: nid("n2"),
            target_port: SmolStr::from("in"),
        };
        if let FlowEvent::ConnectionProposed {
            source_node,
            source_port,
            target_node,
            target_port,
        } = event
        {
            assert_eq!(source_node, nid("n1"));
            assert_eq!(source_port.as_str(), "out");
            assert_eq!(target_node, nid("n2"));
            assert_eq!(target_port.as_str(), "in");
        } else {
            panic!("expected ConnectionProposed");
        }
    }

    // ---- TransactionSummary ----

    #[test]
    fn transaction_summary_default() {
        let summary = TransactionSummary::default();
        assert_eq!(summary.nodes_added, 0);
        assert_eq!(summary.nodes_removed, 0);
        assert_eq!(summary.nodes_updated, 0);
        assert_eq!(summary.edges_added, 0);
        assert_eq!(summary.edges_removed, 0);
        assert_eq!(summary.edges_updated, 0);
    }

    // ---- Diagnostic ----

    #[test]
    fn diagnostic_construction() {
        let diag = Diagnostic {
            severity: DiagnosticSeverity::Error,
            code: SmolStr::from("test-code"),
            message: String::from("test message"),
            node: Some(nid("n1")),
            edge: Some(eid("e1")),
        };
        assert_eq!(diag.severity, DiagnosticSeverity::Error);
        assert_eq!(diag.code.as_str(), "test-code");
        assert_eq!(diag.message, "test message");
        assert!(diag.node.is_some());
        assert!(diag.edge.is_some());
    }

    #[test]
    fn diagnostic_severity_ordering() {
        assert_ne!(DiagnosticSeverity::Error, DiagnosticSeverity::Warning);
        assert_ne!(DiagnosticSeverity::Warning, DiagnosticSeverity::Info);
        assert_ne!(DiagnosticSeverity::Error, DiagnosticSeverity::Info);
    }

    // ---- Patch clone ----

    #[test]
    fn patch_clone_preserves_data() {
        let patch = FlowPatch::InsertNode {
            node: make_node("n1"),
        };
        let cloned = patch.clone();
        if let FlowPatch::InsertNode { node } = cloned {
            assert_eq!(node.id, nid("n1"));
        } else {
            panic!("expected InsertNode after clone");
        }
    }

    // ---- Serialization ----

    #[test]
    fn patch_serialization_roundtrip() {
        let patch = FlowPatch::InsertNode {
            node: make_node("n1"),
        };
        let json = serde_json::to_string(&patch).expect("serialize");
        let back: FlowPatch = serde_json::from_str(&json).expect("deserialize");
        if let FlowPatch::InsertNode { node } = back {
            assert_eq!(node.id, nid("n1"));
        } else {
            panic!("expected InsertNode after roundtrip");
        }
    }

    #[test]
    fn node_change_set_serialization_roundtrip() {
        let cs = NodeChangeSet {
            position: Some([1.0, 2.0]),
            title: Some(SmolStr::from("new-title")),
            ..NodeChangeSet::default()
        };
        let json = serde_json::to_string(&cs).expect("serialize");
        let back: NodeChangeSet = serde_json::from_str(&json).expect("deserialize");
        assert!(back.position.is_some());
        assert!(back.title.is_some());
        assert!(back.size.is_none());
    }

    #[test]
    fn edge_change_set_serialization_roundtrip() {
        let cs = EdgeChangeSet {
            label: Some(Some(SmolStr::from("label"))),
            ..EdgeChangeSet::default()
        };
        let json = serde_json::to_string(&cs).expect("serialize");
        let back: EdgeChangeSet = serde_json::from_str(&json).expect("deserialize");
        assert!(back.label.is_some());
    }

    #[test]
    fn group_change_set_serialization_roundtrip() {
        let cs = GroupChangeSet {
            title: Some(SmolStr::from("group-title")),
            bounds: Some([0.0, 0.0, 100.0, 100.0]),
            ..GroupChangeSet::default()
        };
        let json = serde_json::to_string(&cs).expect("serialize");
        let back: GroupChangeSet = serde_json::from_str(&json).expect("deserialize");
        assert!(back.title.is_some());
        assert!(back.bounds.is_some());
    }

    // ---- EdgeChangeSet label semantics ----

    #[test]
    fn edge_change_set_label_none_means_no_change() {
        let cs = EdgeChangeSet::default();
        assert!(cs.label.is_none());
    }

    #[test]
    fn edge_change_set_label_some_none_means_clear_label() {
        let cs = EdgeChangeSet {
            label: Some(None),
            ..EdgeChangeSet::default()
        };
        // Some(None) = clear the label, not "no change"
        assert!(cs.label.is_some());
        assert!(cs.label.as_ref().is_some_and(|l| l.is_none()));
    }

    #[test]
    fn edge_change_set_label_some_some_means_set_label() {
        let cs = EdgeChangeSet {
            label: Some(Some(SmolStr::from("new"))),
            ..EdgeChangeSet::default()
        };
        assert!(cs.label.as_ref().is_some_and(|l| l.is_some()));
    }
}
