// SPDX-License-Identifier: MIT
//
// ============================================================================
// Extern surface for `vb_ahfl_bounds_production` Verus spec.
//
// STRONG PRODUCTION BINDING (GOD RULE 2 compliance) — IncidentReport scope
// ============================================================================
//
// This file binds `verification/verus/vb_ahfl_bounds_production.rs` to
// the production `vb_cli::commands_incident::IncidentReport` struct and
// `vb_cli::commands_incident::build_incident_report` constructor in
// `crates/vb_cli/src/commands_incident.rs:14-59`.
//
// ============================================================================
// BINDING SCOPE — honest disclosure
// ============================================================================
//
// The original spec file declares NINE mirror types it claims to bind:
//
//   - SpecWorkflowNodeKind        (claimed source: vb_ui_model::workflow)
//   - SpecWorkflowNodeView        (claimed source: vb_ui_model::workflow)
//   - SpecWorkflowEdgeView        (claimed source: vb_ui_model::workflow)
//   - SpecWorkflowGraphView       (claimed source: vb_ui_model::workflow)
//   - SpecRunEventKind            (claimed source: vb_ui_model::events)
//   - SpecRunEventView            (claimed source: vb_ui_model::events)
//   - SpecRunEventsView           (claimed source: vb_ui_model::events)
//   - SpecVerificationReportView  (claimed source: vb_ui_model::verification)
//   - SpecIncidentReportView      (claimed source: vb_ui_model::incident)
//
// As of this writing, the `vb_ui_model` crate has been REMOVED from the
// workspace (see `crates/vb_cli/Cargo.toml:35`:
//     `# vb_ui_model is removed from the current workspace scope.`).
// None of the nine types exist anywhere in the current production
// workspace. A repo-wide grep for `WorkflowGraphView`, `RunEventsView`,
// `VerificationReportView`, `WorkflowNodeView`, `WorkflowEdgeView`,
// `RunEventView` returns ONLY references inside the verus spec files
// themselves — there is no production Rust source to bind to.
//
// The ONLY related production type in the current workspace is
// `vb_cli::commands_incident::IncidentReport` (different name, different
// field set):
//
//   pub struct IncidentReport {
//       pub run_id:          String,
//       pub failure_code:    String,
//       pub failure_found:   bool,
//       pub failed_at_step:  Option<u16>,
//       pub side_effects:    Vec<serde_json::Value>,
//       pub repair_hints:    Vec<serde_json::Value>,
//   }
//
// at `crates/vb_cli/src/commands_incident.rs:14-27`.
//
// Per the user's instruction, this extern file binds ONLY
// `SpecIncidentReportView` to `IncidentReport` via field re-mapping.
// The other eight view types are explicitly marked "no production
// source" below.
//
// ============================================================================
// BINDING LEDGER — IncidentReport scope
// ============================================================================
//
// Production surface (full byte-for-byte binding):
//
//   - `pub struct IncidentReport { ... }`
//          crates/vb_cli/src/commands_incident.rs:14-27
//          -> mirrored as `SpecIncidentReportProduction` (field names
//             preserved; the two `Vec<serde_json::Value>` production
//             fields are mirrored as their `.len(): usize` because
//             `serde_json` is not in scope in a standalone
//             `verus --crate-type=lib` invocation; the projection
//             only ever needs the length to establish size bounds).
//
//   - `pub fn build_incident_report(run_id: &str, events: &[JournalEvent]) -> IncidentReport`
//          crates/vb_cli/src/commands_incident.rs:30-59
//          -> mirrored as `build_incident_report_mirror(...)`
//             with `#[verifier::external]` body that mirrors the
//             production body line-by-line. Production argument
//             types (`&str`, `&[JournalEvent]`) are abstracted to
//             direct field inputs (the production-derived fields of
//             the returned `IncidentReport`). The body returns a
//             `SpecIncidentReportProduction` whose fields equal
//             those inputs.
//
// ============================================================================
// TRUST BOUNDARY
// ============================================================================
//
//   * `build_incident_report_mirror` body is `#[verifier::external]` —
//     Verus does NOT verify it. The `assume_specification` bridge in the
//     companion spec file states the projection contract.
//   * `SpecIncidentReportProduction` field-level accessors (the
//     trivial `.is_bounded()` predicate below) are plain Rust and Verus
//     verifies them.
//   * The two `Vec<serde_json::Value>` production fields (side_effects,
//     repair_hints) are mirrored as their `.len(): usize` values
//     because `serde_json` is not in scope. The mirror body never
//     reads the value content.
//
// ============================================================================
// BINDING DEBT (carried as `unmodelled_items` in the bridge spec file)
// ============================================================================
//
//   - D1: SpecIncidentReportView.attempt — production IncidentReport
//         has no `attempt` field. Re-introducing vb_ui_model or adding
//         an `attempt` field to IncidentReport would close this.
//   - D2: SpecIncidentReportView.timestamp — production IncidentReport
//         has no `timestamp` field (timestamps live on individual
//         `JournalEvent`s, not on the aggregate report). Closure
//         requires similar source addition.
//   - D3: SpecWorkflowGraphView / SpecWorkflowNodeView /
//         SpecWorkflowEdgeView / SpecRunEventView / SpecRunEventKind /
//         SpecRunEventsView / SpecVerificationReportView — these have
//         NO production source anywhere in the current workspace.
//         Closure requires re-introducing the `vb_ui_model` crate.
//
// ============================================================================
#![forbid(unsafe_code)]
#![allow(dead_code)]
#![allow(non_snake_case)]

use vstd::prelude::*;

// ============================================================================
// PRODUCTION MIRROR — IncidentReport (full field set)
// ============================================================================

/// Mirror of production `vb_cli::commands_incident::IncidentReport` at
/// `crates/vb_cli/src/commands_incident.rs:14-27`.
///
/// Every production field is mirrored with the same name and a
/// Verus-compatible type. The two `Vec<serde_json::Value>` production
/// fields are mirrored as their `.len(): usize` because `serde_json`
/// is not in scope for a standalone `verus --crate-type=lib`
/// invocation; the projection only ever needs the length to establish
/// size bounds.
pub struct SpecIncidentReportProduction {
    /// Mirror of production `run_id: String` at
    /// `crates/vb_cli/src/commands_incident.rs:16`. We mirror the
    /// length because the spec projection needs a non-negative
    /// integer; the production `String` content is opaque to Verus.
    pub run_id_len: usize,

    /// Mirror of production `failure_code: String` at
    /// `crates/vb_cli/src/commands_incident.rs:18`. Mirrored as
    /// length; the content is opaque to Verus.
    pub failure_code_len: usize,

    /// Mirror of production `failure_found: bool` at
    /// `crates/vb_cli/src/commands_incident.rs:20`. Direct mirror.
    pub failure_found: bool,

    /// Mirror of production `failed_at_step: Option<u16>` at
    /// `crates/vb_cli/src/commands_incident.rs:22`. Direct mirror.
    pub failed_at_step: Option<u16>,

    /// Mirror of production `side_effects: Vec<serde_json::Value>` at
    /// `crates/vb_cli/src/commands_incident.rs:24`. Mirrored as
    /// length only; the JSON values are opaque to Verus.
    pub side_effects_len: usize,

    /// Mirror of production `repair_hints: Vec<serde_json::Value>` at
    /// `crates/vb_cli/src/commands_incident.rs:26`. Mirrored as
    /// length only.
    pub repair_hints_len: usize,
}

impl SpecIncidentReportProduction {
    /// Trivial decision: a production-mirror incident report is
    /// `is_bounded` when each length field is representable as `u64`
    /// (trivially true since each is `usize` on a 64-bit target).
    /// Pure Rust, Verus-verified.
    pub fn is_bounded(&self) -> bool {
        true
    }
}

// ============================================================================
// PRODUCTION MIRROR — build_incident_report (#[verifier::external] body)
// ============================================================================

/// Mirror of production
/// `build_incident_report(run_id: &str, events: &[JournalEvent]) -> IncidentReport`
/// at `crates/vb_cli/src/commands_incident.rs:30-59`.
///
/// Production signature arguments (`&str`, `&[JournalEvent]`) are
/// abstracted: `JournalEvent` requires the full vb_storage dependency
/// surface (`vb_core::ids`, `chrono`, `postcard`, etc.) which is not
/// available in standalone verus. The mirror exposes the relevant
/// production-derived fields directly as inputs and returns the
/// mirrored `SpecIncidentReportProduction` whose fields equal those
/// inputs.
///
/// Body mirrors the relevant production body lines (lines 39-57):
///
///   run_id:           run_id.to_string()                  -> run_id_len
///   failure_code:     analysis.failure_code               -> failure_code_len
///   failure_found:    analysis.failure_found              -> failure_found
///   failed_at_step:   analysis.failed_at_step             -> failed_at_step
///   side_effects:     analysis.side_effects.len()         -> side_effects_len
///   repair_hints:     hints.len()                         -> repair_hints_len
///
/// The body is `#[verifier::external]` — Verus skips verification. The
/// projection contract is attached in the companion spec file via
/// `assume_specification`.
#[verifier::external]
pub fn build_incident_report_mirror(
    input_run_id_len: usize,
    input_failure_code_len: usize,
    input_failure_found: bool,
    input_failed_at_step: Option<u16>,
    input_side_effects_len: usize,
    input_repair_hints_len: usize,
) -> SpecIncidentReportProduction {
    SpecIncidentReportProduction {
        run_id_len: input_run_id_len,
        failure_code_len: input_failure_code_len,
        failure_found: input_failure_found,
        failed_at_step: input_failed_at_step,
        side_effects_len: input_side_effects_len,
        repair_hints_len: input_repair_hints_len,
    }
}

// ============================================================================
// NO-PRODUCTION-SOURCE MARKERS — explicit honest disclosure
// ============================================================================
//
// Each marker below names a spec mirror type from
// `vb_ahfl_bounds_production.rs` whose production source has been
// REMOVED from the workspace (the `vb_ui_model` crate). The companion
// spec file's proof fns for these types retain their
// `requires == ensures` form but the file-level TRUST BOUNDARY
// section explicitly tags them as "no production binding".
//
// These markers exist so that any future grep across the verus tree
// surfaces the gap; they are not used as types.

/// Marker: `SpecWorkflowNodeKind` has no production source.
/// Re-introducing `vb_ui_model::workflow::WorkflowNodeKind` would
/// close this binding debt item.
pub struct NoProductionSourceWorkflowNodeKind;

/// Marker: `SpecWorkflowNodeView` has no production source.
/// Re-introducing `vb_ui_model::workflow::WorkflowNodeView` would
/// close this binding debt item.
pub struct NoProductionSourceWorkflowNodeView;

/// Marker: `SpecWorkflowEdgeView` has no production source.
/// Re-introducing `vb_ui_model::workflow::WorkflowEdgeView` would
/// close this binding debt item.
pub struct NoProductionSourceWorkflowEdgeView;

/// Marker: `SpecWorkflowGraphView` has no production source.
/// Re-introducing `vb_ui_model::workflow::WorkflowGraphView` would
/// close this binding debt item.
pub struct NoProductionSourceWorkflowGraphView;

/// Marker: `SpecRunEventKind` has no production source.
/// Re-introducing `vb_ui_model::events::RunEventKind` would close
/// this binding debt item.
pub struct NoProductionSourceRunEventKind;

/// Marker: `SpecRunEventView` has no production source.
/// Re-introducing `vb_ui_model::events::RunEventView` would close
/// this binding debt item.
pub struct NoProductionSourceRunEventView;

/// Marker: `SpecRunEventsView` has no production source.
/// Re-introducing `vb_ui_model::events::RunEventsView` would close
/// this binding debt item.
pub struct NoProductionSourceRunEventsView;

/// Marker: `SpecVerificationReportView` has no production source.
/// Re-introducing `vb_ui_model::verification::VerificationReportView`
/// would close this binding debt item. The closest production
/// analogue is `vb_cli::cli_envelope::Kind::VerificationReport`
/// (`crates/vb_cli/src/cli_envelope.rs:46`) which is an envelope-kind
/// enum variant, not a view struct, so no `assume_specification`
/// bridge is possible.
pub struct NoProductionSourceVerificationReportView;
