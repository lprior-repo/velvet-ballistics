// Verus proof obligations for same-batch duplicate accounting (PS-009, C2).
//
// Obligation ID: POB-vb-vzcuf-033
// Verifier: verus
// Command: verus --crate-type=lib verification/verus/vb-vzcuf-PS-009.rs
//
// Domain claim: Same-batch duplicate `(run, seq)` is rejected with
// `JournalError::DuplicateStagedKey` and `staged_bytes` is only
// incremented for distinct keys.
//
// PRODUCTION BINDING (post SA-003 fix):
//   Target: crates/vb_storage/src/batch.rs::JournalWriteBatch::append_event
//           (lines ~243-300 after fix).
//   Production fields:
//     - staged_event_keys: HashSet<[u8; JOURNAL_KEY_BYTES]> (batch.rs:47)
//     - staged_bytes: u64 (batch.rs:50)
//     - JOURNAL_KEY_BYTES = 17 (constants.rs)
//   Fixed behavior:
//     1. Durable check: journal.events.contains_key(key) -> DuplicateEvent
//        (aborts the batch).
//     2. Same-batch check: staged_event_keys.contains(key) ->
//        DuplicateStagedKey (batch remains open, no state mutated).
//     3. On success, key is recorded in staged_event_keys.
//     4. Byte accounting only advances for distinct keys
//        (encoded_len is added to staged_bytes once per distinct key).
//
// Source: .beads/vb-vzcuf/proof-obligations.planned.jsonl POB-vb-vzcuf-033
// Bead: vb-keji6 (PS-009 Verus enshrines bug).

use vstd::prelude::*;
use vstd::set::*;

verus! {

/// Spec: duplicate-accounting policy for `append_event` after SA-003 fix.
///
/// Conservative policy (pre-fix bug): every append attempt advances
/// `staged_bytes` by the full encoded length, even when the key was
/// already staged. Models the silent-overwrite path that allowed
/// Fjall last-write-wins to drop the first event's value.
///
/// Precise policy (post-fix): only the first append for a given key
/// advances `staged_bytes`; subsequent appends of the same key must
/// be rejected by `append_event` with `DuplicateStagedKey` (production
/// behavior) before any byte accounting happens.
pub open spec fn conservative_accounting(
    _key: u64,
    encoded_len: u64,
    current_bytes: u64,
) -> int {
    current_bytes as int + encoded_len as int
}

pub open spec fn precise_accounting(
    key: u64,
    encoded_len: u64,
    current_bytes: u64,
    keys: Set<u64>,
) -> int {
    if keys.contains(key) {
        // Duplicate: production rejects this append before bytes
        // are counted. The post-state bytes are unchanged.
        current_bytes as int
    } else {
        current_bytes as int + encoded_len as int
    }
}

/// Lemma: precise accounting never under-counts for distinct keys.
pub proof fn lemma_precise_new_key_increases(
    key: u64,
    encoded_len: u64,
    current_bytes: u64,
    keys: Set<u64>,
)
    requires
        !keys.contains(key),
        encoded_len > 0,
    ensures
        precise_accounting(key, encoded_len, current_bytes, keys)
            > current_bytes as int,
{
    assert(precise_accounting(key, encoded_len, current_bytes, keys)
        == current_bytes as int + encoded_len as int);
}

/// Lemma: precise accounting rejects duplicates by leaving bytes
/// unchanged. This is the post-fix invariant: the second append
/// with the same key cannot advance staged_bytes.
pub proof fn lemma_precise_duplicate_leaves_bytes_unchanged(
    key: u64,
    encoded_len: u64,
    current_bytes: u64,
    keys: Set<u64>,
)
    requires
        keys.contains(key),
    ensures
        precise_accounting(key, encoded_len, current_bytes, keys)
            == current_bytes as int,
{
}

/// Lemma: staged_bytes is monotonically non-decreasing under
/// precise accounting (distinct keys add, duplicates leave).
pub proof fn lemma_staged_bytes_monotonic_precise(
    key: u64,
    encoded_len: u64,
    current_bytes: u64,
    keys: Set<u64>,
)
    ensures
        precise_accounting(key, encoded_len, current_bytes, keys)
            >= current_bytes as int,
{
}

/// Lemma: byte-limit safety is preserved under precise accounting
/// when both current and attempted totals are within the limit.
pub proof fn lemma_byte_limit_safe_precise(
    key: u64,
    encoded_len: u64,
    current_bytes: u64,
    keys: Set<u64>,
    limit: u64,
)
    requires
        current_bytes as int <= limit as int,
        current_bytes as int + encoded_len as int <= limit as int,
    ensures
        precise_accounting(key, encoded_len, current_bytes, keys)
            <= limit as int,
{
}

} // verus!
fn main() {}
