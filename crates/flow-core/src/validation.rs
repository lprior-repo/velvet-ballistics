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
