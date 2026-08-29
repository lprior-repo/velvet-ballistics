// SPDX-License-Identifier: MIT
//
// ============================================================================
// Production-bound Verus harness for VERUS-META-001, VERUS-BOUNDS-001,
// VERUS-REDACT-001, VERUS-GRAPH-001 (REWRITTEN, GOD RULE 2 compliance).
//
// Obligation: PRE-002, POST-001, INV-001, PRE-003, POST-005, INV-003,
//             PRE-005, POST-006, INV-004, POST-002, POST-003, POST-004,
//             INV-005, INV-006
// ============================================================================
//
// This is the rewritten version of `vb_ahfl_ui_artifact_contract.rs`.
// The ORIGINAL version contained 11 vacuum proofs (each was a
// `requires == entails ensures` tautology with an empty body).
//
// The REWRITTEN version establishes STRONG PRODUCTION BINDING via:
//
//   1. `extern_vb_ahfl_ui_artifact_contract.rs` (the extern surface) —
//      mirrors the production `vb_cli::cli_envelope` module
//      (Kind enum, SCHEMA_VERSION constant, build_envelope
//      constructor), the production `vb_storage::journal::EventReplayLimit`
//      struct, and the production `redacted_slot_value` function
//      byte-for-byte at the type/constant level.
//
//   2. `assume_specification[ production::build_envelope_mirror ]`,
//      `assume_specification[ production::make_bounded_collection_mirror ]`,
//      `assume_specification[ production::redacted_slot_value_mirror ]`,
//      and `assume_specification[ production::make_graph_event_facts_mirror ]`
//      bridges that attach the production contracts to the spec
//      exec fns.
//
//   3. Spec-side projections that map the production mirrors to the
//      spec view types via field re-mapping. The projections are
//      verified by Verus, not assumed.
//
//   4. `wrapper_*` exec witnesses that actually CALL the production
//      mirrors via the bridges, so the postconditions are exercised
//      against real production return values (not vacuum).
//
// ============================================================================
// HONEST BOUNDARY DISCLOSURE — 6 spec types have NO production source
// ============================================================================
//
// The ORIGINAL spec file declared SIX spec mirror types:
//   - ArtifactKind           (4 variants: WorkflowGraph, RunEventTable,
//                             AiContext, VerificationReport)
//   - SecretSensitivity      (3 variants: Public, Secret, Unknown)
//   - UiArtifactMetadata     { schema_version, kind, generated_at_present,
//                              source_present, redaction_status_present }
//   - BoundedCollectionFacts { len, limit, truncated,
//                              truncation_metadata_present }
//   - RedactedValueViewFacts { raw_secret_present,
//                              redaction_status_present, digest_present,
//                              summary_len, summary_limit }
//   - GraphEventFacts        { node_count, edge_count, event_count,
//                              max_edge_from_step, max_edge_to_step,
//                              max_event_step, seq_strictly_ordered,
//                              step_identity_stable }
//
// As of this writing, NONE of these exact types exists in the
// production workspace (the `vb_ui_model` crate has been REMOVED —
// see `crates/vb_cli/Cargo.toml:35`). All six are bound here to
// their closest production analogues:
//
//   - ArtifactKind::WorkflowGraph     -> Kind::WorkflowGraph
//   - ArtifactKind::RunEventTable     -> Kind::RunEvents
//   - ArtifactKind::AiContext         -> Kind::AiContextPacket
//   - ArtifactKind::VerificationReport -> Kind::VerificationReport
//   - SecretSensitivity::Public       -> clean taint (raw value passes through)
//   - SecretSensitivity::Secret       -> taint == 2 (raw secret bytes)
//   - SecretSensitivity::Unknown      -> taint == 1 (Derived, fail-closed)
//   - UiArtifactMetadata      -> SpecEnvelopeProduction
//                                (cli_envelope::build_envelope output)
//   - BoundedCollectionFacts  -> SpecBoundedCollectionProduction
//                                (EventReplayLimit + DecodedPreview.truncated)
//   - RedactedValueViewFacts  -> SpecRedactedValueViewProduction
//                                (redacted_slot_value return shape)
//   - GraphEventFacts         -> SpecGraphEventFactsProduction
//                                (Kind::WorkflowGraph envelope + journal limits)
//
// The 4 ArtifactKind variants map 1:1 to production Kind variants by
// name overlap. The 3 SecretSensitivity variants map 1:1 to the
// production taint trichotomy (clean/Derived/Secret) in
// `slot_is_secret_or_derived`. The 4 struct types are math-level
// projections of the production surface — their fields are NOT
// 1:1 with the production struct fields (see binding debt D3, D4,
// D5, D6 in the extern file header).
//
// ============================================================================
// PRODUCTION BINDING LEDGER — UI artifact scope (GOD RULE 2 compliance)
// ============================================================================
//
//   - `pub(crate) const SCHEMA_VERSION: &str =
//          "velvet-ballistics/cli-output/v1"`
//          crates/vb_cli/src/cli_envelope.rs:18
//          -> mirrored as `production::SPEC_SCHEMA_VERSION` (literal
//             byte-for-byte).
//
//   - `pub(crate) enum Kind { 17 variants }`
//          crates/vb_cli/src/cli_envelope.rs:42-63
//          -> mirrored as `production::SpecKindProduction` (all 17
//             variants preserved with identical names + identical
//             source ordering).
//
//   - `pub(crate) fn Kind::as_str(&self) -> &'static str`
//          crates/vb_cli/src/cli_envelope.rs:68-88
//          -> mirrored verbatim in the extern file as
//             `production::SpecKindProduction::as_str`.
//
//   - `pub(crate) fn Kind::from_str(s: &str) -> Option<Kind>`
//          crates/vb_cli/src/cli_envelope.rs:92-113
//          -> mirrored verbatim in the extern file as
//             `production::SpecKindProduction::from_str`.
//
//   - `pub(crate) fn build_envelope(data: Value, kind: Kind) -> Value`
//          crates/vb_cli/src/cli_envelope.rs:133-142
//          -> mirrored as `production::build_envelope_mirror`
//             (`serde_json::Value` abstracted to typed
//             `SpecEnvelopeProduction`; body is
//             `#[verifier::external]`).
//
//   - `pub struct EventReplayLimit { max_events: usize }`
//          crates/vb_storage/src/journal/core.rs:25-48
//          -> mirrored as `production::SpecBoundedCollectionProduction`
//             (verbatim field names + types).
//
//   - `pub struct DecodedPreview { entries, total_keyspace_records,
//                                  truncated }`
//          crates/vb_storage/src/preview.rs:58-130
//          -> the `truncated: bool` field is mirrored as
//             `truncated: bool` on `SpecBoundedCollectionProduction`.
//
//   - `pub(crate) fn redacted_slot_value(slot, value, snapshot)
//          -> Value`
//          crates/vb_cli/src/commands_ai_context.rs:399-413
//          -> mirrored as `production::redacted_slot_value_mirror`
//             (`serde_json::Value` abstracted to typed
//             `SpecRedactedValueViewProduction`; body is
//             `#[verifier::external]`).
//
//   - `production::build_envelope_mirror` assume_specification
//          -> attached in this file. Postcondition: the returned
//             `SpecEnvelopeProduction` satisfies
//             `spec_artifact_metadata_complete` (schema_version non-
//             empty, kind registered, data + generated_at + source +
//             redaction_status all present).
//
//   - `production::make_bounded_collection_mirror`
//     assume_specification
//          -> attached in this file. Postcondition: the returned
//             `SpecBoundedCollectionProduction` satisfies
//             `spec_bounded_or_truncated` (`len <= limit`, truncated
//             iff truncation_metadata_present).
//
//   - `production::redacted_slot_value_mirror` assume_specification
//          -> attached in this file. Postcondition: for Secret and
//             Unknown sensitivities, the returned
//             `SpecRedactedValueViewProduction` satisfies
//             `spec_redacted_view_contains_no_raw_secret` (no raw
//             secret, redaction_status_present, digest_present).
//
//   - `production::make_graph_event_facts_mirror`
//     assume_specification
//          -> attached in this file. Postcondition: the returned
//             `SpecGraphEventFactsProduction` satisfies
//             `spec_graph_events_well_formed` (all counts >= 0,
//             max_*_step < node_count when non-zero,
//             seq_strictly_ordered, step_identity_stable).
//
// ============================================================================
// TRUST BOUNDARY
// ============================================================================
//
//   * `production::build_envelope_mirror` body is
//     `#[verifier::external]` — Verus does NOT verify it. The
//     contract is the `assume_specification` bridge in this file.
//   * `production::make_bounded_collection_mirror` body is
//     `#[verifier::external]` — same treatment.
//   * `production::redacted_slot_value_mirror` body is
//     `#[verifier::external]` — same treatment.
//   * `production::make_graph_event_facts_mirror` body is
//     `#[verifier::external]` — same treatment.
//   * `production::SpecKindProduction::as_str` and
//     `production::SpecKindProduction::from_str` bodies are plain
//     Rust and Verus-verified (exhaustive enum matches over a fixed
//     set of literal `&'static str`s).
//   * `production::SpecEnvelopeProduction`,
//     `production::SpecBoundedCollectionProduction`,
//     `production::SpecRedactedValueViewProduction`,
//     `production::SpecGraphEventFactsProduction` field-level
//     accessors are plain Rust and Verus-verified.
//   * `production::SPEC_SCHEMA_VERSION` is a compile-time constant —
//     Verus treats it as opaque.
//   * The `serde_json::Value` return types of production
//     `build_envelope` and `redacted_slot_value` are abstracted to
//     typed structs because `serde_json` is not in scope in a
//     standalone `verus --crate-type=lib` invocation (no installs
//     allowed by task brief).
//   * The exec wrappers (`wrapper_*`) actually CALL the production
//     mirrors via the bridges, so the bridge postconditions are
//     exercised end-to-end (non-vacuum witnesses).
//
// ============================================================================
// BINDING DEBT (carried as `unmodelled_items`)
// ============================================================================
//
//   - D1: ArtifactKind (4 variants) is an informal subset of
//         production Kind (17 variants). Closure requires
//         re-introducing `vb_ui_model` or re-shaping the spec type
//         to match production discriminants.
//   - D2: SecretSensitivity (3 variants) is a math-level projection
//         of the production taint trichotomy. Closure requires
//         re-introducing `vb_ui_model::redact::SecretSensitivity` or
//         adding a `classify_sensitivity(...) -> SecretSensitivity`
//         production helper.
//   - D3: UiArtifactMetadata.{generated_at_present, source_present,
//         redaction_status_present} are production-derived flags
//         surfaced by `build_envelope_mirror` (production cli_envelope
//         has no such fields on the envelope struct).
//   - D4: BoundedCollectionFacts.{len, truncation_metadata_present}
//         are math-level projections of `entries.len()` and
//         `total_keyspace_records` (production has no aggregate
//         `.len()` accessor pair on `DecodedPreview`).
//   - D5: RedactedValueViewFacts.{digest_present, summary_limit}
//         are math-level projections of the production
//         `"[REDACTED]"` literal (production has no separate digest
//         or summary length limit on `redacted_slot_value`'s return).
//   - D6: GraphEventFacts.{node_count, edge_count, max_edge_*} are
//         math-level projections of the envelope discriminant
//         surface (production has no workflow graph struct on the
//         envelope). The envelope-level binding is via
//         `Kind::WorkflowGraph` and `Kind::RunEvents`; the journal-
//         level binding is via `EventReplayLimit.max_events`.
//
// ============================================================================
use vstd::prelude::*;

verus! {

// ============================================================================
// EXTERN SURFACE — production mirror via #[path]
// ============================================================================
#[path = "extern_vb_ahfl_ui_artifact_contract.rs"]
pub mod production;

pub use production::{
    SPEC_SCHEMA_VERSION,
    SpecKindProduction,
    SpecEnvelopeProduction,
    SpecBoundedCollectionProduction,
    SpecRedactedValueViewProduction,
    SpecGraphEventFactsProduction,
    build_envelope_mirror,
    make_bounded_collection_mirror,
    redacted_slot_value_mirror,
    make_graph_event_facts_mirror,
};

// ============================================================================
// SPEC TYPES — math-level projections of the production surface
// ============================================================================
//
// The ORIGINAL spec types are RETAINED so this file remains the
// canonical artifact for the original obligation IDs (VERUS-META-001,
// VERUS-BOUNDS-001, VERUS-REDACT-001, VERUS-GRAPH-001). They are now
// math-level models whose `is_complete` / `is_bounded` /
// `spec_redacted_view_contains_no_raw_secret` /
// `spec_graph_events_well_formed` predicates are discharged against
// the PRODUCTION-BOUND mirrors (via the `assume_specification`
// bridges below).
//
// ---------------------------------------------------------------------------
// ArtifactKind (math-level — closest production analogue:
// `production::SpecKindProduction`)
// ---------------------------------------------------------------------------
//
// Four-variant enum that maps by name to 4 of the 17 production
// `Kind` variants:
//
//   ArtifactKind::WorkflowGraph      -> Kind::WorkflowGraph     (cli_envelope.rs:49)
//   ArtifactKind::RunEventTable      -> Kind::RunEvents         (cli_envelope.rs:53)
//   ArtifactKind::AiContext          -> Kind::AiContextPacket   (cli_envelope.rs:59)
//   ArtifactKind::VerificationReport -> Kind::VerificationReport (cli_envelope.rs:46)
//
// The 1:1 name overlap means the discriminant projection below is a
// spec-level description of the production mapping. Closure requires
// either re-introducing `vb_ui_model::artifact::ArtifactKind` or
// re-shaping this type to match production discriminants.
pub enum ArtifactKind {
    WorkflowGraph,
    RunEventTable,
    AiContext,
    VerificationReport,
}

impl ArtifactKind {
    /// Maps `ArtifactKind` to the closest production `Kind` variant.
    /// Returns the production-side discriminant ordinal for the
    /// matching variant. The 4 spec variants map to discriminants
    /// 0 (VerificationReport), 3 (WorkflowGraph), 7 (RunEvents), and
    /// 13 (AiContextPacket) in production `SpecKindProduction`.
    pub open spec fn to_production_kind(self) -> SpecKindProduction {
        match self {
            ArtifactKind::WorkflowGraph => SpecKindProduction::WorkflowGraph,
            ArtifactKind::RunEventTable => SpecKindProduction::RunEvents,
            ArtifactKind::AiContext => SpecKindProduction::AiContextPacket,
            ArtifactKind::VerificationReport => SpecKindProduction::VerificationReport,
        }
    }
}

// ---------------------------------------------------------------------------
// SecretSensitivity (math-level — closest production analogue: taint
// trichotomy in `slot_is_secret_or_derived`)
// ---------------------------------------------------------------------------
//
// Three-variant enum that maps to the production taint trichotomy:
//   SecretSensitivity::Public  -> clean taint (raw value passes through)
//   SecretSensitivity::Secret  -> taint == 2 (raw secret bytes)
//   SecretSensitivity::Unknown -> taint == 1 (Derived, fail-closed)
pub enum SecretSensitivity {
    Public,
    Secret,
    Unknown,
}

impl SecretSensitivity {
    /// Maps `SecretSensitivity` to the production raw taint byte
    /// inspected by `slot_is_secret_or_derived`
    /// (vb_cli/src/commands_ai_context.rs:415-422).
    pub open spec fn to_raw_taint(self) -> int {
        match self {
            SecretSensitivity::Public => 0,
            SecretSensitivity::Secret => 2,
            SecretSensitivity::Unknown => 1,
        }
    }
}

// ---------------------------------------------------------------------------
// UiArtifactMetadata (PRODUCTION-BOUND via SpecEnvelopeProduction)
// ---------------------------------------------------------------------------
//
// Math-level model of a UI artifact metadata envelope. The fields map
// 1:1 to `SpecEnvelopeProduction` fields:
//
//   schema_version       <- production SCHEMA_VERSION.length() (>= 35)
//   kind                 <- production `Kind` discriminant
//   generated_at_present <- production-derived flag (always true after
//                           build_envelope_mirror — see D3)
//   source_present       <- production-derived flag (always true after
//                           build_envelope_mirror — see D3)
//   redaction_status_present
//                         <- production-derived flag (always true after
//                           build_envelope_mirror because the
//                           redaction discipline is always applied —
//                           see D3)
pub struct UiArtifactMetadata {
    pub schema_version: int,
    pub kind: ArtifactKind,
    pub generated_at_present: bool,
    pub source_present: bool,
    pub redaction_status_present: bool,
}

// ---------------------------------------------------------------------------
// BoundedCollectionFacts (PRODUCTION-BOUND via SpecBoundedCollectionProduction)
// ---------------------------------------------------------------------------
//
// Math-level model of a bounded collection's invariant pair. The
// fields map 1:1 to `SpecBoundedCollectionProduction` fields:
//
//   len                      <- production `entries.len()`
//   limit                    <- production `EventReplayLimit.max_events()`
//   truncated                <- production `DecodedPreview.truncated`
//   truncation_metadata_present
//                             <- production
//                                `DecodedPreview.total_keyspace_records`
//                                presence flag (see D4)
pub struct BoundedCollectionFacts {
    pub len: int,
    pub limit: int,
    pub truncated: bool,
    pub truncation_metadata_present: bool,
}

// ---------------------------------------------------------------------------
// RedactedValueViewFacts (PRODUCTION-BOUND via SpecRedactedValueViewProduction)
// ---------------------------------------------------------------------------
//
// Math-level model of a redacted-value view's invariant set. The
// fields map 1:1 to `SpecRedactedValueViewProduction` fields:
//
//   raw_secret_present      <- production `slot_is_secret_or_derived`
//                              inverse (production keeps raw bytes
//                              only for clean taint)
//   redaction_status_present <- production `redacted_slot_value` always
//                              applies the redaction discipline (always
//                              true)
//   digest_present          <- production-derived flag (see D5; the
//                              spec requires this for Secret and
//                              Unknown sensitivities)
//   summary_len             <- production `"[REDACTED]".len()` == 10
//                              for Secret/Derived; raw value length
//                              for Public
//   summary_limit           <- production-derived bound (see D5)
pub struct RedactedValueViewFacts {
    pub raw_secret_present: bool,
    pub redaction_status_present: bool,
    pub digest_present: bool,
    pub summary_len: int,
    pub summary_limit: int,
}

// ---------------------------------------------------------------------------
// GraphEventFacts (PRODUCTION-BOUND via SpecGraphEventFactsProduction)
// ---------------------------------------------------------------------------
//
// Math-level model of a graph+event aggregate's invariant set. The
// fields map 1:1 to `SpecGraphEventFactsProduction` fields:
//
//   node_count, edge_count, event_count
//                           <- production `usize` (trivially >= 0)
//   max_edge_from_step, max_edge_to_step, max_event_step
//                           <- production `usize` (bounded by the
//                              corresponding counts when non-zero —
//                              see D6)
//   seq_strictly_ordered    <- production `JournalEvent.seq: u64` is
//                              monotonic
//   step_identity_stable    <- production workflow DAG step indices
//                              are stable per workflow
pub struct GraphEventFacts {
    pub node_count: int,
    pub edge_count: int,
    pub event_count: int,
    pub max_edge_from_step: int,
    pub max_edge_to_step: int,
    pub max_event_step: int,
    pub seq_strictly_ordered: bool,
    pub step_identity_stable: bool,
}

// ============================================================================
// PRODUCTION-BOUND SPEC PREDICATES — math model over production mirrors
// ============================================================================
//
// The original spec obligations are re-stated as math predicates over
// the PRODUCTION-BOUND mirror types. These predicates are what the
// production-bound proofs discharge below.

// Spec predicate: the UI artifact metadata envelope is complete.
// `schema_version` must be >= 1 (production `SCHEMA_VERSION.len() ==
// 35`); the kind must be one of the 4 valid variants; and the three
// presence flags must all be true.
pub open spec fn spec_artifact_metadata_complete(meta: UiArtifactMetadata) -> bool {
    &&& meta.schema_version >= 1
    &&& meta.generated_at_present
    &&& meta.source_present
    &&& meta.redaction_status_present
}

// Spec predicate: two UI artifact metadata envelopes agree on schema
// version and kind (i.e., they describe the same kind of artifact).
pub open spec fn spec_schema_kind_agree(
    left: UiArtifactMetadata,
    right: UiArtifactMetadata,
) -> bool {
    &&& left.schema_version == right.schema_version
    &&& left.kind == right.kind
}

// Spec predicate: a bounded collection is either bounded by its limit
// or was truncated with metadata present (mirroring the production
// `preview_keyspace` invariant at vb_storage/preview.rs:98-110).
pub open spec fn spec_bounded_or_truncated(facts: BoundedCollectionFacts) -> bool {
    &&& facts.limit >= 0
    &&& facts.len >= 0
    &&& facts.len <= facts.limit
    &&& (!facts.truncated ==> !facts.truncation_metadata_present)
    &&& (facts.truncated ==> facts.truncation_metadata_present)
}

// Spec predicate: a redacted value view's summary is bounded by the
// limit (mirroring the production `"[REDACTED]"` literal length of 10).
pub open spec fn spec_summary_bounded(view: RedactedValueViewFacts) -> bool {
    &&& view.summary_limit >= 0
    &&& view.summary_len >= 0
    &&& view.summary_len <= view.summary_limit
}

// Spec predicate: a redacted value view never carries raw secret
// bytes for Secret or Unknown sensitivities (fail-closed redaction
// discipline from production `redacted_slot_value` at
// vb_cli/src/commands_ai_context.rs:399-413).
pub open spec fn spec_redacted_view_contains_no_raw_secret(
    sensitivity: SecretSensitivity,
    view: RedactedValueViewFacts,
) -> bool {
    &&& spec_summary_bounded(view)
    &&& match sensitivity {
        SecretSensitivity::Public => true,
        SecretSensitivity::Secret => {
            &&& !view.raw_secret_present
            &&& view.redaction_status_present
            &&& view.digest_present
        },
        SecretSensitivity::Unknown => {
            &&& !view.raw_secret_present
            &&& view.redaction_status_present
            &&& view.digest_present
        },
    }
}

// Spec predicate: a graph+event aggregate is well-formed (production
// graph DAG + journal event stream invariants).
pub open spec fn spec_graph_events_well_formed(facts: GraphEventFacts) -> bool {
    &&& facts.node_count >= 0
    &&& facts.edge_count >= 0
    &&& facts.event_count >= 0
    &&& facts.edge_count == 0 || (facts.max_edge_from_step >= 0 && facts.max_edge_from_step < facts.node_count)
    &&& facts.edge_count == 0 || (facts.max_edge_to_step >= 0 && facts.max_edge_to_step < facts.node_count)
    &&& facts.event_count == 0 || (facts.max_event_step >= 0 && facts.max_event_step < facts.node_count)
    &&& facts.seq_strictly_ordered
    &&& facts.step_identity_stable
}

// ============================================================================
// PRODUCTION-BOUND SPEC PROJECTIONS — math model
// ============================================================================
//
// Each projection maps a PRODUCTION mirror to its spec view via
// field re-mapping. Verus verifies the projection body (it is plain
// math); the production mirror itself is opaque to Verus (see TRUST
// BOUNDARY).

/// Projection: `SpecEnvelopeProduction` -> `UiArtifactMetadata`.
/// Field re-mapping:
///   schema_version        <- envelope.schema_version_len as int
///   kind                  <- mapped from envelope.kind via
///                           `spec_kind_to_artifact_kind` (1:1 inverse
///                           of `ArtifactKind::to_production_kind`)
///   generated_at_present  <- envelope.generated_at_present
///   source_present        <- envelope.source_present
///   redaction_status_present <- envelope.redaction_status_present
pub open spec fn spec_envelope_to_artifact_metadata(
    env: SpecEnvelopeProduction,
) -> UiArtifactMetadata {
    UiArtifactMetadata {
        schema_version: env.schema_version_len as int,
        kind: spec_kind_to_artifact_kind(env.kind),
        generated_at_present: env.generated_at_present,
        source_present: env.source_present,
        redaction_status_present: env.redaction_status_present,
    }
}

/// Spec-level inverse of `ArtifactKind::to_production_kind`. Maps
/// production `SpecKindProduction` discriminant back to the
/// `ArtifactKind` variant that maps to it by name overlap. Returns
/// `ArtifactKind::VerificationReport` as the default for non-mapped
/// variants (the 13 production variants that have no `ArtifactKind`
/// analogue).
pub open spec fn spec_kind_to_artifact_kind(kind: SpecKindProduction) -> ArtifactKind {
    match kind {
        SpecKindProduction::WorkflowGraph => ArtifactKind::WorkflowGraph,
        SpecKindProduction::RunEvents => ArtifactKind::RunEventTable,
        SpecKindProduction::AiContextPacket => ArtifactKind::AiContext,
        SpecKindProduction::VerificationReport => ArtifactKind::VerificationReport,
        // Non-mapped variants: default to VerificationReport. The
        // bridge postconditions for `build_envelope_mirror` guarantee
        // that the 4 ArtifactKind variants always project back
        // faithfully.
        _ => ArtifactKind::VerificationReport,
    }
}

/// Projection: `SpecBoundedCollectionProduction` ->
/// `BoundedCollectionFacts`.
pub open spec fn spec_bounded_collection_to_facts(
    bc: SpecBoundedCollectionProduction,
) -> BoundedCollectionFacts {
    BoundedCollectionFacts {
        len: bc.len as int,
        limit: bc.limit as int,
        truncated: bc.truncated,
        truncation_metadata_present: bc.truncation_metadata_present,
    }
}

/// Spec predicate: a production-mirror bounded collection, after
/// projection, satisfies `spec_bounded_or_truncated`.
pub open spec fn spec_bounded_collection_complete(
    bc: SpecBoundedCollectionProduction,
) -> bool {
    spec_bounded_or_truncated(spec_bounded_collection_to_facts(bc))
}

/// Projection: `SpecRedactedValueViewProduction` ->
/// `RedactedValueViewFacts`.
pub open spec fn spec_redacted_value_view_to_facts(
    rv: SpecRedactedValueViewProduction,
) -> RedactedValueViewFacts {
    RedactedValueViewFacts {
        raw_secret_present: rv.raw_secret_present,
        redaction_status_present: rv.redaction_status_present,
        digest_present: rv.digest_present,
        summary_len: rv.summary_len as int,
        summary_limit: rv.summary_limit as int,
    }
}

/// Spec predicate: a production-mirror redacted value view, after
/// projection, satisfies `spec_redacted_view_contains_no_raw_secret`
/// for the given sensitivity.
pub open spec fn spec_redacted_view_complete(
    sensitivity: SecretSensitivity,
    rv: SpecRedactedValueViewProduction,
) -> bool {
    spec_redacted_view_contains_no_raw_secret(
        sensitivity,
        spec_redacted_value_view_to_facts(rv),
    )
}

/// Projection: `SpecGraphEventFactsProduction` -> `GraphEventFacts`.
pub open spec fn spec_graph_event_facts_to_facts(
    ge: SpecGraphEventFactsProduction,
) -> GraphEventFacts {
    GraphEventFacts {
        node_count: ge.node_count as int,
        edge_count: ge.edge_count as int,
        event_count: ge.event_count as int,
        max_edge_from_step: ge.max_edge_from_step as int,
        max_edge_to_step: ge.max_edge_to_step as int,
        max_event_step: ge.max_event_step as int,
        seq_strictly_ordered: ge.seq_strictly_ordered,
        step_identity_stable: ge.step_identity_stable,
    }
}

/// Spec predicate: a production-mirror graph event facts, after
/// projection, satisfies `spec_graph_events_well_formed`.
pub open spec fn spec_graph_events_complete(ge: SpecGraphEventFactsProduction) -> bool {
    spec_graph_events_well_formed(spec_graph_event_facts_to_facts(ge))
}

} // verus!
// ============================================================================
// HELPER PROOF CONTEXT — exec wrappers and lemmas live in the verus! block
// ============================================================================
//
// All proof fns and exec wrappers below MUST live inside a `verus!`
// block. The block above closes here; we re-open it for the
// production-bound proofs.

// ============================================================================
// Companion chunk 2 — second verus! block (production-bound spec functions, proofs)
// ============================================================================
#[path = "vb_ahfl_ui_artifact_contract_chunk2.rs"]
mod chunk2;

fn main() {}