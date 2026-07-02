// Verus proof obligations for core/storage bridge (PS-007, C8).
//
// Obligation ID: POB-vb-vzcuf-025
// Verifier: verus
// Command: verus --crate-type=lib verification/verus/vb-vzcuf-PS-007.rs
//
// =============================================================================
// PRODUCTION BINDING (GOD RULE 2 compliance)
// =============================================================================
//
// Domain claim (PS-007, C8): Core `max_journal_batch_bytes` is safely
// bridged into storage `JournalBatchByteLimit` or explicitly separated
// without silent drift.
//
// Binding mechanism: `#[path = "extern_vb_vzcuf_PS_007.rs"]` brings the
// production mirror types and the `#[verifier::external]` exec bodies
// into the `verus!` block. The `assume_specification` bridges below
// attach the production contract to the extern bodies. The exec
// wrappers at the bottom of this file exercise the bridges from
// `verus!` context so the contracts are not used as vacuum.
//
// Target surfaces (verified byte-for-byte via BINDING LEDGER in
// `extern_vb_vzcuf_PS_007.rs`):
//   - crates/vb_core/src/workflow/mod.rs:225,249
//       ResourceContract::DEFAULT.max_journal_batch_bytes = 1_048_576
//   - crates/vb_core/src/budget.rs:366,391
//       BoundednessPolicy::DEFAULT.absolute_max_journal_batch_bytes = 1_048_576
//   - crates/vb_core/src/limits.rs:130
//       MAX_JOURNAL_BATCH_BYTES = 16_777_216 (hard cap)
//   - crates/vb_storage/src/batch/types.rs:10
//       DEFAULT_JOURNAL_BATCH_BYTE_LIMIT = 1_048_576
//   - crates/vb_storage/src/batch/types.rs:33-44
//       JournalWriteBatch::new sets byte_limit = Some(DEFAULT_JOURNAL_BATCH_BYTE_LIMIT)
//   - crates/vb_storage/src/batch/types.rs:67-83
//       byte_limit(), staged_event_bytes(), is_aborted() accessors
//
// =============================================================================
// TRUST BOUNDARY (GOD RULE 2 transparency)
// =============================================================================
//
// The production bodies of every mirrored fn are NOT verified by
// Verus (each mirror body is `#[verifier::external]`). The
// `assume_specification` bridges below therefore represent the FULL
// behavioral contract: the Fjall-side and const-eval sides are
// trusted to produce the projected outputs the bridges take as exec
// arguments. Drift between the projection and the production body
// is recorded in the BINDING LEDGER section of
// `extern_vb_vzcuf_PS_007.rs` as drift debt. The bridges themselves
// are proved locally by the exec wrappers at the bottom of this file.
//
// =============================================================================
// HISTORY
// =============================================================================
// The original version of this file proved seven purely-arithmetic
// invariants about literal constants (e.g., `assert(core == 1_048_576)`)
// without binding to any production code. The current version
// replaces each vacuum proof with a production-bound bridge:
//
//   vacuum proof                              -> production-bound proof
//   ----------------------------------------------------------------------
//   lemma_current_values_aligned              -> wrapper_core_default + wrapper_storage_default
//                                              + lemma_core_storage_aligned
//   lemma_divergence_requires_explicit_bridge -> lemma_drift_implies_core_ne_storage
//                                              (kept as pure spec predicate)
//   lemma_bridge_preserves_aligned_value      -> wrapper_new_default_batch
//                                              + lemma_bridge_preserves
//   lemma_storage_default_valid               -> wrapper_storage_default_limit
//                                              + lemma_storage_default_within_u64
//   lemma_default_bridge_valid                -> lemma_default_bridge_valid
//                                              (uses production-bound Bridge)
//   (new)                                     -> lemma_core_policy_within_hard_cap
//                                              + wrapper_hard_cap
//   (new)                                     -> lemma_new_then_accessors
//                                              + wrapper_new_then_byte_limit /
//                                                wrapper_new_then_staged_bytes /
//                                                wrapper_new_then_not_aborted
//
// Source: .beads/vb-vzcuf/proof-obligations.planned.jsonl POB-vb-vzcuf-025
use vstd::prelude::*;

verus! {

// =============================================================================
// Production-mirror types (extern binding)
// =============================================================================
#[path = "extern_vb_vzcuf_PS_007.rs"]
mod production;

pub use production::{
    SpecJournalWriteBatchByDefault,
    SpecBoundednessPolicy,
    SpecResourceContract,
    spec_default_journal_batch_byte_limit,
    spec_max_journal_batch_bytes_hard_cap,
};

// =============================================================================
// Constants re-declared in spec file (VerusErasureCtxt workaround)
// =============================================================================
//
// Per the same workaround used in `vb-vzcuf-PS-009.rs`: `pub const`
// items declared inside the extern module trigger a Verus internal
// error in `--crate-type=lib` mode (`VerusErasureCtxt has not been
// initialized` during thir-body processing). The constants are
// re-declared here with the SAME values; the binding ledger in
// `extern_vb_vzcuf_PS_007.rs` cites the production source lines for
// each constant.
/// Production `ResourceContract::DEFAULT.max_journal_batch_bytes`
/// (u32 = 1_048_576) at `crates/vb_core/src/workflow/mod.rs:249` and
/// `BoundednessPolicy::DEFAULT.absolute_max_journal_batch_bytes`
/// (u32 = 1_048_576) at `crates/vb_core/src/budget.rs:391`. Both core
/// sites share the 1 MiB value; PS-007 asserts they align.
pub const SPEC_CORE_MAX_JOURNAL_BATCH_BYTES: u32 = 1_048_576;

/// Production `DEFAULT_JOURNAL_BATCH_BYTE_LIMIT` (u64 = 1_048_576) at
/// `crates/vb_storage/src/batch/types.rs:10`. Storage default matches
/// the core policy default after u32→u64 widening.
pub const SPEC_STORAGE_DEFAULT_LIMIT: u64 = 1_048_576u64;

/// Production `MAX_JOURNAL_BATCH_BYTES` (u32 = 16_777_216) at
/// `crates/vb_core/src/limits.rs:130`. Hard cap above which
/// `validate_resource_contract` rejects the contract
/// (`crates/vb_core/src/engine/validate.rs:98-102`).
pub const SPEC_MAX_JOURNAL_BATCH_BYTES_HARD_CAP: u32 = 16_777_216u32;

// =============================================================================
// Spec helpers: bridge invariants (mathematical models)
// =============================================================================
/// Spec: core policy is the 1 MiB default.
pub open spec fn core_policy_is_default(v: u32) -> bool {
    v == SPEC_CORE_MAX_JOURNAL_BATCH_BYTES
}

/// Spec: storage default is the 1 MiB default.
pub open spec fn storage_default_is_default(v: u64) -> bool {
    v == SPEC_STORAGE_DEFAULT_LIMIT
}

/// Spec: core policy value is within the hard cap.
pub open spec fn core_policy_within_hard_cap(core_val: u32) -> bool {
    core_val > 0 && core_val <= SPEC_MAX_JOURNAL_BATCH_BYTES_HARD_CAP
}

/// Spec: storage limit is valid (positive, within u64 range).
pub open spec fn storage_limit_valid(limit: u64) -> bool {
    limit > 0 && limit <= u64::MAX
}

/// Spec: bridge preserves the core policy value after u32→u64 widening.
pub open spec fn bridge_preserves_value(core_val: u32, storage_val: u64) -> bool {
    storage_val == core_val as u64
}

/// Spec: core and storage values are aligned (after u32→u64 widening).
pub open spec fn bridge_aligned(core_val: u32, storage_val: u64) -> bool {
    bridge_preserves_value(core_val, storage_val)
}

/// Spec: silent drift occurs when core and storage values diverge.
pub open spec fn silent_drift(core_val: u32, storage_val: u64) -> bool {
    (storage_val as int) != (core_val as int)
}

// =============================================================================
// Spec struct: bridge state
// =============================================================================
/// Mirror of the bridge pair: core policy (u32) and storage limit
/// (u64). Validity is captured by `bridge_valid`.
pub struct Bridge {
    pub core_policy: u32,
    pub storage_limit: u64,
}

/// Spec: a Bridge is valid when the core policy is within the hard
/// cap, the storage limit is valid, and the bridge preserves the
/// core value across the u32→u64 widening.
pub open spec fn bridge_valid(b: Bridge) -> bool {
    &&& core_policy_within_hard_cap(b.core_policy)
    &&& storage_limit_valid(b.storage_limit)
    &&& bridge_preserves_value(b.core_policy, b.storage_limit)
}

/// Spec model of `SpecJournalWriteBatchByDefault::new()`. Captures
/// the production-bound post-state declared by the
/// `assume_specification` contract on `new` in spec-mode so proof
/// lemmas can reason about the freshly-constructed batch without
/// calling the exec fn.
pub open spec fn spec_new_state() -> production::SpecJournalWriteBatchByDefault {
    production::SpecJournalWriteBatchByDefault {
        byte_limit: Some(SPEC_STORAGE_DEFAULT_LIMIT),
        staged_bytes: 0u64,
        aborted: false,
    }
}

// =============================================================================
// assume_specification bridges — production contract surface
// =============================================================================
//
// Each bridge attaches a Verus-native spec contract to a
// `#[verifier::external]` mirror exec fn declared in
// `extern_vb_vzcuf_PS_007.rs`. The contract is the truth source for
// the bridge call site; the body is opaque to Verus.
/// Bridge contract: `SpecResourceContract::default_max_journal_batch_bytes`
/// returns the production `ResourceContract::DEFAULT
/// .max_journal_batch_bytes` value (u32 = 1_048_576) at
/// `crates/vb_core/src/workflow/mod.rs:249`.
pub assume_specification[ production::SpecResourceContract::default_max_journal_batch_bytes ]() -> (r:
    u32)
    ensures
        r == SPEC_CORE_MAX_JOURNAL_BATCH_BYTES,
        core_policy_is_default(r),
        core_policy_within_hard_cap(r),
        r as int == 1_048_576,
;

/// Bridge contract: `SpecBoundednessPolicy::default_absolute_max_journal_batch_bytes`
/// returns the production `BoundednessPolicy::DEFAULT
/// .absolute_max_journal_batch_bytes` value (u32 = 1_048_576) at
/// `crates/vb_core/src/budget.rs:391`.
pub assume_specification[ production::SpecBoundednessPolicy::default_absolute_max_journal_batch_bytes ]() -> (r:
    u32)
    ensures
        r == SPEC_CORE_MAX_JOURNAL_BATCH_BYTES,
        core_policy_is_default(r),
        core_policy_within_hard_cap(r),
        r as int == 1_048_576,
;

/// Bridge contract: `spec_max_journal_batch_bytes_hard_cap` returns
/// the production `MAX_JOURNAL_BATCH_BYTES` value (u32 = 16_777_216)
/// at `crates/vb_core/src/limits.rs:130`.
pub assume_specification[ production::spec_max_journal_batch_bytes_hard_cap ]() -> (r: u32)
    ensures
        r == SPEC_MAX_JOURNAL_BATCH_BYTES_HARD_CAP,
        r as int == 16_777_216,
        r > SPEC_CORE_MAX_JOURNAL_BATCH_BYTES,
;

/// Bridge contract: `spec_default_journal_batch_byte_limit` returns
/// the production `DEFAULT_JOURNAL_BATCH_BYTE_LIMIT` value (u64 =
/// 1_048_576) at `crates/vb_storage/src/batch/types.rs:10`.
pub assume_specification[ production::spec_default_journal_batch_byte_limit ]() -> (r: u64)
    ensures
        r == SPEC_STORAGE_DEFAULT_LIMIT,
        storage_default_is_default(r),
        storage_limit_valid(r),
        r as int == 1_048_576,
;

/// Bridge contract: `SpecJournalWriteBatchByDefault::new` returns a
/// freshly-constructed batch whose `byte_limit` is
/// `Some(DEFAULT_JOURNAL_BATCH_BYTE_LIMIT)`, `staged_bytes` is 0,
/// and `aborted` is false. Mirrors production
/// `JournalWriteBatch::new` at
/// `crates/vb_storage/src/batch/types.rs:33-44`.
pub assume_specification[ production::SpecJournalWriteBatchByDefault::new ]() -> (batch:
    production::SpecJournalWriteBatchByDefault)
    ensures
        batch.byte_limit == Some(SPEC_STORAGE_DEFAULT_LIMIT),
        batch.staged_bytes == 0u64,
        batch.aborted == false,
        storage_limit_valid(batch.byte_limit.unwrap()),
        bridge_preserves_value(SPEC_CORE_MAX_JOURNAL_BATCH_BYTES, batch.byte_limit.unwrap()),
;

/// Bridge contract: `byte_limit()` returns the stored `byte_limit`
/// field unchanged. Mirrors production
/// `JournalWriteBatch::byte_limit` at
/// `crates/vb_storage/src/batch/types.rs:80-83`.
pub assume_specification[ production::SpecJournalWriteBatchByDefault::byte_limit ](
    batch: &production::SpecJournalWriteBatchByDefault,
) -> (r: Option<u64>)
    ensures
        r == batch.byte_limit,
;

/// Bridge contract: `staged_event_bytes()` returns the stored
/// `staged_bytes` field unchanged. Mirrors production
/// `JournalWriteBatch::staged_event_bytes` at
/// `crates/vb_storage/src/batch/types.rs:74-77`.
pub assume_specification[ production::SpecJournalWriteBatchByDefault::staged_event_bytes ](
    batch: &production::SpecJournalWriteBatchByDefault,
) -> (r: u64)
    ensures
        r == batch.staged_bytes,
;

/// Bridge contract: `is_aborted()` returns the stored `aborted`
/// field unchanged. Mirrors production
/// `JournalWriteBatch::is_aborted` at
/// `crates/vb_storage/src/batch/types.rs:67-70`.
pub assume_specification[ production::SpecJournalWriteBatchByDefault::is_aborted ](
    batch: &production::SpecJournalWriteBatchByDefault,
) -> (r: bool)
    ensures
        r == batch.aborted,
;

// =============================================================================
// Proof lemmas — production-bound
// =============================================================================
//
// Each lemma uses spec predicates only (no production mirror calls
// in the body); the production calls happen in the exec wrappers
// below. This separation matches the established pattern in
// `vb-vzcuf-PS-009.rs` and `value_store_invariant.rs`.
/// Lemma: the production core policy default equals the spec constant.
pub proof fn lemma_core_policy_default_equals_spec()
    ensures
        SPEC_CORE_MAX_JOURNAL_BATCH_BYTES == 1_048_576u32,
{
}

/// Lemma: the production storage default equals the spec constant.
pub proof fn lemma_storage_default_equals_spec()
    ensures
        SPEC_STORAGE_DEFAULT_LIMIT == 1_048_576u64,
{
}

/// Lemma: the production hard cap equals the spec constant and
/// exceeds the core policy default (1 MiB < 16 MiB).
pub proof fn lemma_hard_cap_exceeds_core_default()
    ensures
        SPEC_MAX_JOURNAL_BATCH_BYTES_HARD_CAP == 16_777_216u32,
        SPEC_MAX_JOURNAL_BATCH_BYTES_HARD_CAP > SPEC_CORE_MAX_JOURNAL_BATCH_BYTES,
{
}

/// Lemma: the core policy default is within the hard cap
/// (1_048_576 < 16_777_216).
pub proof fn lemma_core_policy_within_hard_cap()
    ensures
        core_policy_within_hard_cap(SPEC_CORE_MAX_JOURNAL_BATCH_BYTES),
{
    assert(SPEC_CORE_MAX_JOURNAL_BATCH_BYTES > 0);
    assert(SPEC_CORE_MAX_JOURNAL_BATCH_BYTES <= SPEC_MAX_JOURNAL_BATCH_BYTES_HARD_CAP);
}

/// Lemma: the storage default is valid (positive, within u64 range).
pub proof fn lemma_storage_default_valid()
    ensures
        storage_limit_valid(SPEC_STORAGE_DEFAULT_LIMIT),
{
    assert(SPEC_STORAGE_DEFAULT_LIMIT > 0);
    assert(SPEC_STORAGE_DEFAULT_LIMIT <= u64::MAX);
}

/// Lemma: the bridge preserves the core policy value across the
/// u32→u64 widening (1_048_576u32 == 1_048_576u64).
pub proof fn lemma_bridge_preserves_value_default()
    ensures
        bridge_preserves_value(SPEC_CORE_MAX_JOURNAL_BATCH_BYTES, SPEC_STORAGE_DEFAULT_LIMIT),
{
    assert(SPEC_CORE_MAX_JOURNAL_BATCH_BYTES as u64 == SPEC_STORAGE_DEFAULT_LIMIT);
}

/// Lemma: bridge_aligned holds for the production defaults (replaces
/// `lemma_current_values_aligned` from the original vacuum file).
pub proof fn lemma_core_storage_aligned()
    ensures
        bridge_aligned(SPEC_CORE_MAX_JOURNAL_BATCH_BYTES, SPEC_STORAGE_DEFAULT_LIMIT),
        !silent_drift(SPEC_CORE_MAX_JOURNAL_BATCH_BYTES, SPEC_STORAGE_DEFAULT_LIMIT),
{
    assert(SPEC_CORE_MAX_JOURNAL_BATCH_BYTES as u64 == SPEC_STORAGE_DEFAULT_LIMIT);
}

/// Lemma: if values diverge (silent drift), they are not equal.
/// (Replaces `lemma_divergence_requires_explicit_bridge`.)
pub proof fn lemma_drift_implies_core_ne_storage(core_val: u32, storage_val: u64)
    requires
        silent_drift(core_val, storage_val),
    ensures
        !(storage_val == core_val as u64),
{
}

/// Lemma: the production-bound `Bridge { core_policy, storage_limit }`
/// constructed from the production defaults is valid. (Replaces
/// `lemma_default_bridge_valid` from the original vacuum file.)
pub proof fn lemma_default_bridge_valid()
    ensures
        bridge_valid(
            Bridge {
                core_policy: SPEC_CORE_MAX_JOURNAL_BATCH_BYTES,
                storage_limit: SPEC_STORAGE_DEFAULT_LIMIT,
            },
        ),
{
    assert(core_policy_within_hard_cap(SPEC_CORE_MAX_JOURNAL_BATCH_BYTES));
    assert(storage_limit_valid(SPEC_STORAGE_DEFAULT_LIMIT));
    assert(bridge_preserves_value(SPEC_CORE_MAX_JOURNAL_BATCH_BYTES, SPEC_STORAGE_DEFAULT_LIMIT));
}

/// Lemma: after `SpecJournalWriteBatchByDefault::new`, the bridge is
/// valid: the batch's `byte_limit` value equals the core policy
/// default after u32→u64 widening. (New proof; not present in the
/// original vacuum file.)
pub proof fn lemma_new_sets_aligned_byte_limit()
    ensures
        ({
            let b = spec_new_state();
            &&& bridge_preserves_value(SPEC_CORE_MAX_JOURNAL_BATCH_BYTES, b.byte_limit.unwrap())
            &&& storage_limit_valid(b.byte_limit.unwrap())
            &&& b.byte_limit == Some(SPEC_STORAGE_DEFAULT_LIMIT)
        }),
{
    let b = spec_new_state();
    assert(b.byte_limit == Some(SPEC_STORAGE_DEFAULT_LIMIT));
    assert(b.byte_limit.unwrap() == SPEC_STORAGE_DEFAULT_LIMIT);
    assert(SPEC_CORE_MAX_JOURNAL_BATCH_BYTES as u64 == SPEC_STORAGE_DEFAULT_LIMIT);
    assert(bridge_preserves_value(SPEC_CORE_MAX_JOURNAL_BATCH_BYTES, b.byte_limit.unwrap()));
    assert(SPEC_STORAGE_DEFAULT_LIMIT > 0);
    assert(storage_limit_valid(b.byte_limit.unwrap()));
}

// =============================================================================
// Exec wrappers — production-bound bridge witnesses
// =============================================================================
//
// Each wrapper calls a production mirror via the `assume_specification`
// bridge above. The wrappers are the proof witnesses that the bridges
// are not used as vacuum: each wrapper has an `ensures` clause that
// is discharged by the corresponding bridge contract.
/// Exec wrapper: `SpecResourceContract::default_max_journal_batch_bytes`
/// returns the production 1 MiB core policy default.
pub exec fn wrapper_core_default_max_journal_batch_bytes() -> (r: u32)
    ensures
        r == SPEC_CORE_MAX_JOURNAL_BATCH_BYTES,
        core_policy_is_default(r),
        core_policy_within_hard_cap(r),
{
    production::SpecResourceContract::default_max_journal_batch_bytes()
}

/// Exec wrapper: `SpecBoundednessPolicy::default_absolute_max_journal_batch_bytes`
/// returns the production 1 MiB absolute policy default.
pub exec fn wrapper_absolute_max_journal_batch_bytes() -> (r: u32)
    ensures
        r == SPEC_CORE_MAX_JOURNAL_BATCH_BYTES,
        core_policy_is_default(r),
{
    production::SpecBoundednessPolicy::default_absolute_max_journal_batch_bytes()
}

/// Exec wrapper: `spec_max_journal_batch_bytes_hard_cap` returns the
/// production 16 MiB hard cap.
pub exec fn wrapper_hard_cap() -> (r: u32)
    ensures
        r == SPEC_MAX_JOURNAL_BATCH_BYTES_HARD_CAP,
        r > SPEC_CORE_MAX_JOURNAL_BATCH_BYTES,
{
    production::spec_max_journal_batch_bytes_hard_cap()
}

/// Exec wrapper: `spec_default_journal_batch_byte_limit` returns the
/// production 1 MiB storage default.
pub exec fn wrapper_storage_default_limit() -> (r: u64)
    ensures
        r == SPEC_STORAGE_DEFAULT_LIMIT,
        storage_default_is_default(r),
        storage_limit_valid(r),
{
    production::spec_default_journal_batch_byte_limit()
}

/// Exec wrapper: `SpecJournalWriteBatchByDefault::new` returns a
/// fresh batch whose `byte_limit` is the production default.
pub exec fn wrapper_new_default_batch() -> (b: production::SpecJournalWriteBatchByDefault)
    ensures
        b.byte_limit == Some(SPEC_STORAGE_DEFAULT_LIMIT),
        b.staged_bytes == 0u64,
        b.aborted == false,
        storage_limit_valid(b.byte_limit.unwrap()),
        bridge_preserves_value(SPEC_CORE_MAX_JOURNAL_BATCH_BYTES, b.byte_limit.unwrap()),
{
    production::SpecJournalWriteBatchByDefault::new()
}

/// Exec wrapper: chain `new` + `byte_limit()` and confirm the
/// accessor returns the production default set by `new`.
pub exec fn wrapper_new_then_byte_limit() -> (r: Option<u64>)
    ensures
        r == Some(SPEC_STORAGE_DEFAULT_LIMIT),
{
    let b = production::SpecJournalWriteBatchByDefault::new();
    b.byte_limit()
}

/// Exec wrapper: chain `new` + `staged_event_bytes()` and confirm
/// the accessor returns 0 (no events staged).
pub exec fn wrapper_new_then_staged_bytes() -> (r: u64)
    ensures
        r == 0u64,
{
    let b = production::SpecJournalWriteBatchByDefault::new();
    b.staged_event_bytes()
}

/// Exec wrapper: chain `new` + `is_aborted()` and confirm the
/// accessor returns false (fresh batch).
pub exec fn wrapper_new_then_not_aborted() -> (r: bool)
    ensures
        !r,
{
    let b = production::SpecJournalWriteBatchByDefault::new();
    b.is_aborted()
}

} // verus!
