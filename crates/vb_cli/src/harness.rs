//! Module: harness — autonomous CLI command composing harness types and helpers.
//!
//! Pure types and computation functions for the harness command.
//! The I/O-bound `cmd_harness` function is in the binary tree (main.rs harness
//! submodule) because it uses binary-only macros and output helpers.
//!
//! This module composes the existing replay/diff/incident CLI surfaces
//! into an autonomous testing harness command.

#![forbid(unsafe_code)]

use crate::commands_diff::compute_diff;
use crate::commands_incident::build_incident_report;

/// A single autonomous run produced by the harness.
pub struct HarnessRun {
    /// The seed used for deterministic scheduling.
    pub seed: u64,
    /// The step bound for the run.
    pub step_bound: usize,
    /// The run ID assigned to this run.
    pub run_id: u64,
    /// Events produced by this run.
    pub events: Vec<vb_storage::JournalEvent>,
}

/// A transcript entry describing one aspect of a harness iteration.
pub struct TranscriptEntry {
    /// Iteration index (1-based).
    pub iteration: usize,
    /// Seed used.
    pub seed: u64,
    /// Whether this iteration passed (no divergence).
    pub passed: bool,
    /// Number of differences found (0 means pass).
    pub differences: usize,
    /// Incident report if divergence was detected.
    pub incident: Option<serde_json::Value>,
}

/// Result of a full harness execution.
pub struct HarnessResult {
    /// Number of iterations executed.
    pub iterations: usize,
    /// Number of iterations that passed.
    pub passed: usize,
    /// Number of iterations that failed (diverged).
    pub failed: usize,
    /// Transcript of all iterations.
    pub transcript: Vec<TranscriptEntry>,
    /// Total differences across all iterations.
    pub total_differences: usize,
}

/// Build normalized observations from events.
pub fn build_normalized_observations(events: &[vb_storage::JournalEvent]) -> Vec<serde_json::Value> {
    events
        .iter()
        .map(|event| {
            serde_json::json!({
                "type": crate::commands_diff::event_name(event),
            })
        })
        .collect()
}

/// Build a harness result from events and parameters.
///
/// This composes incident analysis and diff computation into a single
/// HarnessResult suitable for JSON serialization and export.
pub fn build_harness_result(
    events: &[vb_storage::JournalEvent],
    seed: u64,
    step_bound: usize,
) -> HarnessResult {
    let incident = build_incident_report("harness-run", events);
    let divergence = incident.failure_found;

    HarnessResult {
        iterations: 1,
        passed: if divergence { 0 } else { 1 },
        failed: if divergence { 1 } else { 0 },
        transcript: vec![TranscriptEntry {
            iteration: 1,
            seed,
            passed: !divergence,
            differences: if divergence { incident.side_effects.len() } else { 0 },
            incident: if divergence {
                Some(serde_json::json!({
                    "run_id": "harness-run",
                    "failure_code": incident.failure_code,
                    "failed_at_step": incident.failed_at_step,
                    "side_effects": incident.side_effects,
                    "repair_hints": incident.repair_hints,
                    "diverged": true,
                }))
            } else {
                None
            },
        }],
        total_differences: if divergence {
            incident.side_effects.len()
        } else {
            0
        },
    }
}

/// Compute a diff between expected and actual events for the harness.
pub fn build_harness_diff(
    expected: &[vb_storage::JournalEvent],
    actual: &[vb_storage::JournalEvent],
    seed: u64,
    step_bound: usize,
) -> serde_json::Value {
    let result = compute_diff(expected, actual);
    serde_json::json!({
        "schema_version": crate::cli_envelope::SCHEMA_VERSION,
        "kind": "harness_diff",
        "seed": seed,
        "step_bound": step_bound,
        "expected_events": result.events_a,
        "actual_events": result.events_b,
        "diffs": result.diffs,
        "total_differences": result.diffs.len(),
    })
}

/// Export a JSON value to a file in the output directory.
pub fn export_json_file(dir: &std::path::Path, filename: &str, value: &serde_json::Value) {
    let path = dir.join(filename);
    let serialized = serde_json::to_string_pretty(value).unwrap_or_else(|_| "null".to_string());
    if let Err(e) = std::fs::write(&path, serialized) {
        eprintln!("warning: failed to write {}: {e}", path.display());
    }
}

/// Export events to a JSON array file.
pub fn export_events_to_json(dir: &std::path::Path, filename: &str, events: &[vb_storage::JournalEvent]) {
    let json_events: Vec<serde_json::Value> = events
        .iter()
        .map(|e| {
            serde_json::json!({
                "type": crate::commands_diff::event_name(e),
            })
        })
        .collect();
    export_json_file(dir, filename, &serde_json::Value::Array(json_events));
}

#[cfg(test)]
#[path = "harness/tests.rs"]
mod tests;
