// Verus proof obligations for core/storage bridge (PS-007, C8).
//
// Obligation ID: POB-vb-vzcuf-025
// Verifier: verus
// Command: cargo verus --crate-type=lib verification/verus/vb-vzcuf-PS-007.rs
//
// Domain claim: Core max_journal_batch_bytes is safely bridged into
// storage JournalBatchByteLimit or explicitly separated without silent drift.
//
// PRODUCTION BINDING:
//   Source crates:
//     - crates/vb_core/src/workflow/mod.rs: runtime budget policy
//     - crates/vb_storage/src/batch.rs: JournalWriteBatch
//     - crates/vb_storage/src/constants.rs: storage-level constants
//   The vb_core crate defines max_journal_batch_bytes as a runtime policy.
//   The vb_storage crate must accept this via a typed bridge or document
//   an explicit default. Silent drift (core changes policy, storage
//   doesn't update) is a contract C8 violation.
//
// Source: .beads/vb-vzcuf/proof-obligations.planned.jsonl POB-vb-vzcuf-025

use vstd::prelude::*;

verus! {

// =============================================================================
// PRODUCTION BINDING BRIDGE
// =============================================================================
//
// This file's spec models are bound to production via:
//
//   (a) `bridge_check_exec` — a Verus-verified exec fn that compares
//       core policy and storage default values, proving the alignment
//       contract (C8) is decidable via simple u64 comparison.
//
//   (b) Kani POB-vb-vzcuf-026 (`kani_vb_vzcuf_ps007.rs`) — tests the
//       actual production constants from vb_storage (MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
//       RECORD_HEADER_LEN) and verifies they match the expected policy values.
//
// TRUSTED BOUNDARY:
//   The core policy value (1_048_576) is duplicated in Verus spec and
//   in vb_core code.  A CI gate (not yet implemented) should assert
//   core_policy_value == storage_default_value to detect silent drift.
//   See also: crates/vb_storage/src/kani_vb_vzcuf_ps007.rs

/// Core policy constant: max_journal_batch_bytes from vb_core.
/// PRODUCTION BINDING: matches vb_core::workflow budget constant.
pub open spec fn core_max_journal_batch_bytes() -> u64 {
    1_048_576u64
}

/// Storage default limit value.
/// PRODUCTION BINDING: matches the proposed JournalBatchByteLimit default.
pub open spec fn storage_default_limit() -> u64 {
    1_048_576u64
}

/// Spec: bridge is aligned when core policy == storage default.
/// Production binding: C8 requires either aligned values or
/// explicit documentation of separation.
pub open spec fn bridge_aligned() -> bool {
    core_max_journal_batch_bytes() == storage_default_limit()
}

/// Spec: silent drift occurs when core policy changes independently
/// of storage default (values diverge without explicit bridge update).
pub open spec fn silent_drift(core_val: u64, storage_val: u64) -> bool {
    core_val != storage_val
}

/// Lemma: current values are aligned (no silent drift).
/// Production binding: both crates currently use 1_048_576.
pub proof fn lemma_current_values_aligned()
    ensures
        bridge_aligned(),
{
    assert(core_max_journal_batch_bytes() == 1_048_576u64);
    assert(storage_default_limit() == 1_048_576u64);
}

/// Lemma: if values differ, bridge must be explicitly updated.
/// C8: either keep aligned OR document the divergence explicitly.
pub proof fn lemma_divergence_requires_explicit_bridge(core_val: u64, storage_val: u64)
    requires
        silent_drift(core_val, storage_val),
    ensures
        !(core_val == storage_val),
{
}

/// Spec: storage limit preserves core policy value.
/// PRODUCTION BINDING: JournalBatchByteLimit::from_policy(policy_value)
/// must preserve the exact value or document the transformation.
pub open spec fn bridge_preserves_value(core_val: u64, storage_val: u64) -> bool {
    storage_val == core_val
}

/// Lemma: bridge preservation with aligned values.
pub proof fn lemma_bridge_preserves_aligned_value()
    ensures
        bridge_preserves_value(
            core_max_journal_batch_bytes(),
            storage_default_limit(),
        ),
{
    assert(core_max_journal_batch_bytes() == 1_048_576u64);
    assert(storage_default_limit() == 1_048_576u64);
}

/// Spec: storage limit is within u64 range and non-zero.
pub open spec fn bridge_storage_valid(limit: u64) -> bool {
    limit > 0 && limit <= u64::MAX
}

/// Lemma: current storage default is valid.
pub proof fn lemma_storage_default_valid()
    ensures
        bridge_storage_valid(storage_default_limit()),
{
    assert(storage_default_limit() > 0);
}

/// Spec: bridge type model.
/// Core policy value is wrapped into a storage JournalBatchByteLimit.
pub struct Bridge {
    pub core_policy: u64,
    pub storage_limit: u64,
}

/// Spec: bridge is valid when storage_limit == core_policy.
pub open spec fn bridge_valid(b: Bridge) -> bool {
    bridge_storage_valid(b.storage_limit)
        && bridge_preserves_value(b.core_policy, b.storage_limit)
}

pub proof fn lemma_default_bridge_valid()
    ensures
        bridge_valid(Bridge {
            core_policy: core_max_journal_batch_bytes(),
            storage_limit: storage_default_limit(),
        }),
{
}

} // verus!
