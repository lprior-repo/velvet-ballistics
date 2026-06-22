// Verus proof obligations for vb-7ol6y (REDO): recovery hydration contracts
// for the 3 vb_storage::recovery hydration fail-closed bugs.
//
// Bead: vb-7ol6y (P0)
// State: 5 (proof-writer)
// Verifier: Verus
// Command: verus --crate-type=lib verification/verus/recovery_hydration_contracts_v2.rs
//
// This file EXTENDS verification/verus/recovery_hydration_contracts.rs with:
//   - SpecRecoveryError::CorruptSlotTaint { slot: int }
//   - SpecRecoveryError::SlotTaintReadFailed { slot: int }
//   - Spec fns binding the production 2-way discriminator
//     (`legacy_or_corrupt_taint` at crates/vb_storage/src/recovery/replay/
//     summary/slots/taint.rs:59-92) and the typed lattice
//     (`resolve_slot_taint_read`/`observe_slot_taint_read` at
//     crates/vb_storage/src/recovery/event_replay/taint.rs:35-54).
//
// GOD RULE 2 (REDO): every spec fn below is bound to the production
// implementation by (a) referencing its production file:line in the doc
// comment, (b) naming the production function as the contract target,
// and (c) carrying a `proof fn` whose `requires`/`ensures` shape matches
// the production control flow.
//
// REDO FIX for ps-006 routing proof:
//   The previous State 5 had `proof_fail_closed_routes_to_slot_taint_read_failed`
//   with body `{}` and postcondition `true` — vacuous. The REDO version
//   (`proof_tail_fail_closed_routes_to_slot_taint_read_failed` and
//   `proof_compose_uninitialized_routes_to_use_clean`) takes concrete
//   spec inputs, has non-trivial `requires`/`ensures`, and a real proof
//   body that reveals the spec fns being composed.
use vstd::prelude::*;

verus! {

// ============================================================================
// SpecRecoveryError extension (TB-007 spec seam)
// ============================================================================
pub enum SpecRecoveryError {
    NoRecoveryData,
    CorruptSnapshot,
    ReplayDivergence,
    WorkflowSourceDigestMismatch,
    CompiledIrDigestMismatch,
    NonIdempotentActionBlocked,
    FrameDimensionOverflow,
    InvalidRecoveryHydration,
    CollectExtraHydrationFailed,
    /// Slot taint metadata was present but could not be decoded.
    /// Binds to crates/vb_storage/src/recovery/types/error.rs:107-112
    /// `RecoveryError::CorruptSlotTaint { slot }`.
    CorruptSlotTaint { slot: int },
    /// `RunFrame::read_taint` failed for a non-`SlotUninitialized` reason;
    /// the typed lattice resolved the failure to `FailClosed`.
    /// Binds to crates/vb_storage/src/recovery/types/error.rs:101-106
    /// `RecoveryError::SlotTaintReadFailed { slot }`.
    SlotTaintReadFailed { slot: int },
}

// ============================================================================
// Spec types for slot taint classification (ps-001..ps-005)
// ============================================================================
/// Spec mirror of `vb_core::Taint` (3-variant enum).
pub enum SpecTaint {
    Clean,
    DerivedFromSecret,
    Secret,
}

/// Spec mirror of `RecoveredSlotTaint` at
/// crates/vb_storage/src/recovery/replay/summary/slots/taint.rs:34-38.
pub struct SpecRecoveredSlotTaint {
    pub taint: SpecTaint,
    pub unsupported: bool,
}

// ============================================================================
// Spec types for the typed read_taint lattice (ps-003)
// ============================================================================
/// Spec mirror of `SlotTaintReadObservation` at
/// crates/vb_storage/src/recovery/event_replay/taint.rs:13-22.
pub enum SpecSlotTaintReadObservation {
    Existing(SpecTaint),
    Uninitialized,
    Failed,
}

/// Spec mirror of `SlotTaintResolution` at
/// crates/vb_storage/src/recovery/event_replay/taint.rs:24-31.
pub enum SpecSlotTaintResolution {
    Use(SpecTaint),
    FailClosed,
}

/// Spec mirror of `vb_core::CoreError::SlotUninitialized` discriminant.
pub enum SpecCoreError {
    SlotUninitialized,
    Other,
}

// ============================================================================
// Slot taint classification spec (ps-001, ps-002, ps-004, ps-005)
// ============================================================================
/// Spec mirror of `legacy_or_corrupt_taint` at
/// crates/vb_storage/src/recovery/replay/summary/slots/taint.rs:62-95.
///
/// Inputs:
///   - `bytes`: arbitrary byte vector of length 0..=4096
///   - `prefix_len`: SLOT_WRITTEN_EXTRA_PREFIX length (= 5)
///   - `max_payload_len`: MAX_FRAME_EXTRA_BYTES (= 65_536) cap
///   - `decode_envelope`: spec abstraction of `decode_slot_written_extra`:
///       - `None` ⇒ decode returned Err (corrupt)
///       - `Some(true)` ⇒ decode returned Ok(DecodedSlotWrittenExtra::Envelope(_))
///       - `Some(false)` ⇒ decode returned Ok(DecodedSlotWrittenExtra::LegacyFrameExtra(_))
pub open spec fn spec_legacy_or_corrupt_taint(
    bytes: Seq<u8>,
    prefix_len: int,
    max_payload_len: int,
    decode_envelope: Option<bool>,
) -> Result<SpecRecoveredSlotTaint, SpecRecoveryError> {
    if bytes.len() >= prefix_len && bytes.subrange(0, prefix_len) == spec_prefix_literal() {
        let payload_len = bytes.len() - prefix_len;
        if payload_len > max_payload_len {
            Err(SpecRecoveryError::CorruptSlotTaint { slot: 0 })
        } else {
            match decode_envelope {
                Some(true) => Ok(
                    SpecRecoveredSlotTaint {
                        taint: SpecTaint::Clean,
                        unsupported: false,
                    },
                ),
                Some(false) => Err(SpecRecoveryError::CorruptSlotTaint { slot: 0 }),
                None => Err(SpecRecoveryError::CorruptSlotTaint { slot: 0 }),
            }
        }
    } else {
        Ok(SpecRecoveredSlotTaint { taint: SpecTaint::Clean, unsupported: false })
    }
}

/// Spec of `SLOT_WRITTEN_EXTRA_PREFIX` constant at
/// crates/vb_storage/src/slot_extra.rs:9 (`b"VBSE\x01"`, length 5).
pub open spec fn spec_prefix_literal() -> Seq<u8> {
    seq![0x56u8, 0x42u8, 0x53u8, 0x45u8, 0x01u8]
}

/// Spec mirror of `legacy_slot_taint` at
/// crates/vb_storage/src/recovery/replay/summary/slots/taint.rs:112-126.
pub open spec fn spec_legacy_slot_taint(value: int) -> SpecTaint {
    // qi37-1.1 contract: value discriminant drives classification.
    // The spec encodes the production match arm semantics:
    //   0 = Bool(false)   -> Clean
    //   1 = Bool(true)    -> DerivedFromSecret
    //   2 = Null          -> DerivedFromSecret
    //   _ = everything else -> Secret
    if value == 0 {
        SpecTaint::Clean
    } else if value == 1 || value == 2 {
        SpecTaint::DerivedFromSecret
    } else {
        SpecTaint::Secret
    }
}

/// Spec mirror of `legacy_recovered_slot_taint` at
/// crates/vb_storage/src/recovery/replay/summary/slots/taint.rs:97-102.
pub open spec fn spec_legacy_recovered_slot_taint(value: int) -> SpecRecoveredSlotTaint {
    SpecRecoveredSlotTaint { taint: spec_legacy_slot_taint(value), unsupported: false }
}

/// Spec mirror of `recovered_slot_taint` at
/// crates/vb_storage/src/recovery/replay/summary/slots/taint.rs:40-53.
pub open spec fn spec_recovered_slot_taint(
    extra_kind: SpecExtraKind,
    bytes: Seq<u8>,
    prefix_len: int,
    max_payload_len: int,
    decode_envelope: Option<bool>,
    value: int,
) -> Result<SpecRecoveredSlotTaint, SpecRecoveryError> {
    match extra_kind {
        SpecExtraKind::Versioned => Ok(
            SpecRecoveredSlotTaint {
                taint: SpecTaint::Clean,
                unsupported: false,
            },
        ),
        SpecExtraKind::Legacy => {
            spec_legacy_or_corrupt_taint(bytes, prefix_len, max_payload_len, decode_envelope)
        },
        SpecExtraKind::None => Ok(spec_legacy_recovered_slot_taint(value)),
    }
}

/// Discriminant for the 3-arm `match extra` in `recovered_slot_taint`.
pub enum SpecExtraKind {
    Versioned,
    Legacy,
    None,
}

// ============================================================================
// Typed read_taint lattice spec (ps-003)
// ============================================================================
/// Spec mirror of `resolve_slot_taint_read` at
/// crates/vb_storage/src/recovery/event_replay/taint.rs:35-43.
pub open spec fn spec_resolve_slot_taint_read(
    obs: SpecSlotTaintReadObservation,
) -> SpecSlotTaintResolution {
    match obs {
        SpecSlotTaintReadObservation::Existing(t) => SpecSlotTaintResolution::Use(t),
        SpecSlotTaintReadObservation::Uninitialized => {
            SpecSlotTaintResolution::Use(SpecTaint::Clean)
        },
        SpecSlotTaintReadObservation::Failed => SpecSlotTaintResolution::FailClosed,
    }
}

/// Spec mirror of `observe_slot_taint_read` at
/// crates/vb_storage/src/recovery/event_replay/taint.rs:45-54.
pub open spec fn spec_observe_slot_taint_read(
    result: Result<SpecTaint, SpecCoreError>,
) -> SpecSlotTaintReadObservation {
    match result {
        Ok(t) => SpecSlotTaintReadObservation::Existing(t),
        Err(SpecCoreError::SlotUninitialized) => SpecSlotTaintReadObservation::Uninitialized,
        Err(SpecCoreError::Other) => SpecSlotTaintReadObservation::Failed,
    }
}

// ============================================================================
// Spec fn for the typed-error propagation contract (ps-006 routing)
//
// The production chain at event_replay/tail.rs:239-249 is:
//   let decision = resolve_slot_taint_read(observe_slot_taint_read(
//       frame.read_taint(*slot)
//   ));
//   if matches!(decision, SlotTaintResolution::FailClosed) {
//       return Err(RecoveryError::SlotTaintReadFailed { slot: *slot });
//   }
//
// We model this composition and prove that
//   Err(Other) on read_taint
//     == observe_slot_taint_read(...) returns Failed
//     == resolve_slot_taint_read(Failed) returns FailClosed
//     == production routes to Err(SlotTaintReadFailed)
// ============================================================================

/// Spec mirror of the production routing in tail.rs:248.
/// Models `Err(SlotTaintReadFailed { slot })` as a route from the
/// typed lattice composition.
pub open spec fn spec_tail_route_to_slot_taint_read_failed(
    compose: Result<SpecSlotTaintResolution, ()>,
    slot: int,
) -> Result<(), SpecRecoveryError> {
    match compose {
        Ok(SpecSlotTaintResolution::FailClosed) => Err(
            SpecRecoveryError::SlotTaintReadFailed { slot },
        ),
        _ => Ok(()),
    }
}

/// Spec of the full composition: read_taint -> observe -> resolve -> route.
/// Returns Ok(()) when the lattice resolves to Use(_) (no error),
/// Err(SlotTaintReadFailed) when the lattice resolves to FailClosed.
pub open spec fn spec_compose_tail_route(
    read_taint_result: Result<SpecTaint, SpecCoreError>,
    slot: int,
) -> Result<(), SpecRecoveryError> {
    let observation = spec_observe_slot_taint_read(read_taint_result);
    let resolution = spec_resolve_slot_taint_read(observation);
    spec_tail_route_to_slot_taint_read_failed(Ok(resolution), slot)
}

// ============================================================================
// Proofs (ps-001, ps-002, ps-003, ps-004, ps-005, ps-006)
// ============================================================================
// --- ps-001: corrupt envelope fail-closed ---
/// L1-ps-001: prefix-detected bytes that fail decode return
/// `Err(CorruptSlotTaint)`.
pub proof fn proof_corrupt_envelope_returns_corrupt_slot_taint(
    bytes: Seq<u8>,
    prefix_len: int,
    max_payload_len: int,
)
    requires
        prefix_len == 5,
        max_payload_len == 65536,
        bytes.len() >= prefix_len,
        bytes.subrange(0, prefix_len) == spec_prefix_literal(),
        bytes.len() - prefix_len <= max_payload_len,
    ensures
        match spec_legacy_or_corrupt_taint(bytes, prefix_len, max_payload_len, None) {
            Err(SpecRecoveryError::CorruptSlotTaint { slot }) => slot == 0,
            _ => false,
        },
{
    reveal(spec_legacy_or_corrupt_taint);
    reveal(spec_prefix_literal);
}

/// L2-ps-001: oversized envelope payload returns
/// `Err(CorruptSlotTaint)` (TB-006 MAX_FRAME_EXTRA_BYTES cap).
pub proof fn proof_oversized_envelope_returns_corrupt_slot_taint(
    bytes: Seq<u8>,
    prefix_len: int,
    max_payload_len: int,
)
    requires
        prefix_len == 5,
        bytes.len() >= prefix_len,
        bytes.subrange(0, prefix_len) == spec_prefix_literal(),
        bytes.len() - prefix_len > max_payload_len,
    ensures
        match spec_legacy_or_corrupt_taint(bytes, prefix_len, max_payload_len, None) {
            Err(SpecRecoveryError::CorruptSlotTaint { slot }) => slot == 0,
            _ => false,
        },
{
    reveal(spec_legacy_or_corrupt_taint);
    reveal(spec_prefix_literal);
}

/// L3-ps-001: prefix-detected + LegacyFrameExtra decode return
/// `Err(CorruptSlotTaint)`.
pub proof fn prefix_legacy_frame_extra_returns_corrupt_slot_taint(
    bytes: Seq<u8>,
    prefix_len: int,
    max_payload_len: int,
)
    requires
        prefix_len == 5,
        max_payload_len == 65536,
        bytes.len() >= prefix_len,
        bytes.subrange(0, prefix_len) == spec_prefix_literal(),
        bytes.len() - prefix_len <= max_payload_len,
    ensures
        match spec_legacy_or_corrupt_taint(bytes, prefix_len, max_payload_len, Some(false)) {
            Err(SpecRecoveryError::CorruptSlotTaint { slot }) => slot == 0,
            _ => false,
        },
{
    reveal(spec_legacy_or_corrupt_taint);
    reveal(spec_prefix_literal);
}

// --- ps-002 / ps-005: non-prefix legacy returns Clean, unsupported=false ---
/// L1-ps-002: non-prefix bytes return Ok(Clean, unsupported=false).
pub proof fn proof_non_prefix_returns_clean_unsupported_false(
    bytes: Seq<u8>,
    prefix_len: int,
    max_payload_len: int,
)
    requires
        prefix_len == 5,
        max_payload_len == 65536,
        bytes.len() < prefix_len || bytes.subrange(0, prefix_len) != spec_prefix_literal(),
    ensures
        match spec_legacy_or_corrupt_taint(bytes, prefix_len, max_payload_len, None) {
            Ok(r) => r.taint == SpecTaint::Clean && r.unsupported == false,
            _ => false,
        },
{
    reveal(spec_legacy_or_corrupt_taint);
    reveal(spec_prefix_literal);
}

/// L2-ps-002: non-prefix legacy bytes that decode as `Some(false)` ALSO
/// return Clean, unsupported=false (non-prefix branch is unconditional).
pub proof fn proof_non_prefix_ignores_decode_kind(
    bytes: Seq<u8>,
    prefix_len: int,
    max_payload_len: int,
    decode_envelope: Option<bool>,
)
    requires
        prefix_len == 5,
        max_payload_len == 65536,
        bytes.len() < prefix_len || bytes.subrange(0, prefix_len) != spec_prefix_literal(),
    ensures
        match spec_legacy_or_corrupt_taint(bytes, prefix_len, max_payload_len, decode_envelope) {
            Ok(r) => r.taint == SpecTaint::Clean && r.unsupported == false,
            _ => false,
        },
{
    reveal(spec_legacy_or_corrupt_taint);
    reveal(spec_prefix_literal);
}

// --- ps-003: typed read_taint fail-closed ---
/// L1-ps-003: Failed observation resolves to FailClosed (never Use).
pub proof fn proof_failed_resolves_to_fail_closed()
    ensures
        match spec_resolve_slot_taint_read(SpecSlotTaintReadObservation::Failed) {
            SpecSlotTaintResolution::FailClosed => true,
            _ => false,
        },
{
    reveal(spec_resolve_slot_taint_read);
}

/// L2-ps-003: Uninitialized observation resolves to Use(Clean).
pub proof fn proof_uninitialized_resolves_to_use_clean()
    ensures
        match spec_resolve_slot_taint_read(SpecSlotTaintReadObservation::Uninitialized) {
            SpecSlotTaintResolution::Use(SpecTaint::Clean) => true,
            _ => false,
        },
{
    reveal(spec_resolve_slot_taint_read);
}

/// L3-ps-003: Existing(t) observation resolves to Use(t) (preserves
/// exact taint).
pub proof fn proof_existing_resolves_to_use_same(t: SpecTaint)
    ensures
        match spec_resolve_slot_taint_read(SpecSlotTaintReadObservation::Existing(t)) {
            SpecSlotTaintResolution::Use(t2) => t2 == t,
            _ => false,
        },
{
    reveal(spec_resolve_slot_taint_read);
}

/// L4-ps-003: Any non-SlotUninitialized CoreError maps to Failed.
pub proof fn proof_other_core_error_maps_to_failed()
    ensures
        match spec_observe_slot_taint_read(Err(SpecCoreError::Other)) {
            SpecSlotTaintReadObservation::Failed => true,
            _ => false,
        },
{
    reveal(spec_observe_slot_taint_read);
}

/// L5-ps-003: SlotUninitialized CoreError maps to Uninitialized.
pub proof fn proof_slot_uninitialized_maps_to_uninitialized()
    ensures
        match spec_observe_slot_taint_read(Err(SpecCoreError::SlotUninitialized)) {
            SpecSlotTaintReadObservation::Uninitialized => true,
            _ => false,
        },
{
    reveal(spec_observe_slot_taint_read);
}

/// L6-ps-003: Ok(t) result maps to Existing(t) observation.
pub proof fn proof_ok_taint_maps_to_existing(t: SpecTaint)
    ensures
        match spec_observe_slot_taint_read(Ok(t)) {
            SpecSlotTaintReadObservation::Existing(t2) => t2 == t,
            _ => false,
        },
{
    reveal(spec_observe_slot_taint_read);
}

/// L7-ps-003: composition — Err(SlotUninitialized) on read_taint
/// composes through observe + resolve to Use(Clean).
pub proof fn proof_uninitialized_compose_yields_use_clean()
    ensures
        match spec_resolve_slot_taint_read(
            spec_observe_slot_taint_read(Err(SpecCoreError::SlotUninitialized)),
        ) {
            SpecSlotTaintResolution::Use(SpecTaint::Clean) => true,
            _ => false,
        },
{
    reveal(spec_observe_slot_taint_read);
    reveal(spec_resolve_slot_taint_read);
}

/// L8-ps-003: composition — Err(Other) on read_taint composes
/// through observe + resolve to FailClosed.
pub proof fn proof_other_compose_yields_fail_closed()
    ensures
        match spec_resolve_slot_taint_read(
            spec_observe_slot_taint_read(Err(SpecCoreError::Other)),
        ) {
            SpecSlotTaintResolution::FailClosed => true,
            _ => false,
        },
{
    reveal(spec_observe_slot_taint_read);
    reveal(spec_resolve_slot_taint_read);
}

// --- ps-004: None arm returns Secret, unsupported=false (SR-013 regression) ---
/// L1-ps-004: legacy_slot_taint classifies by SlotValue variant
/// per qi37-1.1 contract.
pub proof fn proof_legacy_slot_taint_is_bool_false_clean(value: int)
    requires value == 0,
    ensures spec_legacy_slot_taint(value) == SpecTaint::Clean,
{
    reveal(spec_legacy_slot_taint);
}

pub proof fn proof_legacy_slot_taint_is_bool_true_or_null_derived(value: int)
    requires value == 1 || value == 2,
    ensures spec_legacy_slot_taint(value) == SpecTaint::DerivedFromSecret,
{
    reveal(spec_legacy_slot_taint);
}

pub proof fn proof_legacy_slot_taint_is_other_secret(value: int)
    requires value != 0 && value != 1 && value != 2,
    ensures spec_legacy_slot_taint(value) == SpecTaint::Secret,
{
    reveal(spec_legacy_slot_taint);
}

/// L2-ps-004: legacy_recovered_slot_taint wraps the taint with
/// unsupported=false.
pub proof fn proof_legacy_recovered_slot_taint_unsupported_false(value: int)
    ensures
        match spec_legacy_recovered_slot_taint(value) {
            r => r.unsupported == false,
        },
{
    reveal(spec_legacy_recovered_slot_taint);
    reveal(spec_legacy_slot_taint);
}

/// L3-ps-004: spec_recovered_slot_taint(None) returns Ok with
/// unsupported=false.
pub proof fn proof_recovered_slot_taint_none_unsupported_false(
    bytes: Seq<u8>,
    prefix_len: int,
    max_payload_len: int,
    decode_envelope: Option<bool>,
    value: int,
)
    ensures
        match spec_recovered_slot_taint(
            SpecExtraKind::None,
            bytes,
            prefix_len,
            max_payload_len,
            decode_envelope,
            value,
        ) {
            Ok(r) => r.unsupported == false,
            _ => false,
        },
{
    reveal(spec_recovered_slot_taint);
    reveal(spec_legacy_recovered_slot_taint);
    reveal(spec_legacy_slot_taint);
}

// --- ps-006 (workflow invariants): lattice drives FailClosed to
// SlotTaintReadFailed error variant ---
//
// REDO FIX: the previous State 5 had `proof_fail_closed_routes_to_slot_taint_read_failed`
// with body `{}` and postcondition `true` — vacuous. The REDO version has
// concrete requires/ensures and a real proof body that reveals the spec fns.

/// L1-ps-006: Err(Other) on read_taint composes through observe + resolve
/// to FailClosed, which routes to Err(SlotTaintReadFailed { slot: s }) in
/// the production chain at event_replay/tail.rs:248.
pub proof fn proof_tail_fail_closed_routes_to_slot_taint_read_failed(slot: int)
    ensures
        match spec_compose_tail_route(Err(SpecCoreError::Other), slot) {
            Err(SpecRecoveryError::SlotTaintReadFailed { slot: s }) => s == slot,
            _ => false,
        },
{
    // Reveal the spec fns being composed. Verus needs these to
    // discharge the postcondition.
    reveal(spec_observe_slot_taint_read);
    reveal(spec_resolve_slot_taint_read);
    reveal(spec_tail_route_to_slot_taint_read_failed);
    reveal(spec_compose_tail_route);
}

/// L2-ps-006: Err(SlotUninitialized) on read_taint composes through
/// observe + resolve to Use(Clean), which routes to Ok(()) in the
/// production chain — no error returned.
pub proof fn proof_compose_uninitialized_routes_to_use_clean(slot: int)
    ensures
        match spec_compose_tail_route(Err(SpecCoreError::SlotUninitialized), slot) {
            Ok(()) => true,
            _ => false,
        },
{
    reveal(spec_observe_slot_taint_read);
    reveal(spec_resolve_slot_taint_read);
    reveal(spec_tail_route_to_slot_taint_read_failed);
    reveal(spec_compose_tail_route);
}

/// L3-ps-006: Ok(t) on read_taint composes through observe + resolve to
/// Use(t), which routes to Ok(()) — no error.
pub proof fn proof_compose_ok_taint_routes_to_use(slot: int, t: SpecTaint)
    ensures
        match spec_compose_tail_route(Ok(t), slot) {
            Ok(()) => true,
            _ => false,
        },
{
    reveal(spec_observe_slot_taint_read);
    reveal(spec_resolve_slot_taint_read);
    reveal(spec_tail_route_to_slot_taint_read_failed);
    reveal(spec_compose_tail_route);
}

fn main() {
}

} // verus!
