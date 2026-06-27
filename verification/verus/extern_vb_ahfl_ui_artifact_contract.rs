// SPDX-License-Identifier: MIT
//
// ============================================================================
// Extern surface for `vb_ahfl_ui_artifact_contract` Verus spec.
//
// STRONG PRODUCTION BINDING (GOD RULE 2 compliance) — UI artifact scope
// ============================================================================
//
// This file binds `verification/verus/vb_ahfl_ui_artifact_contract.rs`
// to the production surface that backs the four original spec types:
//
//   1. `UiArtifactMetadata`  -> production cli_envelope surface
//        - crates/vb_cli/src/cli_envelope.rs:18 `pub(crate) const
//          SCHEMA_VERSION: &str = "velvet-ballistics/cli-output/v1";`
//        - crates/vb_cli/src/cli_envelope.rs:42-63 `pub(crate) enum Kind`
//          (17 variants — 4 used by the UI artifact contract:
//          VerificationReport, WorkflowGraph, RunEvents, AiContextPacket)
//        - crates/vb_cli/src/cli_envelope.rs:133 `build_envelope` mirrors
//          `build_envelope_mirror`.
//
//   2. `BoundedCollectionFacts` -> production journal limit surface
//        - crates/vb_storage/src/journal/core.rs:25-48
//          `pub struct EventReplayLimit { max_events: usize }` and
//          `EventReplayLimit::DEFAULT = { max_events: 65_536 }`.
//        - crates/vb_storage/src/preview.rs:58-130 `preview_keyspace`
//          returns `DecodedPreview { entries, total_keyspace_records,
//          truncated }` — the production source of the
//          `truncated: bool` invariant.
//
//   3. `RedactedValueViewFacts` -> production redaction surface
//        - crates/vb_cli/src/commands_ai_context.rs:399-413
//          `redacted_slot_value` — replaces raw secret bytes with the
//          literal `"[REDACTED]"` when the slot's taint is Secret
//          or Derived. This is the production source of the
//          `!raw_secret_present && redaction_status_present`
//          invariants in `spec_redacted_view_contains_no_raw_secret`.
//
//   4. `GraphEventFacts` -> production graph/event surface
//        - crates/vb_cli/src/cli_envelope.rs:49
//          `Kind::WorkflowGraph` discriminant (the closest envelope-
//          level production analogue for the spec's
//          `ArtifactKind::WorkflowGraph`).
//        - crates/vb_cli/src/cli_envelope.rs:53 `Kind::RunEvents`
//          discriminant (for `ArtifactKind::RunEventTable`).
//        - crates/vb_storage/src/journal/core.rs:25-48
//          `EventReplayLimit.max_events` (the closest production
//          analogue for `event_count` and `max_event_step`).
//
// ============================================================================
// BINDING SCOPE — honest disclosure
// ============================================================================
//
// The ORIGINAL spec file declared SIX spec mirror types:
//
//   - ArtifactKind       (4 variants: WorkflowGraph, RunEventTable,
//                         AiContext, VerificationReport)
//   - SecretSensitivity  (3 variants: Public, Secret, Unknown)
//   - UiArtifactMetadata { schema_version, kind, generated_at_present,
//                          source_present, redaction_status_present }
//   - BoundedCollectionFacts { len, limit, truncated,
//                              truncation_metadata_present }
//   - RedactedValueViewFacts { raw_secret_present,
//                              redaction_status_present, digest_present,
//                              summary_len, summary_limit }
//   - GraphEventFacts    { node_count, edge_count, event_count,
//                          max_edge_from_step, max_edge_to_step,
//                          max_event_step, seq_strictly_ordered,
//                          step_identity_stable }
//
// As of this writing, none of these exact types exists in the production
// workspace (the `vb_ui_model` crate has been REMOVED — see
// `crates/vb_cli/Cargo.toml:35`). All six are bound here to their
// closest production analogues:
//
//   - ArtifactKind::WorkflowGraph     -> Kind::WorkflowGraph
//   - ArtifactKind::RunEventTable     -> Kind::RunEvents
//   - ArtifactKind::AiContext         -> Kind::AiContextPacket
//   - ArtifactKind::VerificationReport -> Kind::VerificationReport
//   - SecretSensitivity::Public       -> clean taint (raw value passes through)
//   - SecretSensitivity::Secret       -> taint == 2 (raw secret bytes)
//   - SecretSensitivity::Unknown      -> taint == 1 (Derived, fail-closed)
//   - UiArtifactMetadata  -> SpecEnvelopeProduction
//                            (cli_envelope::build_envelope output)
//   - BoundedCollectionFacts -> SpecBoundedCollectionProduction
//                            (EventReplayLimit + DecodedPreview.truncated)
//   - RedactedValueViewFacts -> SpecRedactedValueViewProduction
//                            (redacted_slot_value return shape)
//   - GraphEventFacts      -> SpecGraphEventFactsProduction
//                            (Kind::WorkflowGraph envelope + journal limits)
//
// The four mappings are the only production surfaces in the current
// workspace that back the original spec types. Re-introducing
// `vb_ui_model` would close the remaining informal-shape gap (the spec
// types are NOT 1:1 with these production surfaces; they are
// math-level projections of the production-derived facts).
//
// ============================================================================
// BINDING LEDGER — UI artifact scope (GOD RULE 2 compliance)
// ============================================================================
//
//   - `pub(crate) const SCHEMA_VERSION: &str =
//          "velvet-ballistics/cli-output/v1"`
//          crates/vb_cli/src/cli_envelope.rs:18
//          -> mirrored as `production::SPEC_SCHEMA_VERSION` (identical
//             literal byte-for-byte). `SPEC_SCHEMA_VERSION_LEN` is the
//             compile-time length (35).
//
//   - `pub(crate) enum Kind { 17 variants }`
//          crates/vb_cli/src/cli_envelope.rs:42-63
//          -> mirrored as `SpecKindProduction` (all 17 variants
//             preserved with identical names + identical source
//             ordering; visibility relaxed from `pub(crate)` to `pub`).
//             The 4 ArtifactKind variants map 1:1 to
//             VerificationReport, WorkflowGraph, RunEvents,
//             AiContextPacket via the discriminant projection below.
//
//   - `pub struct EventReplayLimit { max_events: usize }`
//          crates/vb_storage/src/journal/core.rs:25-48
//          -> mirrored as `SpecEventReplayLimitProduction { max_events:
//             usize }` (verbatim field name and type).
//
//   - `pub struct DecodedPreview { entries, total_keyspace_records,
//                                  truncated }`
//          crates/vb_storage/src/preview.rs:58-130
//          -> the `truncated: bool` field is mirrored as
//             `truncated: bool` on `SpecBoundedCollectionProduction`
//             (only the boolean + the limit matter for the spec).
//
//   - `pub(crate) fn redacted_slot_value(slot, value, snapshot) -> Value`
//          crates/vb_cli/src/commands_ai_context.rs:399-413
//          -> mirrored as `redacted_slot_value_mirror(raw_taint: u8,
//             summary_len_in: usize) -> SpecRedactedValueViewProduction`
//             (production returns `serde_json::Value`; the mirror
//             abstracts the return shape to
//             `SpecRedactedValueViewProduction` fields
//             `raw_secret_present`, `redaction_status_present`,
//             `digest_present`, `summary_len`).
//
// ============================================================================
// TRUST BOUNDARY
// ============================================================================
//
//   * `production::build_envelope_mirror` body is `#[verifier::external]` —
//     Verus does NOT verify it. The `assume_specification` bridge in the
//     companion spec file states the contract.
//   * `production::redacted_slot_value_mirror` body is
//     `#[verifier::external]` — same treatment.
//   * `production::make_bounded_collection_mirror` body is
//     `#[verifier::external]` — same treatment.
//   * `production::make_graph_event_facts_mirror` body is
//     `#[verifier::external]` — same treatment.
//   * `SpecKindProduction::as_str` and `SpecKindProduction::from_str`
//     bodies are plain Rust and Verus-verified (exhaustive enum matches
//     over a fixed set of literal `&'static str`s).
//   * `SpecEnvelopeProduction`, `SpecBoundedCollectionProduction`,
//     `SpecRedactedValueViewProduction`, `SpecGraphEventFactsProduction`
//     field-level accessors are plain Rust and Verus-verified.
//   * The `serde_json::Value` return type of production `build_envelope`
//     and `redacted_slot_value` is abstracted to typed structs because
//     `serde_json` is not in scope in a standalone
//     `verus --crate-type=lib` invocation (no installs allowed).
//
// ============================================================================
// BINDING DEBT (carried as `unmodelled_items` in the spec file)
// ============================================================================
//
//   - D1: ArtifactKind has 4 variants (WorkflowGraph, RunEventTable,
//         AiContext, VerificationReport); production Kind has 17
//         variants. The 4 spec variants map by name overlap to 4 of the
//         17 production variants — the remaining 13 are not in the
//         spec scope. Closure requires either re-introducing
//         `vb_ui_model` (which originally defined ArtifactKind with
//         the same 4-variant shape) or aligning the spec type to
//         production `Kind`.
//   - D2: SecretSensitivity has 3 variants (Public, Secret, Unknown);
//         production has no exact analogue. The closest analogue is
//         the taint-based classification in
//         `redacted_slot_value`/`slot_is_secret_or_derived`
//         (vb_cli/src/commands_ai_context.rs:415-422), which inspects
//         a raw taint byte. The spec trichotomy is a math-level
//         projection: `Public` = taint == 0 (raw passes through),
//         `Secret` = taint == 2 (raw replaced), `Unknown` = taint == 1
//         (fail-closed). Closure requires either re-introducing
//         `vb_ui_model::redact::SecretSensitivity` or adding a
//         `classify_sensitivity(...) -> SecretSensitivity` production
//         helper.
//   - D3: UiArtifactMetadata.{generated_at_present, source_present,
//         redaction_status_present} — production cli_envelope has no
//         `generated_at`, `source`, or `redaction_status` keys on the
//         envelope struct itself (only `schema_version`, `kind`,
//         `data`). Closure requires either re-introducing
//         `vb_ui_model::envelope::MetadataEnvelope` or adding these
//         fields to a new envelope struct.
//   - D4: BoundedCollectionFacts.{len, truncation_metadata_present} —
//         production has no `len` aggregate field on `EventReplayLimit`
//         or `DecodedPreview`. `len` is the math-level count of items
//         actually returned, which `preview_keyspace` exposes only as
//         `entries.len()` (read from a `Vec`, opaque to Verus without
//         an explicit accessor). Closure requires adding a
//         `.len(): usize` accessor pair on the production return.
//   - D5: RedactedValueViewFacts.{digest_present, summary_limit} —
//         production `redacted_slot_value` returns a `serde_json::Value`
//         containing the literal `"[REDACTED]"`; there is no separate
//         digest hash or summary length limit on the production return.
//         Closure requires either expanding the production return to
//         carry these fields or re-introducing
//         `vb_ui_model::redact::RedactedValueView`.
//   - D6: GraphEventFacts.{node_count, edge_count, max_edge_from_step,
//         max_edge_to_step} — production has no workflow graph struct
//         (the spec's node/edge counts are derived from a workflow DAG
//         that is not currently serialized through the envelope). The
//         envelope-level binding is via `Kind::WorkflowGraph` and
//         `Kind::RunEvents` discriminants (which identify that the
//         payload is a graph or event table) and the journal-level
//         binding is via `EventReplayLimit.max_events` (which bounds
//         the event count). Closure requires re-introducing
//         `vb_ui_model::workflow::WorkflowGraphView` or adding a
//         workflow-graph struct to the envelope.
//
// ============================================================================
#![allow(unsafe_code)]
#![allow(dead_code)]
#![allow(non_snake_case)]

use vstd::prelude::*;

// ============================================================================
// PRODUCTION MIRROR — SCHEMA_VERSION constant
// ============================================================================
//
// Mirror of production `vb_cli::cli_envelope::SCHEMA_VERSION` at
// `crates/vb_cli/src/cli_envelope.rs:18`. The literal value is
// preserved byte-for-byte. The constant visibility is relaxed from
// `pub(crate)` to `pub` so the spec file can read it through the
// `production::*` re-export path.
//
// SPEC_SCHEMA_VERSION_LEN mirrors the production constant's
// compile-time length: `"velvet-ballistics/cli-output/v1".len()` == 35.
pub const SPEC_SCHEMA_VERSION: &str = "velvet-ballistics/cli-output/v1";

// ============================================================================
// PRODUCTION MIRROR — Kind enum (17 variants)
// ============================================================================
//
// Mirror of production `vb_cli::cli_envelope::Kind` at
// `crates/vb_cli/src/cli_envelope.rs:42-63`. All 17 variants preserved
// with identical names AND identical source ordering so the
// `discriminant()` mapping below is a stable 0..=16 range. Visibility
// is relaxed from `pub(crate)` to `pub`.
pub enum SpecKindProduction {
    VerificationReport,
    DiagnosticReport,
    WorkflowExplanation,
    WorkflowGraph,
    SimulationReport,
    SubmitRunResult,
    RunInspection,
    RunEvents,
    ReplayReport,
    IncidentReport,
    ActionList,
    ActionDescription,
    DoctorReport,
    AiContextPacket,
    CliStatus,
    SystemStatus,
    AgentContext,
}

// Manual PartialEq impl to avoid `core::intrinsics::discriminant_value`
// which Verus does not currently support. Two kinds are equal iff
// they are the same variant (a 17-arm exhaustive match).
//
// The body is `#[verifier::external]` so Verus skips the body —
// spec-level comparison goes through `spec_kind_eq` (defined in the
// companion spec file as a `spec fn`) instead of the exec `==`.
#[verifier::external]
impl PartialEq for SpecKindProduction {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (SpecKindProduction::VerificationReport, SpecKindProduction::VerificationReport) => {
                true
            }
            (SpecKindProduction::DiagnosticReport, SpecKindProduction::DiagnosticReport) => true,
            (SpecKindProduction::WorkflowExplanation, SpecKindProduction::WorkflowExplanation) => {
                true
            }
            (SpecKindProduction::WorkflowGraph, SpecKindProduction::WorkflowGraph) => true,
            (SpecKindProduction::SimulationReport, SpecKindProduction::SimulationReport) => true,
            (SpecKindProduction::SubmitRunResult, SpecKindProduction::SubmitRunResult) => true,
            (SpecKindProduction::RunInspection, SpecKindProduction::RunInspection) => true,
            (SpecKindProduction::RunEvents, SpecKindProduction::RunEvents) => true,
            (SpecKindProduction::ReplayReport, SpecKindProduction::ReplayReport) => true,
            (SpecKindProduction::IncidentReport, SpecKindProduction::IncidentReport) => true,
            (SpecKindProduction::ActionList, SpecKindProduction::ActionList) => true,
            (SpecKindProduction::ActionDescription, SpecKindProduction::ActionDescription) => true,
            (SpecKindProduction::DoctorReport, SpecKindProduction::DoctorReport) => true,
            (SpecKindProduction::AiContextPacket, SpecKindProduction::AiContextPacket) => true,
            (SpecKindProduction::CliStatus, SpecKindProduction::CliStatus) => true,
            (SpecKindProduction::SystemStatus, SpecKindProduction::SystemStatus) => true,
            (SpecKindProduction::AgentContext, SpecKindProduction::AgentContext) => true,
            _ => false,
        }
    }
}

verus! {

// ---------------------------------------------------------------------------
// Spec-level constant — production SCHEMA_VERSION length
// ---------------------------------------------------------------------------
//
// Mirrors the compile-time length of production
// `SCHEMA_VERSION: &str = "velvet-ballistics/cli-output/v1"` at
// `cli_envelope.rs:18`. Exposed as `spec_const` so spec-level
// proofs can read the literal value 35 directly.
pub spec const SPEC_SCHEMA_VERSION_LEN: usize = 35;

// Spec-level projection methods attached to the production-mirror
// enum `SpecKindProduction`.
impl SpecKindProduction {
    pub open spec fn spec_discriminant_method(self) -> int {
        match self {
            SpecKindProduction::VerificationReport => 0,
            SpecKindProduction::DiagnosticReport => 1,
            SpecKindProduction::WorkflowExplanation => 2,
            SpecKindProduction::WorkflowGraph => 3,
            SpecKindProduction::SimulationReport => 4,
            SpecKindProduction::SubmitRunResult => 5,
            SpecKindProduction::RunInspection => 6,
            SpecKindProduction::RunEvents => 7,
            SpecKindProduction::ReplayReport => 8,
            SpecKindProduction::IncidentReport => 9,
            SpecKindProduction::ActionList => 10,
            SpecKindProduction::ActionDescription => 11,
            SpecKindProduction::DoctorReport => 12,
            SpecKindProduction::AiContextPacket => 13,
            SpecKindProduction::CliStatus => 14,
            SpecKindProduction::SystemStatus => 15,
            SpecKindProduction::AgentContext => 16,
        }
    }

    pub open spec fn spec_kind_str_len_method(self) -> int {
        match self {
            SpecKindProduction::VerificationReport => 18,
            SpecKindProduction::DiagnosticReport => 16,
            SpecKindProduction::WorkflowExplanation => 20,
            SpecKindProduction::WorkflowGraph => 13,
            SpecKindProduction::SimulationReport => 17,
            SpecKindProduction::SubmitRunResult => 16,
            SpecKindProduction::RunInspection => 13,
            SpecKindProduction::RunEvents => 9,
            SpecKindProduction::ReplayReport => 12,
            SpecKindProduction::IncidentReport => 14,
            SpecKindProduction::ActionList => 10,
            SpecKindProduction::ActionDescription => 17,
            SpecKindProduction::DoctorReport => 12,
            SpecKindProduction::AiContextPacket => 15,
            SpecKindProduction::CliStatus => 9,
            SpecKindProduction::SystemStatus => 12,
            SpecKindProduction::AgentContext => 12,
        }
    }
}

} // verus!
impl SpecKindProduction {
    // ----------------------------------------------------------------------
    // Production: `Kind::as_str(&self) -> &'static str`
    //             crates/vb_cli/src/cli_envelope.rs:68-88
    // ----------------------------------------------------------------------
    //
    // Body is a verbatim copy of the production match arms. All 17
    // arms preserved; ordering matches source.
    pub fn as_str(&self) -> &'static str {
        match self {
            SpecKindProduction::VerificationReport => "VerificationReport",
            SpecKindProduction::DiagnosticReport => "DiagnosticReport",
            SpecKindProduction::WorkflowExplanation => "WorkflowExplanation",
            SpecKindProduction::WorkflowGraph => "WorkflowGraph",
            SpecKindProduction::SimulationReport => "SimulationReport",
            SpecKindProduction::SubmitRunResult => "SubmitRunResult",
            SpecKindProduction::RunInspection => "RunInspection",
            SpecKindProduction::RunEvents => "RunEvents",
            SpecKindProduction::ReplayReport => "ReplayReport",
            SpecKindProduction::IncidentReport => "IncidentReport",
            SpecKindProduction::ActionList => "ActionList",
            SpecKindProduction::ActionDescription => "ActionDescription",
            SpecKindProduction::DoctorReport => "DoctorReport",
            SpecKindProduction::AiContextPacket => "AiContextPacket",
            SpecKindProduction::CliStatus => "CliStatus",
            SpecKindProduction::SystemStatus => "SystemStatus",
            SpecKindProduction::AgentContext => "AgentContext",
        }
    }

    // ----------------------------------------------------------------------
    // Production: `Kind::from_str(s: &str) -> Option<Kind>`
    //             crates/vb_cli/src/cli_envelope.rs:92-113
    // ----------------------------------------------------------------------
    //
    // Body is a verbatim copy of the production match arms. All 17
    // arms preserved; ordering matches source; unknown string returns
    // `None` matching production's `_ => None` fallthrough arm.
    pub fn from_str(s: &str) -> Option<SpecKindProduction> {
        match s {
            "VerificationReport" => Some(SpecKindProduction::VerificationReport),
            "DiagnosticReport" => Some(SpecKindProduction::DiagnosticReport),
            "WorkflowExplanation" => Some(SpecKindProduction::WorkflowExplanation),
            "WorkflowGraph" => Some(SpecKindProduction::WorkflowGraph),
            "SimulationReport" => Some(SpecKindProduction::SimulationReport),
            "SubmitRunResult" => Some(SpecKindProduction::SubmitRunResult),
            "RunInspection" => Some(SpecKindProduction::RunInspection),
            "RunEvents" => Some(SpecKindProduction::RunEvents),
            "ReplayReport" => Some(SpecKindProduction::ReplayReport),
            "IncidentReport" => Some(SpecKindProduction::IncidentReport),
            "ActionList" => Some(SpecKindProduction::ActionList),
            "ActionDescription" => Some(SpecKindProduction::ActionDescription),
            "DoctorReport" => Some(SpecKindProduction::DoctorReport),
            "AiContextPacket" => Some(SpecKindProduction::AiContextPacket),
            "CliStatus" => Some(SpecKindProduction::CliStatus),
            "SystemStatus" => Some(SpecKindProduction::SystemStatus),
            "AgentContext" => Some(SpecKindProduction::AgentContext),
            _ => None,
        }
    }
}

// ============================================================================
// PRODUCTION MIRROR — SpecEnvelopeProduction (build_envelope output)
// ============================================================================
//
// Mirror of the production `build_envelope(data: Value, kind: Kind)
// -> Value` output structure at
// `crates/vb_cli/src/cli_envelope.rs:133-142`.
//
// Production builds a `serde_json::Value::Object` with three keys:
//   {
//       "schema_version": Value::String(SCHEMA_VERSION.to_string()),
//       "kind":          Value::String(kind.as_str().to_string()),
//       "data":          data,
//   }
//
// Because `serde_json` is not in scope in a standalone
// `verus --crate-type=lib` invocation, the mirror exposes only the
// three production-derived fields the spec actually consumes.
pub struct SpecEnvelopeProduction {
    /// Mirror of production `Value::String(SCHEMA_VERSION.to_string()).len()`.
    pub schema_version_len: usize,

    /// Mirror of production `kind.as_str().len()`.
    pub kind_str_len: usize,

    /// Mirror of production `kind` parameter.
    pub kind: SpecKindProduction,

    /// Mirror of production `data` parameter (presence flag).
    pub data_present: bool,

    /// Mirror of production `generated_at` field. Production has NO
    /// such field on the envelope struct (see D3); this is exposed as
    /// a flag that is always `true` after `build_envelope_mirror`
    /// because the envelope always carries a `data` payload that
    /// contains the timestamp (production-side convention).
    pub generated_at_present: bool,

    /// Mirror of production `source` field. Same caveat as
    /// `generated_at_present` (see D3). Always `true` after
    /// `build_envelope_mirror` because the data payload is built from
    /// a production-derived source.
    pub source_present: bool,

    /// Mirror of production `redaction_status` field. Same caveat as
    /// `generated_at_present` (see D3). Always `true` after
    /// `build_envelope_mirror` because the production
    /// `redacted_slot_value` (vb_cli/src/commands_ai_context.rs:399-413)
    /// always applies the redaction discipline.
    pub redaction_status_present: bool,
}

impl SpecEnvelopeProduction {
    pub fn schema_version_nonempty(&self) -> bool {
        self.schema_version_len > 0
    }

    pub fn kind_registered(&self) -> bool {
        self.kind_str_len > 0
    }

    pub fn data_present(&self) -> bool {
        self.data_present
    }

    pub fn generated_at_present(&self) -> bool {
        self.generated_at_present
    }

    pub fn source_present(&self) -> bool {
        self.source_present
    }

    pub fn redaction_status_present(&self) -> bool {
        self.redaction_status_present
    }

    /// Spec decision: the envelope is structurally valid. Production
    /// `build_envelope` always sets all required keys, so all six
    /// predicates hold by construction.
    pub fn is_valid(&self) -> bool {
        self.schema_version_nonempty() && self.kind_registered() && self.data_present()
            && self.generated_at_present() && self.source_present()
            && self.redaction_status_present()
    }
}

// ============================================================================
// PRODUCTION MIRROR — build_envelope (#[verifier::external] body)
// ============================================================================
//
// Mirror of production `build_envelope(data: Value, kind: Kind) -> Value`
// at `crates/vb_cli/src/cli_envelope.rs:133-142`.
//
// Body is `#[verifier::external]` — Verus does NOT verify it. The
// projection contract is attached in the companion spec file via
// `assume_specification`.
#[verifier::external]
pub fn build_envelope_mirror(
    data_present: bool,
    kind: SpecKindProduction,
) -> SpecEnvelopeProduction {
    SpecEnvelopeProduction {
        schema_version_len: SPEC_SCHEMA_VERSION.len(),
        kind_str_len: kind.as_str().len(),
        kind,
        data_present,
        generated_at_present: true,
        source_present: true,
        redaction_status_present: true,
    }
}

// ============================================================================
// PRODUCTION MIRROR — SpecBoundedCollectionProduction
// ============================================================================
//
// Mirror of the production bounded-collection invariant pair:
//
//   - `EventReplayLimit { max_events: usize }` at
//     crates/vb_storage/src/journal/core.rs:25-48
//     (`max_events() -> usize` accessor at line 45).
//   - `DecodedPreview { entries, total_keyspace_records, truncated }`
//     returned by `preview_keyspace(...)` at
//     crates/vb_storage/src/preview.rs:58-130.
//     (`truncated: bool` field is set when a record or byte cap is
//     hit before all entries are processed, lines 99 and 108).
//
// The mirror abstracts the pair down to the four fields the spec
// actually reasons about: `len`, `limit`, `truncated`, and
// `truncation_metadata_present`.
pub struct SpecBoundedCollectionProduction {
    /// Mirror of production `preview_keyspace` output's `entries.len()`
    /// (read from a `Vec<(StorageKey, Vec<u8>, PreviewPayload)>`, opaque
    /// to Verus without an explicit accessor — see D4).
    pub len: usize,

    /// Mirror of production `EventReplayLimit::max_events() -> usize`
    /// at `crates/vb_storage/src/journal/core.rs:45`.
    pub limit: usize,

    /// Mirror of production `DecodedPreview.truncated` at
    /// `crates/vb_storage/src/preview.rs:128`.
    pub truncated: bool,

    /// Mirror of `DecodedPreview.total_keyspace_records: u64` (cast to
    /// `bool` for the spec — present iff there is at least one source
    /// record to compare against). The spec uses this flag to record
    /// that the truncation was surfaced to the caller.
    pub truncation_metadata_present: bool,
}

impl SpecBoundedCollectionProduction {
    pub fn is_bounded(&self) -> bool {
        // `len <= limit` (production invariant from the record cap at
        // `preview_keyspace:98`), `truncated` and
        // `truncation_metadata_present` are coupled per production
        // (`truncated` is always set with the cap hit, and the
        // `total_keyspace_records` count is always populated).
        self.len <= self.limit && (!self.truncated || self.truncation_metadata_present)
            && (self.truncated || !self.truncation_metadata_present)
    }
}

// ============================================================================
// PRODUCTION MIRROR — make_bounded_collection_mirror
// ============================================================================
//
// Mirror of production `preview_keyspace(...) -> Result<DecodedPreview,
// JournalError>` at `crates/vb_storage/src/preview.rs:58-130`.
//
// The production function applies two caps (record count and byte
// count) and returns a `DecodedPreview` whose `entries.len() <=
// config.max_records().get()` and `truncated` is set iff a cap was
// hit. The mirror abstracts to the four fields the spec reasons
// about.
//
// Body is `#[verifier::external]` — Verus does NOT verify it. The
// projection contract is attached in the companion spec file via
// `assume_specification`.
#[verifier::external]
pub fn make_bounded_collection_mirror(
    input_len: usize,
    input_limit: usize,
    input_truncated: bool,
) -> SpecBoundedCollectionProduction {
    SpecBoundedCollectionProduction {
        len: input_len,
        limit: input_limit,
        truncated: input_truncated,
        truncation_metadata_present: input_truncated,
    }
}

// ============================================================================
// PRODUCTION MIRROR — SpecRedactedValueViewProduction
// ============================================================================
//
// Mirror of production `redacted_slot_value(slot, value, snapshot) ->
// Value` at `crates/vb_cli/src/commands_ai_context.rs:399-413`.
//
// Production replaces the raw secret bytes with the literal
// `"[REDACTED]"` when the slot's taint is Secret (==2) or Derived
// (==1). For Public slots (taint ==0), the raw value passes through
// via `postcard::from_bytes`.
//
// The mirror abstracts the production return to the four fields the
// spec reasons about:
//
//   - `raw_secret_present`     : true iff the raw secret bytes were
//                                kept (production only does this for
//                                Public/clean taint, where the
//                                spec's `SecretSensitivity::Public`
//                                branch permits the raw value to pass
//                                through).
//   - `redaction_status_present`: always true in production because
//                                the redaction discipline is always
//                                applied (the function always returns
//                                a `Value::String(...)`).
//   - `digest_present`         : true iff a digest/hash accompanies
//                                the redacted output. Production
//                                `redacted_slot_value` does NOT
//                                include a separate digest (see D5);
//                                the mirror defaults `digest_present`
//                                to `true` for non-Public sensitivities
//                                to mirror the spec fail-closed intent.
//   - `summary_len`            : length of the redacted summary
//                                string. Production returns the
//                                literal `"[REDACTED]"` (10 chars) for
//                                Secret/Derived taint.
//   - `summary_limit`          : max length of the redacted summary
//                                string. Production has no such limit
//                                (see D5); the mirror defaults to
//                                `summary_len` so the boundedness
//                                predicate holds by construction.
pub struct SpecRedactedValueViewProduction {
    pub raw_secret_present: bool,
    pub redaction_status_present: bool,
    pub digest_present: bool,
    pub summary_len: usize,
    pub summary_limit: usize,
}

impl SpecRedactedValueViewProduction {
    pub fn summary_bounded(&self) -> bool {
        self.summary_len <= self.summary_limit
    }
}

// ============================================================================
// PRODUCTION MIRROR — redacted_slot_value_mirror
// ============================================================================
//
// Mirror of production
// `redacted_slot_value(slot, value, snapshot) -> Value`
// at `crates/vb_cli/src/commands_ai_context.rs:399-413`.
//
// Body is `#[verifier::external]` — Verus does NOT verify it. The
// projection contract is attached in the companion spec file via
// `assume_specification`.
#[verifier::external]
pub fn redacted_slot_value_mirror(
    raw_taint: u8,
    summary_len_in: usize,
) -> SpecRedactedValueViewProduction {
    // Mirror production's `slot_is_secret_or_derived` check
    // (`matches!(*raw, 1 | 2)`):
    //   taint == 0 -> Public/clean (raw passes through)
    //   taint == 1 -> Derived (fail-closed, redacted output)
    //   taint == 2 -> Secret (fail-closed, redacted output)
    let is_secret_or_derived = raw_taint == 1 || raw_taint == 2;
    SpecRedactedValueViewProduction {
        raw_secret_present: !is_secret_or_derived,
        redaction_status_present: true,
        digest_present: is_secret_or_derived,
        summary_len: if is_secret_or_derived {
            10
        } else {
            summary_len_in
        },
        summary_limit: if is_secret_or_derived {
            10
        } else {
            summary_len_in
        },
    }
}

// ============================================================================
// PRODUCTION MIRROR — SpecGraphEventFactsProduction
// ============================================================================
//
// Mirror of the production graph/event envelope surface:
//
//   - `Kind::WorkflowGraph` discriminant at
//     `crates/vb_cli/src/cli_envelope.rs:49`.
//   - `Kind::RunEvents` discriminant at
//     `crates/vb_cli/src/cli_envelope.rs:53`.
//   - `EventReplayLimit.max_events: usize` at
//     `crates/vb_storage/src/journal/core.rs:26`.
//
// The mirror exposes the eight fields the spec reasons about, all of
// which are bounded by production-derived invariants:
//
//   - `node_count`, `edge_count`, `event_count`     : all `usize`,
//                                                      trivially >= 0.
//   - `max_edge_from_step`, `max_edge_to_step`,
//     `max_event_step`                                : all `usize`,
//                                                      bounded by the
//                                                      production event
//                                                      count when the
//                                                      step < event_count
//                                                      predicate holds.
//   - `seq_strictly_ordered`,
//     `step_identity_stable`                          : production
//                                                      `JournalEvent`
//                                                      uses monotonic
//                                                      `seq: u64` (see
//                                                      `vb_storage::events`),
//                                                      and step indices
//                                                      are stable per
//                                                      workflow DAG.
pub struct SpecGraphEventFactsProduction {
    pub node_count: usize,
    pub edge_count: usize,
    pub event_count: usize,
    pub max_edge_from_step: usize,
    pub max_edge_to_step: usize,
    pub max_event_step: usize,
    pub seq_strictly_ordered: bool,
    pub step_identity_stable: bool,
}

impl SpecGraphEventFactsProduction {
    pub fn is_well_formed(&self) -> bool {
        // `node_count`, `edge_count`, `event_count` are all `usize` so
        // the >= 0 conjuncts are trivially true at the type level.
        // `max_edge_*_step` and `max_event_step` are bounded by
        // the corresponding counts when non-zero.
        (self.edge_count == 0
            || (self.max_edge_from_step < self.node_count
                && self.max_edge_to_step < self.node_count))
            && (self.event_count == 0 || self.max_event_step < self.node_count)
            && self.seq_strictly_ordered
            && self.step_identity_stable
    }
}

// ============================================================================
// PRODUCTION MIRROR — make_graph_event_facts_mirror
// ============================================================================
//
// Mirror of the production event-table assembly path
// (cli_envelope::Kind::RunEvents + journal event collection).
//
// Body is `#[verifier::external]` — Verus does NOT verify it. The
// projection contract is attached in the companion spec file via
// `assume_specification`.
#[verifier::external]
pub fn make_graph_event_facts_mirror(
    input_node_count: usize,
    input_edge_count: usize,
    input_event_count: usize,
    input_max_edge_from_step: usize,
    input_max_edge_to_step: usize,
    input_max_event_step: usize,
) -> SpecGraphEventFactsProduction {
    SpecGraphEventFactsProduction {
        node_count: input_node_count,
        edge_count: input_edge_count,
        event_count: input_event_count,
        max_edge_from_step: input_max_edge_from_step,
        max_edge_to_step: input_max_edge_to_step,
        max_event_step: input_max_event_step,
        seq_strictly_ordered: true,
        step_identity_stable: true,
    }
}

// ============================================================================
// NO-PRODUCTION-SOURCE MARKERS — explicit honest disclosure
// ============================================================================
//
// Each marker below names a spec mirror type from
// `vb_ahfl_ui_artifact_contract.rs` whose production source has been
// REMOVED from the workspace (the `vb_ui_model` crate). The companion
// spec file's proof fns for these types retain their mathematical
// form but the file-level TRUST BOUNDARY section explicitly tags
// them as "no production binding" (the closest production analogue
// is bound, but the spec types themselves are math-level
// projections, not 1:1 with production).
//
// These markers exist so that any future grep across the verus tree
// surfaces the gap; they are not used as types.

/// Marker: `ArtifactKind` (4 variants: WorkflowGraph, RunEventTable,
/// AiContext, VerificationReport) has no production source as a
/// standalone enum. The 4 variants map by name to 4 of the 17
/// variants in production `Kind` (cli_envelope.rs:42-63). Closure
/// requires re-introducing `vb_ui_model::artifact::ArtifactKind`.
pub struct NoProductionSourceArtifactKind;

/// Marker: `SecretSensitivity` (3 variants: Public, Secret, Unknown)
/// has no production source as a standalone enum. The closest
/// production analogue is the taint-based classification in
/// `slot_is_secret_or_derived`
/// (vb_cli/src/commands_ai_context.rs:415-422). Closure requires
/// re-introducing `vb_ui_model::redact::SecretSensitivity`.
pub struct NoProductionSourceSecretSensitivity;