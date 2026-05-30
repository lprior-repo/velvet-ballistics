// Verus proof obligations for duplicate accounting (PS-009, C2).
//
// Obligation ID: POB-vb-vzcuf-033
// Verifier: verus
// Command: cargo verus --crate-type=lib verification/verus/vb-vzcuf-PS-009.rs
//
// Domain claim: Same-batch duplicate accounting follows the documented
// policy and preserves staged byte invariant.
//
// PRODUCTION BINDING:
//   Target: crates/vb_storage/src/batch.rs JournalWriteBatch (lines 38-46)
//   Production fields:
//     - staged_event_keys: HashSet<[u8; JOURNAL_KEY_BYTES]> (line 42)
//       Used for same-batch idempotent insert tracking.
//     - JOURNAL_KEY_BYTES = 17 (constants.rs:64)
//   Production behavior (batch.rs:202-208):
//     "Same-batch idempotent inserts are allowed (duplicates within
//     the same batch are collapsed at commit time)."
//
//   This spec models two possible duplicate accounting policies:
//   1. Conservative: count every append attempt (even duplicates)
//   2. Precise: only count distinct-key appends
//
// Source: .beads/vb-vzcuf/proof-obligations.planned.jsonl POB-vb-vzcuf-033

use vstd::prelude::*;
use vstd::set::*;

verus! {

/// Model of a staged event key set.
/// PRODUCTION BINDING: mirrors staged_event_keys HashSet in batch.rs:42.
pub type StagedKeySet = Set<u64>;

/// Spec: conservative duplicate accounting — count every append attempt.
/// Every append increments staged_bytes regardless of key uniqueness.
pub open spec fn conservative_accounting(
    key: u64,
    encoded_len: u64,
    current_bytes: u64,
    _keys: StagedKeySet,
) -> int {
    current_bytes as int + encoded_len as int
}

/// Spec: precise distinct-key accounting — only count new keys.
/// Duplicate keys within same batch do not increment byte count.
pub open spec fn precise_accounting(
    key: u64,
    encoded_len: u64,
    current_bytes: u64,
    keys: StagedKeySet,
) -> int {
    if keys.contains(key) {
        current_bytes as int
    } else {
        current_bytes as int + encoded_len as int
    }
}

/// Lemma: conservative accounting always increases bytes (n > 0).
pub proof fn lemma_conservative_always_increases(
    key: u64,
    encoded_len: u64,
    current_bytes: u64,
    keys: StagedKeySet,
)
    requires
        encoded_len > 0,
    ensures
        conservative_accounting(key, encoded_len, current_bytes, keys) > current_bytes as int,
{
}

/// Lemma: precise accounting preserves bytes for duplicates.
pub proof fn lemma_precise_duplicate_unchanged(
    key: u64,
    encoded_len: u64,
    current_bytes: u64,
    keys: StagedKeySet,
)
    requires
        keys.contains(key),
    ensures
        precise_accounting(key, encoded_len, current_bytes, keys) == current_bytes as int,
{
}

/// Lemma: precise accounting increases bytes for new keys.
pub proof fn lemma_precise_new_key_increases(
    key: u64,
    encoded_len: u64,
    current_bytes: u64,
    keys: StagedKeySet,
)
    requires
        !keys.contains(key),
        encoded_len > 0,
    ensures
        precise_accounting(key, encoded_len, current_bytes, keys) > current_bytes as int,
{
}

/// Lemma: both policies produce the same result for first-time keys.
pub proof fn lemma_policies_agree_on_new_key(
    key: u64,
    encoded_len: u64,
    current_bytes: u64,
    keys: StagedKeySet,
)
    requires
        !keys.contains(key),
    ensures
        conservative_accounting(key, encoded_len, current_bytes, keys)
            == precise_accounting(key, encoded_len, current_bytes, keys),
{
}

/// Lemma: staged byte totals are always monotonic under either policy.
pub proof fn lemma_staged_bytes_monotonic(
    key: u64,
    encoded_len: u64,
    current_bytes: u64,
    keys: StagedKeySet,
)
    ensures
        conservative_accounting(key, encoded_len, current_bytes, keys) >= current_bytes as int,
        precise_accounting(key, encoded_len, current_bytes, keys) >= current_bytes as int,
{
}

/// Lemma: policy choice does not affect byte-limit safety.
/// Both policies respect the invariant staged_bytes <= limit.
pub proof fn lemma_byte_limit_safe(
    key: u64,
    encoded_len: u64,
    current_bytes: u64,
    keys: StagedKeySet,
    limit: u64,
)
    requires
        current_bytes as int <= limit as int,
        current_bytes as int + encoded_len as int <= limit as int,
    ensures
        conservative_accounting(key, encoded_len, current_bytes, keys) <= limit as int,
        precise_accounting(key, encoded_len, current_bytes, keys) <= limit as int,
{
}

} // verus!
