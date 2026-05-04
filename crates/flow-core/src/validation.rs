use crate::doc::*;
use crate::ids::*;
use crate::patch::{Diagnostic, DiagnosticSeverity};
use smol_str::SmolStr;
use std::collections::{HashMap, HashSet};

// ---------------------------------------------------------------------------
// Graph size limits (DoS prevention)
// ---------------------------------------------------------------------------

const MAX_GRAPH_NODES: usize = 10_000;
const MAX_GRAPH_EDGES: usize = 100_000;
const MAX_NODE_FANOUT: usize = 1_000;

/// Maximum number of entries allowed in validation HashMap/HashSet accumulators.
/// Prevents unbounded memory growth when processing pathological inputs.
const MAX_VALIDATION_ENTRIES: usize = 50_000;

// ---------------------------------------------------------------------------
// Validation level
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ValidationLevel {
    Error,
    Warning,
}

impl ValidationLevel {}

// ---------------------------------------------------------------------------
// Validation finding
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct ValidationFinding {
    pub level: ValidationLevel,
    pub code: SmolStr,
    pub message: String,
    pub node: Option<NodeId>,
    pub edge: Option<EdgeId>,
}

impl ValidationFinding {
    pub fn new(
        level: ValidationLevel,
        code: &str,
        message: String,
        node: Option<NodeId>,
        edge: Option<EdgeId>,
    ) -> Self {
        Self {
            level,
            code: SmolStr::from(code),
            message,
            node,
            edge,
        }
    }
}

fn diagnostic_to_finding(d: &Diagnostic) -> ValidationFinding {
    let level = match d.severity {
        DiagnosticSeverity::Error => ValidationLevel::Error,
        DiagnosticSeverity::Warning | DiagnosticSeverity::Info => ValidationLevel::Warning,
    };
    ValidationFinding {
        level,
        code: d.code.clone(),
        message: d.message.clone(),
        node: d.node.clone(),
        edge: d.edge.clone(),
    }
}

// ---------------------------------------------------------------------------
// Validator trait
// ---------------------------------------------------------------------------

pub trait FlowValidator: Send + Sync {
    fn validate(&self, doc: &FlowDocument) -> Vec<Diagnostic>;
}

// ---------------------------------------------------------------------------
// Graph limits validator (phase 0 - DoS gate)
// ---------------------------------------------------------------------------

pub struct GraphLimitsValidator;

impl FlowValidator for GraphLimitsValidator {
    fn validate(&self, doc: &FlowDocument) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        let node_count = doc.graph.nodes.len();
        if node_count > MAX_GRAPH_NODES {
            diagnostics.push(diag(
                DiagnosticSeverity::Error,
                "graph-limit-nodes-exceeded",
                format!(
                    "graph has {node_count} nodes, exceeding the maximum of {MAX_GRAPH_NODES}"
                ),
                None,
                None,
            ));
        }

        let edge_count = doc.graph.edges.len();
        if edge_count > MAX_GRAPH_EDGES {
            diagnostics.push(diag(
                DiagnosticSeverity::Error,
                "graph-limit-edges-exceeded",
                format!(
                    "graph has {edge_count} edges, exceeding the maximum of {MAX_GRAPH_EDGES}"
                ),
                None,
                None,
            ));
        }

        check_node_fanout(&doc.graph, &mut diagnostics);

        diagnostics
    }
}

fn check_node_fanout(graph: &FlowGraph, diagnostics: &mut Vec<Diagnostic>) {
    let cap = graph.edges.len().min(MAX_VALIDATION_ENTRIES);
    let mut fanout: HashMap<NodeId, usize> = HashMap::with_capacity(cap);
    for edge in graph.edges.values() {
        if graph.nodes.contains_key(&edge.source_node) {
            let count = fanout.entry(edge.source_node.clone()).or_insert(0);
            *count = count.saturating_add(1);
        }
    }
    for (node_id, count) in &fanout {
        if *count > MAX_NODE_FANOUT {
            diagnostics.push(diag(
                DiagnosticSeverity::Error,
                "graph-limit-fanout-exceeded",
                format!(
                    "node '{node_id}' has {count} outgoing edges, exceeding the maximum fanout of {MAX_NODE_FANOUT}"
                ),
                Some(node_id.clone()),
                None,
            ));
        }
    }
}

// ---------------------------------------------------------------------------
// Structural validator (phase 1)
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
// Semantic validator (phase 2)
// ---------------------------------------------------------------------------

pub struct SemanticValidator;

impl FlowValidator for SemanticValidator {
    fn validate(&self, doc: &FlowDocument) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        semantic_check_edge_connectivity(&doc.graph, &mut diagnostics);
        semantic_check_orphan_nodes(&doc.graph, &mut diagnostics);
        semantic_check_group_consistency(&doc.graph, &mut diagnostics);
        semantic_check_overlapping_groups(&doc.graph, &mut diagnostics);
        semantic_check_port_type_compatibility(&doc.graph, &mut diagnostics);
        semantic_check_duplicate_edges(&doc.graph, &mut diagnostics);
        semantic_check_cycles(&doc.graph, &mut diagnostics);
        diagnostics
    }
}

// ---------------------------------------------------------------------------
// Export validator (phase 3)
// ---------------------------------------------------------------------------

const EXPORT_MAX_GRAPH_DIMENSION: f64 = 10_000.0;

pub struct ExportValidator;

impl FlowValidator for ExportValidator {
    fn validate(&self, doc: &FlowDocument) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        check_node_metadata(&doc.graph, &mut diagnostics);
        check_duplicate_node_names(&doc.graph, &mut diagnostics);
        check_graph_bounds(&doc.graph, &mut diagnostics);
        check_overlapping_nodes(&doc.graph, &mut diagnostics);
        check_port_connections(&doc.graph, &mut diagnostics);
        check_edge_node_ids(&doc.graph, &mut diagnostics);
        diagnostics
    }
}

// ---------------------------------------------------------------------------
// Shared diagnostic helper
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

// ---------------------------------------------------------------------------
// Structural checks
// ---------------------------------------------------------------------------

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
                format!("edge '{}' references non-existent source node '{}'", edge_id, edge.source_node),
                Some(edge.source_node.clone()),
                Some(edge_id.clone()),
            ));
        }
        if !graph.nodes.contains_key(&edge.target_node) {
            diagnostics.push(diag(
                DiagnosticSeverity::Error,
                "edge-target-missing",
                format!("edge '{}' references non-existent target node '{}'", edge_id, edge.target_node),
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
                    format!("edge '{}' references non-existent source port '{}' on node '{}'", edge_id, edge.source_port, edge.source_node),
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
                    format!("edge '{}' references non-existent target port '{}' on node '{}'", edge_id, edge.target_port, edge.target_node),
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
                format!("edge '{}' is a self-loop on the same port '{}' of node '{}'", edge_id, edge.source_port, edge.source_node),
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
                format!("node '{}' references non-existent parent group '{}'", node_id, group_id),
                Some(node_id.clone()),
                None,
            ));
        }
    }
}

// ---------------------------------------------------------------------------
// Semantic checks
// ---------------------------------------------------------------------------

fn semantic_check_edge_connectivity(graph: &FlowGraph, diagnostics: &mut Vec<Diagnostic>) {
    for (edge_id, edge) in &graph.edges {
        let source_node = match graph.nodes.get(&edge.source_node) {
            Some(n) => n,
            None => continue,
        };
        let target_node = match graph.nodes.get(&edge.target_node) {
            Some(n) => n,
            None => continue,
        };
        let source_port = source_node.ports.iter().find(|p| p.id == edge.source_port);
        let target_port = target_node.ports.iter().find(|p| p.id == edge.target_port);

        if let Some(sp) = source_port
            && sp.role == PortRole::Target
        {
            diagnostics.push(diag(
                DiagnosticSeverity::Error,
                "edge-source-port-role-mismatch",
                format!("edge '{}' uses port '{}' on node '{}' as source, but port role is Target", edge_id, edge.source_port, edge.source_node),
                Some(edge.source_node.clone()),
                Some(edge_id.clone()),
            ));
        }

        if let Some(tp) = target_port
            && tp.role == PortRole::Source
        {
            diagnostics.push(diag(
                DiagnosticSeverity::Error,
                "edge-target-port-role-mismatch",
                format!("edge '{}' uses port '{}' on node '{}' as target, but port role is Source", edge_id, edge.target_port, edge.target_node),
                Some(edge.target_node.clone()),
                Some(edge_id.clone()),
            ));
        }
    }
}

fn semantic_check_orphan_nodes(graph: &FlowGraph, diagnostics: &mut Vec<Diagnostic>) {
    let cap = graph.edges.len().min(MAX_VALIDATION_ENTRIES);
    let mut connected_nodes: HashSet<NodeId> = HashSet::with_capacity(cap);
    for edge in graph.edges.values() {
        if graph.nodes.contains_key(&edge.source_node) {
            connected_nodes.insert(edge.source_node.clone());
        }
        if graph.nodes.contains_key(&edge.target_node) {
            connected_nodes.insert(edge.target_node.clone());
        }
    }
    for (node_id, node) in &graph.nodes {
        if node.flags.entry || node.flags.terminal {
            continue;
        }
        if !connected_nodes.contains(node_id) {
            diagnostics.push(diag(
                DiagnosticSeverity::Warning,
                "orphan-node",
                format!("node '{}' has no connected edges (not an entry or terminal node)", node_id),
                Some(node_id.clone()),
                None,
            ));
        }
    }
}

fn semantic_check_group_consistency(graph: &FlowGraph, diagnostics: &mut Vec<Diagnostic>) {
    for (group_id, group) in &graph.groups {
        let width = group.bounds[2];
        let height = group.bounds[3];
        if width <= 0.0 || height <= 0.0 {
            diagnostics.push(diag(
                DiagnosticSeverity::Error,
                "group-degenerate-bounds",
                format!("group '{}' has degenerate bounds (width={}, height={})", group_id, width, height),
                None,
                None,
            ));
        }
        let has_members = graph.nodes.values().any(|n| n.parent.as_ref() == Some(group_id));
        if !has_members {
            diagnostics.push(diag(
                DiagnosticSeverity::Warning,
                "group-empty",
                format!("group '{}' has no member nodes", group_id),
                None,
                None,
            ));
        }
    }
}

fn semantic_check_overlapping_groups(graph: &FlowGraph, diagnostics: &mut Vec<Diagnostic>) {
    let groups: Vec<(&GroupId, &FlowGroupRecord)> = graph.groups.iter().collect();
    let len = groups.len();
    for i in 0..len {
        for j in (i.saturating_add(1))..len {
            let (id_a, grp_a) = match groups.get(i) { Some(pair) => pair, None => continue };
            let (id_b, grp_b) = match groups.get(j) { Some(pair) => pair, None => continue };
            let a_x2 = grp_a.bounds[0] + grp_a.bounds[2];
            let a_y2 = grp_a.bounds[1] + grp_a.bounds[3];
            let b_x2 = grp_b.bounds[0] + grp_b.bounds[2];
            let b_y2 = grp_b.bounds[1] + grp_b.bounds[3];
            let overlaps = grp_a.bounds[0] < b_x2 && a_x2 > grp_b.bounds[0]
                && grp_a.bounds[1] < b_y2 && a_y2 > grp_b.bounds[1];
            if overlaps {
                diagnostics.push(diag(
                    DiagnosticSeverity::Warning,
                    "overlapping-groups",
                    format!("groups '{id_a}' and '{id_b}' have overlapping bounds"),
                    None,
                    None,
                ));
            }
        }
    }
}

fn semantic_check_port_type_compatibility(graph: &FlowGraph, diagnostics: &mut Vec<Diagnostic>) {
    let cap = graph.edges.len().min(MAX_VALIDATION_ENTRIES);
    let mut port_edges: HashMap<(NodeId, PortId), Vec<EdgeId>> = HashMap::with_capacity(cap);
    for (edge_id, edge) in &graph.edges {
        if !graph.nodes.contains_key(&edge.source_node) {
            continue;
        }
        port_edges
            .entry((edge.source_node.clone(), edge.source_port.clone()))
            .or_default()
            .push(edge_id.clone());
    }
    for ((node_id, port_id), edge_ids) in &port_edges {
        if edge_ids.len() < 2 {
            continue;
        }
        let node = match graph.nodes.get(node_id) { Some(n) => n, None => continue };
        let port = match node.ports.iter().find(|p| p.id == *port_id) { Some(p) => p, None => continue };
        if port.cardinality == Cardinality::One {
            diagnostics.push(diag(
                DiagnosticSeverity::Error,
                "cardinality-one-multi-source",
                format!("port '{}' on node '{}' has cardinality One but is source for {} edges", port_id, node_id, edge_ids.len()),
                Some(node_id.clone()),
                None,
            ));
        }
    }
}

fn semantic_check_duplicate_edges(graph: &FlowGraph, diagnostics: &mut Vec<Diagnostic>) {
    let cap = graph.edges.len().min(MAX_VALIDATION_ENTRIES);
    let mut seen: HashMap<(NodeId, PortId, NodeId, PortId), EdgeId> = HashMap::with_capacity(cap);
    for (edge_id, edge) in &graph.edges {
        let key = (edge.source_node.clone(), edge.source_port.clone(), edge.target_node.clone(), edge.target_port.clone());
        match seen.entry(key) {
            std::collections::hash_map::Entry::Occupied(entry) => {
                let first_id = entry.get();
                diagnostics.push(diag(
                    DiagnosticSeverity::Error,
                    "duplicate-edge",
                    format!("edge '{}' duplicates edge '{}' (same source port -> target port)", edge_id, first_id),
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

fn semantic_check_cycles(graph: &FlowGraph, diagnostics: &mut Vec<Diagnostic>) {
    if graph.topological_sort().is_none() {
        diagnostics.push(diag(
            DiagnosticSeverity::Warning,
            "graph-contains-cycle",
            String::from("graph contains at least one cycle"),
            None,
            None,
        ));
    }
}

// ---------------------------------------------------------------------------
// Export checks
// ---------------------------------------------------------------------------

fn check_node_metadata(graph: &FlowGraph, diagnostics: &mut Vec<Diagnostic>) {
    for (node_id, node) in &graph.nodes {
        if node.kind.is_empty() {
            diagnostics.push(diag(DiagnosticSeverity::Error, "export-node-kind-empty", format!("node '{node_id}' has an empty kind field"), Some(node_id.clone()), None));
        }
        if node.title.is_empty() {
            diagnostics.push(diag(DiagnosticSeverity::Error, "export-node-title-empty", format!("node '{node_id}' has an empty title field"), Some(node_id.clone()), None));
        }
        let pos_x = node.position[0];
        let pos_y = node.position[1];
        if !pos_x.is_finite() || !pos_y.is_finite() {
            diagnostics.push(diag(DiagnosticSeverity::Error, "export-node-position-invalid", format!("node '{node_id}' has non-finite position ({pos_x}, {pos_y})"), Some(node_id.clone()), None));
        }
    }
}

fn check_duplicate_node_names(graph: &FlowGraph, diagnostics: &mut Vec<Diagnostic>) {
    let cap = graph.nodes.len().min(MAX_VALIDATION_ENTRIES);
    let mut seen: HashMap<SmolStr, NodeId> = HashMap::with_capacity(cap);
    for (node_id, node) in &graph.nodes {
        let title = &node.title;
        if title.is_empty() { continue; }
        match seen.entry(title.clone()) {
            std::collections::hash_map::Entry::Occupied(entry) => {
                let first_id = entry.get();
                diagnostics.push(diag(DiagnosticSeverity::Error, "export-duplicate-node-title", format!("nodes '{first_id}' and '{node_id}' share the same title '{}'", title), Some(node_id.clone()), None));
            }
            std::collections::hash_map::Entry::Vacant(entry) => { entry.insert(node_id.clone()); }
        }
    }
}

fn check_graph_bounds(graph: &FlowGraph, diagnostics: &mut Vec<Diagnostic>) {
    if graph.nodes.is_empty() { return; }
    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    let mut has_valid_position = false;
    for node in graph.nodes.values() {
        let px = node.position[0];
        let py = node.position[1];
        if px.is_finite() && py.is_finite() {
            has_valid_position = true;
            let half_w = node.size[0] / 2.0;
            let half_h = node.size[1] / 2.0;
            let nmin_x = px - half_w;
            let nmin_y = py - half_h;
            let nmax_x = px + half_w;
            let nmax_y = py + half_h;
            if nmin_x < min_x { min_x = nmin_x; }
            if nmin_y < min_y { min_y = nmin_y; }
            if nmax_x > max_x { max_x = nmax_x; }
            if nmax_y > max_y { max_y = nmax_y; }
        }
    }
    if !has_valid_position { return; }
    let width = max_x - min_x;
    let height = max_y - min_y;
    if width > EXPORT_MAX_GRAPH_DIMENSION || height > EXPORT_MAX_GRAPH_DIMENSION {
        diagnostics.push(diag(
            DiagnosticSeverity::Warning,
            "export-graph-bounds-exceeded",
            format!("graph bounds ({width:.0}x{height:.0}) exceed recommended maximum of {EXPORT_MAX_GRAPH_DIMENSION:.0}x{EXPORT_MAX_GRAPH_DIMENSION:.0} pixels"),
            None,
            None,
        ));
    }
}

fn approx_equal(a: f64, b: f64) -> bool {
    let diff = (a - b).abs();
    let largest = a.abs().max(b.abs());
    diff <= largest * f64::EPSILON * 2.0
}

fn check_overlapping_nodes(graph: &FlowGraph, diagnostics: &mut Vec<Diagnostic>) {
    let nodes: Vec<(&NodeId, &FlowNodeRecord)> = graph.nodes.iter().collect();
    let len = nodes.len();
    for i in 0..len {
        for j in (i.saturating_add(1))..len {
            let (id_a, node_a) = match nodes.get(i) { Some(pair) => pair, None => continue };
            let (id_b, node_b) = match nodes.get(j) { Some(pair) => pair, None => continue };
            let ax = node_a.position[0];
            let ay = node_a.position[1];
            let bx = node_b.position[0];
            let by = node_b.position[1];
            if !ax.is_finite() || !ay.is_finite() || !bx.is_finite() || !by.is_finite() { continue; }
            if approx_equal(ax, bx) && approx_equal(ay, by) {
                diagnostics.push(diag(
                    DiagnosticSeverity::Warning,
                    "export-overlapping-nodes",
                    format!("nodes '{id_a}' and '{id_b}' occupy the same center position ({ax}, {ay})"),
                    Some((*id_a).clone()),
                    None,
                ));
            }
        }
    }
}

fn check_port_connections(graph: &FlowGraph, diagnostics: &mut Vec<Diagnostic>) {
    let cap = graph.edges.len().saturating_mul(2).min(MAX_VALIDATION_ENTRIES);
    let mut connected_ports: HashSet<(NodeId, PortId)> = HashSet::with_capacity(cap);
    for edge in graph.edges.values() {
        if graph.nodes.contains_key(&edge.source_node) {
            connected_ports.insert((edge.source_node.clone(), edge.source_port.clone()));
        }
        if graph.nodes.contains_key(&edge.target_node) {
            connected_ports.insert((edge.target_node.clone(), edge.target_port.clone()));
        }
    }
    for (node_id, node) in &graph.nodes {
        let is_entry = node.flags.entry;
        for port in &node.ports {
            let is_connected = connected_ports.contains(&(node_id.clone(), port.id.clone()));
            if !is_connected {
                let is_source_or_bidi = port.role == PortRole::Source || port.role == PortRole::Bidirectional;
                let is_entry_output = is_entry && is_source_or_bidi;
                let is_terminal_input = node.flags.terminal && (port.role == PortRole::Target || port.role == PortRole::Bidirectional);
                if !is_entry_output && !is_terminal_input {
                    diagnostics.push(diag(
                        DiagnosticSeverity::Warning,
                        "export-port-unconnected",
                        format!("port '{}' on node '{}' has no connections", port.id, node_id),
                        Some(node_id.clone()),
                        None,
                    ));
                }
            }
        }
    }
}

fn check_edge_node_ids(graph: &FlowGraph, diagnostics: &mut Vec<Diagnostic>) {
    for (edge_id, edge) in &graph.edges {
        if edge.source_node.is_empty() {
            diagnostics.push(diag(DiagnosticSeverity::Error, "export-edge-source-node-empty", format!("edge '{edge_id}' has an empty source_node"), None, Some(edge_id.clone())));
        }
        if edge.target_node.is_empty() {
            diagnostics.push(diag(DiagnosticSeverity::Error, "export-edge-target-node-empty", format!("edge '{edge_id}' has an empty target_node"), None, Some(edge_id.clone())));
        }
    }
}

// ---------------------------------------------------------------------------
// Validation pipeline
// ---------------------------------------------------------------------------

pub struct ValidationPipeline {
    validators: Vec<Box<dyn FlowValidator>>,
}

impl ValidationPipeline {
    pub fn new() -> Self {
        Self { validators: Vec::new() }
    }

    pub fn standard() -> Self {
        let mut pipeline = Self::new();
        pipeline.add_validator(Box::new(GraphLimitsValidator));
        pipeline.add_validator(Box::new(StructuralValidator));
        pipeline.add_validator(Box::new(SemanticValidator));
        pipeline.add_validator(Box::new(ExportValidator));
        pipeline
    }

    pub fn add_validator(&mut self, validator: Box<dyn FlowValidator>) {
        self.validators.push(validator);
    }

    pub fn validate(&self, doc: &FlowDocument) -> Vec<Diagnostic> {
        let mut all_diagnostics = Vec::new();
        for validator in &self.validators {
            let mut diags = validator.validate(doc);
            all_diagnostics.append(&mut diags);
        }
        all_diagnostics
    }

    pub fn run(doc: &FlowDocument) -> Vec<ValidationFinding> {
        let pipeline = Self::standard();
        let diagnostics = pipeline.validate(doc);
        diagnostics.iter().map(diagnostic_to_finding).collect()
    }
}

impl Default for ValidationPipeline {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::doc::{
        Cardinality, EdgeStyle, FlowEdgeRecord, FlowGroupRecord, FlowNodeRecord, FlowPortRecord,
        GroupKind, NodeFlags, NodeUiState, PortRole, PortSide,
    };
    use crate::ids::{EdgeId, GroupId, NodeId, PortId};
    use smol_str::SmolStr;

    fn nid(s: &str) -> NodeId { SmolStr::from(s) }
    fn eid(s: &str) -> EdgeId { SmolStr::from(s) }
    fn pid(s: &str) -> PortId { SmolStr::from(s) }
    fn gid(s: &str) -> GroupId { SmolStr::from(s) }

    fn make_node_with_ports(id: &str, port_ids: &[&str]) -> FlowNodeRecord {
        FlowNodeRecord {
            id: nid(id),
            kind: SmolStr::from("test"),
            title: SmolStr::from(id),
            position: [0.0, 0.0],
            size: [100.0, 50.0],
            z_index: 0,
            parent: None,
            ports: port_ids.iter().enumerate().map(|(i, p)| FlowPortRecord {
                id: pid(p),
                side: if i % 2 == 0 { PortSide::Left } else { PortSide::Right },
                role: if i % 2 == 0 { PortRole::Target } else { PortRole::Source },
                label: SmolStr::from(*p),
                order: u16::try_from(i).unwrap_or(u16::MAX),
                cardinality: Cardinality::One,
                data_type: None,
            }).collect(),
            flags: NodeFlags::default(),
            data: serde_json::Value::Null,
            ui: NodeUiState::default(),
        }
    }

    fn make_edge_with_ports(id: &str, src: &str, src_port: &str, tgt: &str, tgt_port: &str) -> FlowEdgeRecord {
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

    fn make_port_record(id: &str, role: PortRole, cardinality: Cardinality) -> FlowPortRecord {
        FlowPortRecord {
            id: pid(id),
            side: match role { PortRole::Source | PortRole::Bidirectional => PortSide::Right, PortRole::Target => PortSide::Left },
            role,
            label: SmolStr::from(id),
            order: 0,
            cardinality,
            data_type: None,
        }
    }

    fn valid_document() -> FlowDocument {
        let mut doc = FlowDocument::default();
        doc.graph.nodes.insert(nid("n1"), make_node_with_ports("n1", &["p-out"]));
        doc.graph.nodes.insert(nid("n2"), make_node_with_ports("n2", &["p-in"]));
        doc.graph.edges.insert(eid("e1"), make_edge_with_ports("e1", "n1", "p-out", "n2", "p-in"));
        doc
    }

    fn semantic_doc_two_nodes_connected() -> FlowDocument {
        let mut doc = FlowDocument::default();
        let n1 = FlowNodeRecord {
            id: nid("n1"), kind: SmolStr::from("test"), title: SmolStr::from("n1"),
            position: [0.0, 0.0], size: [100.0, 50.0], z_index: 0, parent: None,
            ports: vec![make_port_record("out", PortRole::Source, Cardinality::One)],
            flags: NodeFlags::default(), data: serde_json::Value::Null, ui: NodeUiState::default(),
        };
        let n2 = FlowNodeRecord {
            id: nid("n2"), kind: SmolStr::from("test"), title: SmolStr::from("n2"),
            position: [200.0, 0.0], size: [100.0, 50.0], z_index: 0, parent: None,
            ports: vec![make_port_record("in", PortRole::Target, Cardinality::One)],
            flags: NodeFlags::default(), data: serde_json::Value::Null, ui: NodeUiState::default(),
        };
        doc.graph.nodes.insert(nid("n1"), n1);
        doc.graph.nodes.insert(nid("n2"), n2);
        doc.graph.edges.insert(eid("e1"), make_edge_with_ports("e1", "n1", "out", "n2", "in"));
        doc
    }

    // -- Structural tests --

    #[test]
    fn structural_valid_document_no_diagnostics() {
        assert!(StructuralValidator.validate(&valid_document()).is_empty());
    }

    #[test]
    fn structural_empty_document_no_diagnostics() {
        assert!(StructuralValidator.validate(&FlowDocument::default()).is_empty());
    }

    #[test]
    fn structural_entry_node_missing() {
        let mut doc = valid_document();
        doc.graph.entry_node = Some(nid("nonexistent"));
        assert!(StructuralValidator.validate(&doc).into_iter().any(|d| d.code.as_str() == "entry-node-missing"));
    }

    #[test]
    fn structural_entry_node_valid() {
        let mut doc = valid_document();
        doc.graph.entry_node = Some(nid("n1"));
        assert!(!StructuralValidator.validate(&doc).into_iter().any(|d| d.code.as_str() == "entry-node-missing"));
    }

    #[test]
    fn structural_edge_source_missing() {
        let mut doc = FlowDocument::default();
        doc.graph.nodes.insert(nid("n2"), make_node_with_ports("n2", &["p-in"]));
        doc.graph.edges.insert(eid("e1"), make_edge_with_ports("e1", "ghost", "p-out", "n2", "p-in"));
        assert!(StructuralValidator.validate(&doc).into_iter().any(|d| d.code.as_str() == "edge-source-missing"));
    }

    #[test]
    fn structural_edge_target_missing() {
        let mut doc = FlowDocument::default();
        doc.graph.nodes.insert(nid("n1"), make_node_with_ports("n1", &["p-out"]));
        doc.graph.edges.insert(eid("e1"), make_edge_with_ports("e1", "n1", "p-out", "ghost", "p-in"));
        assert!(StructuralValidator.validate(&doc).into_iter().any(|d| d.code.as_str() == "edge-target-missing"));
    }

    #[test]
    fn structural_both_endpoints_missing() {
        let mut doc = FlowDocument::default();
        doc.graph.edges.insert(eid("e1"), make_edge_with_ports("e1", "g1", "p-out", "g2", "p-in"));
        let diags = StructuralValidator.validate(&doc);
        assert!(diags.iter().any(|d| d.code.as_str() == "edge-source-missing"));
        assert!(diags.iter().any(|d| d.code.as_str() == "edge-target-missing"));
    }

    #[test]
    fn structural_source_port_missing() {
        let mut doc = FlowDocument::default();
        doc.graph.nodes.insert(nid("n1"), make_node_with_ports("n1", &["p-other"]));
        doc.graph.nodes.insert(nid("n2"), make_node_with_ports("n2", &["p-in"]));
        doc.graph.edges.insert(eid("e1"), make_edge_with_ports("e1", "n1", "p-out", "n2", "p-in"));
        assert!(StructuralValidator.validate(&doc).into_iter().any(|d| d.code.as_str() == "edge-source-port-missing"));
    }

    #[test]
    fn structural_target_port_missing() {
        let mut doc = FlowDocument::default();
        doc.graph.nodes.insert(nid("n1"), make_node_with_ports("n1", &["p-out"]));
        doc.graph.nodes.insert(nid("n2"), make_node_with_ports("n2", &["p-other"]));
        doc.graph.edges.insert(eid("e1"), make_edge_with_ports("e1", "n1", "p-out", "n2", "p-in"));
        assert!(StructuralValidator.validate(&doc).into_iter().any(|d| d.code.as_str() == "edge-target-port-missing"));
    }

    #[test]
    fn structural_self_loop_same_port() {
        let mut doc = FlowDocument::default();
        doc.graph.nodes.insert(nid("n1"), make_node_with_ports("n1", &["p-io"]));
        doc.graph.edges.insert(eid("e1"), make_edge_with_ports("e1", "n1", "p-io", "n1", "p-io"));
        assert!(StructuralValidator.validate(&doc).into_iter().any(|d| d.code.as_str() == "self-loop-same-port"));
    }

    #[test]
    fn structural_self_loop_different_ports() {
        let mut doc = FlowDocument::default();
        doc.graph.nodes.insert(nid("n1"), make_node_with_ports("n1", &["p-out", "p-in"]));
        doc.graph.edges.insert(eid("e1"), make_edge_with_ports("e1", "n1", "p-out", "n1", "p-in"));
        assert!(!StructuralValidator.validate(&doc).into_iter().any(|d| d.code.as_str() == "self-loop-same-port"));
    }

    #[test]
    fn structural_parent_group_missing() {
        let mut doc = FlowDocument::default();
        let mut node = make_node_with_ports("n1", &[]);
        node.parent = Some(gid("ghost"));
        doc.graph.nodes.insert(nid("n1"), node);
        assert!(StructuralValidator.validate(&doc).into_iter().any(|d| d.code.as_str() == "node-parent-group-missing"));
    }

    #[test]
    fn structural_parent_group_valid() {
        let mut doc = FlowDocument::default();
        let mut node = make_node_with_ports("n1", &[]);
        node.parent = Some(gid("g1"));
        doc.graph.nodes.insert(nid("n1"), node);
        doc.graph.groups.insert(gid("g1"), make_group("g1"));
        assert!(!StructuralValidator.validate(&doc).into_iter().any(|d| d.code.as_str() == "node-parent-group-missing"));
    }

    #[test]
    fn structural_multiple_issues() {
        let mut doc = FlowDocument::default();
        doc.graph.entry_node = Some(nid("ghost"));
        doc.graph.edges.insert(eid("e1"), make_edge_with_ports("e1", "g1", "p", "g2", "p"));
        let mut node = make_node_with_ports("n1", &[]);
        node.parent = Some(gid("g-ghost"));
        doc.graph.nodes.insert(nid("n1"), node);
        assert!(StructuralValidator.validate(&doc).len() >= 4);
    }

    #[test]
    fn structural_diagnostic_severity() {
        let mut doc = FlowDocument::default();
        doc.graph.entry_node = Some(nid("ghost"));
        let diag = StructuralValidator.validate(&doc).into_iter().find(|d| d.code.as_str() == "entry-node-missing");
        assert!(diag.is_some_and(|d| d.severity == DiagnosticSeverity::Error));
    }

    #[test]
    fn structural_diagnostic_node_ref() {
        let mut doc = FlowDocument::default();
        doc.graph.entry_node = Some(nid("ghost"));
        let diag = StructuralValidator.validate(&doc).into_iter().find(|d| d.code.as_str() == "entry-node-missing");
        assert!(diag.is_some_and(|d| d.node.as_ref().is_some_and(|n| n == &nid("ghost"))));
    }

    #[test]
    fn structural_diagnostic_edge_ref() {
        let mut doc = FlowDocument::default();
        doc.graph.nodes.insert(nid("n2"), make_node_with_ports("n2", &["p-in"]));
        doc.graph.edges.insert(eid("e1"), make_edge_with_ports("e1", "ghost", "p-out", "n2", "p-in"));
        let diag = StructuralValidator.validate(&doc).into_iter().find(|d| d.code.as_str() == "edge-source-missing");
        assert!(diag.is_some_and(|d| d.edge.as_ref().is_some_and(|e| e == &eid("e1"))));
    }

    #[test]
    fn structural_multiple_self_loops() {
        let mut doc = FlowDocument::default();
        doc.graph.nodes.insert(nid("n1"), make_node_with_ports("n1", &["p-io"]));
        doc.graph.edges.insert(eid("e1"), make_edge_with_ports("e1", "n1", "p-io", "n1", "p-io"));
        doc.graph.edges.insert(eid("e2"), make_edge_with_ports("e2", "n1", "p-io", "n1", "p-io"));
        assert_eq!(StructuralValidator.validate(&doc).into_iter().filter(|d| d.code.as_str() == "self-loop-same-port").count(), 2);
    }

    #[test]
    fn structural_trait_object() {
        let v: Box<dyn FlowValidator> = Box::new(StructuralValidator);
        assert!(v.validate(&FlowDocument::default()).is_empty());
    }

    #[test]
    fn structural_large_valid() {
        let mut doc = FlowDocument::default();
        for i in 0..10usize {
            let ports = vec![
                FlowPortRecord { id: pid(&format!("p-out-{i}")), side: PortSide::Right, role: PortRole::Source, label: SmolStr::from(format!("p-out-{i}")), order: 0, cardinality: Cardinality::One, data_type: None },
                FlowPortRecord { id: pid(&format!("p-in-{i}")), side: PortSide::Left, role: PortRole::Target, label: SmolStr::from(format!("p-in-{i}")), order: 1, cardinality: Cardinality::One, data_type: None },
            ];
            doc.graph.nodes.insert(nid(&format!("n{i}")), FlowNodeRecord {
                id: nid(&format!("n{i}")), kind: SmolStr::from("test"), title: SmolStr::from(format!("n{i}")),
                position: [0.0, 0.0], size: [100.0, 50.0], z_index: 0, parent: None, ports,
                flags: NodeFlags::default(), data: serde_json::Value::Null, ui: NodeUiState::default(),
            });
        }
        for i in 0usize..9 {
            let next = i.saturating_add(1);
            doc.graph.edges.insert(eid(&format!("e{i}")), FlowEdgeRecord {
                id: eid(&format!("e{i}")), source_node: nid(&format!("n{i}")), source_port: pid(&format!("p-out-{i}")),
                target_node: nid(&format!("n{next}")), target_port: pid(&format!("p-in-{next}")),
                label: None, style: EdgeStyle::default(), data: serde_json::Value::Null, ui: EdgeUiState::default(),
            });
        }
        assert!(StructuralValidator.validate(&doc).is_empty());
    }

    // -- Semantic tests --

    #[test]
    fn semantic_valid_doc() {
        assert!(SemanticValidator.validate(&semantic_doc_two_nodes_connected()).is_empty());
    }

    #[test]
    fn semantic_empty_doc() {
        assert!(SemanticValidator.validate(&FlowDocument::default()).is_empty());
    }

    #[test]
    fn semantic_source_port_is_target_role() {
        let mut doc = FlowDocument::default();
        let n1 = FlowNodeRecord { id: nid("n1"), kind: SmolStr::from("test"), title: SmolStr::from("n1"), position: [0.0, 0.0], size: [100.0, 50.0], z_index: 0, parent: None, ports: vec![make_port_record("p", PortRole::Target, Cardinality::One)], flags: NodeFlags::default(), data: serde_json::Value::Null, ui: NodeUiState::default() };
        let n2 = FlowNodeRecord { id: nid("n2"), kind: SmolStr::from("test"), title: SmolStr::from("n2"), position: [200.0, 0.0], size: [100.0, 50.0], z_index: 0, parent: None, ports: vec![make_port_record("in", PortRole::Target, Cardinality::One)], flags: NodeFlags::default(), data: serde_json::Value::Null, ui: NodeUiState::default() };
        doc.graph.nodes.insert(nid("n1"), n1);
        doc.graph.nodes.insert(nid("n2"), n2);
        doc.graph.edges.insert(eid("e1"), make_edge_with_ports("e1", "n1", "p", "n2", "in"));
        assert!(SemanticValidator.validate(&doc).into_iter().any(|d| d.code.as_str() == "edge-source-port-role-mismatch"));
    }

    #[test]
    fn semantic_target_port_is_source_role() {
        let mut doc = FlowDocument::default();
        let n1 = FlowNodeRecord { id: nid("n1"), kind: SmolStr::from("test"), title: SmolStr::from("n1"), position: [0.0, 0.0], size: [100.0, 50.0], z_index: 0, parent: None, ports: vec![make_port_record("out", PortRole::Source, Cardinality::One)], flags: NodeFlags::default(), data: serde_json::Value::Null, ui: NodeUiState::default() };
        let n2 = FlowNodeRecord { id: nid("n2"), kind: SmolStr::from("test"), title: SmolStr::from("n2"), position: [200.0, 0.0], size: [100.0, 50.0], z_index: 0, parent: None, ports: vec![make_port_record("p", PortRole::Source, Cardinality::One)], flags: NodeFlags::default(), data: serde_json::Value::Null, ui: NodeUiState::default() };
        doc.graph.nodes.insert(nid("n1"), n1);
        doc.graph.nodes.insert(nid("n2"), n2);
        doc.graph.edges.insert(eid("e1"), make_edge_with_ports("e1", "n1", "out", "n2", "p"));
        assert!(SemanticValidator.validate(&doc).into_iter().any(|d| d.code.as_str() == "edge-target-port-role-mismatch"));
    }

    #[test]
    fn semantic_bidirectional_allowed() {
        let mut doc = FlowDocument::default();
        let n1 = FlowNodeRecord { id: nid("n1"), kind: SmolStr::from("test"), title: SmolStr::from("n1"), position: [0.0, 0.0], size: [100.0, 50.0], z_index: 0, parent: None, ports: vec![make_port_record("p", PortRole::Bidirectional, Cardinality::Many)], flags: NodeFlags::default(), data: serde_json::Value::Null, ui: NodeUiState::default() };
        let n2 = FlowNodeRecord { id: nid("n2"), kind: SmolStr::from("test"), title: SmolStr::from("n2"), position: [200.0, 0.0], size: [100.0, 50.0], z_index: 0, parent: None, ports: vec![make_port_record("in", PortRole::Target, Cardinality::One)], flags: NodeFlags::default(), data: serde_json::Value::Null, ui: NodeUiState::default() };
        doc.graph.nodes.insert(nid("n1"), n1);
        doc.graph.nodes.insert(nid("n2"), n2);
        doc.graph.edges.insert(eid("e1"), make_edge_with_ports("e1", "n1", "p", "n2", "in"));
        let diags = SemanticValidator.validate(&doc);
        assert!(!diags.iter().any(|d| d.code.as_str() == "edge-source-port-role-mismatch" || d.code.as_str() == "edge-target-port-role-mismatch"));
    }

    #[test]
    fn semantic_orphan_node() {
        let mut doc = FlowDocument::default();
        let n1 = FlowNodeRecord { id: nid("n1"), kind: SmolStr::from("test"), title: SmolStr::from("n1"), position: [0.0, 0.0], size: [100.0, 50.0], z_index: 0, parent: None, ports: vec![make_port_record("out", PortRole::Source, Cardinality::One)], flags: NodeFlags::default(), data: serde_json::Value::Null, ui: NodeUiState::default() };
        let n2 = FlowNodeRecord { id: nid("n2"), kind: SmolStr::from("test"), title: SmolStr::from("n2"), position: [200.0, 0.0], size: [100.0, 50.0], z_index: 0, parent: None, ports: vec![make_port_record("in", PortRole::Target, Cardinality::One)], flags: NodeFlags::default(), data: serde_json::Value::Null, ui: NodeUiState::default() };
        doc.graph.nodes.insert(nid("n1"), n1);
        doc.graph.nodes.insert(nid("n2"), n2);
        doc.graph.edges.insert(eid("e1"), make_edge_with_ports("e1", "n1", "out", "n1", "out"));
        assert!(SemanticValidator.validate(&doc).into_iter().any(|d| d.code.as_str() == "orphan-node" && d.node.as_ref() == Some(&nid("n2"))));
    }

    #[test]
    fn semantic_entry_not_orphan() {
        let mut doc = FlowDocument::default();
        doc.graph.nodes.insert(nid("entry"), FlowNodeRecord { id: nid("entry"), kind: SmolStr::from("entry"), title: SmolStr::from("entry"), position: [0.0, 0.0], size: [100.0, 50.0], z_index: 0, parent: None, ports: vec![], flags: NodeFlags { entry: true, ..NodeFlags::default() }, data: serde_json::Value::Null, ui: NodeUiState::default() });
        assert!(!SemanticValidator.validate(&doc).into_iter().any(|d| d.code.as_str() == "orphan-node"));
    }

    #[test]
    fn semantic_terminal_not_orphan() {
        let mut doc = FlowDocument::default();
        doc.graph.nodes.insert(nid("exit"), FlowNodeRecord { id: nid("exit"), kind: SmolStr::from("exit"), title: SmolStr::from("exit"), position: [0.0, 0.0], size: [100.0, 50.0], z_index: 0, parent: None, ports: vec![], flags: NodeFlags { terminal: true, ..NodeFlags::default() }, data: serde_json::Value::Null, ui: NodeUiState::default() });
        assert!(!SemanticValidator.validate(&doc).into_iter().any(|d| d.code.as_str() == "orphan-node"));
    }

    #[test]
    fn semantic_group_degenerate_bounds() {
        let mut doc = semantic_doc_two_nodes_connected();
        doc.graph.groups.insert(gid("g1"), FlowGroupRecord { id: gid("g1"), kind: GroupKind::Generic, title: SmolStr::from("g1"), bounds: [10.0, 10.0, 0.0, 50.0], data: serde_json::Value::Null });
        assert!(SemanticValidator.validate(&doc).into_iter().any(|d| d.code.as_str() == "group-degenerate-bounds"));
    }

    #[test]
    fn semantic_group_empty() {
        let mut doc = semantic_doc_two_nodes_connected();
        doc.graph.groups.insert(gid("g1"), FlowGroupRecord { id: gid("g1"), kind: GroupKind::Generic, title: SmolStr::from("g1"), bounds: [0.0, 0.0, 200.0, 200.0], data: serde_json::Value::Null });
        assert!(SemanticValidator.validate(&doc).into_iter().any(|d| d.code.as_str() == "group-empty"));
    }

    #[test]
    fn semantic_cardinality_one_multi_source() {
        let mut doc = FlowDocument::default();
        let n1 = FlowNodeRecord { id: nid("n1"), kind: SmolStr::from("test"), title: SmolStr::from("n1"), position: [0.0, 0.0], size: [100.0, 50.0], z_index: 0, parent: None, ports: vec![make_port_record("out", PortRole::Source, Cardinality::One)], flags: NodeFlags::default(), data: serde_json::Value::Null, ui: NodeUiState::default() };
        let n2 = FlowNodeRecord { id: nid("n2"), kind: SmolStr::from("test"), title: SmolStr::from("n2"), position: [200.0, 0.0], size: [100.0, 50.0], z_index: 0, parent: None, ports: vec![make_port_record("in", PortRole::Target, Cardinality::One)], flags: NodeFlags::default(), data: serde_json::Value::Null, ui: NodeUiState::default() };
        let n3 = FlowNodeRecord { id: nid("n3"), kind: SmolStr::from("test"), title: SmolStr::from("n3"), position: [400.0, 0.0], size: [100.0, 50.0], z_index: 0, parent: None, ports: vec![make_port_record("in", PortRole::Target, Cardinality::One)], flags: NodeFlags::default(), data: serde_json::Value::Null, ui: NodeUiState::default() };
        doc.graph.nodes.insert(nid("n1"), n1);
        doc.graph.nodes.insert(nid("n2"), n2);
        doc.graph.nodes.insert(nid("n3"), n3);
        doc.graph.edges.insert(eid("e1"), make_edge_with_ports("e1", "n1", "out", "n2", "in"));
        doc.graph.edges.insert(eid("e2"), make_edge_with_ports("e2", "n1", "out", "n3", "in"));
        assert!(SemanticValidator.validate(&doc).into_iter().any(|d| d.code.as_str() == "cardinality-one-multi-source"));
    }

    #[test]
    fn semantic_cardinality_many_ok() {
        let mut doc = FlowDocument::default();
        let n1 = FlowNodeRecord { id: nid("n1"), kind: SmolStr::from("test"), title: SmolStr::from("n1"), position: [0.0, 0.0], size: [100.0, 50.0], z_index: 0, parent: None, ports: vec![make_port_record("out", PortRole::Source, Cardinality::Many)], flags: NodeFlags::default(), data: serde_json::Value::Null, ui: NodeUiState::default() };
        let n2 = FlowNodeRecord { id: nid("n2"), kind: SmolStr::from("test"), title: SmolStr::from("n2"), position: [200.0, 0.0], size: [100.0, 50.0], z_index: 0, parent: None, ports: vec![make_port_record("in", PortRole::Target, Cardinality::One)], flags: NodeFlags::default(), data: serde_json::Value::Null, ui: NodeUiState::default() };
        let n3 = FlowNodeRecord { id: nid("n3"), kind: SmolStr::from("test"), title: SmolStr::from("n3"), position: [400.0, 0.0], size: [100.0, 50.0], z_index: 0, parent: None, ports: vec![make_port_record("in", PortRole::Target, Cardinality::One)], flags: NodeFlags::default(), data: serde_json::Value::Null, ui: NodeUiState::default() };
        doc.graph.nodes.insert(nid("n1"), n1);
        doc.graph.nodes.insert(nid("n2"), n2);
        doc.graph.nodes.insert(nid("n3"), n3);
        doc.graph.edges.insert(eid("e1"), make_edge_with_ports("e1", "n1", "out", "n2", "in"));
        doc.graph.edges.insert(eid("e2"), make_edge_with_ports("e2", "n1", "out", "n3", "in"));
        assert!(!SemanticValidator.validate(&doc).into_iter().any(|d| d.code.as_str() == "cardinality-one-multi-source"));
    }

    #[test]
    fn semantic_duplicate_edge() {
        let mut doc = FlowDocument::default();
        let n1 = FlowNodeRecord { id: nid("n1"), kind: SmolStr::from("test"), title: SmolStr::from("n1"), position: [0.0, 0.0], size: [100.0, 50.0], z_index: 0, parent: None, ports: vec![make_port_record("out", PortRole::Source, Cardinality::Many)], flags: NodeFlags::default(), data: serde_json::Value::Null, ui: NodeUiState::default() };
        let n2 = FlowNodeRecord { id: nid("n2"), kind: SmolStr::from("test"), title: SmolStr::from("n2"), position: [200.0, 0.0], size: [100.0, 50.0], z_index: 0, parent: None, ports: vec![make_port_record("in", PortRole::Target, Cardinality::Many)], flags: NodeFlags::default(), data: serde_json::Value::Null, ui: NodeUiState::default() };
        doc.graph.nodes.insert(nid("n1"), n1);
        doc.graph.nodes.insert(nid("n2"), n2);
        doc.graph.edges.insert(eid("e1"), make_edge_with_ports("e1", "n1", "out", "n2", "in"));
        doc.graph.edges.insert(eid("e2"), make_edge_with_ports("e2", "n1", "out", "n2", "in"));
        assert!(SemanticValidator.validate(&doc).into_iter().any(|d| d.code.as_str() == "duplicate-edge"));
    }

    #[test]
    fn semantic_no_cycle() {
        assert!(!SemanticValidator.validate(&semantic_doc_two_nodes_connected()).into_iter().any(|d| d.code.as_str() == "graph-contains-cycle"));
    }

    #[test]
    fn semantic_cycle() {
        let mut doc = FlowDocument::default();
        let na = FlowNodeRecord { id: nid("a"), kind: SmolStr::from("test"), title: SmolStr::from("a"), position: [0.0, 0.0], size: [100.0, 50.0], z_index: 0, parent: None, ports: vec![make_port_record("out", PortRole::Source, Cardinality::One), make_port_record("in", PortRole::Target, Cardinality::One)], flags: NodeFlags::default(), data: serde_json::Value::Null, ui: NodeUiState::default() };
        let nb = FlowNodeRecord { id: nid("b"), kind: SmolStr::from("test"), title: SmolStr::from("b"), position: [200.0, 0.0], size: [100.0, 50.0], z_index: 0, parent: None, ports: vec![make_port_record("out", PortRole::Source, Cardinality::One), make_port_record("in", PortRole::Target, Cardinality::One)], flags: NodeFlags::default(), data: serde_json::Value::Null, ui: NodeUiState::default() };
        doc.graph.nodes.insert(nid("a"), na);
        doc.graph.nodes.insert(nid("b"), nb);
        doc.graph.edges.insert(eid("e1"), make_edge_with_ports("e1", "a", "out", "b", "in"));
        doc.graph.edges.insert(eid("e2"), make_edge_with_ports("e2", "b", "out", "a", "in"));
        assert!(SemanticValidator.validate(&doc).into_iter().any(|d| d.code.as_str() == "graph-contains-cycle"));
    }

    #[test]
    fn semantic_overlapping_groups() {
        let mut doc = FlowDocument::default();
        doc.graph.groups.insert(gid("g1"), FlowGroupRecord { id: gid("g1"), kind: GroupKind::Generic, title: SmolStr::from("g1"), bounds: [0.0, 0.0, 200.0, 200.0], data: serde_json::Value::Null });
        doc.graph.groups.insert(gid("g2"), FlowGroupRecord { id: gid("g2"), kind: GroupKind::Generic, title: SmolStr::from("g2"), bounds: [100.0, 100.0, 200.0, 200.0], data: serde_json::Value::Null });
        assert!(SemanticValidator.validate(&doc).into_iter().any(|d| d.code.as_str() == "overlapping-groups"));
    }

    #[test]
    fn semantic_non_overlapping_groups() {
        let mut doc = FlowDocument::default();
        doc.graph.groups.insert(gid("g1"), FlowGroupRecord { id: gid("g1"), kind: GroupKind::Generic, title: SmolStr::from("g1"), bounds: [0.0, 0.0, 100.0, 100.0], data: serde_json::Value::Null });
        doc.graph.groups.insert(gid("g2"), FlowGroupRecord { id: gid("g2"), kind: GroupKind::Generic, title: SmolStr::from("g2"), bounds: [200.0, 200.0, 100.0, 100.0], data: serde_json::Value::Null });
        assert!(!SemanticValidator.validate(&doc).into_iter().any(|d| d.code.as_str() == "overlapping-groups"));
    }

    #[test]
    fn semantic_trait_object() {
        let v: Box<dyn FlowValidator> = Box::new(SemanticValidator);
        assert!(v.validate(&FlowDocument::default()).is_empty());
    }

    // -- Export tests --

    #[test]
    fn export_empty_doc() { assert!(ExportValidator.validate(&FlowDocument::default()).is_empty()); }

    #[test]
    fn export_empty_kind() {
        let mut doc = FlowDocument::default();
        let mut node = make_node_with_ports("n1", &[]);
        node.kind = SmolStr::new("");
        doc.graph.nodes.insert(nid("n1"), node);
        assert!(ExportValidator.validate(&doc).into_iter().any(|d| d.code.as_str() == "export-node-kind-empty"));
    }

    #[test]
    fn export_empty_title() {
        let mut doc = FlowDocument::default();
        let mut node = make_node_with_ports("n1", &[]);
        node.title = SmolStr::new("");
        doc.graph.nodes.insert(nid("n1"), node);
        assert!(ExportValidator.validate(&doc).into_iter().any(|d| d.code.as_str() == "export-node-title-empty"));
    }

    #[test]
    fn export_inf_position() {
        let mut doc = FlowDocument::default();
        let mut node = make_node_with_ports("n1", &[]);
        node.position = [f64::INFINITY, 0.0];
        doc.graph.nodes.insert(nid("n1"), node);
        assert!(ExportValidator.validate(&doc).into_iter().any(|d| d.code.as_str() == "export-node-position-invalid"));
    }

    #[test]
    fn export_nan_position() {
        let mut doc = FlowDocument::default();
        let mut node = make_node_with_ports("n1", &[]);
        node.position = [f64::NAN, 0.0];
        doc.graph.nodes.insert(nid("n1"), node);
        assert!(ExportValidator.validate(&doc).into_iter().any(|d| d.code.as_str() == "export-node-position-invalid"));
    }

    #[test]
    fn export_duplicate_titles() {
        let mut doc = FlowDocument::default();
        doc.graph.nodes.insert(nid("n1"), FlowNodeRecord { id: nid("n1"), kind: SmolStr::from("test"), title: SmolStr::from("same"), position: [0.0, 0.0], size: [100.0, 50.0], z_index: 0, parent: None, ports: vec![], flags: NodeFlags::default(), data: serde_json::Value::Null, ui: NodeUiState::default() });
        doc.graph.nodes.insert(nid("n2"), FlowNodeRecord { id: nid("n2"), kind: SmolStr::from("test"), title: SmolStr::from("same"), position: [200.0, 0.0], size: [100.0, 50.0], z_index: 0, parent: None, ports: vec![], flags: NodeFlags::default(), data: serde_json::Value::Null, ui: NodeUiState::default() });
        assert!(ExportValidator.validate(&doc).into_iter().any(|d| d.code.as_str() == "export-duplicate-node-title"));
    }

    #[test]
    fn export_unique_titles() {
        let mut doc = FlowDocument::default();
        doc.graph.nodes.insert(nid("n1"), FlowNodeRecord { id: nid("n1"), kind: SmolStr::from("test"), title: SmolStr::from("alpha"), position: [0.0, 0.0], size: [100.0, 50.0], z_index: 0, parent: None, ports: vec![], flags: NodeFlags::default(), data: serde_json::Value::Null, ui: NodeUiState::default() });
        doc.graph.nodes.insert(nid("n2"), FlowNodeRecord { id: nid("n2"), kind: SmolStr::from("test"), title: SmolStr::from("beta"), position: [200.0, 0.0], size: [100.0, 50.0], z_index: 0, parent: None, ports: vec![], flags: NodeFlags::default(), data: serde_json::Value::Null, ui: NodeUiState::default() });
        assert!(!ExportValidator.validate(&doc).into_iter().any(|d| d.code.as_str() == "export-duplicate-node-title"));
    }

    #[test]
    fn export_bounds_exceeded() {
        let mut doc = FlowDocument::default();
        doc.graph.nodes.insert(nid("n1"), FlowNodeRecord { id: nid("n1"), kind: SmolStr::from("test"), title: SmolStr::from("n1"), position: [0.0, 0.0], size: [100.0, 50.0], z_index: 0, parent: None, ports: vec![], flags: NodeFlags::default(), data: serde_json::Value::Null, ui: NodeUiState::default() });
        doc.graph.nodes.insert(nid("n2"), FlowNodeRecord { id: nid("n2"), kind: SmolStr::from("test"), title: SmolStr::from("n2"), position: [20000.0, 0.0], size: [100.0, 50.0], z_index: 0, parent: None, ports: vec![], flags: NodeFlags::default(), data: serde_json::Value::Null, ui: NodeUiState::default() });
        assert!(ExportValidator.validate(&doc).into_iter().any(|d| d.code.as_str() == "export-graph-bounds-exceeded"));
    }

    #[test]
    fn export_bounds_ok() {
        let mut doc = FlowDocument::default();
        doc.graph.nodes.insert(nid("n1"), FlowNodeRecord { id: nid("n1"), kind: SmolStr::from("test"), title: SmolStr::from("n1"), position: [0.0, 0.0], size: [100.0, 50.0], z_index: 0, parent: None, ports: vec![], flags: NodeFlags::default(), data: serde_json::Value::Null, ui: NodeUiState::default() });
        doc.graph.nodes.insert(nid("n2"), FlowNodeRecord { id: nid("n2"), kind: SmolStr::from("test"), title: SmolStr::from("n2"), position: [500.0, 0.0], size: [100.0, 50.0], z_index: 0, parent: None, ports: vec![], flags: NodeFlags::default(), data: serde_json::Value::Null, ui: NodeUiState::default() });
        assert!(!ExportValidator.validate(&doc).into_iter().any(|d| d.code.as_str() == "export-graph-bounds-exceeded"));
    }

    #[test]
    fn export_overlapping_nodes() {
        let mut doc = FlowDocument::default();
        doc.graph.nodes.insert(nid("n1"), FlowNodeRecord { id: nid("n1"), kind: SmolStr::from("test"), title: SmolStr::from("n1"), position: [100.0, 100.0], size: [100.0, 50.0], z_index: 0, parent: None, ports: vec![], flags: NodeFlags::default(), data: serde_json::Value::Null, ui: NodeUiState::default() });
        doc.graph.nodes.insert(nid("n2"), FlowNodeRecord { id: nid("n2"), kind: SmolStr::from("test"), title: SmolStr::from("n2"), position: [100.0, 100.0], size: [100.0, 50.0], z_index: 0, parent: None, ports: vec![], flags: NodeFlags::default(), data: serde_json::Value::Null, ui: NodeUiState::default() });
        assert!(ExportValidator.validate(&doc).into_iter().any(|d| d.code.as_str() == "export-overlapping-nodes"));
    }

    #[test]
    fn export_unconnected_port() {
        let mut doc = FlowDocument::default();
        doc.graph.nodes.insert(nid("n1"), FlowNodeRecord { id: nid("n1"), kind: SmolStr::from("test"), title: SmolStr::from("n1"), position: [0.0, 0.0], size: [100.0, 50.0], z_index: 0, parent: None, ports: vec![make_port_record("out", PortRole::Source, Cardinality::One)], flags: NodeFlags::default(), data: serde_json::Value::Null, ui: NodeUiState::default() });
        assert!(ExportValidator.validate(&doc).into_iter().any(|d| d.code.as_str() == "export-port-unconnected"));
    }

    #[test]
    fn export_entry_unconnected_ok() {
        let mut doc = FlowDocument::default();
        doc.graph.nodes.insert(nid("n1"), FlowNodeRecord { id: nid("n1"), kind: SmolStr::from("entry"), title: SmolStr::from("entry"), position: [0.0, 0.0], size: [100.0, 50.0], z_index: 0, parent: None, ports: vec![make_port_record("out", PortRole::Source, Cardinality::One)], flags: NodeFlags { entry: true, ..NodeFlags::default() }, data: serde_json::Value::Null, ui: NodeUiState::default() });
        assert!(!ExportValidator.validate(&doc).into_iter().any(|d| d.code.as_str() == "export-port-unconnected"));
    }

    #[test]
    fn export_terminal_unconnected_ok() {
        let mut doc = FlowDocument::default();
        doc.graph.nodes.insert(nid("n1"), FlowNodeRecord { id: nid("n1"), kind: SmolStr::from("exit"), title: SmolStr::from("exit"), position: [0.0, 0.0], size: [100.0, 50.0], z_index: 0, parent: None, ports: vec![make_port_record("in", PortRole::Target, Cardinality::One)], flags: NodeFlags { terminal: true, ..NodeFlags::default() }, data: serde_json::Value::Null, ui: NodeUiState::default() });
        assert!(!ExportValidator.validate(&doc).into_iter().any(|d| d.code.as_str() == "export-port-unconnected"));
    }

    #[test]
    fn export_edge_empty_source() {
        let mut doc = FlowDocument::default();
        let mut edge = make_edge_with_ports("e1", "n1", "out", "n2", "in");
        edge.source_node = SmolStr::new("");
        doc.graph.edges.insert(eid("e1"), edge);
        assert!(ExportValidator.validate(&doc).into_iter().any(|d| d.code.as_str() == "export-edge-source-node-empty"));
    }

    #[test]
    fn export_edge_empty_target() {
        let mut doc = FlowDocument::default();
        let mut edge = make_edge_with_ports("e1", "n1", "out", "n2", "in");
        edge.target_node = SmolStr::new("");
        doc.graph.edges.insert(eid("e1"), edge);
        assert!(ExportValidator.validate(&doc).into_iter().any(|d| d.code.as_str() == "export-edge-target-node-empty"));
    }

    #[test]
    fn export_empty_title_no_dup_check() {
        let mut doc = FlowDocument::default();
        doc.graph.nodes.insert(nid("n1"), FlowNodeRecord { id: nid("n1"), kind: SmolStr::from("test"), title: SmolStr::new(""), position: [0.0, 0.0], size: [100.0, 50.0], z_index: 0, parent: None, ports: vec![], flags: NodeFlags::default(), data: serde_json::Value::Null, ui: NodeUiState::default() });
        doc.graph.nodes.insert(nid("n2"), FlowNodeRecord { id: nid("n2"), kind: SmolStr::from("test"), title: SmolStr::new(""), position: [200.0, 0.0], size: [100.0, 50.0], z_index: 0, parent: None, ports: vec![], flags: NodeFlags::default(), data: serde_json::Value::Null, ui: NodeUiState::default() });
        assert!(!ExportValidator.validate(&doc).into_iter().any(|d| d.code.as_str() == "export-duplicate-node-title"));
    }

    #[test]
    fn export_trait_object() {
        let v: Box<dyn FlowValidator> = Box::new(ExportValidator);
        assert!(v.validate(&FlowDocument::default()).is_empty());
    }

    // -- ValidationLevel / ValidationFinding --

    #[test]
    fn validation_level_distinct() { assert_ne!(ValidationLevel::Error, ValidationLevel::Warning); }

    #[test]
    fn validation_finding_new() {
        let f = ValidationFinding::new(ValidationLevel::Error, "test", String::from("msg"), Some(nid("n1")), Some(eid("e1")));
        assert_eq!(f.level, ValidationLevel::Error);
        assert_eq!(f.code.as_str(), "test");
    }

    #[test]
    fn validation_finding_clone() {
        let f = ValidationFinding::new(ValidationLevel::Warning, "c", String::from("m"), Some(nid("n1")), None);
        let c = f.clone();
        assert_eq!(c.level, f.level);
    }

    // -- Pipeline tests --

    #[test]
    fn pipeline_new_empty() { assert!(ValidationPipeline::new().validate(&FlowDocument::default()).is_empty()); }

    #[test]
    fn pipeline_default_empty() { assert!(ValidationPipeline::default().validate(&FlowDocument::default()).is_empty()); }

    #[test]
    fn pipeline_standard_empty_doc() { assert!(ValidationPipeline::standard().validate(&FlowDocument::default()).is_empty()); }

    #[test]
    fn pipeline_standard_catches_all_phases() {
        let mut doc = FlowDocument::default();
        doc.graph.entry_node = Some(nid("ghost"));
        let mut node = make_node_with_ports("n1", &[]);
        node.kind = SmolStr::new("");
        doc.graph.nodes.insert(nid("n1"), node);
        let diags = ValidationPipeline::standard().validate(&doc);
        assert!(diags.iter().any(|d| d.code.as_str() == "entry-node-missing"));
        assert!(diags.iter().any(|d| d.code.as_str() == "export-node-kind-empty"));
    }

    #[test]
    fn pipeline_run_findings() {
        let mut doc = FlowDocument::default();
        doc.graph.entry_node = Some(nid("ghost"));
        let findings = ValidationPipeline::run(&doc);
        assert!(!findings.is_empty());
    }

    #[test]
    fn pipeline_run_empty() { assert!(ValidationPipeline::run(&FlowDocument::default()).is_empty()); }

    #[test]
    fn pipeline_run_valid() {
        assert!(ValidationPipeline::run(&semantic_doc_two_nodes_connected()).is_empty());
    }

    #[test]
    fn pipeline_add_custom() {
        struct AlwaysError;
        impl FlowValidator for AlwaysError {
            fn validate(&self, _doc: &FlowDocument) -> Vec<Diagnostic> {
                vec![Diagnostic { severity: DiagnosticSeverity::Error, code: SmolStr::from("always"), message: String::from("fail"), node: None, edge: None }]
            }
        }
        let mut p = ValidationPipeline::new();
        p.add_validator(Box::new(AlwaysError));
        assert_eq!(p.validate(&FlowDocument::default()).len(), 1);
    }

    #[test]
    fn pipeline_run_error_level() {
        let mut doc = FlowDocument::default();
        doc.graph.entry_node = Some(nid("ghost"));
        let findings = ValidationPipeline::run(&doc);
        assert!(findings.iter().find(|f| f.code.as_str() == "entry-node-missing").is_some_and(|f| f.level == ValidationLevel::Error));
    }

    #[test]
    fn pipeline_run_warning_level() {
        let mut doc = FlowDocument::default();
        doc.graph.nodes.insert(nid("n1"), FlowNodeRecord { id: nid("n1"), kind: SmolStr::from("test"), title: SmolStr::from("n1"), position: [100.0, 100.0], size: [100.0, 50.0], z_index: 0, parent: None, ports: vec![], flags: NodeFlags::default(), data: serde_json::Value::Null, ui: NodeUiState::default() });
        doc.graph.nodes.insert(nid("n2"), FlowNodeRecord { id: nid("n2"), kind: SmolStr::from("test"), title: SmolStr::from("n2"), position: [100.0, 100.0], size: [100.0, 50.0], z_index: 0, parent: None, ports: vec![], flags: NodeFlags::default(), data: serde_json::Value::Null, ui: NodeUiState::default() });
        let findings = ValidationPipeline::run(&doc);
        assert!(findings.iter().find(|f| f.code.as_str() == "export-overlapping-nodes").is_some_and(|f| f.level == ValidationLevel::Warning));
    }

    #[test]
    fn pipeline_accumulate() {
        struct Err1;
        impl FlowValidator for Err1 {
            fn validate(&self, _: &FlowDocument) -> Vec<Diagnostic> {
                vec![Diagnostic { severity: DiagnosticSeverity::Error, code: SmolStr::from("a"), message: String::from("1"), node: None, edge: None }]
            }
        }
        struct Err2;
        impl FlowValidator for Err2 {
            fn validate(&self, _: &FlowDocument) -> Vec<Diagnostic> {
                vec![Diagnostic { severity: DiagnosticSeverity::Warning, code: SmolStr::from("b"), message: String::from("2"), node: None, edge: None }]
            }
        }
        let mut p = ValidationPipeline::new();
        p.add_validator(Box::new(Err1));
        p.add_validator(Box::new(Err2));
        assert_eq!(p.validate(&FlowDocument::default()).len(), 2);
    }

    // =========================================================================
    // INTEGRATION TESTS: ValidationPipeline combining all three validators
    // with complex multi-issue documents
    // =========================================================================

    // Helper: create a well-formed node with one Source port and one Target port
    fn make_full_node(id: &str, pos: [f64; 2]) -> FlowNodeRecord {
        FlowNodeRecord {
            id: nid(id),
            kind: SmolStr::from("processor"),
            title: SmolStr::from(id),
            position: pos,
            size: [100.0, 50.0],
            z_index: 0,
            parent: None,
            ports: vec![
                FlowPortRecord {
                    id: pid(&format!("{id}-in")),
                    side: PortSide::Left,
                    role: PortRole::Target,
                    label: SmolStr::from("in"),
                    order: 0,
                    cardinality: Cardinality::One,
                    data_type: Some(SmolStr::from("data")),
                },
                FlowPortRecord {
                    id: pid(&format!("{id}-out")),
                    side: PortSide::Right,
                    role: PortRole::Source,
                    label: SmolStr::from("out"),
                    order: 1,
                    cardinality: Cardinality::Many,
                    data_type: Some(SmolStr::from("data")),
                },
            ],
            flags: NodeFlags::default(),
            data: serde_json::Value::Null,
            ui: NodeUiState::default(),
        }
    }

    // Helper: connect two nodes via their ports
    fn connect(edge_id: &str, src: &str, tgt: &str) -> FlowEdgeRecord {
        FlowEdgeRecord {
            id: eid(edge_id),
            source_node: nid(src),
            source_port: pid(&format!("{src}-out")),
            target_node: nid(tgt),
            target_port: pid(&format!("{tgt}-in")),
            label: None,
            style: EdgeStyle::default(),
            data: serde_json::Value::Null,
            ui: EdgeUiState::default(),
        }
    }

    // ---- Integration: fully valid pipeline produces no diagnostics ----

    #[test]
    fn integration_pipeline_valid_diamond_dag() {
        let mut doc = FlowDocument::default();
        // a -> b, a -> c, b -> d, c -> d (diamond DAG)
        // a is entry (only out-port), d is terminal (only in-port)
        // b and c are middle nodes with both ports

        // Node a: entry, only output port
        let a = FlowNodeRecord {
            id: nid("a"),
            kind: SmolStr::from("processor"),
            title: SmolStr::from("a"),
            position: [0.0, 0.0],
            size: [100.0, 50.0],
            z_index: 0,
            parent: None,
            ports: vec![FlowPortRecord {
                id: pid("a-out"),
                side: PortSide::Right,
                role: PortRole::Source,
                label: SmolStr::from("out"),
                order: 0,
                cardinality: Cardinality::Many,
                data_type: Some(SmolStr::from("data")),
            }],
            flags: NodeFlags { entry: true, ..NodeFlags::default() },
            data: serde_json::Value::Null,
            ui: NodeUiState::default(),
        };
        doc.graph.nodes.insert(nid("a"), a);
        doc.graph.nodes.insert(nid("b"), make_full_node("b", [200.0, -100.0]));
        doc.graph.nodes.insert(nid("c"), make_full_node("c", [200.0, 100.0]));

        // Node d: terminal, only input port
        let d = FlowNodeRecord {
            id: nid("d"),
            kind: SmolStr::from("processor"),
            title: SmolStr::from("d"),
            position: [400.0, 0.0],
            size: [100.0, 50.0],
            z_index: 0,
            parent: None,
            ports: vec![FlowPortRecord {
                id: pid("d-in"),
                side: PortSide::Left,
                role: PortRole::Target,
                label: SmolStr::from("in"),
                order: 0,
                cardinality: Cardinality::One,
                data_type: Some(SmolStr::from("data")),
            }],
            flags: NodeFlags { terminal: true, ..NodeFlags::default() },
            data: serde_json::Value::Null,
            ui: NodeUiState::default(),
        };
        doc.graph.nodes.insert(nid("d"), d);

        doc.graph.edges.insert(eid("e1"), connect("e1", "a", "b"));
        doc.graph.edges.insert(eid("e2"), connect("e2", "a", "c"));
        doc.graph.edges.insert(eid("e3"), connect("e3", "b", "d"));
        doc.graph.edges.insert(eid("e4"), connect("e4", "c", "d"));
        doc.graph.entry_node = Some(nid("a"));

        let diags = ValidationPipeline::standard().validate(&doc);
        assert!(diags.is_empty(), "expected no diagnostics for valid diamond DAG, got: {diags:?}");
    }

    #[test]
    fn integration_pipeline_valid_linear_chain() {
        let mut doc = FlowDocument::default();
        let count: usize = 10;
        // First node: entry, only output port
        let mut first = FlowNodeRecord {
            id: nid("n0"),
            kind: SmolStr::from("processor"),
            title: SmolStr::from("n0"),
            position: [0.0, 0.0],
            size: [100.0, 50.0],
            z_index: 0,
            parent: None,
            ports: vec![FlowPortRecord {
                id: pid("n0-out"),
                side: PortSide::Right,
                role: PortRole::Source,
                label: SmolStr::from("out"),
                order: 0,
                cardinality: Cardinality::Many,
                data_type: Some(SmolStr::from("data")),
            }],
            flags: NodeFlags { entry: true, ..NodeFlags::default() },
            data: serde_json::Value::Null,
            ui: NodeUiState::default(),
        };
        first.flags.entry = true;
        doc.graph.nodes.insert(nid("n0"), first);

        // Middle nodes
        for i in 1..count.saturating_sub(1) {
            let pos = [f64::from(u32::try_from(i).unwrap_or(u32::MAX)).mul_add(150.0, 0.0), 0.0];
            doc.graph.nodes.insert(
                nid(&format!("n{i}")),
                make_full_node(&format!("n{i}"), pos),
            );
        }

        // Last node: terminal, only input port
        let last_idx = count.saturating_sub(1);
        let last_pos = [f64::from(u32::try_from(last_idx).unwrap_or(u32::MAX)).mul_add(150.0, 0.0), 0.0];
        let last = FlowNodeRecord {
            id: nid(&format!("n{last_idx}")),
            kind: SmolStr::from("processor"),
            title: SmolStr::from(format!("n{last_idx}")),
            position: last_pos,
            size: [100.0, 50.0],
            z_index: 0,
            parent: None,
            ports: vec![FlowPortRecord {
                id: pid(&format!("n{last_idx}-in")),
                side: PortSide::Left,
                role: PortRole::Target,
                label: SmolStr::from("in"),
                order: 0,
                cardinality: Cardinality::One,
                data_type: Some(SmolStr::from("data")),
            }],
            flags: NodeFlags { terminal: true, ..NodeFlags::default() },
            data: serde_json::Value::Null,
            ui: NodeUiState::default(),
        };
        doc.graph.nodes.insert(nid(&format!("n{last_idx}")), last);

        for i in 0..count.saturating_sub(1) {
            let next = i.saturating_add(1);
            doc.graph.edges.insert(
                eid(&format!("e{i}")),
                connect(&format!("e{i}"), &format!("n{i}"), &format!("n{next}")),
            );
        }
        doc.graph.entry_node = Some(nid("n0"));

        let diags = ValidationPipeline::standard().validate(&doc);
        assert!(diags.is_empty(), "expected no diagnostics for valid chain, got: {diags:?}");
    }

    // ---- Integration: multi-issue document triggers all three phases ----

    #[test]
    fn integration_multi_issue_document_triggers_all_phases() {
        let mut doc = FlowDocument::default();

        // Structural issue: entry_node references nonexistent node
        doc.graph.entry_node = Some(nid("nonexistent-entry"));

        // Structural issue: edge references nonexistent source
        doc.graph.edges.insert(
            eid("bad-edge"),
            FlowEdgeRecord {
                id: eid("bad-edge"),
                source_node: nid("ghost-src"),
                source_port: pid("out"),
                target_node: nid("ghost-tgt"),
                target_port: pid("in"),
                label: None,
                style: EdgeStyle::default(),
                data: serde_json::Value::Null,
                ui: EdgeUiState::default(),
            },
        );

        // Export issue: node with empty kind and empty title
        let mut bad_node = make_full_node("bad-node", [0.0, 0.0]);
        bad_node.kind = SmolStr::new("");
        bad_node.title = SmolStr::new("");
        doc.graph.nodes.insert(nid("bad-node"), bad_node);

        // Export issue: overlapping nodes
        doc.graph.nodes.insert(
            nid("overlap1"),
            FlowNodeRecord {
                id: nid("overlap1"),
                kind: SmolStr::from("test"),
                title: SmolStr::from("overlap1"),
                position: [50.0, 50.0],
                size: [100.0, 50.0],
                z_index: 0,
                parent: None,
                ports: vec![],
                flags: NodeFlags::default(),
                data: serde_json::Value::Null,
                ui: NodeUiState::default(),
            },
        );
        doc.graph.nodes.insert(
            nid("overlap2"),
            FlowNodeRecord {
                id: nid("overlap2"),
                kind: SmolStr::from("test"),
                title: SmolStr::from("overlap2"),
                position: [50.0, 50.0],
                size: [100.0, 50.0],
                z_index: 0,
                parent: None,
                ports: vec![],
                flags: NodeFlags::default(),
                data: serde_json::Value::Null,
                ui: NodeUiState::default(),
            },
        );

        let diags = ValidationPipeline::standard().validate(&doc);
        let codes: Vec<&str> = diags.iter().map(|d| d.code.as_str()).collect();

        // Structural phase catches
        assert!(codes.contains(&"entry-node-missing"), "missing entry-node-missing in {codes:?}");
        assert!(codes.contains(&"edge-source-missing"), "missing edge-source-missing in {codes:?}");
        assert!(codes.contains(&"edge-target-missing"), "missing edge-target-missing in {codes:?}");

        // Export phase catches
        assert!(codes.contains(&"export-node-kind-empty"), "missing export-node-kind-empty in {codes:?}");
        assert!(codes.contains(&"export-node-title-empty"), "missing export-node-title-empty in {codes:?}");
        assert!(codes.contains(&"export-overlapping-nodes"), "missing export-overlapping-nodes in {codes:?}");
    }

    #[test]
    fn integration_semantic_issues_with_pipeline() {
        let mut doc = FlowDocument::default();

        // Two nodes with an edge using wrong port role
        let n1 = FlowNodeRecord {
            id: nid("n1"),
            kind: SmolStr::from("test"),
            title: SmolStr::from("n1"),
            position: [0.0, 0.0],
            size: [100.0, 50.0],
            z_index: 0,
            parent: None,
            ports: vec![make_port_record("bad-out", PortRole::Target, Cardinality::One)],
            flags: NodeFlags::default(),
            data: serde_json::Value::Null,
            ui: NodeUiState::default(),
        };
        let n2 = FlowNodeRecord {
            id: nid("n2"),
            kind: SmolStr::from("test"),
            title: SmolStr::from("n2"),
            position: [200.0, 0.0],
            size: [100.0, 50.0],
            z_index: 0,
            parent: None,
            ports: vec![make_port_record("bad-in", PortRole::Source, Cardinality::One)],
            flags: NodeFlags::default(),
            data: serde_json::Value::Null,
            ui: NodeUiState::default(),
        };
        doc.graph.nodes.insert(nid("n1"), n1);
        doc.graph.nodes.insert(nid("n2"), n2);
        doc.graph.edges.insert(
            eid("e1"),
            make_edge_with_ports("e1", "n1", "bad-out", "n2", "bad-in"),
        );

        // Add a cycle for the cycle checker
        let n3 = FlowNodeRecord {
            id: nid("n3"),
            kind: SmolStr::from("test"),
            title: SmolStr::from("n3"),
            position: [400.0, 0.0],
            size: [100.0, 50.0],
            z_index: 0,
            parent: None,
            ports: vec![
                make_port_record("in3", PortRole::Target, Cardinality::One),
                make_port_record("out3", PortRole::Source, Cardinality::One),
            ],
            flags: NodeFlags::default(),
            data: serde_json::Value::Null,
            ui: NodeUiState::default(),
        };
        let n4 = FlowNodeRecord {
            id: nid("n4"),
            kind: SmolStr::from("test"),
            title: SmolStr::from("n4"),
            position: [600.0, 0.0],
            size: [100.0, 50.0],
            z_index: 0,
            parent: None,
            ports: vec![
                make_port_record("in4", PortRole::Target, Cardinality::One),
                make_port_record("out4", PortRole::Source, Cardinality::One),
            ],
            flags: NodeFlags::default(),
            data: serde_json::Value::Null,
            ui: NodeUiState::default(),
        };
        doc.graph.nodes.insert(nid("n3"), n3);
        doc.graph.nodes.insert(nid("n4"), n4);
        doc.graph.edges.insert(eid("e2"), make_edge_with_ports("e2", "n3", "out3", "n4", "in4"));
        doc.graph.edges.insert(eid("e3"), make_edge_with_ports("e3", "n4", "out4", "n3", "in3"));

        let diags = ValidationPipeline::standard().validate(&doc);
        let codes: Vec<&str> = diags.iter().map(|d| d.code.as_str()).collect();

        assert!(codes.contains(&"edge-source-port-role-mismatch"), "missing edge-source-port-role-mismatch in {codes:?}");
        assert!(codes.contains(&"edge-target-port-role-mismatch"), "missing edge-target-port-role-mismatch in {codes:?}");
        assert!(codes.contains(&"graph-contains-cycle"), "missing graph-contains-cycle in {codes:?}");
    }

    // ---- Integration: orphan nodes + empty group + degenerate group ----

    #[test]
    fn integration_semantic_orphans_and_groups() {
        let mut doc = FlowDocument::default();

        // Orphan node (not entry, not terminal, no edges)
        doc.graph.nodes.insert(
            nid("orphan"),
            FlowNodeRecord {
                id: nid("orphan"),
                kind: SmolStr::from("test"),
                title: SmolStr::from("orphan"),
                position: [0.0, 0.0],
                size: [100.0, 50.0],
                z_index: 0,
                parent: None,
                ports: vec![make_port_record("p", PortRole::Source, Cardinality::One)],
                flags: NodeFlags::default(),
                data: serde_json::Value::Null,
                ui: NodeUiState::default(),
            },
        );

        // Empty group
        doc.graph.groups.insert(
            gid("empty-group"),
            FlowGroupRecord {
                id: gid("empty-group"),
                kind: GroupKind::Generic,
                title: SmolStr::from("empty-group"),
                bounds: [0.0, 0.0, 200.0, 200.0],
                data: serde_json::Value::Null,
            },
        );

        // Degenerate group (zero width)
        doc.graph.groups.insert(
            gid("degen-group"),
            FlowGroupRecord {
                id: gid("degen-group"),
                kind: GroupKind::Generic,
                title: SmolStr::from("degen-group"),
                bounds: [0.0, 0.0, 0.0, 100.0],
                data: serde_json::Value::Null,
            },
        );

        // Overlapping groups
        doc.graph.groups.insert(
            gid("overlap-a"),
            FlowGroupRecord {
                id: gid("overlap-a"),
                kind: GroupKind::Generic,
                title: SmolStr::from("overlap-a"),
                bounds: [0.0, 0.0, 200.0, 200.0],
                data: serde_json::Value::Null,
            },
        );
        doc.graph.groups.insert(
            gid("overlap-b"),
            FlowGroupRecord {
                id: gid("overlap-b"),
                kind: GroupKind::Generic,
                title: SmolStr::from("overlap-b"),
                bounds: [100.0, 100.0, 200.0, 200.0],
                data: serde_json::Value::Null,
            },
        );

        let diags = ValidationPipeline::standard().validate(&doc);
        let codes: Vec<&str> = diags.iter().map(|d| d.code.as_str()).collect();

        assert!(codes.contains(&"orphan-node"), "missing orphan-node in {codes:?}");
        assert!(codes.contains(&"group-empty"), "missing group-empty in {codes:?}");
        assert!(codes.contains(&"group-degenerate-bounds"), "missing group-degenerate-bounds in {codes:?}");
        assert!(codes.contains(&"overlapping-groups"), "missing overlapping-groups in {codes:?}");
    }

    // ---- Integration: export issues -- duplicate titles, bounds exceeded, unconnected ports ----

    #[test]
    fn integration_export_issues_combined() {
        let mut doc = FlowDocument::default();

        // Two nodes with same title at different positions (export: duplicate titles)
        doc.graph.nodes.insert(
            nid("n1"),
            FlowNodeRecord {
                id: nid("n1"),
                kind: SmolStr::from("test"),
                title: SmolStr::from("same-title"),
                position: [0.0, 0.0],
                size: [100.0, 50.0],
                z_index: 0,
                parent: None,
                ports: vec![make_port_record("out", PortRole::Source, Cardinality::One)],
                flags: NodeFlags::default(),
                data: serde_json::Value::Null,
                ui: NodeUiState::default(),
            },
        );
        doc.graph.nodes.insert(
            nid("n2"),
            FlowNodeRecord {
                id: nid("n2"),
                kind: SmolStr::from("test"),
                title: SmolStr::from("same-title"),
                position: [20000.0, 0.0],
                size: [100.0, 50.0],
                z_index: 0,
                parent: None,
                ports: vec![make_port_record("in", PortRole::Target, Cardinality::One)],
                flags: NodeFlags::default(),
                data: serde_json::Value::Null,
                ui: NodeUiState::default(),
            },
        );

        let diags = ValidationPipeline::standard().validate(&doc);
        let codes: Vec<&str> = diags.iter().map(|d| d.code.as_str()).collect();

        assert!(codes.contains(&"export-duplicate-node-title"), "missing export-duplicate-node-title in {codes:?}");
        assert!(codes.contains(&"export-graph-bounds-exceeded"), "missing export-graph-bounds-exceeded in {codes:?}");
        assert!(codes.contains(&"export-port-unconnected"), "missing export-port-unconnected in {codes:?}");
    }

    // ---- Integration: cardinality + duplicate edges + parent group missing ----

    #[test]
    fn integration_cardinality_duplicate_parent_issues() {
        let mut doc = FlowDocument::default();

        // Node with Cardinality::One source port and nonexistent parent group
        let n1 = FlowNodeRecord {
            id: nid("n1"),
            kind: SmolStr::from("test"),
            title: SmolStr::from("n1"),
            position: [0.0, 0.0],
            size: [100.0, 50.0],
            z_index: 0,
            parent: Some(gid("nonexistent-group")),
            ports: vec![make_port_record("out", PortRole::Source, Cardinality::One)],
            flags: NodeFlags::default(),
            data: serde_json::Value::Null,
            ui: NodeUiState::default(),
        };
        let n2 = FlowNodeRecord {
            id: nid("n2"),
            kind: SmolStr::from("test"),
            title: SmolStr::from("n2"),
            position: [200.0, 0.0],
            size: [100.0, 50.0],
            z_index: 0,
            parent: None,
            ports: vec![make_port_record("in", PortRole::Target, Cardinality::Many)],
            flags: NodeFlags::default(),
            data: serde_json::Value::Null,
            ui: NodeUiState::default(),
        };
        let n3 = FlowNodeRecord {
            id: nid("n3"),
            kind: SmolStr::from("test"),
            title: SmolStr::from("n3"),
            position: [400.0, 0.0],
            size: [100.0, 50.0],
            z_index: 0,
            parent: None,
            ports: vec![make_port_record("in", PortRole::Target, Cardinality::Many)],
            flags: NodeFlags::default(),
            data: serde_json::Value::Null,
            ui: NodeUiState::default(),
        };
        doc.graph.nodes.insert(nid("n1"), n1);
        doc.graph.nodes.insert(nid("n2"), n2);
        doc.graph.nodes.insert(nid("n3"), n3);

        // Two edges from the same source port (Cardinality::One violation)
        doc.graph.edges.insert(eid("e1"), make_edge_with_ports("e1", "n1", "out", "n2", "in"));
        doc.graph.edges.insert(eid("e2"), make_edge_with_ports("e2", "n1", "out", "n3", "in"));

        // Also add a duplicate edge
        doc.graph.edges.insert(eid("e3"), make_edge_with_ports("e3", "n1", "out", "n2", "in"));

        let diags = ValidationPipeline::standard().validate(&doc);
        let codes: Vec<&str> = diags.iter().map(|d| d.code.as_str()).collect();

        assert!(codes.contains(&"node-parent-group-missing"), "missing node-parent-group-missing in {codes:?}");
        assert!(codes.contains(&"cardinality-one-multi-source"), "missing cardinality-one-multi-source in {codes:?}");
        assert!(codes.contains(&"duplicate-edge"), "missing duplicate-edge in {codes:?}");
    }

    // ---- Integration: export edge with empty node IDs ----

    #[test]
    fn integration_export_empty_edge_node_ids() {
        let mut doc = FlowDocument::default();
        let mut edge = make_edge_with_ports("e1", "", "out", "", "in");
        edge.source_node = SmolStr::new("");
        edge.target_node = SmolStr::new("");
        doc.graph.edges.insert(eid("e1"), edge);

        let diags = ValidationPipeline::standard().validate(&doc);
        let codes: Vec<&str> = diags.iter().map(|d| d.code.as_str()).collect();

        assert!(codes.contains(&"export-edge-source-node-empty"), "missing export-edge-source-node-empty in {codes:?}");
        assert!(codes.contains(&"export-edge-target-node-empty"), "missing export-edge-target-node-empty in {codes:?}");
        assert!(codes.contains(&"edge-source-missing"), "missing edge-source-missing in {codes:?}");
        assert!(codes.contains(&"edge-target-missing"), "missing edge-target-missing in {codes:?}");
    }

    // ---- Integration: self-loop on same port ----

    #[test]
    fn integration_self_loop_same_port_catches_all_phases() {
        let mut doc = FlowDocument::default();
        let node = FlowNodeRecord {
            id: nid("self"),
            kind: SmolStr::from("test"),
            title: SmolStr::from("self"),
            position: [0.0, 0.0],
            size: [100.0, 50.0],
            z_index: 0,
            parent: None,
            ports: vec![make_port_record("io", PortRole::Bidirectional, Cardinality::Many)],
            flags: NodeFlags::default(),
            data: serde_json::Value::Null,
            ui: NodeUiState::default(),
        };
        doc.graph.nodes.insert(nid("self"), node);
        doc.graph.edges.insert(eid("e1"), make_edge_with_ports("e1", "self", "io", "self", "io"));

        let diags = ValidationPipeline::standard().validate(&doc);
        let codes: Vec<&str> = diags.iter().map(|d| d.code.as_str()).collect();

        assert!(codes.contains(&"self-loop-same-port"), "missing self-loop-same-port in {codes:?}");
    }

    // ---- Integration: non-finite node positions ----

    #[test]
    fn integration_non_finite_positions() {
        let mut doc = FlowDocument::default();

        // Node with NaN position
        doc.graph.nodes.insert(
            nid("nan-node"),
            FlowNodeRecord {
                id: nid("nan-node"),
                kind: SmolStr::from("test"),
                title: SmolStr::from("nan-node"),
                position: [f64::NAN, f64::NAN],
                size: [100.0, 50.0],
                z_index: 0,
                parent: None,
                ports: vec![],
                flags: NodeFlags::default(),
                data: serde_json::Value::Null,
                ui: NodeUiState::default(),
            },
        );

        // Node with infinity position
        doc.graph.nodes.insert(
            nid("inf-node"),
            FlowNodeRecord {
                id: nid("inf-node"),
                kind: SmolStr::from("test"),
                title: SmolStr::from("inf-node"),
                position: [f64::INFINITY, f64::NEG_INFINITY],
                size: [100.0, 50.0],
                z_index: 0,
                parent: None,
                ports: vec![],
                flags: NodeFlags::default(),
                data: serde_json::Value::Null,
                ui: NodeUiState::default(),
            },
        );

        let diags = ValidationPipeline::standard().validate(&doc);
        let pos_codes: Vec<&str> = diags
            .iter()
            .filter(|d| d.code.as_str() == "export-node-position-invalid")
            .map(|d| d.code.as_str())
            .collect();
        // Both nodes should produce position-invalid errors
        assert!(pos_codes.len() >= 2, "expected at least 2 position-invalid diagnostics, got: {diags:?}");
    }

    // ---- Integration: Pipeline.run() produces correct ValidationFindings ----

    #[test]
    fn integration_run_produces_findings_with_correct_levels() {
        let mut doc = FlowDocument::default();
        // Mix of errors and warnings
        doc.graph.entry_node = Some(nid("ghost")); // Error (structural)
        doc.graph.nodes.insert(
            nid("n1"),
            FlowNodeRecord {
                id: nid("n1"),
                kind: SmolStr::from("test"),
                title: SmolStr::from("n1"),
                position: [0.0, 0.0],
                size: [100.0, 50.0],
                z_index: 0,
                parent: None,
                ports: vec![],
                flags: NodeFlags::default(),
                data: serde_json::Value::Null,
                ui: NodeUiState::default(),
            },
        );
        doc.graph.nodes.insert(
            nid("n2"),
            FlowNodeRecord {
                id: nid("n2"),
                kind: SmolStr::from("test"),
                title: SmolStr::from("n2"),
                position: [0.0, 0.0],
                size: [100.0, 50.0],
                z_index: 0,
                parent: None,
                ports: vec![],
                flags: NodeFlags::default(),
                data: serde_json::Value::Null,
                ui: NodeUiState::default(),
            },
        );

        let findings = ValidationPipeline::run(&doc);
        assert!(!findings.is_empty());

        // entry-node-missing should be Error level
        let entry_finding = findings.iter().find(|f| f.code.as_str() == "entry-node-missing");
        assert!(entry_finding.is_some_and(|f| f.level == ValidationLevel::Error));

        // orphan-node should be Warning level
        let orphan_findings: Vec<&ValidationFinding> = findings.iter().filter(|f| f.code.as_str() == "orphan-node").collect();
        assert!(!orphan_findings.is_empty());
        assert!(orphan_findings.iter().all(|f| f.level == ValidationLevel::Warning));

        // export-overlapping-nodes should be Warning
        let overlap_finding = findings.iter().find(|f| f.code.as_str() == "export-overlapping-nodes");
        assert!(overlap_finding.is_some_and(|f| f.level == ValidationLevel::Warning));
    }

    // ---- Integration: complex document with groups, edges, and multiple issues ----

    #[test]
    fn integration_complex_multi_issue_document() {
        let mut doc = FlowDocument::default();

        // Valid connected pair: n1 -> n2
        doc.graph.nodes.insert(nid("n1"), make_full_node("n1", [0.0, 0.0]));
        doc.graph.nodes.insert(nid("n2"), make_full_node("n2", [200.0, 0.0]));
        doc.graph.edges.insert(eid("e1"), connect("e1", "n1", "n2"));

        // Orphan node with nonexistent parent group
        let mut orphan = make_full_node("orphan", [400.0, 0.0]);
        orphan.parent = Some(gid("ghost-group"));
        doc.graph.nodes.insert(nid("orphan"), orphan);

        // Entry node references a nonexistent node
        doc.graph.entry_node = Some(nid("missing-entry"));

        // Degenerate group
        doc.graph.groups.insert(
            gid("degen"),
            FlowGroupRecord {
                id: gid("degen"),
                kind: GroupKind::Subflow,
                title: SmolStr::from("degen"),
                bounds: [0.0, 0.0, -10.0, 50.0],
                data: serde_json::Value::Null,
            },
        );

        // Empty group
        doc.graph.groups.insert(
            gid("empty"),
            FlowGroupRecord {
                id: gid("empty"),
                kind: GroupKind::Generic,
                title: SmolStr::from("empty"),
                bounds: [0.0, 0.0, 100.0, 100.0],
                data: serde_json::Value::Null,
            },
        );

        let diags = ValidationPipeline::standard().validate(&doc);
        let codes: Vec<&str> = diags.iter().map(|d| d.code.as_str()).collect();

        // Structural
        assert!(codes.contains(&"entry-node-missing"), "missing entry-node-missing");
        assert!(codes.contains(&"node-parent-group-missing"), "missing node-parent-group-missing");

        // Semantic
        assert!(codes.contains(&"orphan-node"), "missing orphan-node");
        assert!(codes.contains(&"group-degenerate-bounds"), "missing group-degenerate-bounds");
        assert!(codes.contains(&"group-empty"), "missing group-empty");

        // Export -- unconnected ports on orphan node
        assert!(codes.contains(&"export-port-unconnected"), "missing export-port-unconnected");
    }

    // ---- Integration: empty document through pipeline produces nothing ----

    #[test]
    fn integration_empty_document_all_validators_clean() {
        let doc = FlowDocument::default();
        let s = StructuralValidator.validate(&doc);
        let sem = SemanticValidator.validate(&doc);
        let exp = ExportValidator.validate(&doc);
        let pipe = ValidationPipeline::standard().validate(&doc);
        let findings = ValidationPipeline::run(&doc);

        assert!(s.is_empty(), "structural: {s:?}");
        assert!(sem.is_empty(), "semantic: {sem:?}");
        assert!(exp.is_empty(), "export: {exp:?}");
        assert!(pipe.is_empty(), "pipeline: {pipe:?}");
        assert!(findings.is_empty(), "findings: {findings:?}");
    }

    // ---- Integration: large valid document does not produce spurious warnings ----

    #[test]
    fn integration_large_valid_chain_no_false_positives() {
        let mut doc = FlowDocument::default();
        let count: usize = 30;

        // First node: entry, only output port
        let mut first = FlowNodeRecord {
            id: nid("n0"),
            kind: SmolStr::from("processor"),
            title: SmolStr::from("n0"),
            position: [0.0, 0.0],
            size: [100.0, 50.0],
            z_index: 0,
            parent: None,
            ports: vec![FlowPortRecord {
                id: pid("n0-out"),
                side: PortSide::Right,
                role: PortRole::Source,
                label: SmolStr::from("out"),
                order: 0,
                cardinality: Cardinality::Many,
                data_type: Some(SmolStr::from("data")),
            }],
            flags: NodeFlags { entry: true, ..NodeFlags::default() },
            data: serde_json::Value::Null,
            ui: NodeUiState::default(),
        };
        first.flags.entry = true;
        doc.graph.nodes.insert(nid("n0"), first);

        // Middle nodes
        for i in 1..count.saturating_sub(1) {
            let pos = [f64::from(u32::try_from(i).unwrap_or(u32::MAX)).mul_add(120.0, 0.0), 0.0];
            doc.graph.nodes.insert(
                nid(&format!("n{i}")),
                make_full_node(&format!("n{i}"), pos),
            );
        }

        // Last node: terminal, only input port
        let last_idx = count.saturating_sub(1);
        let last_pos = [f64::from(u32::try_from(last_idx).unwrap_or(u32::MAX)).mul_add(120.0, 0.0), 0.0];
        let last = FlowNodeRecord {
            id: nid(&format!("n{last_idx}")),
            kind: SmolStr::from("processor"),
            title: SmolStr::from(format!("n{last_idx}")),
            position: last_pos,
            size: [100.0, 50.0],
            z_index: 0,
            parent: None,
            ports: vec![FlowPortRecord {
                id: pid(&format!("n{last_idx}-in")),
                side: PortSide::Left,
                role: PortRole::Target,
                label: SmolStr::from("in"),
                order: 0,
                cardinality: Cardinality::One,
                data_type: Some(SmolStr::from("data")),
            }],
            flags: NodeFlags { terminal: true, ..NodeFlags::default() },
            data: serde_json::Value::Null,
            ui: NodeUiState::default(),
        };
        doc.graph.nodes.insert(nid(&format!("n{last_idx}")), last);

        for i in 0..count.saturating_sub(1) {
            let next = i.saturating_add(1);
            doc.graph.edges.insert(
                eid(&format!("e{i}")),
                connect(&format!("e{i}"), &format!("n{i}"), &format!("n{next}")),
            );
        }
        doc.graph.entry_node = Some(nid("n0"));

        let diags = ValidationPipeline::standard().validate(&doc);
        assert!(diags.is_empty(), "unexpected diagnostics for valid large chain: {diags:?}");
    }

    // ---- Integration: duplicate edges detected alongside other issues ----

    #[test]
    fn integration_duplicate_edge_with_cardinality_violation() {
        let mut doc = FlowDocument::default();
        let n1 = FlowNodeRecord {
            id: nid("n1"),
            kind: SmolStr::from("test"),
            title: SmolStr::from("n1"),
            position: [0.0, 0.0],
            size: [100.0, 50.0],
            z_index: 0,
            parent: None,
            ports: vec![make_port_record("out", PortRole::Source, Cardinality::One)],
            flags: NodeFlags::default(),
            data: serde_json::Value::Null,
            ui: NodeUiState::default(),
        };
        let n2 = FlowNodeRecord {
            id: nid("n2"),
            kind: SmolStr::from("test"),
            title: SmolStr::from("n2"),
            position: [200.0, 0.0],
            size: [100.0, 50.0],
            z_index: 0,
            parent: None,
            ports: vec![make_port_record("in", PortRole::Target, Cardinality::Many)],
            flags: NodeFlags::default(),
            data: serde_json::Value::Null,
            ui: NodeUiState::default(),
        };
        doc.graph.nodes.insert(nid("n1"), n1);
        doc.graph.nodes.insert(nid("n2"), n2);

        // Three identical edges -> duplicate-edge + cardinality violation
        doc.graph.edges.insert(eid("e1"), make_edge_with_ports("e1", "n1", "out", "n2", "in"));
        doc.graph.edges.insert(eid("e2"), make_edge_with_ports("e2", "n1", "out", "n2", "in"));
        doc.graph.edges.insert(eid("e3"), make_edge_with_ports("e3", "n1", "out", "n2", "in"));

        let diags = ValidationPipeline::standard().validate(&doc);
        let codes: Vec<&str> = diags.iter().map(|d| d.code.as_str()).collect();

        // Should have at least 2 duplicate-edge (e2 duplicates e1, e3 duplicates e1)
        let dup_count = codes.iter().filter(|&&c| c == "duplicate-edge").count();
        assert!(dup_count >= 2, "expected at least 2 duplicate-edge, got {dup_count} in {codes:?}");
        assert!(codes.contains(&"cardinality-one-multi-source"), "missing cardinality-one-multi-source in {codes:?}");
    }

    // ---- Integration: node with all flags set (entry + terminal) ----

    #[test]
    fn integration_entry_and_terminal_node_exemptions() {
        let mut doc = FlowDocument::default();
        // A single node that is both entry and terminal -- should not be flagged as orphan
        doc.graph.nodes.insert(
            nid("et"),
            FlowNodeRecord {
                id: nid("et"),
                kind: SmolStr::from("test"),
                title: SmolStr::from("et"),
                position: [0.0, 0.0],
                size: [100.0, 50.0],
                z_index: 0,
                parent: None,
                ports: vec![
                    make_port_record("out", PortRole::Source, Cardinality::One),
                    make_port_record("in", PortRole::Target, Cardinality::One),
                ],
                flags: NodeFlags { entry: true, terminal: true, ..NodeFlags::default() },
                data: serde_json::Value::Null,
                ui: NodeUiState::default(),
            },
        );
        doc.graph.entry_node = Some(nid("et"));

        let diags = ValidationPipeline::standard().validate(&doc);
        // Should have no orphan warning
        assert!(!diags.iter().any(|d| d.code.as_str() == "orphan-node"), "entry+terminal should not be orphan");
        // Ports should not warn because entry output + terminal input are exempt
        let port_diags: Vec<&Diagnostic> = diags.iter().filter(|d| d.code.as_str() == "export-port-unconnected").collect();
        assert!(port_diags.is_empty(), "entry/terminal port exemptions should suppress warnings, got: {port_diags:?}");
    }

    // ---- Integration: custom validator injected into pipeline ----

    #[test]
    fn integration_custom_validator_with_standard_pipeline() {
        struct NoMoreThanThreeNodes;
        impl FlowValidator for NoMoreThanThreeNodes {
            fn validate(&self, doc: &FlowDocument) -> Vec<Diagnostic> {
                let count = doc.graph.nodes.len();
                if count > 3 {
                    return vec![Diagnostic {
                        severity: DiagnosticSeverity::Error,
                        code: SmolStr::from("too-many-nodes"),
                        message: format!("document has {count} nodes, max is 3"),
                        node: None,
                        edge: None,
                    }];
                }
                Vec::new()
            }
        }

        let mut doc = FlowDocument::default();
        for i in 0..5u16 {
            let pos = [f64::from(i).mul_add(150.0, 0.0), 0.0];
            doc.graph.nodes.insert(
                nid(&format!("n{i}")),
                make_full_node(&format!("n{i}"), pos),
            );
        }

        let mut pipeline = ValidationPipeline::standard();
        pipeline.add_validator(Box::new(NoMoreThanThreeNodes));
        let diags = pipeline.validate(&doc);
        assert!(diags.iter().any(|d| d.code.as_str() == "too-many-nodes"));
        // Standard validators also produce orphan warnings etc.
        assert!(diags.len() > 1);
    }
}
