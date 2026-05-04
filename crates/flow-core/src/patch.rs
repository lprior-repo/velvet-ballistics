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

// ---------------------------------------------------------------------------
// Patch engine - applies patches to a FlowDocument with undo/redo support
// ---------------------------------------------------------------------------

/// Error returned when a patch cannot be applied.
#[derive(Clone, Debug)]
pub struct PatchError {
    pub message: String,
}

/// Patch engine that tracks undo history.
#[derive(Clone, Debug)]
pub struct PatchEngine {
    undo_stack: Vec<Vec<FlowPatch>>,
    redo_stack: Vec<Vec<FlowPatch>>,
}

impl Default for PatchEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl PatchEngine {
    pub fn new() -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
        }
    }

    /// Apply a single patch to the document. Returns the inverse patch on success.
    pub fn apply_patch(
        &mut self,
        doc: &mut FlowDocument,
        patch: FlowPatch,
    ) -> Result<Option<FlowPatch>, PatchError> {
        let inverse = apply_single(doc, &patch)?;
        if let Some(inv) = inverse.as_ref()
            && let Some(last) = self.undo_stack.last_mut()
        {
            last.push(inv.clone());
        }
        Ok(inverse)
    }

    /// Begin recording patches for a new undo frame.
    pub fn begin_undo_frame(&mut self) {
        self.undo_stack.push(Vec::new());
        self.redo_stack.clear();
    }

    /// Apply a full transaction (begin frame, apply patches, finalize).
    pub fn apply_transaction(
        &mut self,
        doc: &mut FlowDocument,
        txn: &FlowTransaction,
    ) -> Result<TransactionSummary, PatchError> {
        self.begin_undo_frame();
        let mut summary = TransactionSummary::default();
        for patch in &txn.patches {
            apply_single(doc, patch)?;
            match patch {
                FlowPatch::InsertNode { .. } => {
                    summary.nodes_added = summary.nodes_added.saturating_add(1);
                }
                FlowPatch::RemoveNode { .. } => {
                    summary.nodes_removed = summary.nodes_removed.saturating_add(1);
                }
                FlowPatch::UpdateNode { .. } => {
                    summary.nodes_updated = summary.nodes_updated.saturating_add(1);
                }
                FlowPatch::InsertEdge { .. } => {
                    summary.edges_added = summary.edges_added.saturating_add(1);
                }
                FlowPatch::RemoveEdge { .. } => {
                    summary.edges_removed = summary.edges_removed.saturating_add(1);
                }
                FlowPatch::UpdateEdge { .. } => {
                    summary.edges_updated = summary.edges_updated.saturating_add(1);
                }
                _ => {}
            }
        }
        Ok(summary)
    }

    /// Undo the last transaction frame. Returns the inverted patches, or None if
    /// the undo stack is empty.
    pub fn undo(&mut self, doc: &mut FlowDocument) -> Option<Vec<FlowPatch>> {
        let frame = self.undo_stack.pop()?;
        let mut redo_frame: Vec<FlowPatch> = Vec::new();
        let mut inverses: Vec<FlowPatch> = frame.into_iter().rev().collect();
        for inv_patch in &inverses {
            let redo_patch = apply_single(doc, inv_patch).ok().flatten();
            if let Some(rp) = redo_patch {
                redo_frame.push(rp);
            }
        }
        let result = core::mem::take(&mut inverses);
        self.redo_stack.push(redo_frame);
        Some(result)
    }

    /// Redo the last undone transaction frame.
    pub fn redo(&mut self, doc: &mut FlowDocument) -> Option<Vec<FlowPatch>> {
        let frame = self.redo_stack.pop()?;
        let mut undo_frame: Vec<FlowPatch> = Vec::new();
        let mut redos: Vec<FlowPatch> = frame.into_iter().rev().collect();
        for redo_patch in &redos {
            let undo_patch = apply_single(doc, redo_patch).ok().flatten();
            if let Some(up) = undo_patch {
                undo_frame.push(up);
            }
        }
        let result = core::mem::take(&mut redos);
        self.undo_stack.push(undo_frame);
        Some(result)
    }

    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    pub fn undo_depth(&self) -> usize {
        self.undo_stack.len()
    }

    pub fn redo_depth(&self) -> usize {
        self.redo_stack.len()
    }
}

/// Apply a single patch to the document, returning the inverse patch if applicable.
fn apply_single(
    doc: &mut FlowDocument,
    patch: &FlowPatch,
) -> Result<Option<FlowPatch>, PatchError> {
    match patch {
        FlowPatch::InsertNode { node } => {
            if doc.graph.nodes.contains_key(&node.id) {
                return Err(PatchError {
                    message: format!("node '{}' already exists", node.id),
                });
            }
            let id = node.id.clone();
            doc.graph.nodes.insert(id, node.clone());
            Ok(Some(FlowPatch::RemoveNode {
                id: node.id.clone(),
            }))
        }

        FlowPatch::UpdateNode { id, changes } => {
            let node = doc.graph.nodes.get_mut(id).ok_or_else(|| PatchError {
                message: format!("node '{}' not found for update", id),
            })?;
            apply_node_changes(node, changes);
            Ok(None)
        }

        FlowPatch::RemoveNode { id } => {
            let removed = doc
                .graph
                .nodes
                .shift_remove(id)
                .ok_or_else(|| PatchError {
                    message: format!("node '{}' not found for removal", id),
                })?;
            let connected: Vec<EdgeId> = doc
                .graph
                .edges
                .iter()
                .filter(|(_, e)| e.source_node == *id || e.target_node == *id)
                .map(|(eid, _)| eid.clone())
                .collect();
            for edge_id in connected {
                doc.graph.edges.shift_remove(&edge_id);
            }
            Ok(Some(FlowPatch::InsertNode { node: removed }))
        }

        FlowPatch::InsertEdge { edge } => {
            if doc.graph.edges.contains_key(&edge.id) {
                return Err(PatchError {
                    message: format!("edge '{}' already exists", edge.id),
                });
            }
            let id = edge.id.clone();
            doc.graph.edges.insert(id, edge.clone());
            Ok(Some(FlowPatch::RemoveEdge {
                id: edge.id.clone(),
            }))
        }

        FlowPatch::UpdateEdge { id, changes } => {
            let edge = doc.graph.edges.get_mut(id).ok_or_else(|| PatchError {
                message: format!("edge '{}' not found for update", id),
            })?;
            apply_edge_changes(edge, changes);
            Ok(None)
        }

        FlowPatch::RemoveEdge { id } => {
            let removed = doc
                .graph
                .edges
                .shift_remove(id)
                .ok_or_else(|| PatchError {
                    message: format!("edge '{}' not found for removal", id),
                })?;
            Ok(Some(FlowPatch::InsertEdge { edge: removed }))
        }

        FlowPatch::InsertGroup { group } => {
            if doc.graph.groups.contains_key(&group.id) {
                return Err(PatchError {
                    message: format!("group '{}' already exists", group.id),
                });
            }
            let id = group.id.clone();
            doc.graph.groups.insert(id, group.clone());
            Ok(Some(FlowPatch::RemoveGroup {
                id: group.id.clone(),
            }))
        }

        FlowPatch::UpdateGroup { id, changes } => {
            let group = doc.graph.groups.get_mut(id).ok_or_else(|| PatchError {
                message: format!("group '{}' not found for update", id),
            })?;
            apply_group_changes(group, changes);
            Ok(None)
        }

        FlowPatch::RemoveGroup { id } => {
            let removed = doc
                .graph
                .groups
                .shift_remove(id)
                .ok_or_else(|| PatchError {
                    message: format!("group '{}' not found for removal", id),
                })?;
            for node in doc.graph.nodes.values_mut() {
                if node.parent.as_ref() == Some(id) {
                    node.parent = None;
                }
            }
            Ok(Some(FlowPatch::InsertGroup { group: removed }))
        }

        FlowPatch::SetViewport { viewport } => {
            let old = doc.editor.viewport.clone();
            doc.editor.viewport = viewport.clone();
            Ok(Some(FlowPatch::SetViewport { viewport: old }))
        }

        FlowPatch::SetEntryNode { node } => {
            let old = doc.graph.entry_node.clone();
            doc.graph.entry_node = node.clone();
            Ok(Some(FlowPatch::SetEntryNode { node: old }))
        }

        FlowPatch::ReparentNodes {
            node_ids,
            new_parent,
        } => {
            for nid in node_ids {
                let node = doc.graph.nodes.get_mut(nid).ok_or_else(|| PatchError {
                    message: format!("node '{}' not found for reparent", nid),
                })?;
                node.parent = new_parent.clone();
            }
            Ok(None)
        }
    }
}

fn apply_node_changes(node: &mut FlowNodeRecord, changes: &NodeChangeSet) {
    if let Some(pos) = changes.position {
        node.position = pos;
    }
    if let Some(size) = changes.size {
        node.size = size;
    }
    if let Some(ref title) = changes.title {
        node.title = title.clone();
    }
    if let Some(ref kind) = changes.kind {
        node.kind = kind.clone();
    }
    if let Some(ref data) = changes.data {
        node.data = data.clone();
    }
    if let Some(ref flags) = changes.flags {
        node.flags = flags.clone();
    }
    if let Some(ref ui) = changes.ui {
        node.ui = ui.clone();
    }
}

fn apply_edge_changes(edge: &mut FlowEdgeRecord, changes: &EdgeChangeSet) {
    if let Some(ref label) = changes.label {
        edge.label = label.clone();
    }
    if let Some(ref style) = changes.style {
        edge.style = style.clone();
    }
    if let Some(ref data) = changes.data {
        edge.data = data.clone();
    }
}

fn apply_group_changes(group: &mut FlowGroupRecord, changes: &GroupChangeSet) {
    if let Some(ref title) = changes.title {
        group.title = title.clone();
    }
    if let Some(bounds) = changes.bounds {
        group.bounds = bounds;
    }
    if let Some(ref data) = changes.data {
        group.data = data.clone();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::doc::{
        EdgeMarker, EdgeStyle, FlowEdgeRecord, FlowGroupRecord, FlowNodeRecord, GroupKind,
        LineStyle, NodeFlags, NodeUiState, SelectionState, ViewportState,
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
    fn patch_insert_node() -> Result<(), String> {
        let node = make_node("n1");
        let patch = FlowPatch::InsertNode { node };
        match &patch {
            FlowPatch::InsertNode { node: n } => {
                if n.id != nid("n1") {
                    return Err(String::from("expected node id n1"));
                }
                Ok(())
            }
            other => Err(format!("expected InsertNode variant, got {other:?}")),
        }
    }

    #[test]
    fn patch_update_node() -> Result<(), String> {
        let changes = NodeChangeSet {
            position: Some([10.0, 20.0]),
            ..NodeChangeSet::default()
        };
        let patch = FlowPatch::UpdateNode {
            id: nid("n1"),
            changes,
        };
        match patch {
            FlowPatch::UpdateNode { id, changes } => {
                if id != nid("n1") {
                    return Err(String::from("expected id n1"));
                }
                if changes.position.is_none() {
                    return Err(String::from("expected position to be set"));
                }
                Ok(())
            }
            other => Err(format!("expected UpdateNode variant, got {other:?}")),
        }
    }

    #[test]
    fn patch_remove_node() -> Result<(), String> {
        let patch = FlowPatch::RemoveNode { id: nid("n1") };
        match patch {
            FlowPatch::RemoveNode { id } => {
                if id != nid("n1") {
                    return Err(String::from("expected id n1"));
                }
                Ok(())
            }
            other => Err(format!("expected RemoveNode variant, got {other:?}")),
        }
    }

    #[test]
    fn patch_insert_edge() -> Result<(), String> {
        let edge = make_edge("e1", "n1", "n2");
        let patch = FlowPatch::InsertEdge { edge };
        match &patch {
            FlowPatch::InsertEdge { edge: e } => {
                if e.id != eid("e1") {
                    return Err(String::from("expected edge id e1"));
                }
                Ok(())
            }
            other => Err(format!("expected InsertEdge variant, got {other:?}")),
        }
    }

    #[test]
    fn patch_update_edge() -> Result<(), String> {
        let changes = EdgeChangeSet {
            label: Some(Some(SmolStr::from("new-label"))),
            ..EdgeChangeSet::default()
        };
        let patch = FlowPatch::UpdateEdge {
            id: eid("e1"),
            changes,
        };
        match patch {
            FlowPatch::UpdateEdge { id, changes } => {
                if id != eid("e1") {
                    return Err(String::from("expected id e1"));
                }
                if changes.label.is_none() {
                    return Err(String::from("expected label to be set"));
                }
                Ok(())
            }
            other => Err(format!("expected UpdateEdge variant, got {other:?}")),
        }
    }

    #[test]
    fn patch_remove_edge() -> Result<(), String> {
        let patch = FlowPatch::RemoveEdge { id: eid("e1") };
        match patch {
            FlowPatch::RemoveEdge { id } => {
                if id != eid("e1") {
                    return Err(String::from("expected id e1"));
                }
                Ok(())
            }
            other => Err(format!("expected RemoveEdge variant, got {other:?}")),
        }
    }

    #[test]
    fn patch_insert_group() -> Result<(), String> {
        let group = make_group("g1");
        let patch = FlowPatch::InsertGroup { group };
        match &patch {
            FlowPatch::InsertGroup { group: g } => {
                if g.id != gid("g1") {
                    return Err(String::from("expected group id g1"));
                }
                Ok(())
            }
            other => Err(format!("expected InsertGroup variant, got {other:?}")),
        }
    }

    #[test]
    fn patch_update_group() -> Result<(), String> {
        let changes = GroupChangeSet {
            title: Some(SmolStr::from("renamed")),
            ..GroupChangeSet::default()
        };
        let patch = FlowPatch::UpdateGroup {
            id: gid("g1"),
            changes,
        };
        match patch {
            FlowPatch::UpdateGroup { id, changes } => {
                if id != gid("g1") {
                    return Err(String::from("expected id g1"));
                }
                if changes.title.is_none() {
                    return Err(String::from("expected title to be set"));
                }
                Ok(())
            }
            other => Err(format!("expected UpdateGroup variant, got {other:?}")),
        }
    }

    #[test]
    fn patch_remove_group() -> Result<(), String> {
        let patch = FlowPatch::RemoveGroup { id: gid("g1") };
        match patch {
            FlowPatch::RemoveGroup { id } => {
                if id != gid("g1") {
                    return Err(String::from("expected id g1"));
                }
                Ok(())
            }
            other => Err(format!("expected RemoveGroup variant, got {other:?}")),
        }
    }

    #[test]
    fn patch_set_viewport() -> Result<(), String> {
        let vp = ViewportState {
            pan_x: 10.0,
            pan_y: -5.0,
            zoom: 2.0,
        };
        let patch = FlowPatch::SetViewport { viewport: vp };
        match patch {
            FlowPatch::SetViewport { viewport } => {
                if (viewport.pan_x - 10.0).abs() >= f64::EPSILON {
                    return Err(String::from("pan_x mismatch"));
                }
                if (viewport.zoom - 2.0).abs() >= f64::EPSILON {
                    return Err(String::from("zoom mismatch"));
                }
                Ok(())
            }
            other => Err(format!("expected SetViewport variant, got {other:?}")),
        }
    }

    #[test]
    fn patch_set_entry_node_some() -> Result<(), String> {
        let patch = FlowPatch::SetEntryNode {
            node: Some(nid("n1")),
        };
        match patch {
            FlowPatch::SetEntryNode { node } => {
                if node.is_none() {
                    return Err(String::from("expected Some"));
                }
                Ok(())
            }
            other => Err(format!("expected SetEntryNode variant, got {other:?}")),
        }
    }

    #[test]
    fn patch_set_entry_node_none() -> Result<(), String> {
        let patch = FlowPatch::SetEntryNode { node: None };
        match patch {
            FlowPatch::SetEntryNode { node } => {
                if node.is_some() {
                    return Err(String::from("expected None"));
                }
                Ok(())
            }
            other => Err(format!("expected SetEntryNode variant, got {other:?}")),
        }
    }

    #[test]
    fn patch_reparent_nodes() -> Result<(), String> {
        let patch = FlowPatch::ReparentNodes {
            node_ids: vec![nid("n1"), nid("n2")],
            new_parent: Some(gid("g1")),
        };
        match patch {
            FlowPatch::ReparentNodes {
                node_ids,
                new_parent,
            } => {
                if node_ids.len() != 2 {
                    return Err(String::from("expected 2 node_ids"));
                }
                if new_parent.is_none() {
                    return Err(String::from("expected Some parent"));
                }
                Ok(())
            }
            other => Err(format!("expected ReparentNodes variant, got {other:?}")),
        }
    }

    #[test]
    fn patch_reparent_nodes_remove_parent() -> Result<(), String> {
        let patch = FlowPatch::ReparentNodes {
            node_ids: vec![nid("n1")],
            new_parent: None,
        };
        match patch {
            FlowPatch::ReparentNodes {
                node_ids,
                new_parent,
            } => {
                if node_ids.len() != 1 {
                    return Err(String::from("expected 1 node_id"));
                }
                if new_parent.is_some() {
                    return Err(String::from("expected None parent"));
                }
                Ok(())
            }
            other => Err(format!("expected ReparentNodes variant, got {other:?}")),
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
    fn command_apply_transaction() -> Result<(), String> {
        let txn = FlowTransaction {
            id: 1,
            label: SmolStr::from("test"),
            patches: Vec::new(),
            origin: ChangeOrigin::User,
            merge_key: None,
        };
        let cmd = FlowCommand::ApplyTransaction(txn);
        match cmd {
            FlowCommand::ApplyTransaction(t) => {
                if t.id != 1 {
                    return Err(String::from("expected id 1"));
                }
                Ok(())
            }
            other => Err(format!("expected ApplyTransaction, got {other:?}")),
        }
    }

    #[test]
    fn command_variants_exist() {
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
    fn event_transaction_committed() -> Result<(), String> {
        let summary = TransactionSummary::default();
        let event = FlowEvent::TransactionCommitted { summary };
        match event {
            FlowEvent::TransactionCommitted { summary } => {
                if summary.nodes_added != 0 {
                    return Err(String::from("expected 0 nodes_added"));
                }
                Ok(())
            }
            other => Err(format!("expected TransactionCommitted, got {other:?}")),
        }
    }

    #[test]
    fn event_transaction_rejected() -> Result<(), String> {
        let event = FlowEvent::TransactionRejected {
            reason: String::from("validation failed"),
        };
        match event {
            FlowEvent::TransactionRejected { reason } => {
                if reason != "validation failed" {
                    return Err(format!("unexpected reason: {reason}"));
                }
                Ok(())
            }
            other => Err(format!("expected TransactionRejected, got {other:?}")),
        }
    }

    #[test]
    fn event_selection_changed() -> Result<(), String> {
        let sel = SelectionState::default();
        let event = FlowEvent::SelectionChanged(sel);
        match event {
            FlowEvent::SelectionChanged(s) => {
                if !s.selected_nodes.is_empty() {
                    return Err(String::from("expected empty selection"));
                }
                Ok(())
            }
            other => Err(format!("expected SelectionChanged, got {other:?}")),
        }
    }

    #[test]
    fn event_viewport_changed() -> Result<(), String> {
        let vp = ViewportState::default();
        let event = FlowEvent::ViewportChanged(vp);
        match event {
            FlowEvent::ViewportChanged(v) => {
                if (v.zoom - 1.0).abs() >= f64::EPSILON {
                    return Err(String::from("zoom mismatch"));
                }
                Ok(())
            }
            other => Err(format!("expected ViewportChanged, got {other:?}")),
        }
    }

    #[test]
    fn event_diagnostics_changed() -> Result<(), String> {
        let event = FlowEvent::DiagnosticsChanged(Vec::new());
        match event {
            FlowEvent::DiagnosticsChanged(d) => {
                if !d.is_empty() {
                    return Err(String::from("expected empty diagnostics"));
                }
                Ok(())
            }
            other => Err(format!("expected DiagnosticsChanged, got {other:?}")),
        }
    }

    #[test]
    fn event_connection_proposed() -> Result<(), String> {
        let event = FlowEvent::ConnectionProposed {
            source_node: nid("n1"),
            source_port: SmolStr::from("out"),
            target_node: nid("n2"),
            target_port: SmolStr::from("in"),
        };
        match event {
            FlowEvent::ConnectionProposed {
                source_node,
                source_port,
                target_node,
                target_port,
            } => {
                if source_node != nid("n1") {
                    return Err(String::from("source_node mismatch"));
                }
                if source_port.as_str() != "out" {
                    return Err(String::from("source_port mismatch"));
                }
                if target_node != nid("n2") {
                    return Err(String::from("target_node mismatch"));
                }
                if target_port.as_str() != "in" {
                    return Err(String::from("target_port mismatch"));
                }
                Ok(())
            }
            other => Err(format!("expected ConnectionProposed, got {other:?}")),
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
    fn patch_clone_preserves_data() -> Result<(), String> {
        let patch = FlowPatch::InsertNode {
            node: make_node("n1"),
        };
        let cloned = patch.clone();
        match cloned {
            FlowPatch::InsertNode { node } => {
                if node.id != nid("n1") {
                    return Err(String::from("id mismatch after clone"));
                }
                Ok(())
            }
            other => Err(format!("expected InsertNode after clone, got {other:?}")),
        }
    }

    // ---- Serialization ----

    #[test]
    fn patch_serialization_roundtrip() -> Result<(), String> {
        let patch = FlowPatch::InsertNode {
            node: make_node("n1"),
        };
        let json = serde_json::to_string(&patch).map_err(|e| e.to_string())?;
        let back: FlowPatch = serde_json::from_str(&json).map_err(|e| e.to_string())?;
        match back {
            FlowPatch::InsertNode { node } => {
                if node.id != nid("n1") {
                    return Err(String::from("id mismatch after roundtrip"));
                }
                Ok(())
            }
            other => Err(format!("expected InsertNode after roundtrip, got {other:?}")),
        }
    }

    #[test]
    fn node_change_set_serialization_roundtrip() -> Result<(), String> {
        let cs = NodeChangeSet {
            position: Some([1.0, 2.0]),
            title: Some(SmolStr::from("new-title")),
            ..NodeChangeSet::default()
        };
        let json = serde_json::to_string(&cs).map_err(|e| e.to_string())?;
        let back: NodeChangeSet = serde_json::from_str(&json).map_err(|e| e.to_string())?;
        if back.position.is_none() || back.title.is_none() || back.size.is_some() {
            return Err(String::from("roundtrip field mismatch"));
        }
        Ok(())
    }

    #[test]
    fn edge_change_set_serialization_roundtrip() -> Result<(), String> {
        let cs = EdgeChangeSet {
            label: Some(Some(SmolStr::from("label"))),
            ..EdgeChangeSet::default()
        };
        let json = serde_json::to_string(&cs).map_err(|e| e.to_string())?;
        let back: EdgeChangeSet = serde_json::from_str(&json).map_err(|e| e.to_string())?;
        if back.label.is_none() {
            return Err(String::from("label lost in roundtrip"));
        }
        Ok(())
    }

    #[test]
    fn group_change_set_serialization_roundtrip() -> Result<(), String> {
        let cs = GroupChangeSet {
            title: Some(SmolStr::from("group-title")),
            bounds: Some([0.0, 0.0, 100.0, 100.0]),
            ..GroupChangeSet::default()
        };
        let json = serde_json::to_string(&cs).map_err(|e| e.to_string())?;
        let back: GroupChangeSet = serde_json::from_str(&json).map_err(|e| e.to_string())?;
        if back.title.is_none() || back.bounds.is_none() {
            return Err(String::from("fields lost in roundtrip"));
        }
        Ok(())
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

    // =========================================================================
    // NEW: PatchEngine - apply, undo, redo tests
    // =========================================================================

    // ---- PatchEngine basic construction ----

    #[test]
    fn engine_new_is_empty() {
        let engine = PatchEngine::new();
        assert!(!engine.can_undo());
        assert!(!engine.can_redo());
        assert_eq!(engine.undo_depth(), 0);
        assert_eq!(engine.redo_depth(), 0);
    }

    #[test]
    fn engine_default_is_new() {
        let engine = PatchEngine::default();
        assert!(!engine.can_undo());
        assert!(!engine.can_redo());
    }

    // ---- InsertNode via engine ----

    #[test]
    fn engine_apply_insert_node() {
        let mut doc = FlowDocument::default();
        let mut engine = PatchEngine::new();
        engine.begin_undo_frame();
        let result = engine.apply_patch(
            &mut doc,
            FlowPatch::InsertNode {
                node: make_node("n1"),
            },
        );
        assert!(result.is_ok());
        assert!(doc.graph.nodes.contains_key(&nid("n1")));
        assert_eq!(doc.graph.nodes.len(), 1);
    }

    #[test]
    fn engine_apply_insert_duplicate_node_fails() {
        let mut doc = FlowDocument::default();
        let mut engine = PatchEngine::new();
        engine.begin_undo_frame();
        let _ = engine.apply_patch(
            &mut doc,
            FlowPatch::InsertNode {
                node: make_node("n1"),
            },
        );
        let result = engine.apply_patch(
            &mut doc,
            FlowPatch::InsertNode {
                node: make_node("n1"),
            },
        );
        assert!(result.is_err());
    }

    // ---- RemoveNode via engine ----

    #[test]
    fn engine_apply_remove_node() {
        let mut doc = FlowDocument::default();
        doc.graph.nodes.insert(nid("n1"), make_node("n1"));
        let mut engine = PatchEngine::new();
        engine.begin_undo_frame();
        let result = engine.apply_patch(
            &mut doc,
            FlowPatch::RemoveNode { id: nid("n1") },
        );
        assert!(result.is_ok());
        assert!(!doc.graph.nodes.contains_key(&nid("n1")));
    }

    #[test]
    fn engine_apply_remove_nonexistent_node_fails() {
        let mut doc = FlowDocument::default();
        let mut engine = PatchEngine::new();
        engine.begin_undo_frame();
        let result = engine.apply_patch(
            &mut doc,
            FlowPatch::RemoveNode { id: nid("ghost") },
        );
        assert!(result.is_err());
    }

    #[test]
    fn engine_remove_node_also_removes_connected_edges() {
        let mut doc = FlowDocument::default();
        doc.graph.nodes.insert(nid("n1"), make_node("n1"));
        doc.graph.nodes.insert(nid("n2"), make_node("n2"));
        doc.graph.edges.insert(eid("e1"), make_edge("e1", "n1", "n2"));
        doc.graph.edges.insert(eid("e2"), make_edge("e2", "n2", "n1"));
        let mut engine = PatchEngine::new();
        engine.begin_undo_frame();
        let _ = engine.apply_patch(
            &mut doc,
            FlowPatch::RemoveNode { id: nid("n1") },
        );
        assert!(!doc.graph.edges.contains_key(&eid("e1")));
        assert!(!doc.graph.edges.contains_key(&eid("e2")));
        assert_eq!(doc.graph.edges.len(), 0);
    }

    // ---- UpdateNode via engine ----

    #[test]
    fn engine_apply_update_node_position() {
        let mut doc = FlowDocument::default();
        doc.graph.nodes.insert(nid("n1"), make_node("n1"));
        let mut engine = PatchEngine::new();
        engine.begin_undo_frame();
        let result = engine.apply_patch(
            &mut doc,
            FlowPatch::UpdateNode {
                id: nid("n1"),
                changes: NodeChangeSet {
                    position: Some([50.0, 100.0]),
                    ..NodeChangeSet::default()
                },
            },
        );
        assert!(result.is_ok());
        let node = doc.graph.nodes.get(&nid("n1"));
        assert!(node.is_some_and(|n| (n.position[0] - 50.0).abs() < f64::EPSILON));
        assert!(node.is_some_and(|n| (n.position[1] - 100.0).abs() < f64::EPSILON));
    }

    #[test]
    fn engine_apply_update_node_title() {
        let mut doc = FlowDocument::default();
        doc.graph.nodes.insert(nid("n1"), make_node("n1"));
        let mut engine = PatchEngine::new();
        engine.begin_undo_frame();
        let result = engine.apply_patch(
            &mut doc,
            FlowPatch::UpdateNode {
                id: nid("n1"),
                changes: NodeChangeSet {
                    title: Some(SmolStr::from("renamed")),
                    ..NodeChangeSet::default()
                },
            },
        );
        assert!(result.is_ok());
        let node = doc.graph.nodes.get(&nid("n1"));
        assert!(node.is_some_and(|n| n.title.as_str() == "renamed"));
    }

    #[test]
    fn engine_apply_update_node_flags() {
        let mut doc = FlowDocument::default();
        doc.graph.nodes.insert(nid("n1"), make_node("n1"));
        let mut engine = PatchEngine::new();
        engine.begin_undo_frame();
        let result = engine.apply_patch(
            &mut doc,
            FlowPatch::UpdateNode {
                id: nid("n1"),
                changes: NodeChangeSet {
                    flags: Some(NodeFlags {
                        locked: true,
                        hidden: true,
                        ..NodeFlags::default()
                    }),
                    ..NodeChangeSet::default()
                },
            },
        );
        assert!(result.is_ok());
        let node = doc.graph.nodes.get(&nid("n1"));
        assert!(node.is_some_and(|n| n.flags.locked));
        assert!(node.is_some_and(|n| n.flags.hidden));
    }

    #[test]
    fn engine_apply_update_nonexistent_node_fails() {
        let mut doc = FlowDocument::default();
        let mut engine = PatchEngine::new();
        engine.begin_undo_frame();
        let result = engine.apply_patch(
            &mut doc,
            FlowPatch::UpdateNode {
                id: nid("ghost"),
                changes: NodeChangeSet::default(),
            },
        );
        assert!(result.is_err());
    }

    #[test]
    fn engine_apply_update_node_partial_changes() {
        let mut doc = FlowDocument::default();
        let mut node = make_node("n1");
        node.position = [10.0, 20.0];
        node.title = SmolStr::from("original");
        doc.graph.nodes.insert(nid("n1"), node);
        let mut engine = PatchEngine::new();
        engine.begin_undo_frame();
        let result = engine.apply_patch(
            &mut doc,
            FlowPatch::UpdateNode {
                id: nid("n1"),
                changes: NodeChangeSet {
                    position: Some([99.0, 88.0]),
                    ..NodeChangeSet::default()
                },
            },
        );
        assert!(result.is_ok());
        let n = doc.graph.nodes.get(&nid("n1"));
        assert!(n.is_some_and(|n| (n.position[0] - 99.0).abs() < f64::EPSILON));
        assert!(n.is_some_and(|n| n.title.as_str() == "original"));
    }

    // ---- InsertEdge via engine ----

    #[test]
    fn engine_apply_insert_edge() {
        let mut doc = FlowDocument::default();
        doc.graph.nodes.insert(nid("n1"), make_node("n1"));
        doc.graph.nodes.insert(nid("n2"), make_node("n2"));
        let mut engine = PatchEngine::new();
        engine.begin_undo_frame();
        let result = engine.apply_patch(
            &mut doc,
            FlowPatch::InsertEdge {
                edge: make_edge("e1", "n1", "n2"),
            },
        );
        assert!(result.is_ok());
        assert!(doc.graph.edges.contains_key(&eid("e1")));
    }

    #[test]
    fn engine_apply_insert_duplicate_edge_fails() {
        let mut doc = FlowDocument::default();
        doc.graph.nodes.insert(nid("n1"), make_node("n1"));
        doc.graph.nodes.insert(nid("n2"), make_node("n2"));
        doc.graph.edges.insert(eid("e1"), make_edge("e1", "n1", "n2"));
        let mut engine = PatchEngine::new();
        engine.begin_undo_frame();
        let result = engine.apply_patch(
            &mut doc,
            FlowPatch::InsertEdge {
                edge: make_edge("e1", "n1", "n2"),
            },
        );
        assert!(result.is_err());
    }

    // ---- RemoveEdge via engine ----

    #[test]
    fn engine_apply_remove_edge() {
        let mut doc = FlowDocument::default();
        doc.graph.nodes.insert(nid("n1"), make_node("n1"));
        doc.graph.nodes.insert(nid("n2"), make_node("n2"));
        doc.graph.edges.insert(eid("e1"), make_edge("e1", "n1", "n2"));
        let mut engine = PatchEngine::new();
        engine.begin_undo_frame();
        let result = engine.apply_patch(
            &mut doc,
            FlowPatch::RemoveEdge { id: eid("e1") },
        );
        assert!(result.is_ok());
        assert!(!doc.graph.edges.contains_key(&eid("e1")));
    }

    #[test]
    fn engine_apply_remove_nonexistent_edge_fails() {
        let mut doc = FlowDocument::default();
        let mut engine = PatchEngine::new();
        engine.begin_undo_frame();
        let result = engine.apply_patch(
            &mut doc,
            FlowPatch::RemoveEdge { id: eid("ghost") },
        );
        assert!(result.is_err());
    }

    // ---- UpdateEdge via engine ----

    #[test]
    fn engine_apply_update_edge_label() {
        let mut doc = FlowDocument::default();
        doc.graph.nodes.insert(nid("n1"), make_node("n1"));
        doc.graph.nodes.insert(nid("n2"), make_node("n2"));
        doc.graph.edges.insert(eid("e1"), make_edge("e1", "n1", "n2"));
        let mut engine = PatchEngine::new();
        engine.begin_undo_frame();
        let result = engine.apply_patch(
            &mut doc,
            FlowPatch::UpdateEdge {
                id: eid("e1"),
                changes: EdgeChangeSet {
                    label: Some(Some(SmolStr::from("new-label"))),
                    ..EdgeChangeSet::default()
                },
            },
        );
        assert!(result.is_ok());
        let edge = doc.graph.edges.get(&eid("e1"));
        assert!(
            edge.is_some_and(|e| e
                .label
                .as_ref()
                .is_some_and(|l| l.as_str() == "new-label"))
        );
    }

    #[test]
    fn engine_apply_update_edge_clear_label() {
        let mut doc = FlowDocument::default();
        doc.graph.nodes.insert(nid("n1"), make_node("n1"));
        doc.graph.nodes.insert(nid("n2"), make_node("n2"));
        let mut edge = make_edge("e1", "n1", "n2");
        edge.label = Some(SmolStr::from("old"));
        doc.graph.edges.insert(eid("e1"), edge);
        let mut engine = PatchEngine::new();
        engine.begin_undo_frame();
        let result = engine.apply_patch(
            &mut doc,
            FlowPatch::UpdateEdge {
                id: eid("e1"),
                changes: EdgeChangeSet {
                    label: Some(None),
                    ..EdgeChangeSet::default()
                },
            },
        );
        assert!(result.is_ok());
        let edge = doc.graph.edges.get(&eid("e1"));
        assert!(edge.is_some_and(|e| e.label.is_none()));
    }

    #[test]
    fn engine_apply_update_nonexistent_edge_fails() {
        let mut doc = FlowDocument::default();
        let mut engine = PatchEngine::new();
        engine.begin_undo_frame();
        let result = engine.apply_patch(
            &mut doc,
            FlowPatch::UpdateEdge {
                id: eid("ghost"),
                changes: EdgeChangeSet::default(),
            },
        );
        assert!(result.is_err());
    }

    // ---- InsertGroup via engine ----

    #[test]
    fn engine_apply_insert_group() {
        let mut doc = FlowDocument::default();
        let mut engine = PatchEngine::new();
        engine.begin_undo_frame();
        let result = engine.apply_patch(
            &mut doc,
            FlowPatch::InsertGroup {
                group: make_group("g1"),
            },
        );
        assert!(result.is_ok());
        assert!(doc.graph.groups.contains_key(&gid("g1")));
    }

    #[test]
    fn engine_apply_insert_duplicate_group_fails() {
        let mut doc = FlowDocument::default();
        doc.graph.groups.insert(gid("g1"), make_group("g1"));
        let mut engine = PatchEngine::new();
        engine.begin_undo_frame();
        let result = engine.apply_patch(
            &mut doc,
            FlowPatch::InsertGroup {
                group: make_group("g1"),
            },
        );
        assert!(result.is_err());
    }

    // ---- RemoveGroup via engine ----

    #[test]
    fn engine_apply_remove_group() {
        let mut doc = FlowDocument::default();
        doc.graph.groups.insert(gid("g1"), make_group("g1"));
        let mut engine = PatchEngine::new();
        engine.begin_undo_frame();
        let result = engine.apply_patch(
            &mut doc,
            FlowPatch::RemoveGroup { id: gid("g1") },
        );
        assert!(result.is_ok());
        assert!(!doc.graph.groups.contains_key(&gid("g1")));
    }

    #[test]
    fn engine_remove_group_clears_child_parent_refs() {
        let mut doc = FlowDocument::default();
        doc.graph.groups.insert(gid("g1"), make_group("g1"));
        let mut n1 = make_node("n1");
        n1.parent = Some(gid("g1"));
        doc.graph.nodes.insert(nid("n1"), n1);
        let mut engine = PatchEngine::new();
        engine.begin_undo_frame();
        let _ = engine.apply_patch(
            &mut doc,
            FlowPatch::RemoveGroup { id: gid("g1") },
        );
        let node = doc.graph.nodes.get(&nid("n1"));
        assert!(node.is_some_and(|n| n.parent.is_none()));
    }

    // ---- UpdateGroup via engine ----

    #[test]
    fn engine_apply_update_group_title() {
        let mut doc = FlowDocument::default();
        doc.graph.groups.insert(gid("g1"), make_group("g1"));
        let mut engine = PatchEngine::new();
        engine.begin_undo_frame();
        let result = engine.apply_patch(
            &mut doc,
            FlowPatch::UpdateGroup {
                id: gid("g1"),
                changes: GroupChangeSet {
                    title: Some(SmolStr::from("renamed-group")),
                    ..GroupChangeSet::default()
                },
            },
        );
        assert!(result.is_ok());
        let group = doc.graph.groups.get(&gid("g1"));
        assert!(group.is_some_and(|g| g.title.as_str() == "renamed-group"));
    }

    #[test]
    fn engine_apply_update_group_bounds() {
        let mut doc = FlowDocument::default();
        doc.graph.groups.insert(gid("g1"), make_group("g1"));
        let mut engine = PatchEngine::new();
        engine.begin_undo_frame();
        let result = engine.apply_patch(
            &mut doc,
            FlowPatch::UpdateGroup {
                id: gid("g1"),
                changes: GroupChangeSet {
                    bounds: Some([10.0, 20.0, 300.0, 400.0]),
                    ..GroupChangeSet::default()
                },
            },
        );
        assert!(result.is_ok());
        let group = doc.graph.groups.get(&gid("g1"));
        assert!(group.is_some_and(|g| (g.bounds[0] - 10.0).abs() < f64::EPSILON));
        assert!(group.is_some_and(|g| (g.bounds[2] - 300.0).abs() < f64::EPSILON));
    }

    // ---- SetViewport via engine ----

    #[test]
    fn engine_apply_set_viewport() {
        let mut doc = FlowDocument::default();
        let mut engine = PatchEngine::new();
        engine.begin_undo_frame();
        let new_vp = ViewportState {
            pan_x: 50.0,
            pan_y: -25.0,
            zoom: 3.0,
        };
        let result = engine.apply_patch(
            &mut doc,
            FlowPatch::SetViewport {
                viewport: new_vp,
            },
        );
        assert!(result.is_ok());
        assert!((doc.editor.viewport.pan_x - 50.0).abs() < f64::EPSILON);
        assert!((doc.editor.viewport.zoom - 3.0).abs() < f64::EPSILON);
    }

    // ---- SetEntryNode via engine ----

    #[test]
    fn engine_apply_set_entry_node() {
        let mut doc = FlowDocument::default();
        let mut engine = PatchEngine::new();
        engine.begin_undo_frame();
        let result = engine.apply_patch(
            &mut doc,
            FlowPatch::SetEntryNode {
                node: Some(nid("start")),
            },
        );
        assert!(result.is_ok());
        assert!(doc.graph.entry_node.is_some());
        assert!(
            doc.graph
                .entry_node
                .as_ref()
                .is_some_and(|n| n == &nid("start"))
        );
    }

    #[test]
    fn engine_apply_clear_entry_node() {
        let mut doc = FlowDocument::default();
        doc.graph.entry_node = Some(nid("old"));
        let mut engine = PatchEngine::new();
        engine.begin_undo_frame();
        let result = engine.apply_patch(
            &mut doc,
            FlowPatch::SetEntryNode { node: None },
        );
        assert!(result.is_ok());
        assert!(doc.graph.entry_node.is_none());
    }

    // ---- ReparentNodes via engine ----

    #[test]
    fn engine_apply_reparent_nodes() {
        let mut doc = FlowDocument::default();
        doc.graph.groups.insert(gid("g1"), make_group("g1"));
        doc.graph.nodes.insert(nid("n1"), make_node("n1"));
        doc.graph.nodes.insert(nid("n2"), make_node("n2"));
        let mut engine = PatchEngine::new();
        engine.begin_undo_frame();
        let result = engine.apply_patch(
            &mut doc,
            FlowPatch::ReparentNodes {
                node_ids: vec![nid("n1"), nid("n2")],
                new_parent: Some(gid("g1")),
            },
        );
        assert!(result.is_ok());
        assert!(
            doc.graph
                .nodes
                .get(&nid("n1"))
                .is_some_and(|n| n.parent == Some(gid("g1")))
        );
        assert!(
            doc.graph
                .nodes
                .get(&nid("n2"))
                .is_some_and(|n| n.parent == Some(gid("g1")))
        );
    }

    #[test]
    fn engine_apply_reparent_nodes_remove_parent() {
        let mut doc = FlowDocument::default();
        doc.graph.groups.insert(gid("g1"), make_group("g1"));
        let mut n1 = make_node("n1");
        n1.parent = Some(gid("g1"));
        doc.graph.nodes.insert(nid("n1"), n1);
        let mut engine = PatchEngine::new();
        engine.begin_undo_frame();
        let result = engine.apply_patch(
            &mut doc,
            FlowPatch::ReparentNodes {
                node_ids: vec![nid("n1")],
                new_parent: None,
            },
        );
        assert!(result.is_ok());
        assert!(
            doc.graph
                .nodes
                .get(&nid("n1"))
                .is_some_and(|n| n.parent.is_none())
        );
    }

    #[test]
    fn engine_apply_reparent_nonexistent_node_fails() {
        let mut doc = FlowDocument::default();
        let mut engine = PatchEngine::new();
        engine.begin_undo_frame();
        let result = engine.apply_patch(
            &mut doc,
            FlowPatch::ReparentNodes {
                node_ids: vec![nid("ghost")],
                new_parent: None,
            },
        );
        assert!(result.is_err());
    }

    // ---- Transaction application ----

    #[test]
    fn engine_apply_transaction_insert_nodes_and_edges() {
        let mut doc = FlowDocument::default();
        let mut engine = PatchEngine::new();
        let txn = FlowTransaction {
            id: 1,
            label: SmolStr::from("setup"),
            patches: vec![
                FlowPatch::InsertNode {
                    node: make_node("n1"),
                },
                FlowPatch::InsertNode {
                    node: make_node("n2"),
                },
                FlowPatch::InsertEdge {
                    edge: make_edge("e1", "n1", "n2"),
                },
            ],
            origin: ChangeOrigin::User,
            merge_key: None,
        };
        let result = engine.apply_transaction(&mut doc, &txn);
        assert!(result.is_ok());
        if let Ok(summary) = result {
            assert_eq!(summary.nodes_added, 2);
            assert_eq!(summary.edges_added, 1);
        }
        assert_eq!(doc.graph.nodes.len(), 2);
        assert_eq!(doc.graph.edges.len(), 1);
        assert!(engine.can_undo());
    }

    #[test]
    fn engine_apply_transaction_fails_partially() {
        let mut doc = FlowDocument::default();
        let mut engine = PatchEngine::new();
        let txn = FlowTransaction {
            id: 1,
            label: SmolStr::from("bad"),
            patches: vec![
                FlowPatch::InsertNode {
                    node: make_node("n1"),
                },
                FlowPatch::UpdateNode {
                    id: nid("ghost"),
                    changes: NodeChangeSet::default(),
                },
            ],
            origin: ChangeOrigin::User,
            merge_key: None,
        };
        let result = engine.apply_transaction(&mut doc, &txn);
        assert!(result.is_err());
        // n1 was already inserted before the failure -- partial application
        assert!(doc.graph.nodes.contains_key(&nid("n1")));
    }

    // ---- Undo ----

    #[test]
    fn engine_undo_insert_node() {
        let mut doc = FlowDocument::default();
        let mut engine = PatchEngine::new();
        engine.begin_undo_frame();
        let _ = engine.apply_patch(
            &mut doc,
            FlowPatch::InsertNode {
                node: make_node("n1"),
            },
        );
        assert!(doc.graph.nodes.contains_key(&nid("n1")));
        assert!(engine.can_undo());

        let undone = engine.undo(&mut doc);
        assert!(undone.is_some());
        assert!(!doc.graph.nodes.contains_key(&nid("n1")));
        assert!(!engine.can_undo());
        assert!(engine.can_redo());
    }

    #[test]
    fn engine_undo_remove_node_restores_it() {
        let mut doc = FlowDocument::default();
        doc.graph.nodes.insert(nid("n1"), make_node("n1"));
        let mut engine = PatchEngine::new();
        engine.begin_undo_frame();
        let _ = engine.apply_patch(
            &mut doc,
            FlowPatch::RemoveNode { id: nid("n1") },
        );
        assert!(!doc.graph.nodes.contains_key(&nid("n1")));

        let _ = engine.undo(&mut doc);
        assert!(doc.graph.nodes.contains_key(&nid("n1")));
    }

    #[test]
    fn engine_undo_set_entry_node() {
        let mut doc = FlowDocument::default();
        let mut engine = PatchEngine::new();
        engine.begin_undo_frame();
        let _ = engine.apply_patch(
            &mut doc,
            FlowPatch::SetEntryNode {
                node: Some(nid("start")),
            },
        );
        assert!(doc.graph.entry_node.is_some());

        let _ = engine.undo(&mut doc);
        assert!(doc.graph.entry_node.is_none());
    }

    #[test]
    fn engine_undo_set_viewport() {
        let mut doc = FlowDocument::default();
        let original_zoom = doc.editor.viewport.zoom;
        let mut engine = PatchEngine::new();
        engine.begin_undo_frame();
        let _ = engine.apply_patch(
            &mut doc,
            FlowPatch::SetViewport {
                viewport: ViewportState {
                    pan_x: 100.0,
                    pan_y: 200.0,
                    zoom: 5.0,
                },
            },
        );
        let _ = engine.undo(&mut doc);
        assert!((doc.editor.viewport.zoom - original_zoom).abs() < f64::EPSILON);
    }

    #[test]
    fn engine_undo_empty_returns_none() {
        let mut doc = FlowDocument::default();
        let mut engine = PatchEngine::new();
        assert!(engine.undo(&mut doc).is_none());
    }

    // ---- Redo ----

    #[test]
    fn engine_redo_after_undo() {
        let mut doc = FlowDocument::default();
        let mut engine = PatchEngine::new();
        engine.begin_undo_frame();
        let _ = engine.apply_patch(
            &mut doc,
            FlowPatch::InsertNode {
                node: make_node("n1"),
            },
        );
        let _ = engine.undo(&mut doc);
        assert!(!doc.graph.nodes.contains_key(&nid("n1")));

        let _ = engine.redo(&mut doc);
        assert!(doc.graph.nodes.contains_key(&nid("n1")));
        assert!(!engine.can_redo());
    }

    #[test]
    fn engine_redo_empty_returns_none() {
        let mut doc = FlowDocument::default();
        let mut engine = PatchEngine::new();
        assert!(engine.redo(&mut doc).is_none());
    }

    #[test]
    fn engine_redo_clears_on_new_frame() {
        let mut doc = FlowDocument::default();
        let mut engine = PatchEngine::new();
        engine.begin_undo_frame();
        let _ = engine.apply_patch(
            &mut doc,
            FlowPatch::InsertNode {
                node: make_node("n1"),
            },
        );
        let _ = engine.undo(&mut doc);
        assert!(engine.can_redo());

        // Beginning a new frame should clear the redo stack
        engine.begin_undo_frame();
        assert!(!engine.can_redo());
    }

    // ---- Multiple undo/redo levels ----

    #[test]
    fn engine_multiple_undo_levels() {
        let mut doc = FlowDocument::default();
        let mut engine = PatchEngine::new();

        // Frame 1: insert n1
        engine.begin_undo_frame();
        let _ = engine.apply_patch(
            &mut doc,
            FlowPatch::InsertNode {
                node: make_node("n1"),
            },
        );

        // Frame 2: insert n2
        engine.begin_undo_frame();
        let _ = engine.apply_patch(
            &mut doc,
            FlowPatch::InsertNode {
                node: make_node("n2"),
            },
        );

        assert_eq!(engine.undo_depth(), 2);

        // Undo frame 2
        let _ = engine.undo(&mut doc);
        assert_eq!(doc.graph.nodes.len(), 1);
        assert!(doc.graph.nodes.contains_key(&nid("n1")));

        // Undo frame 1
        let _ = engine.undo(&mut doc);
        assert!(doc.graph.nodes.is_empty());
    }

    // ---- Edge update style changes ----

    #[test]
    fn engine_apply_update_edge_style() {
        let mut doc = FlowDocument::default();
        doc.graph.nodes.insert(nid("n1"), make_node("n1"));
        doc.graph.nodes.insert(nid("n2"), make_node("n2"));
        doc.graph.edges.insert(eid("e1"), make_edge("e1", "n1", "n2"));
        let mut engine = PatchEngine::new();
        engine.begin_undo_frame();
        let new_style = EdgeStyle {
            line_style: LineStyle::Dashed,
            width: 4.0,
            animated: true,
            marker: EdgeMarker::Circle,
        };
        let result = engine.apply_patch(
            &mut doc,
            FlowPatch::UpdateEdge {
                id: eid("e1"),
                changes: EdgeChangeSet {
                    style: Some(new_style),
                    ..EdgeChangeSet::default()
                },
            },
        );
        assert!(result.is_ok());
        let edge = doc.graph.edges.get(&eid("e1"));
        assert!(edge.is_some_and(|e| e.style.line_style == LineStyle::Dashed));
        assert!(edge.is_some_and(|e| e.style.animated));
        assert!(edge.is_some_and(|e| e.style.marker == EdgeMarker::Circle));
    }

    // ---- PatchError ----

    #[test]
    fn patch_error_has_message() {
        let err = PatchError {
            message: String::from("something failed"),
        };
        assert_eq!(err.message, "something failed");
    }

    // ---- Serialization of all patch variants ----

    #[test]
    fn all_patch_variants_serialize_roundtrip() {
        let patches: Vec<FlowPatch> = vec![
            FlowPatch::InsertNode {
                node: make_node("n1"),
            },
            FlowPatch::UpdateNode {
                id: nid("n1"),
                changes: NodeChangeSet {
                    position: Some([1.0, 2.0]),
                    ..NodeChangeSet::default()
                },
            },
            FlowPatch::RemoveNode { id: nid("n1") },
            FlowPatch::InsertEdge {
                edge: make_edge("e1", "n1", "n2"),
            },
            FlowPatch::UpdateEdge {
                id: eid("e1"),
                changes: EdgeChangeSet {
                    label: Some(Some(SmolStr::from("label"))),
                    ..EdgeChangeSet::default()
                },
            },
            FlowPatch::RemoveEdge { id: eid("e1") },
            FlowPatch::InsertGroup {
                group: make_group("g1"),
            },
            FlowPatch::UpdateGroup {
                id: gid("g1"),
                changes: GroupChangeSet {
                    title: Some(SmolStr::from("new")),
                    ..GroupChangeSet::default()
                },
            },
            FlowPatch::RemoveGroup { id: gid("g1") },
            FlowPatch::SetViewport {
                viewport: ViewportState {
                    pan_x: 1.0,
                    pan_y: 2.0,
                    zoom: 3.0,
                },
            },
            FlowPatch::SetEntryNode {
                node: Some(nid("n1")),
            },
            FlowPatch::ReparentNodes {
                node_ids: vec![nid("n1")],
                new_parent: Some(gid("g1")),
            },
        ];
        for patch in patches {
            let json = serde_json::to_string(&patch).ok();
            assert!(json.is_some());
            let back: Option<FlowPatch> = json.and_then(|j| serde_json::from_str(&j).ok());
            assert!(back.is_some());
        }
    }

    // ---- Transaction summary counting ----

    #[test]
    fn transaction_summary_mixed_operations() {
        let mut doc = FlowDocument::default();
        doc.graph.nodes.insert(nid("n1"), make_node("n1"));
        let mut engine = PatchEngine::new();
        let txn = FlowTransaction {
            id: 1,
            label: SmolStr::from("mixed"),
            patches: vec![
                FlowPatch::InsertNode {
                    node: make_node("n2"),
                },
                FlowPatch::UpdateNode {
                    id: nid("n1"),
                    changes: NodeChangeSet {
                        title: Some(SmolStr::from("updated")),
                        ..NodeChangeSet::default()
                    },
                },
                FlowPatch::InsertEdge {
                    edge: make_edge("e1", "n1", "n2"),
                },
            ],
            origin: ChangeOrigin::User,
            merge_key: None,
        };
        let result = engine.apply_transaction(&mut doc, &txn);
        if let Ok(summary) = result {
            assert_eq!(summary.nodes_added, 1);
            assert_eq!(summary.nodes_updated, 1);
            assert_eq!(summary.edges_added, 1);
            assert_eq!(summary.nodes_removed, 0);
        }
    }

    // ---- Complex undo/redo round trip ----

    #[test]
    fn engine_undo_redo_roundtrip_preserves_document() {
        let mut doc = FlowDocument::default();
        let mut engine = PatchEngine::new();

        // Build up a document
        engine.begin_undo_frame();
        let _ = engine.apply_patch(
            &mut doc,
            FlowPatch::InsertNode {
                node: make_node("n1"),
            },
        );
        let _ = engine.apply_patch(
            &mut doc,
            FlowPatch::InsertNode {
                node: make_node("n2"),
            },
        );
        let _ = engine.apply_patch(
            &mut doc,
            FlowPatch::InsertEdge {
                edge: make_edge("e1", "n1", "n2"),
            },
        );
        let _ = engine.apply_patch(
            &mut doc,
            FlowPatch::SetEntryNode {
                node: Some(nid("n1")),
            },
        );

        // Snapshot state
        assert_eq!(doc.graph.nodes.len(), 2);
        assert_eq!(doc.graph.edges.len(), 1);
        let entry_before = doc.graph.entry_node.clone();

        // Undo everything
        let _ = engine.undo(&mut doc);
        assert!(doc.graph.entry_node.is_none());

        // Redo everything
        let _ = engine.redo(&mut doc);
        assert_eq!(doc.graph.nodes.len(), 2);
        assert_eq!(doc.graph.edges.len(), 1);
        assert_eq!(doc.graph.entry_node, entry_before);
    }

    // ---- Engine with empty transaction ----

    #[test]
    fn engine_apply_empty_transaction() {
        let mut doc = FlowDocument::default();
        let mut engine = PatchEngine::new();
        let txn = FlowTransaction {
            id: 1,
            label: SmolStr::from("empty"),
            patches: Vec::new(),
            origin: ChangeOrigin::User,
            merge_key: None,
        };
        let result = engine.apply_transaction(&mut doc, &txn);
        assert!(result.is_ok());
        if let Ok(summary) = result {
            assert_eq!(summary.nodes_added, 0);
            assert_eq!(summary.edges_added, 0);
        }
    }

    // ---- UpdateNode with all fields ----

    #[test]
    fn engine_apply_update_node_all_fields() {
        let mut doc = FlowDocument::default();
        doc.graph.nodes.insert(nid("n1"), make_node("n1"));
        let mut engine = PatchEngine::new();
        engine.begin_undo_frame();
        let result = engine.apply_patch(
            &mut doc,
            FlowPatch::UpdateNode {
                id: nid("n1"),
                changes: NodeChangeSet {
                    position: Some([11.0, 22.0]),
                    size: Some([200.0, 150.0]),
                    title: Some(SmolStr::from("all-fields")),
                    kind: Some(SmolStr::from("transform")),
                    data: Some(serde_json::Value::Bool(true)),
                    flags: Some(NodeFlags {
                        locked: true,
                        entry: true,
                        ..NodeFlags::default()
                    }),
                    ui: Some(NodeUiState {
                        collapsed: true,
                        color_override: Some([1.0, 0.0, 0.0, 1.0]),
                    }),
                },
            },
        );
        assert!(result.is_ok());
        let node = doc.graph.nodes.get(&nid("n1"));
        assert!(node.is_some_and(|n| (n.position[0] - 11.0).abs() < f64::EPSILON));
        assert!(node.is_some_and(|n| (n.size[0] - 200.0).abs() < f64::EPSILON));
        assert!(node.is_some_and(|n| n.title.as_str() == "all-fields"));
        assert!(node.is_some_and(|n| n.kind.as_str() == "transform"));
        assert!(node.is_some_and(|n| n.flags.locked));
        assert!(node.is_some_and(|n| n.flags.entry));
        assert!(node.is_some_and(|n| n.ui.collapsed));
    }

    // ---- Engine clone ----

    #[test]
    fn engine_clone_preserves_state() {
        let mut engine = PatchEngine::new();
        engine.begin_undo_frame();
        assert_eq!(engine.undo_depth(), 1);
        let cloned = engine.clone();
        assert_eq!(cloned.undo_depth(), 1);
    }
}
