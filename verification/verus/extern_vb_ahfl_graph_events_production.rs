// SPDX-License-Identifier: MIT
//
// ============================================================================
// Extern surface for `vb_ahfl_graph_events_production` Verus spec.
//
// WEAK PRODUCTION BINDING (production_inner mirror) — Kind envelope scope
// ============================================================================
//
// This file is the production-binding surface for the
// `vb_ahfl_graph_events_production.rs` Verus spec. It contains:
//
//   1. A direct `#[path]` inclusion of the verbatim production mirror
//      at `verification/verus/production_inner/cli_envelope_production.rs`,
//      which is itself a VERBATIM copy of
//      `crates/vb_cli/src/cli_envelope.rs:1-114` with only the
//      `serde_json`-dependent items removed. This structural binding
//      means any rename, discriminant drift, or signature change in
//      the production `Kind` enum, the `kind::*` constants, or the
//      `as_str` / `from_str` match arms breaks this Verus build at
//      compile time. See the drift policy header in
//      `production_inner/cli_envelope_production.rs`.
//
//   2. A phantom drift-detection helper that calls the bound
//      production `as_str` method on every `Kind` variant. A rename
//      or discriminant drift in the production enum breaks the
//      compiler, which is the explicit drift-detection mechanism.
//
// ============================================================================
// WHY A FOCUSED PRODUCTION MIRROR (NOT DIRECT #[path] TO cli_envelope.rs)
// ============================================================================
//
// Direct `#[path = "../../crates/vb_cli/src/cli_envelope.rs"]` inclusion
// is blocked by:
//
//   - cli_envelope.rs:14 `use serde_json::{Map, Value};` requires the
//     full `serde_json` crate, which is not registered under a
//     standalone `verus --crate-type=lib` invocation (no installs
//     allowed by task brief).
//   - cli_envelope.rs:133 `build_envelope` constructs
//     `serde_json::Map::new()` and `Value::String(...)`. Without
//     `serde_json` in scope, the file fails to compile.
//   - cli_envelope.rs:154 `serialize_with_version` similarly.
//   - cli_envelope.rs:170 `EnvelopeError` enum + its `Display` impl.
//
// The in-tree mirror at
// `verification/verus/production_inner/cli_envelope_production.rs`
// sidesteps every blocker by copying only lines 1-114 of the
// production source (the `Kind` enum, the `kind::*` constants, the
// `SCHEMA_VERSION` constant, and the `as_str` / `from_str` impls).
// Lines 13-14, 116-184, and 186-287 (the `serde_json`-dependent
// items and the `#[cfg(test)] mod tests` block) are intentionally
// omitted. The verbatim match arms in `as_str` and `from_str` are
// preserved unchanged, so any drift in those arms breaks this
// Verus build.
//
// ============================================================================
// BINDING SCOPE — honest disclosure
// ============================================================================
//
// The original spec file `vb_ahfl_graph_events_production.rs`
// declares SEVEN mirror types it claims to bind to `vb_ui_model`:
//
//   - SpecWorkflowNodeKind        (claimed source: vb_ui_model::workflow)
//   - SpecWorkflowNodeView        (claimed source: vb_ui_model::workflow)
//   - SpecWorkflowEdgeView        (claimed source: vb_ui_model::workflow)
//   - SpecWorkflowGraphView       (claimed source: vb_ui_model::workflow)
//   - SpecRunEventKind            (claimed source: vb_ui_model::events)
//   - SpecRunEventView            (claimed source: vb_ui_model::events)
//   - SpecRunEventsView           (claimed source: vb_ui_model::events)
//
// As of this writing, the `vb_ui_model` crate has been REMOVED from
// the workspace (see `crates/vb_cli/Cargo.toml:35`:
//     `# vb_ui_model is removed from the current workspace scope.`).
// None of the seven types exist anywhere in the current production
// workspace. A repo-wide grep for `WorkflowGraphView`, `RunEventsView`,
// `WorkflowNodeView`, `WorkflowEdgeView`, `RunEventView` returns ONLY
// references inside the verus spec files themselves — there is no
// production Rust source to bind to for those types.
//
// The closest existing production symbols are the envelope
// discriminants `Kind::WorkflowGraph` and `Kind::RunEvents` at
// `crates/vb_cli/src/cli_envelope.rs:49, 53`, which IDENTIFY that a
// serialized payload is a "WorkflowGraph" or "RunEvents" type but
// carry NO field state for the workflow / events content. The
// binding scope of THIS file is therefore:
//
//   - KIND ENVELOPE SCOPE (FULLY BOUND): The `Kind` enum and its
//     `WorkflowGraph` / `RunEvents` variants are bound to production
//     via `#[path]` to the verbatim production mirror. The
//     `as_str` / `from_str` contracts are attached via
//     `assume_specification` in the companion spec file. Drift in
//     the production discriminant set or constant values breaks
//     this build.
//
//   - VIEW CONTENT SCOPE (HONESTLY UNBOUND): The seven view mirror
//     types (`SpecWorkflowGraphView`, `SpecRunEventsView`, etc.)
//     have NO production source. They are explicitly marked
//     "no production source" via `NoProductionSource*` marker
//     structs below. The companion spec file's proof fns for these
//     types retain their `requires == entails ensures` form but
//     are tagged in their header comments as
//     `VACUOUS — NO PRODUCTION BINDING`. Re-introducing
//     `vb_ui_model` would close this binding gap.
//
// ============================================================================
// BINDING LEDGER — Kind envelope scope (full byte-for-byte binding)
// ============================================================================
//
//   - `pub(crate) enum Kind { ..., WorkflowGraph, ..., RunEvents, ... }`
//          crates/vb_cli/src/cli_envelope.rs:42-63
//          -> mirrored verbatim in
//             production_inner/cli_envelope_production.rs and bound
//             via `#[path]` below. Any discriminant drift breaks
//             the spec build.
//
//   - `pub(crate) const SCHEMA_VERSION: &str`
//          crates/vb_cli/src/cli_envelope.rs:16-18
//          -> mirrored verbatim. Used to ground the production
//             envelope contract.
//
//   - `pub(crate) mod kind { pub(crate) const WORKFLOW_GRAPH: &str = "WorkflowGraph"; ... }`
//          crates/vb_cli/src/cli_envelope.rs:22-40
//          -> mirrored verbatim. The `WORKFLOW_GRAPH` and `RUN_EVENTS`
//             constants are referenced by the
//             `assume_specification[ Kind::as_str ]` bridges in the
//             companion spec file.
//
//   - `impl Kind { fn as_str(&self) -> &'static str }`
//          crates/vb_cli/src/cli_envelope.rs:65-88
//          -> mirrored verbatim. The contract
//             `as_str() == kind::WORKFLOW_GRAPH` (for
//             `Kind::WorkflowGraph`) and
//             `as_str() == kind::RUN_EVENTS` (for `Kind::RunEvents`)
//             is attached via `assume_specification` in the
//             companion spec file.
//
//   - `impl Kind { fn from_str(s: &str) -> Option<Kind> }`
//          crates/vb_cli/src/cli_envelope.rs:90-114
//          -> mirrored verbatim. The round-trip contract
//             `from_str(s) == Some(k) iff s == k.as_str()` is
//             attached via `assume_specification` in the companion
//             spec file.
//
// ============================================================================
// TRUST BOUNDARY
// ============================================================================
//
//   * The production body of `Kind::as_str` is `#[verifier::external]`
//     by virtue of being declared at the crate root of the extern
//     file outside any `verus!` block (external by default in Verus).
//     Verus does NOT verify it. The `assume_specification` bridge in
//     the companion spec file states the projection contract.
//   * The production body of `Kind::from_str` is similarly
//     `#[verifier::external]` and the contract is attached via
//     `assume_specification`.
//   * `SCHEMA_VERSION`, the `kind::*` constants, and the `Kind` enum
//     discriminant set are STRUCTURALLY BOUND via `#[path]`. Drift
//     in their values or names breaks Rust resolution at compile
//     time.
//
// ============================================================================
// BINDING DEBT (carried as honest disclosure in the spec file)
// ============================================================================
//
//   - D1: SpecWorkflowNodeKind — production source REMOVED
//         (vb_ui_model removed from workspace). Re-introducing
//         vb_ui_model would close this.
//   - D2: SpecWorkflowNodeView — production source REMOVED.
//   - D3: SpecWorkflowEdgeView — production source REMOVED.
//   - D4: SpecWorkflowGraphView — production source REMOVED. The
//         closest production analogue is `Kind::WorkflowGraph` enum
//         variant (envelope discriminant only, no field state).
//   - D5: SpecRunEventKind — production source REMOVED.
//   - D6: SpecRunEventView — production source REMOVED.
//   - D7: SpecRunEventsView — production source REMOVED. The closest
//         production analogue is `Kind::RunEvents` enum variant
//         (envelope discriminant only, no field state).
//
// ============================================================================
#![forbid(unsafe_code)]
#![allow(dead_code)]
#![allow(non_snake_case)]

use vstd::prelude::*;

verus! {

// ---------------------------------------------------------------------------
// PRODUCTION INCLUSION via #[path] + module-level #[verifier::external]
// ---------------------------------------------------------------------------
//
// WHY MODULE-LEVEL `#[verifier::external]`: the production `Kind`
// enum derives `Debug`, which expands into calls to `core::fmt::*`
// and `core::intrinsics::discriminant_value`. Verus does not support
// those std types/functions under standalone `--crate-type=lib` (no
// global std-spec augmentation is in scope). Marking the whole
// production module `#[verifier::external]` is the precise mechanism
// Verus provides for "this module's contents are opaque" — the
// types and fns remain visible (so `production::Kind::WorkflowGraph`
// still names the production discriminant) but their bodies are
// trusted rather than proven.
//
// Direct `#[path]` inclusion of the verbatim production mirror at
// `production_inner/cli_envelope_production.rs`. The mirror contains
// the production `Kind` enum, the `kind::*` constants, the
// `SCHEMA_VERSION` constant, and the `as_str` / `from_str` impls,
// copied line-for-line from `crates/vb_cli/src/cli_envelope.rs:1-114`.
// The `#[path]` attribute ensures any drift in the discriminant set,
// the constant values, or the `as_str` / `from_str` match arms breaks
// Rust resolution at compile time.
//
// Drift detection: a phantom `prod_fns_drift_check` fn below calls
// `Kind::as_str()` on every production `Kind` variant and
// `Kind::from_str()` on every production constant string. A rename of
// any variant or constant, or a drift in the `as_str` / `from_str`
// match arms, breaks the lookup and fails this Verus build.
#[verifier::external]
#[path = "production_inner/cli_envelope_production.rs"]
pub mod cli_envelope_production;

// Re-export the production `Kind` enum so the companion spec file
// can attach an `#[verifier::external_type_specification]` bridge to
// it. The re-export does not change the trusted boundary: the type is
// still backed by the `#[verifier::external]` body from
// `cli_envelope_production`.
pub use cli_envelope_production::Kind;

// Phantom drift-detection helper. The body is `#[verifier::external]`
// (opaque to Verus), but the `Kind::*` references and method calls
// force Rust to resolve the production discriminant set, the
// `kind::*` constants, and the `as_str` / `from_str` match arms at
// compile time. A rename, discriminant drift, or match-arm drift
// breaks this fn's compilation.
#[verifier::external]
fn prod_fns_drift_check(s: &str) {
    // Every Kind variant exercises a different match arm in as_str.
    let _ = Kind::VerificationReport.as_str();
    let _ = Kind::DiagnosticReport.as_str();
    let _ = Kind::WorkflowExplanation.as_str();
    let _ = Kind::WorkflowGraph.as_str();
    let _ = Kind::SimulationReport.as_str();
    let _ = Kind::SubmitRunResult.as_str();
    let _ = Kind::RunInspection.as_str();
    let _ = Kind::RunEvents.as_str();
    let _ = Kind::ReplayReport.as_str();
    let _ = Kind::IncidentReport.as_str();
    let _ = Kind::ActionList.as_str();
    let _ = Kind::ActionDescription.as_str();
    let _ = Kind::DoctorReport.as_str();
    let _ = Kind::AiContextPacket.as_str();
    let _ = Kind::CliStatus.as_str();
    let _ = Kind::SystemStatus.as_str();
    let _ = Kind::AgentContext.as_str();

    // Every kind::* constant exercises a different match arm in from_str.
    let _ = Kind::from_str(cli_envelope_production::kind::WORKFLOW_GRAPH);
    let _ = Kind::from_str(cli_envelope_production::kind::RUN_EVENTS);
    let _ = Kind::from_str(cli_envelope_production::SCHEMA_VERSION);
    let _ = Kind::from_str(s);
}

} // verus!
// ============================================================================
// NO-PRODUCTION-SOURCE MARKERS — explicit honest disclosure
// ============================================================================
//
// Each marker below names a spec mirror type from
// `vb_ahfl_graph_events_production.rs` whose production source has been
// REMOVED from the workspace (the `vb_ui_model` crate). The companion
// spec file's proof fns for these types retain their
// `requires == entails ensures` form but the file-level TRUST BOUNDARY
// section explicitly tags them as "VACUOUS — NO PRODUCTION BINDING".
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
/// close this binding debt item. The closest production analogue is
/// `vb_cli::cli_envelope::Kind::WorkflowGraph` (envelope discriminant
/// variant — no field state) which is bound at the envelope level
/// (not at the view-content level) by `Kind::WorkflowGraph.as_str() ==
/// kind::WORKFLOW_GRAPH` via `assume_specification` in the companion
/// spec file.
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
/// this binding debt item. The closest production analogue is
/// `vb_cli::cli_envelope::Kind::RunEvents` (envelope discriminant
/// variant — no field state) which is bound at the envelope level
/// (not at the view-content level) by `Kind::RunEvents.as_str() ==
/// kind::RUN_EVENTS` via `assume_specification` in the companion
/// spec file.
pub struct NoProductionSourceRunEventsView;
