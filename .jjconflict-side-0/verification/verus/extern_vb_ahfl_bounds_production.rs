// SPDX-License-Identifier: MIT
//
// ============================================================================
// Extern surface for `vb_ahfl_bounds_production` Verus spec.
//
// WEAK PRODUCTION BINDING (GOD RULE 2 compliance) — IncidentReport scope
// ============================================================================
//
// This file is the production-binding surface for the
// `vb_ahfl_bounds_production.rs` Verus spec. It contains a direct
// `#[path]` inclusion of the in-tree mirror at
// `verification/verus/production_inner/vb_ahfl_bounds_production_inner.rs`,
// which is a verbatim copy of the relevant production surface from
// `crates/vb_cli/src/commands_incident.rs:14-59` with the two
// `Vec<serde_json::Value>` fields abstracted to their lengths (because
// `serde_json` is not in scope in a standalone `verus --crate-type=lib`
// invocation — no installs allowed by the task brief).
//
// The mirror is included via `#[path]` from inside `verus!` (WITHOUT
// module-level `#[verifier::external]`) so the type declarations are
// nameable in spec mode. The companion spec file
// `vb_ahfl_bounds_production.rs` attaches `assume_specification`
// contracts to the production-bound exec methods.
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
// source" via `NoProductionSource*` marker structs in the mirror file.
//
// ============================================================================
// TRUST BOUNDARY
// ============================================================================
//
//   * `build_incident_report_mirror` body is `#[verifier::external]` —
//     Verus does NOT verify it. The `assume_specification` bridge in
//     the companion spec file states the projection contract.
//   * `SpecIncidentReportProduction` is plain Rust.
//   * The exec wrapper `wrapper_build_incident_report_then_bounded`
//     in the companion spec file actually CALLS the production
//     mirror, so the bridge postcondition is exercised end-to-end.
//   * `SpecIncidentReportView` projection is verified by Verus.
// ============================================================================

#![forbid(unsafe_code)]
#![allow(dead_code)]
#![allow(non_snake_case)]

use vstd::prelude::*;

verus! {

// ---------------------------------------------------------------------------
// PRODUCTION MIRROR INCLUSION via #[path]
// ---------------------------------------------------------------------------
//
// Direct `#[path]` inclusion of the in-tree mirror at
// `production_inner/vb_ahfl_bounds_production_inner.rs`. The mirror
// is a verbatim copy of `crates/vb_cli/src/commands_incident.rs:14-59`
// with two substitutions:
//   1. The two `Vec<serde_json::Value>` production fields are mirrored
//      as their `.len(): usize` (serde_json is not in scope).
//   2. The `build_incident_report` body is `#[verifier::external]` and
//      the input `&[JournalEvent]` is abstracted to direct field
//      inputs (the production-derived fields of the returned
//      `IncidentReport`).
// Any drift in field NAME or method signature breaks the verification
// build (the `assume_specification` bridge becomes inconsistent).
#[path = "production_inner/vb_ahfl_bounds_production_inner.rs"]
pub mod production_incident;

} // verus!

// Re-export the production types so the spec file can reference them
// via `crate::production::production_incident::SpecIncidentReportProduction`.
pub use production_incident::{
    SpecIncidentReportProduction,
    build_incident_report_mirror,
    NoProductionSourceWorkflowNodeKind,
    NoProductionSourceWorkflowNodeView,
    NoProductionSourceWorkflowEdgeView,
    NoProductionSourceWorkflowGraphView,
    NoProductionSourceRunEventKind,
    NoProductionSourceRunEventView,
    NoProductionSourceRunEventsView,
    NoProductionSourceVerificationReportView,
};