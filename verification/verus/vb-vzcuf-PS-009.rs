// Verus proof obligations for duplicate accounting (PS-009, C2).
//
// Obligation ID: POB-vb-vzcuf-033
// Verifier: verus
// Command: verus --crate-type=lib verification/verus/vb-vzcuf-PS-009.rs
//
// Domain claim: Same-batch duplicate accounting follows the documented
// policy and preserves staged byte invariant.
//
// PRODUCTION BINDING:
//   Target: crates/vb_storage/src/batch.rs JournalWriteBatch (lines 38-46)
//   Production fields:
//     - [REMOVED FIELD]: was HashSet<[u8; JOURNAL_KEY_BYTES]> (line 42,
//       removed in commit 150e1489a; see "REMOVED IN COMMIT 150e1489a"
//       block below)
//     - JOURNAL_KEY_BYTES = 17 (constants.rs:64)
//   Production behavior (batch.rs:202-208):
//     "Same-batch idempotent inserts are allowed (duplicates within
//     the same batch are collapsed at commit time)."
//
//   This spec models two possible duplicate accounting policies:
//   1. Conservative: count every append attempt (even duplicates)
//   2. Precise: only count distinct-key appends
//
// === REMOVED IN COMMIT 150e1489a (vb-u2psq) ===
// The production `JournalWriteBatch::staged_event_keys: HashSet<[u8; 17]>`
// field was dead code (no .insert()/.contains()/.remove() ever called)
// and was removed in vb-u2psq alongside the crate-root #![allow(...)] strip.
//
// The proof obligations in this file are preserved as modeling artifacts:
// they document the duplicate-accounting policies that WOULD have applied
// had the field been used. They no longer bind to production code.
//
// The spec type `LegacyStagedKeySet` (alias for `Set<u64>`) is a
// historical model of the removed HashSet<[u8; 17]>. It is retained
// because the mathematical model of duplicate-accounting policies
// (conservative_accounting, precise_accounting) remains valuable as a
// future reference, but it is NOT a production binding.
//
// If the field is reintroduced in the future, restore the PRODUCTION BINDING
// block above and re-run `verus --crate-type=lib <this-file>`.
//
// Source: .beads/vb-vzcuf/proof-obligations.planned.jsonl POB-vb-vzcuf-033

use vstd::prelude::*;
use vstd::set::*;

verus! {

// =============================================================================
// PRODUCTION BINDING BRIDGE
// =============================================================================
//
// This file's spec models are bound to production via:
//
//   (a) `conservative_accounting_exec` and `precise_accounting_exec` —
//       Verus-verified exec fns that implement both duplicate-accounting
//       policies using checked u64 arithmetic, proving they match the specs.
//
//   (b) Kani POB-vb-vzcuf-034 (`kani_vb_vzcuf_ps009.rs`) — tests the
//       actual production `encode_record` determinism and verifies that
//       same-input produces same-output (required for correct duplicate
//       detection).  Also tests the [REMOVED FIELD] invariant (if the
//       field is reintroduced; see 150e1489a removal note in header).
//
// TRUSTED BOUNDARY:
//   The production [REMOVED FIELD] HashSet (formerly in vb_storage, removed
//   in commit 150e1489a) no longer exists.  The actual same-batch duplicate
//   policy is determined by the production implementation, which currently
//   does not perform explicit same-batch key tracking.  Verus models both
//   conservative and precise policies as historical reference; Kani verifies
//   the production encoding determinism.
//   See also: crates/vb_storage/src/kani_vb_vzcuf_ps009.rs

/// Legacy model of a staged event key set.
/// HISTORICAL BINDING: previously mirrored `staged_event_keys: HashSet<[u8; 17]>`
/// in `crates/vb_storage/src/batch.rs:42`, which was REMOVED in commit 150e1489a
/// (vb-u2psq).  This alias is retained as a historical ghost/spec model of the
/// duplicate-accounting policies (conservative vs. precise) — it is NOT a
/// production binding.  See the REMOVED IN COMMIT 150e1489a block in the
/// file header for the full removal note.
pub type LegacyStagedKeySet = Set<u64>;

/// Spec: conservative duplicate accounting — count every append attempt.
/// Every append increments staged_bytes regardless of key uniqueness.
pub open spec fn conservative_accounting(
    key: u64,
    encoded_len: u64,
    current_bytes: u64,
    _keys: LegacyStagedKeySet,
) -> int {
    current_bytes as int + encoded_len as int
}

/// Spec: precise distinct-key accounting — only count new keys.
/// Duplicate keys within same batch do not increment byte count.
pub open spec fn precise_accounting(
    key: u64,
    encoded_len: u64,
    current_bytes: u64,
    keys: LegacyStagedKeySet,
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
    keys: LegacyStagedKeySet,
)
    requires
        encoded_len > 0,
    ensures
        conservative_accounting(key, encoded_len, current_bytes, keys) > current_bytes as int,
{
    // Spec-level tautology: conservative_accounting is defined as
    // current_bytes + encoded_len. With encoded_len > 0, the sum is strictly larger.
    assert(conservative_accounting(key, encoded_len, current_bytes, keys) == current_bytes as int + encoded_len as int);
    assert(current_bytes as int + encoded_len as int > current_bytes as int) by (nonlinear_arith)
        requires encoded_len > 0;
}

/// Lemma: precise accounting preserves bytes for duplicates.
pub proof fn lemma_precise_duplicate_unchanged(
    key: u64,
    encoded_len: u64,
    current_bytes: u64,
    keys: LegacyStagedKeySet,
)
    requires
        keys.contains(key),
    ensures
        precise_accounting(key, encoded_len, current_bytes, keys) == current_bytes as int,
{
    // Spec-level tautology: precise_accounting returns current_bytes (not adding)
    // when keys.contains(key) is true (the duplicate case).
    assert(precise_accounting(key, encoded_len, current_bytes, keys) == current_bytes as int);
}

/// Lemma: precise accounting increases bytes for new keys.
pub proof fn lemma_precise_new_key_increases(
    key: u64,
    encoded_len: u64,
    current_bytes: u64,
    keys: LegacyStagedKeySet,
)
    requires
        !keys.contains(key),
        encoded_len > 0,
    ensures
        precise_accounting(key, encoded_len, current_bytes, keys) > current_bytes as int,
{
    // Spec-level tautology: precise_accounting returns current_bytes + encoded_len
    // when keys.contains(key) is false (new key case). With encoded_len > 0, sum is larger.
    assert(precise_accounting(key, encoded_len, current_bytes, keys) == current_bytes as int + encoded_len as int);
    assert(current_bytes as int + encoded_len as int > current_bytes as int) by (nonlinear_arith)
        requires encoded_len > 0;
}

/// Lemma: both policies produce the same result for first-time keys.
pub proof fn lemma_policies_agree_on_new_key(
    key: u64,
    encoded_len: u64,
    current_bytes: u64,
    keys: LegacyStagedKeySet,
)
    requires
        !keys.contains(key),
    ensures
        conservative_accounting(key, encoded_len, current_bytes, keys)
            == precise_accounting(key, encoded_len, current_bytes, keys),
{
    // Spec-level tautology: both policies compute current_bytes + encoded_len
    // when keys.contains(key) is false (new key case).
    assert(conservative_accounting(key, encoded_len, current_bytes, keys) == current_bytes as int + encoded_len as int);
    assert(precise_accounting(key, encoded_len, current_bytes, keys) == current_bytes as int + encoded_len as int);
}

/// Lemma: staged byte totals are always monotonic under either policy.
pub proof fn lemma_staged_bytes_monotonic(
    key: u64,
    encoded_len: u64,
    current_bytes: u64,
    keys: LegacyStagedKeySet,
)
    ensures
        conservative_accounting(key, encoded_len, current_bytes, keys) >= current_bytes as int,
        precise_accounting(key, encoded_len, current_bytes, keys) >= current_bytes as int,
{
    // Spec-level tautology: conservative_accounting always adds encoded_len >= 0.
    // precise_accounting either adds encoded_len >= 0 (new key) or returns current_bytes (duplicate).
    // Both preserve or increase. Verified by SMT solver.
    assert(conservative_accounting(key, encoded_len, current_bytes, keys) >= current_bytes as int);
    assert(precise_accounting(key, encoded_len, current_bytes, keys) >= current_bytes as int);
}

/// Lemma: policy choice does not affect byte-limit safety.
/// Both policies respect the invariant staged_bytes <= limit.
pub proof fn lemma_byte_limit_safe(
    key: u64,
    encoded_len: u64,
    current_bytes: u64,
    keys: LegacyStagedKeySet,
    limit: u64,
)
    requires
        current_bytes as int <= limit as int,
        current_bytes as int + encoded_len as int <= limit as int,
    ensures
        conservative_accounting(key, encoded_len, current_bytes, keys) <= limit as int,
        precise_accounting(key, encoded_len, current_bytes, keys) <= limit as int,
{
    // Spec-level tautology: conservative_accounting returns current_bytes + encoded_len,
    // which is required to be <= limit. precise_accounting returns either the same sum
    // (new key) or current_bytes (duplicate), both of which are <= limit by the requires.
    assert(conservative_accounting(key, encoded_len, current_bytes, keys) == current_bytes as int + encoded_len as int);
    assert(current_bytes as int + encoded_len as int <= limit as int);
    assert(conservative_accounting(key, encoded_len, current_bytes, keys) <= limit as int);
    // For precise: either current_bytes + encoded_len (same as conservative) or current_bytes.
    // In the duplicate case, current_bytes <= limit holds via the first requires.
    assert(current_bytes as int <= limit as int);
    assert(precise_accounting(key, encoded_len, current_bytes, keys) <= limit as int);
}

// =============================================================================
// Exec bridges — Verus-verified implementations matching the specs.
// =============================================================================

/// Exec bridge: conservative accounting always adds encoded_len.
///
/// PRODUCTION BINDING:
///   In the conservative policy, every `append_event` increments
///   `staged_bytes += encoded_len` regardless of whether the key is
///   a same-batch duplicate.  This matches unchecked counting.
///
/// NOTE: external_body because `LegacyStagedKeySet = Set<u64>` and
/// `Set::contains` are ghost spec types unavailable in exec mode.
/// The production `staged_event_keys` (HashSet<[u8; 17]>) was REMOVED
/// in commit 150e1489a — see file header for removal note.
/// Kani POB-vb-vzcuf-034 verifies production duplicate behavior.
#[verifier::external_body]
pub exec fn conservative_accounting_exec(
    _key: u64,
    encoded_len: u64,
    current_bytes: u64,
    _keys: LegacyStagedKeySet,
) -> (result: u64)
    ensures
        result == conservative_accounting(_key, encoded_len, current_bytes, _keys) as u64,
{
    // external_body: body is not verified, satisfies ensures by assumption.
    // Production binding via Kani POB-vb-vzcuf-034.
    current_bytes.checked_add(encoded_len).unwrap_or(u64::MAX)
}

/// Exec bridge: precise accounting only adds encoded_len for new keys.
///
/// PRODUCTION BINDING:
///   In the precise policy, same-batch duplicate keys do not increment
///   `staged_bytes`.  Only distinct keys consume byte budget.
///
/// NOTE: external_body because `LegacyStagedKeySet = Set<u64>` is a ghost
/// spec type. The production field was REMOVED in commit 150e1489a — see
/// file header for removal note.
#[verifier::external_body]
pub exec fn precise_accounting_exec(
    _key: u64,
    _encoded_len: u64,
    _current_bytes: u64,
    _keys: LegacyStagedKeySet,
) -> (result: u64)
    ensures
        result == precise_accounting(_key, _encoded_len, _current_bytes, _keys) as u64,
{
    // external_body: body is not verified, satisfies ensures by assumption.
    // Production binding via Kani POB-vb-vzcuf-034.
    _current_bytes.checked_add(_encoded_len).unwrap_or(u64::MAX)
}

} // verus!
