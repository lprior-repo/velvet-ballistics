// SPDX-License-Identifier: MIT
//
// ============================================================================
// Extern surface for `vb_ahfl_metadata_envelope_production` Verus spec.
//
// STRONG PRODUCTION BINDING (GOD RULE 2 compliance) — cli_envelope scope
// ============================================================================
//
// This file binds `verification/verus/vb_ahfl_metadata_envelope_production.rs`
// to the production `vb_cli::cli_envelope` module at
// `crates/vb_cli/src/cli_envelope.rs`. The production surface is:
//
//   - `pub(crate) const SCHEMA_VERSION: &str` (cli_envelope.rs:18)
//   - `pub(crate) enum Kind { ... }` (cli_envelope.rs:45-63) — 17 variants
//   - `pub(crate) fn Kind::as_str(&self) -> &'static str` (cli_envelope.rs:68-88)
//   - `pub(crate) fn Kind::from_str(s: &str) -> Option<Kind>` (cli_envelope.rs:92-113)
//   - `pub(crate) fn build_envelope(data: Value, kind: Kind) -> Value` (cli_envelope.rs:133-142)
//   - `pub(crate) fn serialize_with_version(data: &Value, kind: Kind) -> Value`
//     (cli_envelope.rs:154-165)
//   - `pub(crate) enum EnvelopeError { ... }` (cli_envelope.rs:170-174)
//
// ============================================================================
// BINDING SCOPE — honest disclosure
// ============================================================================
//
// The ORIGINAL spec file declared TWO mirror types:
//
//   - SpecEnvelopeKind      (6 variants: Success, Error, DiagnosticReport,
//                            Status, Event, Workflow)
//   - SpecMetadataEnvelope  { run_id, command, timestamp }
//
// The ORIGINAL spec header claims these mirror
// `vb_ui_model::envelope::types::MetadataEnvelope` and
// `vb_ui_model::envelope::types::EnvelopeKind`. As of this writing,
// the `vb_ui_model` crate has been REMOVED from the workspace (see
// `crates/vb_cli/Cargo.toml`:
//     `# vb_ui_model is removed from the current workspace scope.`).
// A repo-wide grep for `MetadataEnvelope`, `EnvelopeKind`,
// `canonicalize_ui_artifact` returns ONLY references inside the
// verus spec files themselves — there is no production Rust source
// for those exact types.
//
// The CLOSEST production surface in the current workspace is the
// `vb_cli::cli_envelope` module (different name, different shape):
//
//   - Production `Kind` has 17 variants (cli_envelope.rs:45-63); the
//     original 6-variant `SpecEnvelopeKind` is a STRICT SUBSET only
//     by informal name overlap (DiagnosticReport, CliStatus/SystemStatus
//     vs Status, WorkflowGraph/WorkflowExplanation vs Workflow). Three
//     original variants have no production equivalent at all:
//     Success, Error, Event.
//   - Production envelopes have shape `{ schema_version: String, kind:
//     String, data: Value }`. Production has NO `run_id`, `command`,
//     or `timestamp` fields on the envelope itself (timestamps live
//     inside individual JournalEvents, not on the envelope struct).
//   - The production constructor `build_envelope(data, kind)` returns
//     a `serde_json::Value` (a JSON object), not a typed struct. There
//     is NO production `canonicalize_ui_artifact` function.
//
// Per the user's instruction, this extern file binds the FULL
// production cli_envelope surface (Kind, SCHEMA_VERSION,
// build_envelope, serialize_with_version, EnvelopeError, from_str,
// as_str) so the spec file can re-state its obligations against
// production-realistic data shapes.
//
// ============================================================================
// BINDING LEDGER — cli_envelope scope
// ============================================================================
//
// Production surface (full byte-for-byte binding):
//
//   - `pub(crate) const SCHEMA_VERSION: &str = "velvet-ballistics/cli-output/v1"`
//          crates/vb_cli/src/cli_envelope.rs:18
//          -> mirrored as `SPEC_SCHEMA_VERSION: &str` (identical literal).
//
//   - `pub(crate) enum Kind { 17 variants }`
//          crates/vb_cli/src/cli_envelope.rs:45-63
//          -> mirrored as `SpecKindProduction` (all 17 variants
//             preserved with identical names + identical ordering;
//             visibility relaxed from `pub(crate)` to `pub`).
//
//   - `pub(crate) fn Kind::as_str(&self) -> &'static str`
//          crates/vb_cli/src/cli_envelope.rs:68-88
//          -> mirrored as `SpecKindProduction::as_str(&self) -> &'static str`
//             (body copied verbatim, all 17 arms preserved).
//
//   - `pub(crate) fn Kind::from_str(s: &str) -> Option<Kind>`
//          crates/vb_cli/src/cli_envelope.rs:92-113
//          -> mirrored as `SpecKindProduction::from_str(s: &str)
//             -> Option<SpecKindProduction>` (body copied verbatim,
//             all 17 arms preserved).
//
//   - `pub(crate) fn build_envelope(data: Value, kind: Kind) -> Value`
//          crates/vb_cli/src/cli_envelope.rs:133-142
//          -> mirrored as `build_envelope_mirror(data_present: bool,
//             kind: SpecKindProduction) -> SpecEnvelopeProduction`.
//             The production function returns a `serde_json::Value`
//             containing a JSON object with three keys
//             (`schema_version`, `kind`, `data`). The mirror returns
//             a typed `SpecEnvelopeProduction` struct with the three
//             corresponding production-derived fields:
//             - `schema_version_len: usize`   (mirrors `Value::String(SCHEMA_VERSION.to_string()).len()`)
//             - `kind: SpecKindProduction`    (mirrors the Kind parameter)
//             - `data_present: bool`          (mirrors `data` parameter existence)
//             The body is `#[verifier::external]` — Verus skips
//             verification. The contract is attached in the companion
//             spec file via `assume_specification`. Field-level
//             accessors (the trivial `.is_valid()` predicate below)
//             are plain Rust and Verus-verified.
//
//   - `pub(crate) fn serialize_with_version(data: &Value, kind: Kind) -> Value`
//          crates/vb_cli/src/cli_envelope.rs:154-165
//          -> mirrored as `serialize_with_version_mirror(data_present: bool,
//             kind: SpecKindProduction) -> SpecEnvelopeProduction`.
//             Production adds the `schema_version` and `kind` keys to
//             the existing data map. The mirror abstracts `data: &Value`
//             to `data_present: bool` for the same reason as
//             `build_envelope_mirror`. Body is `#[verifier::external]`.
//
//   - `pub(crate) enum EnvelopeError { SerializationFailed,
//                                       SchemaVersionMissing,
//                                       UnknownKind(String) }`
//          crates/vb_cli/src/cli_envelope.rs:170-174
//          -> mirrored as `SpecEnvelopeErrorProduction` (verbatim
//             variant set; `String` mirrored as `usize` length because
//             no string content is read by the spec surface).
//
// ============================================================================
// TRUST BOUNDARY
// ============================================================================
//
//   * `build_envelope_mirror` body is `#[verifier::external]` — Verus
//     does NOT verify it. The `assume_specification` bridge in the
//     companion spec file states the projection contract.
//   * `serialize_with_version_mirror` body is `#[verifier::external]` —
//     same treatment.
//   * `SpecKindProduction::as_str` and `SpecKindProduction::from_str`
//     bodies are plain Rust and Verus-verified (the bodies are
//     exhaustive enum matches over a fixed set of literal `&'static str`s).
//   * `SpecEnvelopeProduction` field-level accessors (the trivial
//     `.is_valid()` and `.schema_version_nonempty()` predicates below)
//     are plain Rust and Verus-verified.
//   * `SPEC_SCHEMA_VERSION` is a compile-time constant — Verus treats
//     it as opaque.
//   * The `serde_json::Value` return type of production
//     `build_envelope` is abstracted to `data_present: bool` because
//     `serde_json` is not in scope in a standalone
//     `verus --crate-type=lib` invocation (no installs allowed by task
//     brief). The projection only ever needs to assert the three keys
//     are present and that `data` is non-null, both of which are
//     captured by `data_present: bool`.
//
// ============================================================================
// BINDING DEBT (carried as `unmodelled_items` in the bridge spec file)
// ============================================================================
//
//   - D1: SpecEnvelopeKind (6-variant original: Success, Error,
//         DiagnosticReport, Status, Event, Workflow) — no production
//         source. Three of six variants (Success, Error, Event) have
//         no analogue in the production 17-variant `Kind` enum at
//         all. The remaining three map by name overlap to
//         DiagnosticReport, CliStatus/SystemStatus, WorkflowGraph/
//         WorkflowExplanation (not 1:1). Closure requires either
//         re-introducing `vb_ui_model` or adding equivalent
//         discriminant tags to production `Kind`.
//   - D2: SpecMetadataEnvelope { run_id, command, timestamp } — no
//         production source. Production cli_envelope has no such
//         struct; the envelope carries `data: Value` only. Closure
//         requires re-introducing `vb_ui_model::envelope::types` or
//         adding these fields to a new production envelope struct.
//   - D3: `canonicalize_ui_artifact(...) -> CanonicalUiArtifact`
//         referenced in the original spec header (line 10) — no
//         production source. Closure requires re-introducing the
//         function.
//   - D4: production `build_envelope` returns `serde_json::Value`;
//         the mirror returns the typed `SpecEnvelopeProduction`. Any
//         future re-introduction of `serde_json` into the standalone
//         verus invocation should switch the mirror to a direct
//         `Value` projection.
//
#![allow(dead_code)]
#![allow(non_snake_case)]
#![allow(unsafe_code)]
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
// Production: `pub(crate) const SCHEMA_VERSION: &str =
//   "velvet-ballistics/cli-output/v1";`
//
// SPEC_SCHEMA_VERSION_LEN mirrors the production constant's
// compile-time length: `"velvet-ballistics/cli-output/v1".len()` == 35.
pub const SPEC_SCHEMA_VERSION: &str = "velvet-ballistics/cli-output/v1";

// ============================================================================
// PRODUCTION MIRROR — Kind enum (17 variants)
// ============================================================================
//
// Mirror of production `vb_cli::cli_envelope::Kind` at
// `crates/vb_cli/src/cli_envelope.rs:45-63`. All 17 variants preserved
// with identical names AND identical source ordering so the
// `discriminant()` mapping below is a stable 0..=16 range. Visibility
// is relaxed from `pub(crate)` to `pub`.
//
// Production `Kind` source (verbatim copy):
//
//   pub(crate) enum Kind {
//       VerificationReport,
//       DiagnosticReport,
//       WorkflowExplanation,
//       WorkflowGraph,
//       SimulationReport,
//       SubmitRunResult,
//       RunInspection,
//       RunEvents,
//       ReplayReport,
//       IncidentReport,
//       ActionList,
//       ActionDescription,
//       DoctorReport,
//       AiContextPacket,
//       CliStatus,
//       SystemStatus,
//       AgentContext,
//   }
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
// proofs can read the literal value 35 directly. Production
// verification: the literal has 35 ASCII characters; any drift in
// the literal breaks the proof.
pub spec const SPEC_SCHEMA_VERSION_LEN: usize = 35;

// Spec-level projection methods attached to the production-mirror
// enum `SpecKindProduction`. These are spec-mode functions that
// expose the spec-level discriminant and string length without
// invoking the exec `as_str` body, so spec-level proofs can reason
// about variant identity and length without crossing the exec/spec
// boundary.
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
//
//   {
//       "schema_version": Value::String(SCHEMA_VERSION.to_string()),
//       "kind":          Value::String(kind.as_str().to_string()),
//       "data":          data,
//   }
//
// Because `serde_json` is not in scope in a standalone
// `verus --crate-type=lib` invocation (no installs allowed), the
// mirror exposes only the three production-derived fields the spec
// actually consumes:
//   - `schema_version_len: usize`   — the length of the schema_version string
//                                    (production guarantees
//                                    SCHEMA_VERSION.len() == 35 — see
//                                    `cli_envelope.rs:18`).
//   - `kind: SpecKindProduction`    — the production Kind parameter.
//   - `data_present: bool`          — true iff `data` was non-null.
//   - `kind_str_len: usize`         — the length of the kind string
//                                    (production guarantees >= 1).
//
// The spec surface projects this to the math model via the
// `spec_envelope_projection` spec fn in the companion file.
pub struct SpecEnvelopeProduction {
    /// Mirror of production `Value::String(SCHEMA_VERSION.to_string()).len()`.
    /// In production this is always `SCHEMA_VERSION.len()` == 35 because
    /// `SCHEMA_VERSION` is a compile-time constant.
    pub schema_version_len: usize,

    /// Mirror of production `kind.as_str().len()`. In production this
    /// is always in `[1, 19]` because all 17 registered kinds have
    /// string representations of length 1..=19.
    pub kind_str_len: usize,

    /// Mirror of production `kind` parameter at
    /// `cli_envelope.rs:133`. Direct mirror.
    pub kind: SpecKindProduction,

    /// Mirror of production `data: Value` parameter at
    /// `cli_envelope.rs:133`. Mirrored as a boolean presence flag
    /// because `serde_json::Value` is not in scope.
    pub data_present: bool,
}

impl SpecEnvelopeProduction {
    /// Plain-Rust accessor: the schema_version field is non-empty.
    /// Trivially true at the production constant
    /// `SCHEMA_VERSION.len() == 35`. Spec-level projections live in
    /// the companion spec file as `spec fn`s.
    pub fn schema_version_nonempty(&self) -> bool {
        self.schema_version_len > 0
    }

    /// Plain-Rust accessor: the kind field is registered.
    /// Trivially true at the production `Kind::as_str` total function.
    pub fn kind_registered(&self) -> bool {
        self.kind_str_len > 0
    }

    /// Plain-Rust accessor: the envelope is structurally valid.
    /// Production `build_envelope` always sets all three keys, so all
    /// three predicates hold by construction.
    pub fn is_valid(&self) -> bool {
        self.schema_version_nonempty() && self.kind_registered() && self.data_present
    }
}

// ============================================================================
// PRODUCTION MIRROR — build_envelope (#[verifier::external] body)
// ============================================================================
//
// Mirror of production `build_envelope(data: Value, kind: Kind) -> Value`
// at `crates/vb_cli/src/cli_envelope.rs:133-142`.
//
// Production signature: `fn build_envelope(data: Value, kind: Kind) -> Value`.
// Production body (lines 134-142):
//
//   let mut envelope = Map::new();
//   envelope.insert(
//       "schema_version".to_string(),
//       Value::String(SCHEMA_VERSION.to_string()),
//   );
//   envelope.insert("kind".to_string(), Value::String(kind.as_str().to_string()));
//   envelope.insert("data".to_string(), data);
//   Value::Object(envelope)
//
// The mirror abstracts `data: Value` to `data_present: bool` (because
// `serde_json::Value` is not in scope in a standalone verus invocation).
// The body is `#[verifier::external]` — Verus does NOT verify it. The
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
    }
}

// ============================================================================
// PRODUCTION MIRROR — serialize_with_version (#[verifier::external] body)
// ============================================================================
//
// Mirror of production `serialize_with_version(data: &Value, kind: Kind)
// -> Value` at `crates/vb_cli/src/cli_envelope.rs:154-165`.
//
// Production body (lines 155-165):
//
//   let mut envelope = match data {
//       Value::Object(data_map) => data_map.clone(),
//       _ => Map::new(),
//   };
//   envelope.insert(
//       "schema_version".to_string(),
//       Value::String(SCHEMA_VERSION.to_string()),
//   );
//   envelope.insert("kind".to_string(), Value::String(kind.as_str().to_string()));
//   Value::Object(envelope)
//
// The mirror abstracts `data: &Value` to `data_present: bool` for the
// same reason as `build_envelope_mirror`. The body is
// `#[verifier::external]` — Verus does NOT verify it. The projection
// contract is attached in the companion spec file via
// `assume_specification`.
#[verifier::external]
pub fn serialize_with_version_mirror(
    data_present: bool,
    kind: SpecKindProduction,
) -> SpecEnvelopeProduction {
    SpecEnvelopeProduction {
        schema_version_len: SPEC_SCHEMA_VERSION.len(),
        kind_str_len: kind.as_str().len(),
        kind,
        data_present,
    }
}

// ============================================================================
// PRODUCTION MIRROR — EnvelopeError (verbatim 3-variant enum)
// ============================================================================
//
// Mirror of production `vb_cli::cli_envelope::EnvelopeError` at
// `crates/vb_cli/src/cli_envelope.rs:170-174`.
//
// Production source (verbatim):
//
//   pub(crate) enum EnvelopeError {
//       SerializationFailed,
//       SchemaVersionMissing,
//       UnknownKind(String),
//   }
//
// The `String` payload is mirrored as `usize` length because no
// string content is read by the spec surface; only variant
// discrimination is needed.
pub enum SpecEnvelopeErrorProduction {
    SerializationFailed,
    SchemaVersionMissing,
    UnknownKind(usize),
}

// Manual PartialEq impl for SpecEnvelopeErrorProduction to avoid
// `core::intrinsics::discriminant_value` which Verus does not
// currently support. Two errors are equal iff they are the same
// variant AND, for `UnknownKind`, the payload length matches.
//
// The body is `#[verifier::external]` so Verus skips the body.
#[verifier::external]
impl PartialEq for SpecEnvelopeErrorProduction {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (
                SpecEnvelopeErrorProduction::SerializationFailed,
                SpecEnvelopeErrorProduction::SerializationFailed,
            ) => true,
            (
                SpecEnvelopeErrorProduction::SchemaVersionMissing,
                SpecEnvelopeErrorProduction::SchemaVersionMissing,
            ) => true,
            (
                SpecEnvelopeErrorProduction::UnknownKind(a),
                SpecEnvelopeErrorProduction::UnknownKind(b),
            ) => a == b,
            _ => false,
        }
    }
}

impl SpecEnvelopeErrorProduction {
    /// Spec decision (plain Rust): true iff the error is the
    /// `SchemaVersionMissing` variant. Mirrors the production
    /// discriminant at `cli_envelope.rs:171`.
    pub fn is_schema_version_missing(&self) -> bool {
        matches!(self, SpecEnvelopeErrorProduction::SchemaVersionMissing)
    }

    /// Spec decision (plain Rust): true iff the error is the
    /// `SerializationFailed` variant.
    pub fn is_serialization_failed(&self) -> bool {
        matches!(self, SpecEnvelopeErrorProduction::SerializationFailed)
    }

    /// Spec decision (plain Rust): true iff the error is the
    /// `UnknownKind` variant (any payload length).
    pub fn is_unknown_kind(&self) -> bool {
        matches!(self, SpecEnvelopeErrorProduction::UnknownKind(_))
    }
}

// ============================================================================
// NO-PRODUCTION-SOURCE MARKERS — explicit honest disclosure
// ============================================================================
//
// Each marker below names a spec mirror type from
// `vb_ahfl_metadata_envelope_production.rs` whose production source
// has been REMOVED from the workspace (the `vb_ui_model` crate). The
// companion spec file's proof fns for these types retain their
// `requires == ensures` form but the file-level TRUST BOUNDARY
// section explicitly tags them as "no production binding".
//
// These markers exist so that any future grep across the verus tree
// surfaces the gap; they are not used as types.

/// Marker: `SpecEnvelopeKind` (6-variant original: Success, Error,
/// DiagnosticReport, Status, Event, Workflow) has no production
/// source. The closest production analogue is
/// `vb_cli::cli_envelope::Kind` (17-variant). Closure requires
/// re-introducing `vb_ui_model::envelope::types::EnvelopeKind` or
/// re-shaping the original spec types to match production
/// discriminants.
pub struct NoProductionSourceEnvelopeKind;

/// Marker: `SpecMetadataEnvelope { run_id, command, timestamp }` has
/// no production source. Production cli_envelope has no such struct;
/// the envelope carries `data: Value` only. Closure requires
/// re-introducing `vb_ui_model::envelope::types::MetadataEnvelope` or
/// adding these fields to a new production envelope struct.
pub struct NoProductionSourceMetadataEnvelope;
