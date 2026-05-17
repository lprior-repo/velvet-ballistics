//! Canonicalization for CLI/UI artifact parity comparison.
//!
//! Provides deterministic projection of CLI JSON and UI model artifacts
//! into a canonical form that enables comparison while ignoring
//! representation-only ordering or formatting differences.

#![forbid(unsafe_code)]

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use serde::{Deserialize, Serialize};

use crate::envelope::{EnvelopeKind, MetadataEnvelope, SchemaVersion};
use crate::workflow::WorkflowGraphView;

/// Maximum length for a redaction summary string.
pub const MAX_REDACTION_SUMMARY_LEN: usize = 64;

/// Canonical representation of a UI artifact for parity comparison.
/// Contains only the fields that affect semantic equality
/// between CLI-emitted and UI-model artifacts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CanonicalUiArtifact {
    /// Schema version for structural compatibility checking.
    pub schema_version: SchemaVersion,
    /// Envelope kind identifying the artifact type.
    pub kind: EnvelopeKind,
    /// Run identifier provenance.
    pub run_id: u64,
    /// Timestamp provenance.
    pub timestamp: i64,
    /// Workflow graph canonical form if present.
    pub workflow_graph: Option<CanonicalWorkflowGraph>,
    /// Event sequence bounds if present.
    pub event_bounds: Option<CanonicalEventBounds>,
}

/// Canonical form of a workflow graph for parity comparison.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CanonicalWorkflowGraph {
    /// Workflow identifier.
    pub workflow_id: u64,
    /// Node count for structural comparison.
    pub node_count: usize,
    /// Edge count for structural comparison.
    pub edge_count: usize,
    /// Step indices present in nodes (sorted, deduplicated).
    pub step_indices: Vec<u16>,
}

/// Canonical bounds for event sequences.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CanonicalEventBounds {
    /// First sequence number.
    pub from_seq: u64,
    /// Last sequence number.
    pub to_seq: u64,
    /// Event count.
    pub event_count: usize,
}

/// Result of CLI/UI artifact parity comparison.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParityMatch {
    /// Whether the artifacts match in canonical form.
    pub is_parity: bool,
    /// Human-readable diagnostic if not parity.
    pub diagnostic: Option<String>,
}

/// Canonicalizes a CLI-emitted JSON artifact into canonical form.
/// Returns `None` if the JSON structure is incompatible with canonicalization.
pub fn canonicalize_cli_artifact(
    json: &serde_json::Value,
    kind: EnvelopeKind,
) -> Option<CanonicalUiArtifact> {
    let obj = json.as_object()?;

    let schema_version = obj
        .get("schema_version")
        .and_then(|v| v.as_u64())
        .and_then(|v| u16::try_from(v).ok())
        .and_then(|v| SchemaVersion::new(v).ok())
        .unwrap_or(SchemaVersion::CURRENT);

    let run_id = obj.get("run_id").and_then(|v| v.as_u64()).unwrap_or(0);

    let timestamp = obj.get("timestamp").and_then(|v| v.as_i64()).unwrap_or(0);

    let workflow_graph = obj
        .get("workflow")
        .and_then(canonicalize_workflow_graph_from_json);

    let event_bounds = obj.get("events").and_then(canonicalize_events_from_json);

    Some(CanonicalUiArtifact {
        schema_version,
        kind,
        run_id,
        timestamp,
        workflow_graph,
        event_bounds,
    })
}

/// Canonicalizes a UI model artifact into canonical form.
pub fn canonicalize_ui_artifact(
    schema_version: SchemaVersion,
    kind: EnvelopeKind,
    metadata: &MetadataEnvelope,
    workflow_graph: Option<&WorkflowGraphView>,
    from_seq: Option<u64>,
    to_seq: Option<u64>,
    event_count: Option<usize>,
) -> CanonicalUiArtifact {
    let workflow_graph = workflow_graph.map(canonicalize_workflow_graph);
    let event_bounds = match (from_seq, to_seq, event_count) {
        (Some(from), Some(to), Some(count)) => Some(CanonicalEventBounds {
            from_seq: from,
            to_seq: to,
            event_count: count,
        }),
        _ => None,
    };

    CanonicalUiArtifact {
        schema_version,
        kind,
        run_id: metadata.run_id().get(),
        timestamp: metadata.timestamp(),
        workflow_graph,
        event_bounds,
    }
}

/// Compares a CLI artifact canonical form with a UI artifact canonical form.
pub fn compare_cli_ui_artifacts(
    cli: &CanonicalUiArtifact,
    ui: &CanonicalUiArtifact,
) -> ParityMatch {
    if cli.schema_version != ui.schema_version {
        return ParityMatch {
            is_parity: false,
            diagnostic: Some(format!(
                "schema version mismatch: CLI={}, UI={}",
                cli.schema_version.get(),
                ui.schema_version.get()
            )),
        };
    }

    if cli.kind != ui.kind {
        return ParityMatch {
            is_parity: false,
            diagnostic: Some(format!(
                "kind mismatch: CLI={}, UI={}",
                cli.kind.name(),
                ui.kind.name()
            )),
        };
    }

    if cli.run_id != ui.run_id {
        return ParityMatch {
            is_parity: false,
            diagnostic: Some(format!(
                "run_id mismatch: CLI={}, UI={}",
                cli.run_id, ui.run_id
            )),
        };
    }

    if cli.timestamp != ui.timestamp {
        return ParityMatch {
            is_parity: false,
            diagnostic: Some(format!(
                "timestamp mismatch: CLI={}, UI={}",
                cli.timestamp, ui.timestamp
            )),
        };
    }

    match (&cli.workflow_graph, &ui.workflow_graph) {
        (Some(cli_wf), Some(ui_wf)) => {
            if cli_wf.workflow_id != ui_wf.workflow_id {
                return ParityMatch {
                    is_parity: false,
                    diagnostic: Some(format!(
                        "workflow_id mismatch: CLI={}, UI={}",
                        cli_wf.workflow_id, ui_wf.workflow_id
                    )),
                };
            }
            if cli_wf.node_count != ui_wf.node_count {
                return ParityMatch {
                    is_parity: false,
                    diagnostic: Some(format!(
                        "node count mismatch: CLI={}, UI={}",
                        cli_wf.node_count, ui_wf.node_count
                    )),
                };
            }
            if cli_wf.edge_count != ui_wf.edge_count {
                return ParityMatch {
                    is_parity: false,
                    diagnostic: Some(format!(
                        "edge count mismatch: CLI={}, UI={}",
                        cli_wf.edge_count, ui_wf.edge_count
                    )),
                };
            }
        }
        (None, None) => {}
        _ => {
            return ParityMatch {
                is_parity: false,
                diagnostic: Some("workflow graph presence mismatch".to_string()),
            };
        }
    }

    match (&cli.event_bounds, &ui.event_bounds) {
        (Some(cli_ev), Some(ui_ev)) => {
            if cli_ev.from_seq != ui_ev.from_seq
                || cli_ev.to_seq != ui_ev.to_seq
                || cli_ev.event_count != ui_ev.event_count
            {
                return ParityMatch {
                    is_parity: false,
                    diagnostic: Some(format!(
                        "event bounds mismatch: CLI=({}-{},{}), UI=({}-{},{})",
                        cli_ev.from_seq,
                        cli_ev.to_seq,
                        cli_ev.event_count,
                        ui_ev.from_seq,
                        ui_ev.to_seq,
                        ui_ev.event_count
                    )),
                };
            }
        }
        (None, None) => {}
        _ => {
            return ParityMatch {
                is_parity: false,
                diagnostic: Some("event bounds presence mismatch".to_string()),
            };
        }
    }

    ParityMatch {
        is_parity: true,
        diagnostic: None,
    }
}

fn canonicalize_workflow_graph_from_json(
    json: &serde_json::Value,
) -> Option<CanonicalWorkflowGraph> {
    let obj = json.as_object()?;

    let workflow_id = obj.get("workflow_id").and_then(|v| v.as_u64()).unwrap_or(0);

    let nodes = obj.get("nodes").and_then(|v| v.as_array())?;
    let edges = obj.get("edges").and_then(|v| v.as_array())?;

    let node_count = nodes.len();
    let edge_count = edges.len();

    let mut step_indices: Vec<u16> = nodes
        .iter()
        .filter_map(|n| {
            n.as_object()
                .and_then(|o| o.get("step_idx"))
                .and_then(|v| v.as_u64())
                .and_then(|v| u16::try_from(v).ok())
        })
        .collect();
    step_indices.sort();
    step_indices.dedup();

    Some(CanonicalWorkflowGraph {
        workflow_id,
        node_count,
        edge_count,
        step_indices,
    })
}

fn canonicalize_events_from_json(json: &serde_json::Value) -> Option<CanonicalEventBounds> {
    let arr = json.as_array()?;
    let count = arr.len();

    if count == 0 {
        return Some(CanonicalEventBounds {
            from_seq: 0,
            to_seq: 0,
            event_count: 0,
        });
    }

    let from_seq = arr
        .first()
        .and_then(|e| e.as_object())
        .and_then(|o| o.get("seq"))
        .and_then(|v| v.as_u64())
        .unwrap_or(0);

    let to_seq = arr
        .last()
        .and_then(|e| e.as_object())
        .and_then(|o| o.get("seq"))
        .and_then(|v| v.as_u64())
        .unwrap_or(0);

    Some(CanonicalEventBounds {
        from_seq,
        to_seq,
        event_count: count,
    })
}

fn canonicalize_workflow_graph(view: &WorkflowGraphView) -> CanonicalWorkflowGraph {
    let mut step_indices: Vec<u16> = view.nodes.iter().map(|n| n.step_idx.get()).collect();
    step_indices.sort();
    step_indices.dedup();

    CanonicalWorkflowGraph {
        workflow_id: u64::from(view.workflow_id.get()),
        node_count: view.nodes.len(),
        edge_count: view.edges.len(),
        step_indices,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_cli_artifact_basic() {
        let json = serde_json::json!({
            "schema_version": 1,
            "run_id": 42,
            "timestamp": 1234567890,
            "kind": "Success"
        });

        let result = canonicalize_cli_artifact(&json, EnvelopeKind::Success);
        assert!(result.is_some());
        let artifact = result.unwrap();
        assert_eq!(artifact.schema_version.get(), 1);
        assert_eq!(artifact.run_id, 42);
        assert_eq!(artifact.timestamp, 1234567890);
    }

    #[test]
    fn parity_match_positive() {
        let cli = CanonicalUiArtifact {
            schema_version: SchemaVersion::CURRENT,
            kind: EnvelopeKind::Success,
            run_id: 123,
            timestamp: 1000,
            workflow_graph: None,
            event_bounds: None,
        };

        let ui = CanonicalUiArtifact {
            schema_version: SchemaVersion::CURRENT,
            kind: EnvelopeKind::Success,
            run_id: 123,
            timestamp: 1000,
            workflow_graph: None,
            event_bounds: None,
        };

        let result = compare_cli_ui_artifacts(&cli, &ui);
        assert!(result.is_parity);
        assert!(result.diagnostic.is_none());
    }

    #[test]
    fn parity_match_schema_mismatch() {
        let cli = CanonicalUiArtifact {
            schema_version: SchemaVersion::new(1).unwrap(),
            kind: EnvelopeKind::Success,
            run_id: 123,
            timestamp: 1000,
            workflow_graph: None,
            event_bounds: None,
        };

        let ui = CanonicalUiArtifact {
            schema_version: SchemaVersion::new(2).unwrap(),
            kind: EnvelopeKind::Success,
            run_id: 123,
            timestamp: 1000,
            workflow_graph: None,
            event_bounds: None,
        };

        let result = compare_cli_ui_artifacts(&cli, &ui);
        assert!(!result.is_parity);
        assert!(result.diagnostic.is_some());
        assert!(
            result
                .diagnostic
                .unwrap()
                .contains("schema version mismatch")
        );
    }
}
