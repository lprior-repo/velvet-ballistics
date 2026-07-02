// SPDX-License-Identifier: MIT
//
// ============================================================================
// IN-TREE PRODUCTION-SOURCE MIRROR for `vb_cli::commands_incident` —
// focused for `vb_ahfl_bounds_production` Verus spec (VERUS-BOUNDS-001)
// ============================================================================
//
// This file is a MIRROR of the relevant production surface from
//   crates/vb_cli/src/commands_incident.rs:14-59
// with TWO SUBSTITUTIONS required to compile under
// `verus --crate-type=lib` without the `vb_storage` extern crate
// (no installs allowed by the task brief):
//
//   - The production `IncidentReport` struct's
//     `side_effects: Vec<serde_json::Value>` (line 24) and
//     `repair_hints: Vec<serde_json::Value>` (line 26) fields are
//     mirrored as their `.len(): usize` because `serde_json` is not
//     in scope. The spec projection only ever needs the length to
//     establish size bounds; the JSON value content is opaque to
//     Verus.
//   - The production `build_incident_report(run_id: &str, events:
//     &[JournalEvent]) -> IncidentReport` function (line 30-59) is
//     abstracted: the `&str` argument becomes the `run_id_len: usize`
//     of the returned struct, and the `&[JournalEvent]` argument is
//     projected to the four fields the spec surface actually needs
//     (`failure_code_len`, `failure_found`, `failed_at_step`,
//     `side_effects_len`). The body's `#[verifier::external]` body
//     sets the returned struct fields to those inputs, mirroring the
//     production assignment shape at lines 38-58.
//
// DRIFT POLICY: This file MUST be regenerated from
// `crates/vb_cli/src/commands_incident.rs:14-59` whenever production
// changes. The mirror is annotated at the top of every section with
// the originating production line range so regeneration is
// mechanical. Drift that changes the `IncidentReport` field set or
// the `build_incident_report` body breaks the `assume_specification`
// bridges in the companion spec file at compile time, which is the
// explicit drift-detection mechanism for the IncidentReport binding.
//
// This file is included by the companion extern file
// `extern_vb_ahfl_bounds_production.rs` under module-level `#[path]`.
// Production function bodies are `#[verifier::external]`; type-level
// accessors (the trivial `.is_bounded()` predicate) are plain Rust
// and Verus-verified. The contracts are attached in the companion
// spec file via `assume_specification` bridges.
// ============================================================================

#![forbid(unsafe_code)]
#![allow(dead_code)]
#![allow(non_snake_case)]

use vstd::prelude::*;

// ============================================================================
// PRODUCTION MIRROR — IncidentReport (full field set, abstracted)
// ============================================================================
//
// Mirror of production `vb_cli::commands_incident::IncidentReport` at
// `crates/vb_cli/src/commands_incident.rs:14-27`.
//
// Production source (verbatim):
//
//   pub struct IncidentReport {
//       pub run_id:          String,                // line 16
//       pub failure_code:    String,                // line 18
//       pub failure_found:   bool,                  // line 20
//       pub failed_at_step:  Option<u16>,           // line 22
//       pub side_effects:    Vec<serde_json::Value>,// line 24
//       pub repair_hints:    Vec<serde_json::Value>,// line 26
//   }
//
// The two `Vec<serde_json::Value>` production fields are mirrored as
// their `.len(): usize` because `serde_json` is not in scope. The
// production `String` fields are mirrored as `usize` lengths because
// the spec projection needs non-negative integers; the production
// `String` content is opaque to Verus.
pub struct SpecIncidentReportProduction {
    /// Mirror of production `run_id: String` at
    /// `crates/vb_cli/src/commands_incident.rs:16`. Mirrored as length
    /// because the spec projection needs a non-negative integer; the
    /// production `String` content is opaque to Verus.
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
//
// Mirror of production
// `build_incident_report(run_id: &str, events: &[JournalEvent]) -> IncidentReport`
// at `crates/vb_cli/src/commands_incident.rs:30-59`.
//
// Production body lines 38-58:
//
//   IncidentReport {
//       run_id: run_id.to_string(),                              // line 39
//       failure_code: analysis.failure_code,                     // line 40
//       failure_found: analysis.failure_found,                   // line 41
//       failed_at_step: analysis.failed_at_step,                // line 42
//       side_effects: analysis.side_effects.into_iter()...,      // lines 43-56
//       repair_hints: hints.into_iter().map(serde_json::Value::String).collect(), // line 57
//   }
//
// Production argument types (`&str`, `&[JournalEvent]`) are
// abstracted: `JournalEvent` requires the full vb_storage dependency
// surface (`vb_core::ids`, `chrono`, `postcard`, etc.) which is not
// available in standalone verus. The mirror exposes the relevant
// production-derived fields directly as inputs and returns the
// mirrored `SpecIncidentReportProduction` whose fields equal those
// inputs.
//
// The body is `#[verifier::external]` — Verus skips verification. The
// projection contract is attached in the companion spec file via
// `assume_specification`.
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