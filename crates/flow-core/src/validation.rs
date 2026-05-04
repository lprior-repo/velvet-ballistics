use crate::doc::*;
use crate::ids::*;
use crate::patch::{Diagnostic, DiagnosticSeverity};
use smol_str::SmolStr;
use std::collections::{HashMap, HashSet};

// ---------------------------------------------------------------------------
// Validator trait
// ---------------------------------------------------------------------------

pub trait FlowValidator: Send + Sync {
    fn validate(&self, doc: &FlowDocument) -> Vec<Diagnostic>;
}

// ---------------------------------------------------------------------------
// Structural validator
// ---------------------------------------------------------------------------

pub struct StructuralValidator;

impl FlowValidator for StructuralValidator {
    fn validate(&self, doc: &FlowDocument) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        check_entry_node_exists(&doc.graph, &mut diagnostics);
        check_edge_endpoints(&doc.graph, &mut diagnostics);
        check_edge_ports(&doc.graph, &mut diagnostics);
        check_self_loops(&doc.graph, &mut diagnostics);
        check_group_members(&doc.graph, &mut diagnostics);

        diagnostics
    }
}

// ---------------------------------------------------------------------------
// Structural checks
// ---------------------------------------------------------------------------

fn diag(
    severity: DiagnosticSeverity,
    code: &str,
    message: String,
    node: Option<NodeId>,
    edge: Option<EdgeId>,
) -> Diagnostic {
    Diagnostic {
        severity,
        code: SmolStr::from(code),
        message,
        node,
        edge,
    }
}

fn check_entry_node_exists(graph: &FlowGraph, diagnostics: &mut Vec<Diagnostic>) {
    if let Some(ref entry_id) = graph.entry_node
        && !graph.nodes.contains_key(entry_id)
    {
        diagnostics.push(diag(
            DiagnosticSeverity::Error,
            "entry-node-missing",
            format!("entry_node '{entry_id}' does not reference a valid node"),
            Some(entry_id.clone()),
            None,
        ));
    }
}

fn check_edge_endpoints(graph: &FlowGraph, diagnostics: &mut Vec<Diagnostic>) {
    for (edge_id, edge) in &graph.edges {
        if !graph.nodes.contains_key(&edge.source_node) {
            diagnostics.push(diag(
                DiagnosticSeverity::Error,
                "edge-source-missing",
                format!(
                    "edge '{}' references non-existent source node '{}'",
                    edge_id, edge.source_node
                ),
                Some(edge.source_node.clone()),
                Some(edge_id.clone()),
            ));
        }
        if !graph.nodes.contains_key(&edge.target_node) {
            diagnostics.push(diag(
                DiagnosticSeverity::Error,
                "edge-target-missing",
                format!(
                    "edge '{}' references non-existent target node '{}'",
                    edge_id, edge.target_node
                ),
                Some(edge.target_node.clone()),
                Some(edge_id.clone()),
            ));
        }
    }
}

fn check_edge_ports(graph: &FlowGraph, diagnostics: &mut Vec<Diagnostic>) {
    for (edge_id, edge) in &graph.edges {
        if let Some(source_node) = graph.nodes.get(&edge.source_node) {
            let port_exists = source_node.ports.iter().any(|p| p.id == edge.source_port);
            if !port_exists {
                diagnostics.push(diag(
                    DiagnosticSeverity::Error,
                    "edge-source-port-missing",
                    format!(
                        "edge '{}' references non-existent source port '{}' on node '{}'",
                        edge_id, edge.source_port, edge.source_node
                    ),
                    Some(edge.source_node.clone()),
                    Some(edge_id.clone()),
                ));
            }
        }

        if let Some(target_node) = graph.nodes.get(&edge.target_node) {
            let port_exists = target_node.ports.iter().any(|p| p.id == edge.target_port);
            if !port_exists {
                diagnostics.push(diag(
                    DiagnosticSeverity::Error,
                    "edge-target-port-missing",
                    format!(
                        "edge '{}' references non-existent target port '{}' on node '{}'",
                        edge_id, edge.target_port, edge.target_node
                    ),
                    Some(edge.target_node.clone()),
                    Some(edge_id.clone()),
                ));
            }
        }
    }
}

fn check_self_loops(graph: &FlowGraph, diagnostics: &mut Vec<Diagnostic>) {
    for (edge_id, edge) in &graph.edges {
        if edge.source_node == edge.target_node && edge.source_port == edge.target_port {
            diagnostics.push(diag(
                DiagnosticSeverity::Warning,
                "self-loop-same-port",
                format!(
                    "edge '{}' is a self-loop on the same port '{}' of node '{}'",
                    edge_id, edge.source_port, edge.source_node
                ),
                Some(edge.source_node.clone()),
                Some(edge_id.clone()),
            ));
        }
    }
}

fn check_group_members(graph: &FlowGraph, diagnostics: &mut Vec<Diagnostic>) {
    for (node_id, node) in &graph.nodes {
        if let Some(ref group_id) = node.parent
            && !graph.groups.contains_key(group_id)
        {
            diagnostics.push(diag(
                DiagnosticSeverity::Error,
                "node-parent-group-missing",
                format!(
                    "node '{}' references non-existent parent group '{}'",
                    node_id, group_id
                ),
                Some(node_id.clone()),
                None,
            ));
        }
    }
}

// ---------------------------------------------------------------------------
// Semantic validator
// ---------------------------------------------------------------------------

pub struct SemanticValidator;

impl FlowValidator for SemanticValidator {
    fn validate(&self, doc: &FlowDocument) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        semantic_check_edge_node_existence(&doc.graph, &mut diagnostics);
        semantic_check_nonempty_kind_orphans(&doc.graph, &mut diagnostics);
        semantic_check_group_member_validity(&doc.graph, &mut diagnostics);
        semantic_check_duplicate_port_connections(&doc.graph, &mut diagnostics);

        diagnostics
    }
}

// ---------------------------------------------------------------------------
// Semantic checks
// ---------------------------------------------------------------------------

/// Check 1: Every edge's source_node and target_node exist as nodes in the
/// document.
fn semantic_check_edge_node_existence(graph: &FlowGraph, diagnostics: &mut Vec<Diagnostic>) {
    for (edge_id, edge) in &graph.edges {
        if !graph.nodes.contains_key(&edge.source_node) {
            diagnostics.push(diag(
                DiagnosticSeverity::Error,
                "semantic-edge-source-missing",
                format!(
                    "edge '{}' references non-existent source node '{}'",
                    edge_id, edge.source_node
                ),
                Some(edge.source_node.clone()),
                Some(edge_id.clone()),
            ));
        }
        if !graph.nodes.contains_key(&edge.target_node) {
            diagnostics.push(diag(
                DiagnosticSeverity::Error,
                "semantic-edge-target-missing",
                format!(
                    "edge '{}' references non-existent target node '{}'",
                    edge_id, edge.target_node
                ),
                Some(edge.target_node.clone()),
                Some(edge_id.clone()),
            ));
        }
    }
}

/// Check 2: Every node whose `kind` field is non-empty must have at least one
/// connected edge.  Nodes with an empty `kind` are treated as placeholders
/// and are exempt from this requirement.
fn semantic_check_nonempty_kind_orphans(graph: &FlowGraph, diagnostics: &mut Vec<Diagnostic>) {
    let mut connected: HashSet<NodeId> = HashSet::new();
    for edge in graph.edges.values() {
        if graph.nodes.contains_key(&edge.source_node) {
            connected.insert(edge.source_node.clone());
        }
        if graph.nodes.contains_key(&edge.target_node) {
            connected.insert(edge.target_node.clone());
        }
    }

    for (node_id, node) in &graph.nodes {
        if node.kind.is_empty() {
            continue;
        }
        if !connected.contains(node_id) {
            diagnostics.push(diag(
                DiagnosticSeverity::Warning,
                "semantic-orphan-node",
                format!(
                    "node '{}' with kind '{}' has no connected edges",
                    node_id, node.kind
                ),
                Some(node_id.clone()),
                None,
            ));
        }
    }
}

/// Check 3: Every node that claims a parent group references a group that
/// actually exists in the document.
fn semantic_check_group_member_validity(graph: &FlowGraph, diagnostics: &mut Vec<Diagnostic>) {
    for (node_id, node) in &graph.nodes {
        if let Some(ref group_id) = node.parent
            && !graph.groups.contains_key(group_id)
        {
            diagnostics.push(diag(
                DiagnosticSeverity::Error,
                "semantic-node-parent-group-missing",
                format!(
                    "node '{}' references non-existent parent group '{}'",
                    node_id, group_id
                ),
                Some(node_id.clone()),
                None,
            ));
        }
    }
}

/// Check 4: No two edges connect the same (source_node, source_port) to the
/// same (target_node, target_port).
fn semantic_check_duplicate_port_connections(
    graph: &FlowGraph,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let mut seen: HashMap<(NodeId, PortId, NodeId, PortId), EdgeId> = HashMap::new();
    for (edge_id, edge) in &graph.edges {
        let key = (
            edge.source_node.clone(),
            edge.source_port.clone(),
            edge.target_node.clone(),
            edge.target_port.clone(),
        );
        match seen.entry(key) {
            std::collections::hash_map::Entry::Occupied(entry) => {
                let first_id = entry.get();
                diagnostics.push(diag(
                    DiagnosticSeverity::Error,
                    "semantic-duplicate-edge",
                    format!(
                        "edge '{}' duplicates edge '{}' (same source port -> target port connection)",
                        edge_id, first_id
                    ),
                    None,
                    Some(edge_id.clone()),
                ));
            }
            std::collections::hash_map::Entry::Vacant(entry) => {
                entry.insert(edge_id.clone());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::doc::{
        Cardinality, EdgeStyle, FlowEdgeRecord, FlowGroupRecord, FlowNodeRecord, FlowPortRecord,
        GroupKind, NodeFlags, NodeUiState, PortRole, PortSide,
    };
    use crate::ids::{EdgeId, GroupId, NodeId, PortId};
    use smol_str::SmolStr;

    fn nid(s: &str) -> NodeId {
        SmolStr::from(s)
    }

    fn eid(s: &str) -> EdgeId {
        SmolStr::from(s)
    }

    fn pid(s: &str) -> PortId {
        SmolStr::from(s)
    }

    fn gid(s: &str) -> GroupId {
        SmolStr::from(s)
    }

    fn make_node_with_ports(id: &str, port_ids: &[&str]) -> FlowNodeRecord {
        FlowNodeRecord {
            id: nid(id),
            kind: SmolStr::from("test"),
            title: SmolStr::from(id),
            position: [0.0, 0.0],
            size: [100.0, 50.0],
            z_index: 0,
            parent: None,
            ports: port_ids
                .iter()
                .enumerate()
                .map(|(i, p)| FlowPortRecord {
                    id: pid(p),
                    side: if i % 2 == 0 {
                        PortSide::Left
                    } else {
                        PortSide::Right
                    },
                    role: if i % 2 == 0 {
                        PortRole::Target
                    } else {
                        PortRole::Source
                    },
                    label: SmolStr::from(*p),
                    order: if let Ok(v) = u16::try_from(i) {
                        v
                    } else {
                        u16::MAX
                    },
                    cardinality: Cardinality::One,
                    data_type: None,
                })
                .collect(),
            flags: NodeFlags::default(),
            data: serde_json::Value::Null,
            ui: NodeUiState::default(),
        }
    }

    fn make_edge_with_ports(
        id: &str,
        src: &str,
        src_port: &str,
        tgt: &str,
        tgt_port: &str,
    ) -> FlowEdgeRecord {
        FlowEdgeRecord {
            id: eid(id),
            source_node: nid(src),
            source_port: pid(src_port),
            target_node: nid(tgt),
            target_port: pid(tgt_port),
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

    fn valid_document() -> FlowDocument {
        let mut doc = FlowDocument::default();
        doc.graph
            .nodes
            .insert(nid("n1"), make_node_with_ports("n1", &["p-out"]));
        doc.graph
            .nodes
            .insert(nid("n2"), make_node_with_ports("n2", &["p-in"]));
        doc.graph.edges.insert(
            eid("e1"),
            make_edge_with_ports("e1", "n1", "p-out", "n2", "p-in"),
        );
        doc
    }

    // ---- valid document produces no diagnostics ----

    #[test]
    fn valid_document_no_diagnostics() {
        let doc = valid_document();
        let validator = StructuralValidator;
        let diags = validator.validate(&doc);
        assert!(diags.is_empty());
    }

    #[test]
    fn empty_document_no_diagnostics() {
        let doc = FlowDocument::default();
        let validator = StructuralValidator;
        let diags = validator.validate(&doc);
        assert!(diags.is_empty());
    }

    // ---- entry node checks ----

    #[test]
    fn entry_node_missing_produces_error() {
        let mut doc = valid_document();
        doc.graph.entry_node = Some(nid("nonexistent"));
        let validator = StructuralValidator;
        let diags = validator.validate(&doc);
        assert!(!diags.is_empty());
        let has_error = diags
            .iter()
            .any(|d| d.code.as_str() == "entry-node-missing");
        assert!(has_error);
    }

    #[test]
    fn entry_node_valid_no_diagnostic() {
        let mut doc = valid_document();
        doc.graph.entry_node = Some(nid("n1"));
        let validator = StructuralValidator;
        let diags = validator.validate(&doc);
        let has_entry_error = diags
            .iter()
            .any(|d| d.code.as_str() == "entry-node-missing");
        assert!(!has_entry_error);
    }

    #[test]
    fn entry_node_none_no_diagnostic() {
        let doc = FlowDocument::default();
        let validator = StructuralValidator;
        let diags = validator.validate(&doc);
        let has_entry_error = diags
            .iter()
            .any(|d| d.code.as_str() == "entry-node-missing");
        assert!(!has_entry_error);
    }

    // ---- edge endpoint checks ----

    #[test]
    fn edge_missing_source_node_produces_error() {
        let mut doc = FlowDocument::default();
        doc.graph
            .nodes
            .insert(nid("n2"), make_node_with_ports("n2", &["p-in"]));
        doc.graph.edges.insert(
            eid("e1"),
            make_edge_with_ports("e1", "ghost", "p-out", "n2", "p-in"),
        );
        let validator = StructuralValidator;
        let diags = validator.validate(&doc);
        let has_error = diags
            .iter()
            .any(|d| d.code.as_str() == "edge-source-missing");
        assert!(has_error);
    }

    #[test]
    fn edge_missing_target_node_produces_error() {
        let mut doc = FlowDocument::default();
        doc.graph
            .nodes
            .insert(nid("n1"), make_node_with_ports("n1", &["p-out"]));
        doc.graph.edges.insert(
            eid("e1"),
            make_edge_with_ports("e1", "n1", "p-out", "ghost", "p-in"),
        );
        let validator = StructuralValidator;
        let diags = validator.validate(&doc);
        let has_error = diags
            .iter()
            .any(|d| d.code.as_str() == "edge-target-missing");
        assert!(has_error);
    }

    #[test]
    fn edge_both_endpoints_missing_produces_two_errors() {
        let mut doc = FlowDocument::default();
        doc.graph.edges.insert(
            eid("e1"),
            make_edge_with_ports("e1", "ghost1", "p-out", "ghost2", "p-in"),
        );
        let validator = StructuralValidator;
        let diags = validator.validate(&doc);
        let source_missing = diags
            .iter()
            .any(|d| d.code.as_str() == "edge-source-missing");
        let target_missing = diags
            .iter()
            .any(|d| d.code.as_str() == "edge-target-missing");
        assert!(source_missing);
        assert!(target_missing);
    }

    // ---- edge port checks ----

    #[test]
    fn edge_missing_source_port_produces_error() {
        let mut doc = FlowDocument::default();
        doc.graph
            .nodes
            .insert(nid("n1"), make_node_with_ports("n1", &["p-other"]));
        doc.graph
            .nodes
            .insert(nid("n2"), make_node_with_ports("n2", &["p-in"]));
        doc.graph.edges.insert(
            eid("e1"),
            make_edge_with_ports("e1", "n1", "p-out", "n2", "p-in"),
        );
        let validator = StructuralValidator;
        let diags = validator.validate(&doc);
        let has_error = diags
            .iter()
            .any(|d| d.code.as_str() == "edge-source-port-missing");
        assert!(has_error);
    }

    #[test]
    fn edge_missing_target_port_produces_error() {
        let mut doc = FlowDocument::default();
        doc.graph
            .nodes
            .insert(nid("n1"), make_node_with_ports("n1", &["p-out"]));
        doc.graph
            .nodes
            .insert(nid("n2"), make_node_with_ports("n2", &["p-other"]));
        doc.graph.edges.insert(
            eid("e1"),
            make_edge_with_ports("e1", "n1", "p-out", "n2", "p-in"),
        );
        let validator = StructuralValidator;
        let diags = validator.validate(&doc);
        let has_error = diags
            .iter()
            .any(|d| d.code.as_str() == "edge-target-port-missing");
        assert!(has_error);
    }

    #[test]
    fn edge_valid_ports_no_port_errors() {
        let doc = valid_document();
        let validator = StructuralValidator;
        let diags = validator.validate(&doc);
        let port_errors = diags.iter().any(|d| {
            d.code.as_str() == "edge-source-port-missing"
                || d.code.as_str() == "edge-target-port-missing"
        });
        assert!(!port_errors);
    }

    // ---- self-loop checks ----

    #[test]
    fn self_loop_same_port_produces_warning() {
        let mut doc = FlowDocument::default();
        doc.graph
            .nodes
            .insert(nid("n1"), make_node_with_ports("n1", &["p-io"]));
        doc.graph.edges.insert(
            eid("e1"),
            make_edge_with_ports("e1", "n1", "p-io", "n1", "p-io"),
        );
        let validator = StructuralValidator;
        let diags = validator.validate(&doc);
        let has_warning = diags
            .iter()
            .any(|d| d.code.as_str() == "self-loop-same-port");
        assert!(has_warning);
    }

    #[test]
    fn self_loop_different_ports_no_self_loop_warning() {
        let mut doc = FlowDocument::default();
        doc.graph
            .nodes
            .insert(nid("n1"), make_node_with_ports("n1", &["p-out", "p-in"]));
        doc.graph.edges.insert(
            eid("e1"),
            make_edge_with_ports("e1", "n1", "p-out", "n1", "p-in"),
        );
        let validator = StructuralValidator;
        let diags = validator.validate(&doc);
        let has_warning = diags
            .iter()
            .any(|d| d.code.as_str() == "self-loop-same-port");
        assert!(!has_warning);
    }

    // ---- group member checks ----

    #[test]
    fn node_parent_group_missing_produces_error() {
        let mut doc = FlowDocument::default();
        let mut node = make_node_with_ports("n1", &[]);
        node.parent = Some(gid("ghost-group"));
        doc.graph.nodes.insert(nid("n1"), node);
        let validator = StructuralValidator;
        let diags = validator.validate(&doc);
        let has_error = diags
            .iter()
            .any(|d| d.code.as_str() == "node-parent-group-missing");
        assert!(has_error);
    }

    #[test]
    fn node_parent_group_valid_no_error() {
        let mut doc = FlowDocument::default();
        let mut node = make_node_with_ports("n1", &[]);
        node.parent = Some(gid("g1"));
        doc.graph.nodes.insert(nid("n1"), node);
        doc.graph.groups.insert(gid("g1"), make_group("g1"));
        let validator = StructuralValidator;
        let diags = validator.validate(&doc);
        let has_error = diags
            .iter()
            .any(|d| d.code.as_str() == "node-parent-group-missing");
        assert!(!has_error);
    }

    // ---- multiple errors ----

    #[test]
    fn multiple_issues_produce_multiple_diagnostics() {
        let mut doc = FlowDocument::default();
        // entry node missing
        doc.graph.entry_node = Some(nid("entry-ghost"));
        // orphan edge
        doc.graph.edges.insert(
            eid("e1"),
            make_edge_with_ports("e1", "ghost1", "p", "ghost2", "p"),
        );
        // node with missing parent group
        let mut node = make_node_with_ports("n1", &[]);
        node.parent = Some(gid("g-ghost"));
        doc.graph.nodes.insert(nid("n1"), node);

        let validator = StructuralValidator;
        let diags = validator.validate(&doc);
        // Should have: entry-node-missing, edge-source-missing, edge-target-missing,
        // node-parent-group-missing
        assert!(diags.len() >= 4);
    }

    // ---- diagnostic fields ----

    #[test]
    fn diagnostic_has_correct_severity() {
        let mut doc = FlowDocument::default();
        doc.graph.entry_node = Some(nid("ghost"));
        let validator = StructuralValidator;
        let diags = validator.validate(&doc);
        let entry_diag = diags
            .iter()
            .find(|d| d.code.as_str() == "entry-node-missing");
        assert!(entry_diag.is_some_and(|d| d.severity == DiagnosticSeverity::Error));
    }

    #[test]
    fn diagnostic_references_correct_node() {
        let mut doc = FlowDocument::default();
        doc.graph.entry_node = Some(nid("ghost"));
        let validator = StructuralValidator;
        let diags = validator.validate(&doc);
        let entry_diag = diags
            .iter()
            .find(|d| d.code.as_str() == "entry-node-missing");
        assert!(entry_diag.is_some_and(|d| d.node.as_ref().is_some_and(|n| n == &nid("ghost"))));
    }

    #[test]
    fn edge_diagnostic_references_edge_id() {
        let mut doc = FlowDocument::default();
        doc.graph
            .nodes
            .insert(nid("n2"), make_node_with_ports("n2", &["p-in"]));
        doc.graph.edges.insert(
            eid("e1"),
            make_edge_with_ports("e1", "ghost", "p-out", "n2", "p-in"),
        );
        let validator = StructuralValidator;
        let diags = validator.validate(&doc);
        let src_diag = diags
            .iter()
            .find(|d| d.code.as_str() == "edge-source-missing");
        assert!(src_diag.is_some_and(|d| d.edge.as_ref().is_some_and(|e| e == &eid("e1"))));
    }

    // =========================================================================
    // NEW: Additional validation edge-case tests
    // =========================================================================

    // ---- Multiple self-loops produce multiple warnings ----

    #[test]
    fn multiple_self_loops_produce_multiple_warnings() {
        let mut doc = FlowDocument::default();
        doc.graph
            .nodes
            .insert(nid("n1"), make_node_with_ports("n1", &["p-io"]));
        doc.graph.edges.insert(
            eid("e1"),
            make_edge_with_ports("e1", "n1", "p-io", "n1", "p-io"),
        );
        doc.graph.edges.insert(
            eid("e2"),
            make_edge_with_ports("e2", "n1", "p-io", "n1", "p-io"),
        );
        let validator = StructuralValidator;
        let diags = validator.validate(&doc);
        let self_loop_count = diags
            .iter()
            .filter(|d| d.code.as_str() == "self-loop-same-port")
            .count();
        assert_eq!(self_loop_count, 2);
    }

    // ---- Multiple missing entry nodes produce only one error ----

    #[test]
    fn single_entry_node_missing_error() {
        let mut doc = FlowDocument::default();
        doc.graph.entry_node = Some(nid("ghost"));
        let validator = StructuralValidator;
        let diags = validator.validate(&doc);
        let entry_errors = diags
            .iter()
            .filter(|d| d.code.as_str() == "entry-node-missing")
            .count();
        assert_eq!(entry_errors, 1);
    }

    // ---- Multiple edges with missing endpoints ----

    #[test]
    fn multiple_edges_with_missing_endpoints() {
        let mut doc = FlowDocument::default();
        doc.graph.edges.insert(
            eid("e1"),
            make_edge_with_ports("e1", "ghost1", "p-out", "ghost2", "p-in"),
        );
        doc.graph.edges.insert(
            eid("e2"),
            make_edge_with_ports("e2", "ghost3", "p-out", "ghost4", "p-in"),
        );
        let validator = StructuralValidator;
        let diags = validator.validate(&doc);
        let source_missing = diags
            .iter()
            .filter(|d| d.code.as_str() == "edge-source-missing")
            .count();
        let target_missing = diags
            .iter()
            .filter(|d| d.code.as_str() == "edge-target-missing")
            .count();
        assert_eq!(source_missing, 2);
        assert_eq!(target_missing, 2);
    }

    // ---- Nodes with and without parents in same document ----

    #[test]
    fn mixed_parent_refs() {
        let mut doc = FlowDocument::default();
        doc.graph.groups.insert(gid("g1"), make_group("g1"));
        // Node n1 has valid parent
        let mut n1 = make_node_with_ports("n1", &[]);
        n1.parent = Some(gid("g1"));
        doc.graph.nodes.insert(nid("n1"), n1);
        // Node n2 has invalid parent
        let mut n2 = make_node_with_ports("n2", &[]);
        n2.parent = Some(gid("ghost"));
        doc.graph.nodes.insert(nid("n2"), n2);
        // Node n3 has no parent
        doc.graph.nodes.insert(nid("n3"), make_node_with_ports("n3", &[]));

        let validator = StructuralValidator;
        let diags = validator.validate(&doc);
        let parent_errors: Vec<_> = diags
            .iter()
            .filter(|d| d.code.as_str() == "node-parent-group-missing")
            .collect();
        assert_eq!(parent_errors.len(), 1);
        assert!(parent_errors
            .first()
            .is_some_and(|d| d.node.as_ref().is_some_and(|n| n == &nid("n2"))));
    }

    // ---- Document with many nodes and edges passes validation ----

    #[test]
    fn large_valid_document() {
        let mut doc = FlowDocument::default();
        // Create 10 nodes, each with an in and out port
        for i in 0..10 {
            let port_name_out = format!("p-out-{i}");
            let port_name_in = format!("p-in-{i}");
            let ports = vec![
                FlowPortRecord {
                    id: pid(&port_name_out),
                    side: PortSide::Right,
                    role: PortRole::Source,
                    label: SmolStr::from(&port_name_out),
                    order: 0,
                    cardinality: Cardinality::One,
                    data_type: None,
                },
                FlowPortRecord {
                    id: pid(&port_name_in),
                    side: PortSide::Left,
                    role: PortRole::Target,
                    label: SmolStr::from(&port_name_in),
                    order: 1,
                    cardinality: Cardinality::One,
                    data_type: None,
                },
            ];
            let node = FlowNodeRecord {
                id: nid(&format!("n{i}")),
                kind: SmolStr::from("test"),
                title: SmolStr::from(format!("n{i}")),
                position: [0.0, 0.0],
                size: [100.0, 50.0],
                z_index: 0,
                parent: None,
                ports,
                flags: NodeFlags::default(),
                data: serde_json::Value::Null,
                ui: NodeUiState::default(),
            };
            doc.graph.nodes.insert(nid(&format!("n{i}")), node);
        }
        // Chain edges: n0 -> n1 -> n2 -> ... -> n9
        for i in 0usize..9 {
            let next = i.saturating_add(1);
            let edge = FlowEdgeRecord {
                id: eid(&format!("e{i}")),
                source_node: nid(&format!("n{i}")),
                source_port: pid(&format!("p-out-{i}")),
                target_node: nid(&format!("n{next}")),
                target_port: pid(&format!("p-in-{next}")),
                label: None,
                style: EdgeStyle::default(),
                data: serde_json::Value::Null,
                ui: EdgeUiState::default(),
            };
            doc.graph
                .edges
                .insert(eid(&format!("e{i}")), edge);
        }
        doc.graph.entry_node = Some(nid("n0"));

        let validator = StructuralValidator;
        let diags = validator.validate(&doc);
        assert!(diags.is_empty());
    }

    // ---- Self-loop with different ports is not flagged ----

    #[test]
    fn self_loop_different_ports_no_warning() {
        let mut doc = FlowDocument::default();
        doc.graph
            .nodes
            .insert(nid("n1"), make_node_with_ports("n1", &["p-out", "p-in"]));
        doc.graph.edges.insert(
            eid("e1"),
            make_edge_with_ports("e1", "n1", "p-out", "n1", "p-in"),
        );
        let validator = StructuralValidator;
        let diags = validator.validate(&doc);
        let has_self_loop_warning = diags
            .iter()
            .any(|d| d.code.as_str() == "self-loop-same-port");
        assert!(!has_self_loop_warning);
    }

    // ---- Validation diagnostic severity is correct for warnings ----

    #[test]
    fn self_loop_warning_has_correct_severity() {
        let mut doc = FlowDocument::default();
        doc.graph
            .nodes
            .insert(nid("n1"), make_node_with_ports("n1", &["p-io"]));
        doc.graph.edges.insert(
            eid("e1"),
            make_edge_with_ports("e1", "n1", "p-io", "n1", "p-io"),
        );
        let validator = StructuralValidator;
        let diags = validator.validate(&doc);
        let self_loop_diag = diags
            .iter()
            .find(|d| d.code.as_str() == "self-loop-same-port");
        assert!(
            self_loop_diag.is_some_and(|d| d.severity == DiagnosticSeverity::Warning)
        );
    }

    // ---- FlowValidator trait object ----

    #[test]
    fn validator_trait_object_works() {
        let validator: Box<dyn FlowValidator> = Box::new(StructuralValidator);
        let doc = FlowDocument::default();
        let diags = validator.validate(&doc);
        assert!(diags.is_empty());
    }
}

// ---------------------------------------------------------------------------
// SemanticValidator tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod semantic_tests {
    use super::*;
    use crate::doc::{
        Cardinality, EdgeStyle, FlowEdgeRecord, FlowGroupRecord, FlowNodeRecord, FlowPortRecord,
        GroupKind, NodeFlags, NodeUiState, PortRole, PortSide,
    };
    use crate::ids::{EdgeId, GroupId, NodeId, PortId};
    use smol_str::SmolStr;

    fn nid(s: &str) -> NodeId {
        SmolStr::from(s)
    }

    fn eid(s: &str) -> EdgeId {
        SmolStr::from(s)
    }

    fn pid(s: &str) -> PortId {
        SmolStr::from(s)
    }

    fn gid(s: &str) -> GroupId {
        SmolStr::from(s)
    }

    fn make_port_record(id: &str, role: PortRole) -> FlowPortRecord {
        FlowPortRecord {
            id: pid(id),
            side: match role {
                PortRole::Source | PortRole::Bidirectional => PortSide::Right,
                PortRole::Target => PortSide::Left,
            },
            role,
            label: SmolStr::from(id),
            order: 0,
            cardinality: Cardinality::One,
            data_type: None,
        }
    }

    fn make_node(id: &str, kind: &str) -> FlowNodeRecord {
        FlowNodeRecord {
            id: nid(id),
            kind: SmolStr::from(kind),
            title: SmolStr::from(id),
            position: [0.0, 0.0],
            size: [100.0, 50.0],
            z_index: 0,
            parent: None,
            ports: vec![
                make_port_record("out", PortRole::Source),
                make_port_record("in", PortRole::Target),
            ],
            flags: NodeFlags::default(),
            data: serde_json::Value::Null,
            ui: NodeUiState::default(),
        }
    }

    fn make_node_empty_kind(id: &str) -> FlowNodeRecord {
        FlowNodeRecord {
            id: nid(id),
            kind: SmolStr::new(""),
            title: SmolStr::from(id),
            position: [0.0, 0.0],
            size: [100.0, 50.0],
            z_index: 0,
            parent: None,
            ports: vec![],
            flags: NodeFlags::default(),
            data: serde_json::Value::Null,
            ui: NodeUiState::default(),
        }
    }

    fn make_edge(id: &str, src: &str, src_port: &str, tgt: &str, tgt_port: &str) -> FlowEdgeRecord {
        FlowEdgeRecord {
            id: eid(id),
            source_node: nid(src),
            source_port: pid(src_port),
            target_node: nid(tgt),
            target_port: pid(tgt_port),
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

    // =========================================================================
    // SemanticValidator integration tests
    // =========================================================================

    #[test]
    fn semantic_valid_document_no_diagnostics() {
        let mut doc = FlowDocument::default();
        doc.graph.nodes.insert(nid("n1"), make_node("n1", "source"));
        doc.graph.nodes.insert(nid("n2"), make_node("n2", "sink"));
        doc.graph.edges.insert(
            eid("e1"),
            make_edge("e1", "n1", "out", "n2", "in"),
        );
        let validator = SemanticValidator;
        let diags = validator.validate(&doc);
        assert!(diags.is_empty(), "expected no diagnostics, got: {diags:?}");
    }

    #[test]
    fn semantic_empty_document_no_diagnostics() {
        let doc = FlowDocument::default();
        let validator = SemanticValidator;
        let diags = validator.validate(&doc);
        assert!(diags.is_empty());
    }

    #[test]
    fn semantic_validator_trait_object_works() {
        let validator: Box<dyn FlowValidator> = Box::new(SemanticValidator);
        let doc = FlowDocument::default();
        let diags = validator.validate(&doc);
        assert!(diags.is_empty());
    }

    // =========================================================================
    // Check 1: edge node existence
    // =========================================================================

    #[test]
    fn edge_source_node_missing_produces_error() {
        let mut doc = FlowDocument::default();
        doc.graph.nodes.insert(nid("n2"), make_node("n2", "sink"));
        doc.graph.edges.insert(
            eid("e1"),
            make_edge("e1", "ghost", "out", "n2", "in"),
        );
        let validator = SemanticValidator;
        let diags = validator.validate(&doc);
        assert!(diags
            .iter()
            .any(|d| d.code.as_str() == "semantic-edge-source-missing"));
    }

    #[test]
    fn edge_target_node_missing_produces_error() {
        let mut doc = FlowDocument::default();
        doc.graph.nodes.insert(nid("n1"), make_node("n1", "source"));
        doc.graph.edges.insert(
            eid("e1"),
            make_edge("e1", "n1", "out", "ghost", "in"),
        );
        let validator = SemanticValidator;
        let diags = validator.validate(&doc);
        assert!(diags
            .iter()
            .any(|d| d.code.as_str() == "semantic-edge-target-missing"));
    }

    #[test]
    fn edge_both_endpoints_missing_produces_two_errors() {
        let mut doc = FlowDocument::default();
        doc.graph.edges.insert(
            eid("e1"),
            make_edge("e1", "ghost1", "out", "ghost2", "in"),
        );
        let validator = SemanticValidator;
        let diags = validator.validate(&doc);
        assert!(diags
            .iter()
            .any(|d| d.code.as_str() == "semantic-edge-source-missing"));
        assert!(diags
            .iter()
            .any(|d| d.code.as_str() == "semantic-edge-target-missing"));
        assert_eq!(diags.len(), 2);
    }

    // =========================================================================
    // Check 2: orphan nodes with non-empty kind
    // =========================================================================

    #[test]
    fn connected_node_not_flagged_as_orphan() {
        let mut doc = FlowDocument::default();
        doc.graph.nodes.insert(nid("n1"), make_node("n1", "transform"));
        doc.graph.nodes.insert(nid("n2"), make_node("n2", "output"));
        doc.graph.edges.insert(
            eid("e1"),
            make_edge("e1", "n1", "out", "n2", "in"),
        );
        let validator = SemanticValidator;
        let diags = validator.validate(&doc);
        assert!(!diags
            .iter()
            .any(|d| d.code.as_str() == "semantic-orphan-node"));
    }

    #[test]
    fn disconnected_node_with_kind_flagged_as_orphan() {
        let mut doc = FlowDocument::default();
        doc.graph.nodes.insert(nid("n1"), make_node("n1", "transform"));
        let validator = SemanticValidator;
        let diags = validator.validate(&doc);
        assert!(diags
            .iter()
            .any(|d| d.code.as_str() == "semantic-orphan-node"));
        let orphan = diags
            .iter()
            .find(|d| d.code.as_str() == "semantic-orphan-node");
        assert!(orphan.is_some_and(|d| d.node.as_ref() == Some(&nid("n1"))));
    }

    #[test]
    fn disconnected_node_with_empty_kind_not_flagged() {
        let mut doc = FlowDocument::default();
        doc.graph
            .nodes
            .insert(nid("n1"), make_node_empty_kind("n1"));
        let validator = SemanticValidator;
        let diags = validator.validate(&doc);
        assert!(!diags
            .iter()
            .any(|d| d.code.as_str() == "semantic-orphan-node"));
    }

    #[test]
    fn multiple_orphan_nodes_all_flagged() {
        let mut doc = FlowDocument::default();
        doc.graph
            .nodes
            .insert(nid("n1"), make_node("n1", "alpha"));
        doc.graph
            .nodes
            .insert(nid("n2"), make_node("n2", "beta"));
        doc.graph
            .nodes
            .insert(nid("n3"), make_node("n3", "gamma"));
        let validator = SemanticValidator;
        let diags = validator.validate(&doc);
        let orphan_count = diags
            .iter()
            .filter(|d| d.code.as_str() == "semantic-orphan-node")
            .count();
        assert_eq!(orphan_count, 3);
    }

    // =========================================================================
    // Check 3: group member node validity
    // =========================================================================

    #[test]
    fn node_with_valid_parent_group_no_diagnostic() {
        let mut doc = FlowDocument::default();
        doc.graph.groups.insert(gid("g1"), make_group("g1"));
        let mut node = make_node("n1", "test");
        node.parent = Some(gid("g1"));
        doc.graph.nodes.insert(nid("n1"), node);
        doc.graph
            .nodes
            .insert(nid("n2"), make_node("n2", "test"));
        doc.graph.edges.insert(
            eid("e1"),
            make_edge("e1", "n1", "out", "n2", "in"),
        );
        let validator = SemanticValidator;
        let diags = validator.validate(&doc);
        assert!(!diags
            .iter()
            .any(|d| d.code.as_str() == "semantic-node-parent-group-missing"));
    }

    #[test]
    fn node_with_missing_parent_group_produces_error() {
        let mut doc = FlowDocument::default();
        let mut node = make_node("n1", "test");
        node.parent = Some(gid("ghost-group"));
        doc.graph.nodes.insert(nid("n1"), node);
        let validator = SemanticValidator;
        let diags = validator.validate(&doc);
        assert!(diags
            .iter()
            .any(|d| d.code.as_str() == "semantic-node-parent-group-missing"));
    }

    #[test]
    fn node_with_no_parent_no_diagnostic() {
        let mut doc = FlowDocument::default();
        doc.graph
            .nodes
            .insert(nid("n1"), make_node("n1", "test"));
        let validator = SemanticValidator;
        let diags = validator.validate(&doc);
        assert!(!diags
            .iter()
            .any(|d| d.code.as_str() == "semantic-node-parent-group-missing"));
    }

    // =========================================================================
    // Check 4: duplicate port connections
    // =========================================================================

    #[test]
    fn unique_edges_no_duplicate_diagnostic() {
        let mut doc = FlowDocument::default();
        doc.graph.nodes.insert(nid("n1"), make_node("n1", "src"));
        doc.graph.nodes.insert(nid("n2"), make_node("n2", "tgt"));
        doc.graph.nodes.insert(nid("n3"), make_node("n3", "tgt2"));
        doc.graph.edges.insert(
            eid("e1"),
            make_edge("e1", "n1", "out", "n2", "in"),
        );
        doc.graph.edges.insert(
            eid("e2"),
            make_edge("e2", "n1", "out", "n3", "in"),
        );
        let validator = SemanticValidator;
        let diags = validator.validate(&doc);
        assert!(!diags
            .iter()
            .any(|d| d.code.as_str() == "semantic-duplicate-edge"));
    }

    #[test]
    fn exact_duplicate_edge_produces_error() {
        let mut doc = FlowDocument::default();
        doc.graph.nodes.insert(nid("n1"), make_node("n1", "src"));
        doc.graph.nodes.insert(nid("n2"), make_node("n2", "tgt"));
        doc.graph.edges.insert(
            eid("e1"),
            make_edge("e1", "n1", "out", "n2", "in"),
        );
        doc.graph.edges.insert(
            eid("e2"),
            make_edge("e2", "n1", "out", "n2", "in"),
        );
        let validator = SemanticValidator;
        let diags = validator.validate(&doc);
        assert!(diags
            .iter()
            .any(|d| d.code.as_str() == "semantic-duplicate-edge"));
    }

    #[test]
    fn same_nodes_different_ports_no_duplicate() {
        let mut doc = FlowDocument::default();
        let mut n1 = make_node("n1", "src");
        n1.ports = vec![
            make_port_record("out-a", PortRole::Source),
            make_port_record("out-b", PortRole::Source),
        ];
        let mut n2 = make_node("n2", "tgt");
        n2.ports = vec![
            make_port_record("in-a", PortRole::Target),
            make_port_record("in-b", PortRole::Target),
        ];
        doc.graph.nodes.insert(nid("n1"), n1);
        doc.graph.nodes.insert(nid("n2"), n2);
        doc.graph.edges.insert(
            eid("e1"),
            make_edge("e1", "n1", "out-a", "n2", "in-a"),
        );
        doc.graph.edges.insert(
            eid("e2"),
            make_edge("e2", "n1", "out-b", "n2", "in-b"),
        );
        let validator = SemanticValidator;
        let diags = validator.validate(&doc);
        assert!(!diags
            .iter()
            .any(|d| d.code.as_str() == "semantic-duplicate-edge"));
    }

    #[test]
    fn three_duplicate_edges_produce_two_errors() {
        let mut doc = FlowDocument::default();
        doc.graph.nodes.insert(nid("n1"), make_node("n1", "src"));
        doc.graph.nodes.insert(nid("n2"), make_node("n2", "tgt"));
        doc.graph.edges.insert(
            eid("e1"),
            make_edge("e1", "n1", "out", "n2", "in"),
        );
        doc.graph.edges.insert(
            eid("e2"),
            make_edge("e2", "n1", "out", "n2", "in"),
        );
        doc.graph.edges.insert(
            eid("e3"),
            make_edge("e3", "n1", "out", "n2", "in"),
        );
        let validator = SemanticValidator;
        let diags = validator.validate(&doc);
        let dup_count = diags
            .iter()
            .filter(|d| d.code.as_str() == "semantic-duplicate-edge")
            .count();
        // e2 duplicates e1, e3 duplicates e1 => 2 errors
        assert_eq!(dup_count, 2);
    }

    // =========================================================================
    // Integration: all four checks together
    // =========================================================================

    #[test]
    fn all_semantic_checks_on_valid_document() {
        let mut doc = FlowDocument::default();
        doc.graph.groups.insert(gid("g1"), make_group("g1"));
        let mut n1 = make_node("n1", "source");
        n1.parent = Some(gid("g1"));
        let mut n2 = make_node("n2", "sink");
        n2.parent = Some(gid("g1"));
        doc.graph.nodes.insert(nid("n1"), n1);
        doc.graph.nodes.insert(nid("n2"), n2);
        doc.graph.edges.insert(
            eid("e1"),
            make_edge("e1", "n1", "out", "n2", "in"),
        );
        let validator = SemanticValidator;
        let diags = validator.validate(&doc);
        assert!(
            diags.is_empty(),
            "valid document should produce no diagnostics, got: {diags:?}"
        );
    }

    #[test]
    fn all_semantic_checks_accumulate_from_multiple_issues() {
        let mut doc = FlowDocument::default();
        // No groups -- but node references a ghost group
        let mut n1 = make_node("n1", "orphan-with-bad-group");
        n1.parent = Some(gid("ghost"));
        doc.graph.nodes.insert(nid("n1"), n1);
        // Edge references nonexistent nodes
        doc.graph.edges.insert(
            eid("e1"),
            make_edge("e1", "missing-src", "out", "missing-tgt", "in"),
        );

        let validator = SemanticValidator;
        let diags = validator.validate(&doc);

        assert!(diags
            .iter()
            .any(|d| d.code.as_str() == "semantic-edge-source-missing"));
        assert!(diags
            .iter()
            .any(|d| d.code.as_str() == "semantic-edge-target-missing"));
        assert!(diags
            .iter()
            .any(|d| d.code.as_str() == "semantic-orphan-node"));
        assert!(diags
            .iter()
            .any(|d| d.code.as_str() == "semantic-node-parent-group-missing"));
        assert!(diags.len() >= 4);
    }
}
