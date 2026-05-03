use crate::doc::*;
use crate::ids::*;
use crate::patch::{Diagnostic, DiagnosticSeverity};
use smol_str::SmolStr;

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
            let port_exists = source_node
                .ports
                .iter()
                .any(|p| p.id == edge.source_port);
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
            let port_exists = target_node
                .ports
                .iter()
                .any(|p| p.id == edge.target_port);
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
                    side: if i % 2 == 0 { PortSide::Left } else { PortSide::Right },
                    role: if i % 2 == 0 { PortRole::Target } else { PortRole::Source },
                    label: SmolStr::from(*p),
                    order: i as u16,
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
        doc.graph.nodes.insert(
            nid("n1"),
            make_node_with_ports("n1", &["p-out"]),
        );
        doc.graph.nodes.insert(
            nid("n2"),
            make_node_with_ports("n2", &["p-in"]),
        );
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
        doc.graph.nodes.insert(nid("n2"), make_node_with_ports("n2", &["p-in"]));
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
        doc.graph.nodes.insert(nid("n1"), make_node_with_ports("n1", &["p-out"]));
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
        doc.graph.nodes.insert(nid("n1"), make_node_with_ports("n1", &["p-other"]));
        doc.graph.nodes.insert(nid("n2"), make_node_with_ports("n2", &["p-in"]));
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
        doc.graph.nodes.insert(nid("n1"), make_node_with_ports("n1", &["p-out"]));
        doc.graph.nodes.insert(nid("n2"), make_node_with_ports("n2", &["p-other"]));
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
        doc.graph.nodes.insert(nid("n1"), make_node_with_ports("n1", &["p-io"]));
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
        doc.graph.nodes.insert(nid("n1"), make_node_with_ports("n1", &["p-out", "p-in"]));
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
        let entry_diag = diags.iter().find(|d| d.code.as_str() == "entry-node-missing");
        assert!(entry_diag.is_some_and(|d| d.severity == DiagnosticSeverity::Error));
    }

    #[test]
    fn diagnostic_references_correct_node() {
        let mut doc = FlowDocument::default();
        doc.graph.entry_node = Some(nid("ghost"));
        let validator = StructuralValidator;
        let diags = validator.validate(&doc);
        let entry_diag = diags.iter().find(|d| d.code.as_str() == "entry-node-missing");
        assert!(entry_diag.is_some_and(|d| d.node.as_ref().is_some_and(|n| n == &nid("ghost"))));
    }

    #[test]
    fn edge_diagnostic_references_edge_id() {
        let mut doc = FlowDocument::default();
        doc.graph.nodes.insert(nid("n2"), make_node_with_ports("n2", &["p-in"]));
        doc.graph.edges.insert(
            eid("e1"),
            make_edge_with_ports("e1", "ghost", "p-out", "n2", "p-in"),
        );
        let validator = StructuralValidator;
        let diags = validator.validate(&doc);
        let src_diag = diags.iter().find(|d| d.code.as_str() == "edge-source-missing");
        assert!(src_diag.is_some_and(|d| d.edge.as_ref().is_some_and(|e| e == &eid("e1"))));
    }
}
