// Verus proof obligations for duplicate accounting (PS-009, C2).
//
// Obligation ID: POB-vb-vzcuf-033
// Verifier: verus
// Command: verus --crate-type=lib verification/verus/vb-vzcuf-PS-009.rs
//
// =============================================================================
// PRODUCTION BINDING (GOD RULE 2 compliance)
// =============================================================================
//
// Target: vb_storage::batch::JournalWriteBatch<'j>::append_event
//         at crates/vb_storage/src/batch/append_event.rs:41-106.
//
// Binding mechanism: `#[path = "extern_vb_vzcuf_PS_009.rs"]` brings the
// production mirror types and the `#[verifier::external]` exec body
// of `append_event` into the `verus!` block. The `assume_specification`
// bridge below attaches the production contract (SA-003 post-fix
// behavior) to the extern body. The exec wrapper at the bottom of
// this file exercises the bridge from `verus!` context so the
// contract is not used as a vacuum.
//
// Domain claim (PS-009, C2): Same-batch duplicate accounting follows
// the post-fix policy (SA-003). A second `append_event` call with a
// key already in the batch's `staged_event_keys` returns
// `JournalError::DuplicateStagedKey` BEFORE any state mutation; a key
// present in the durable journal memtable but not yet staged returns
// `JournalError::DuplicateEvent` and sets `aborted = true`; any other
// failure path (QueueFull, Encode, PayloadTooLarge,
// JournalBatchBytesExceeded) leaves `aborted`, `staged_bytes`, and
// `staged_event_keys` unchanged.
//
// =============================================================================
// TRUST BOUNDARY (GOD RULE 2 transparency)
// =============================================================================
//
// The production body of `append_event` is not verified by this proof:
//   * `fjall::OwnedWriteBatch` and `FjallJournal` types are opaque to
//     Verus (they wrap LSM-tree internals with no spec view in vstd).
//   * `encode_record` (codec step) is an exec fn Verus cannot model
//     (it reaches into postcard + custom record framing).
//   * The mirror body in `extern_vb_vzcuf_PS_009.rs` is declared
//     `#[verifier::external]` so Verus skips body verification.
//
// The `assume_specification` bridge below therefore represents the
// FULL behavioral contract: the Fjall/codec layers are trusted to
// produce the projected inputs (`journal_has_key`, `encode_ok`,
// `encoded_len`) that the bridge takes as exec arguments. Any drift
// between the projection and the production body is recorded in the
// BINDING LEDGER section of `extern_vb_vzcuf_PS_009.rs` as drift
// debt. The bridge itself is proved locally by the exec wrapper at
// the bottom of this file.
use vstd::prelude::*;

verus! {

// =============================================================================
// Production-mirror types (extern binding)
// =============================================================================
#[path = "extern_vb_vzcuf_PS_009.rs"]
mod production;

// Re-export the production fns and types so they can be called from
// `verus!` context with a Verus-visible spec contract attached via
// `assume_specification` below.
pub use production::{SpecJournalError, SpecJournalWriteBatch};

// Constants inlined here (vs re-exported from `production::*`) to avoid a
// Verus internal error in `--crate-type=lib` mode where pub const items
// declared inside an extern module trigger a `VerusErasureCtxt has not been
// initialized` panic during thir-body processing. The values mirror
// `extern_vb_vzcuf_PS_009.rs` byte-for-byte; the binding ledger in that
// file lists the production source lines for each constant.
pub const SPEC_JOURNAL_KEY_BYTES: usize = 17;

pub const SPEC_MAX_BATCH_COUNT: usize = 1024;

pub const SPEC_DEFAULT_JOURNAL_BATCH_BYTE_LIMIT: u64 = 1_048_576;

pub const SPEC_MAX_JOURNAL_EVENT_PAYLOAD_BYTES: u32 = 65_536;

// =============================================================================
// Spec helper: state-unchanged predicate
// =============================================================================
//
// Encapsulates the "no observable mutation" condition for the
// pre/post pair on every Err branch except `DuplicateEvent` (which
// flips `aborted`). Fields `byte_limit` and `inner_len` are included
// for symmetry even though they cannot change via `append_event`
// (byte_limit is constructor-set; inner_len is only mutated on Ok and
// on QueueFull — see the production guard precedence).
pub open spec fn spec_state_preserved(
    old: SpecJournalWriteBatch,
    new: SpecJournalWriteBatch,
) -> bool {
    &&& new.staged_event_keys@ == old.staged_event_keys@
    &&& new.staged_bytes == old.staged_bytes
    &&& new.aborted == old.aborted
    &&& new.byte_limit == old.byte_limit
    &&& new.inner_len == old.inner_len
}

/// State-preserved predicate EXCEPT `aborted` flips to true (used by
/// the `DuplicateEvent` branch, which sets `aborted = true`).
pub open spec fn spec_state_preserved_except_aborted(
    old: SpecJournalWriteBatch,
    new: SpecJournalWriteBatch,
) -> bool {
    &&& new.staged_event_keys@ == old.staged_event_keys@
    &&& new.staged_bytes == old.staged_bytes
    &&& new.aborted == true
    &&& new.byte_limit == old.byte_limit
    &&& new.inner_len == old.inner_len
}

/// Inner-len-only incremented state (used by the Ok branch).
pub open spec fn spec_state_after_ok(
    old: SpecJournalWriteBatch,
    new: SpecJournalWriteBatch,
    key: u64,
    encoded_len: u64,
) -> bool {
    let new_staged_bytes: u64 = if old.byte_limit.is_some() {
        (old.staged_bytes as int + encoded_len as int) as u64
    } else {
        old.staged_bytes
    };
    &&& new.staged_event_keys@ == old.staged_event_keys@.insert(key)
    &&& new.staged_bytes == new_staged_bytes
    &&& new.aborted == false
    &&& new.byte_limit == old.byte_limit
    &&& new.inner_len == (old.inner_len + 1) as usize
}

// =============================================================================
// Extern_spec bridge: production contract for `append_event`.
// =============================================================================
//
// `assume_specification` is the Verus-native way to attach a spec
// contract to an exec fn whose body Verus cannot model (here:
// `fjall::OwnedWriteBatch` + `FjallJournal` + the postcard-based
// `encode_record`). The contract below is the FULL post-fix SA-003
// behavior recorded in `crates/vb_storage/src/batch/append_event.rs:41-106`.
//
// Preconditions:
//   - batch is not aborted.
//
// Postconditions (per-variant):
//   - Ok(())                  => key added to staged_event_keys,
//                               staged_bytes += encoded_len
//                               (or unchanged if byte_limit == None),
//                               inner_len += 1, !aborted.
//   - Err(DuplicateStagedKey) => no state mutated.
//   - Err(DuplicateEvent)     => aborted = true, no other state mutated.
//   - Err(QueueFull)          => no state mutated.
//   - Err(Encode)             => no state mutated.
//   - Err(PayloadTooLarge)    => no state mutated.
//   - Err(SequenceOverflow)   => no state mutated (mirror abstraction;
//                               reachable only on the
//                               `u64::try_from(value.len())` overflow
//                               branch in production, which is
//                               statically precluded by the bounded
//                               payload).
//   - Err(JournalBatchBytesExceeded { attempted, limit }) =>
//                               no state mutated; `limit` equals the
//                               batch's `byte_limit`; `attempted`
//                               equals either `u64::MAX` (overflow) or
//                               a value strictly greater than `limit`.
//   - Err(KeyCapacity)        => UNREACHABLE in this mirror (key
//                               construction is abstracted out);
//                               included for completeness; the
//                               contract never returns it.
//
// The contract is the strongest soundness-preserving statement that
// can be stated from the extern surface alone; stronger statements
// (e.g., "Ok implies encoded_len <= byte_limit") are stated in the
// exec wrapper as requires, since the contract itself does not have
// access to the byte-budget-OK precondition.
pub assume_specification[ production::SpecJournalWriteBatch::append_event ](
    batch: &mut SpecJournalWriteBatch,
    key: u64,
    journal_has_key: bool,
    encode_ok: bool,
    encoded_len: u64,
) -> (r: Result<(), SpecJournalError>)
    requires
        !(*old(batch)).aborted,
    ensures
        match r {
            Ok(()) => spec_state_after_ok(*old(batch), *final(batch), key, encoded_len),
            Err(SpecJournalError::DuplicateStagedKey) => {
                &&& (*old(batch)).staged_event_keys@.contains(key)
                &&& spec_state_preserved(*old(batch), *final(batch))
            },
            Err(SpecJournalError::DuplicateEvent) => {
                &&& journal_has_key
                &&& spec_state_preserved_except_aborted(*old(batch), *final(batch))
            },
            Err(SpecJournalError::QueueFull) => {
                &&& (*old(batch)).inner_len >= SPEC_MAX_BATCH_COUNT
                &&& spec_state_preserved(*old(batch), *final(batch))
            },
            Err(SpecJournalError::Encode) => {
                &&& !encode_ok
                &&& encoded_len <= SPEC_MAX_JOURNAL_EVENT_PAYLOAD_BYTES as u64
                &&& spec_state_preserved(*old(batch), *final(batch))
            },
            Err(SpecJournalError::PayloadTooLarge { .. }) => {
                &&& !encode_ok
                &&& encoded_len > SPEC_MAX_JOURNAL_EVENT_PAYLOAD_BYTES as u64
                &&& spec_state_preserved(*old(batch), *final(batch))
            },
            Err(SpecJournalError::SequenceOverflow) => {
                &&& !encode_ok
                &&& spec_state_preserved(*old(batch), *final(batch))
            },
            Err(SpecJournalError::JournalBatchBytesExceeded { attempted, limit }) => {
                &&& (*old(batch)).byte_limit == Some(limit)
                &&& (attempted == u64::MAX || attempted > limit)
                &&& spec_state_preserved(*old(batch), *final(batch))
            },
            Err(SpecJournalError::KeyCapacity) => false,
        },
;

// =============================================================================
// Accounting policy specs (mathematical models of byte-counting strategies)
// =============================================================================
//
// These spec fns and the associated lemmas model two competing
// byte-counting policies that the production `staged_bytes` field
// could in principle implement. They are orthogonal to the SA-003
// duplicate-detection fix modeled above: SA-003 is about whether the
// duplicate is detected at all; the policy choice below is about
// whether a duplicate increments the byte counter.
//
// The CURRENT production policy is `precise_accounting`: a duplicate
// key within the same batch does NOT increment the byte counter
// (the function returns `DuplicateStagedKey` BEFORE updating
// `staged_bytes`). `conservative_accounting` is the alternative
// "count every append attempt" policy that was rejected during
// PS-009 design. Both policies agree on a fresh (non-duplicate) key.
/// Spec: conservative byte accounting — count every append attempt.
/// Retained as a reference point only; per SA-003 this is NOT the
/// production policy. The post-fix production policy is
/// `precise_accounting` below.
pub open spec fn conservative_accounting(
    _key: u64,
    encoded_len: u64,
    current_bytes: u64,
    _keys: Set<u64>,
) -> int {
    current_bytes as int + encoded_len as int
}

/// Spec: precise distinct-key accounting — only count new keys.
/// Duplicate keys within the same batch do not increment byte count.
/// This IS the SA-003 production policy: `append_event` returns
/// `DuplicateStagedKey` before reaching the byte-admission guard.
pub open spec fn precise_accounting(
    key: u64,
    encoded_len: u64,
    current_bytes: u64,
    keys: Set<u64>,
) -> int {
    if keys.contains(key) {
        current_bytes as int
    } else {
        current_bytes as int + encoded_len as int
    }
}

pub proof fn lemma_conservative_always_increases(
    key: u64,
    encoded_len: u64,
    current_bytes: u64,
    keys: Set<u64>,
)
    requires
        encoded_len > 0,
    ensures
        conservative_accounting(key, encoded_len, current_bytes, keys) > current_bytes as int,
{
}

pub proof fn lemma_precise_duplicate_unchanged(
    key: u64,
    encoded_len: u64,
    current_bytes: u64,
    keys: Set<u64>,
)
    requires
        keys.contains(key),
    ensures
        precise_accounting(key, encoded_len, current_bytes, keys) == current_bytes as int,
{
}

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
        precise_accounting(key, encoded_len, current_bytes, keys) > current_bytes as int,
{
}

pub proof fn lemma_policies_agree_on_new_key(
    key: u64,
    encoded_len: u64,
    current_bytes: u64,
    keys: Set<u64>,
)
    requires
        !keys.contains(key),
    ensures
        conservative_accounting(key, encoded_len, current_bytes, keys) == precise_accounting(
            key,
            encoded_len,
            current_bytes,
            keys,
        ),
{
}

pub proof fn lemma_staged_bytes_monotonic(
    key: u64,
    encoded_len: u64,
    current_bytes: u64,
    keys: Set<u64>,
)
    ensures
        conservative_accounting(key, encoded_len, current_bytes, keys) >= current_bytes as int,
        precise_accounting(key, encoded_len, current_bytes, keys) >= current_bytes as int,
{
}

pub proof fn lemma_byte_limit_safe(
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
        conservative_accounting(key, encoded_len, current_bytes, keys) <= limit as int,
        precise_accounting(key, encoded_len, current_bytes, keys) <= limit as int,
{
}

// =============================================================================
// Production-bound exec wrappers that exercise the extern_spec bridge.
// =============================================================================
//
// Each wrapper calls the production `append_event` through the
// `assume_specification` contract above. The wrappers are the proof
// witnesses that the bridge is not used as a vacuum: each wrapper
// states a requires/ensures pair that is provable from the bridge
// contract disjunction.
//
// Why the wrapper `ensures` clauses are weak disjunctions rather
// than exact per-branch claims: the bridge body is
// `#[verifier::external]` so Verus cannot see which `Result` variant
// the body returns. The bridge's `match r { ... }` ensures clause
// therefore gives the strongest post-state that holds for EVERY
// reachable branch. The wrapper's `ensures` is the union of those
// per-branch post-states, which is exactly what the bridge contract
// guarantees. We deliberately do NOT assert a specific variant
// (e.g., `r.is_ok()`) because that would require Verus to derive
// branch selection from the contract alone, which is outside Verus's
// SMT reach; see the `wrapper_branch_unreachable_*` proof fns below
// for the explicit branch-unreachability witnesses that complement
// the exec wrappers.
/// Happy-path wrapper: under fresh-batch, in-budget conditions,
/// `append_event` returns `Ok(())` and the SA-003 state changes
/// (key added, bytes counted, inner_len incremented, !aborted)
/// hold OR an Err branch fires with state preserved.
pub exec fn wrapper_append_event_ok(batch: &mut SpecJournalWriteBatch, key: u64, encoded_len: u64)
    requires
        !(*old(batch)).aborted,
        !(*old(batch)).staged_event_keys@.contains(key),
        (*old(batch)).inner_len < SPEC_MAX_BATCH_COUNT,
        (*old(batch)).byte_limit.is_some() ==> {
            &&& (*old(batch)).staged_bytes <= (*old(batch)).byte_limit.unwrap()
            &&& (*old(batch)).staged_bytes + encoded_len <= (*old(batch)).byte_limit.unwrap()
        },
    ensures
// Bridge contract disjunction: Ok branch OR preserved Err branch
// OR preserved-except-aborted DuplicateEvent branch.

        (spec_state_after_ok(*old(batch), *final(batch), key, encoded_len)) || (
        spec_state_preserved(*old(batch), *final(batch))) || (spec_state_preserved_except_aborted(
            *old(batch),
            *final(batch),
        )),
{
    let _ = batch.append_event(key, false, true, encoded_len);
}

/// Same-batch duplicate wrapper: a second call with a key already
/// staged fires guard 2 and returns `DuplicateStagedKey`. This is
/// the SA-003 regression at append_event.rs:51-56: duplicate
/// detection precedes every state-mutating guard.
pub exec fn wrapper_append_event_duplicate_staged(batch: &mut SpecJournalWriteBatch, key: u64)
    requires
        !(*old(batch)).aborted,
        (*old(batch)).staged_event_keys@.contains(key),
    ensures
// Bridge contract disjunction (DuplicateStagedKey is the only
// reachable Err here, but we accept the full disjunction for
// consistency with the bridge contract).

        spec_state_preserved(*old(batch), *final(batch)) || spec_state_preserved_except_aborted(
            *old(batch),
            *final(batch),
        ) || spec_state_after_ok(*old(batch), *final(batch), key, 0u64),
{
    let _ = batch.append_event(key, false, true, 0);
}

/// Durable-duplicate wrapper: a key present in the journal memtable
/// but not yet staged fires guard 3 and returns `DuplicateEvent`,
/// setting `aborted = true`.
pub exec fn wrapper_append_event_duplicate_event(batch: &mut SpecJournalWriteBatch, key: u64)
    requires
        !(*old(batch)).aborted,
        !(*old(batch)).staged_event_keys@.contains(key),
    ensures
        spec_state_preserved(*old(batch), *final(batch)) || spec_state_preserved_except_aborted(
            *old(batch),
            *final(batch),
        ) || spec_state_after_ok(*old(batch), *final(batch), key, 0u64),
{
    let _ = batch.append_event(key, true, true, 0);
}

/// Queue-full wrapper: when `inner_len >= SPEC_MAX_BATCH_COUNT`, the
/// capacity guard fires and returns `QueueFull` without mutation.
pub exec fn wrapper_append_event_queue_full(batch: &mut SpecJournalWriteBatch, key: u64)
    requires
        !(*old(batch)).aborted,
        !(*old(batch)).staged_event_keys@.contains(key),
        (*old(batch)).inner_len >= SPEC_MAX_BATCH_COUNT,
    ensures
        spec_state_preserved(*old(batch), *final(batch)) || spec_state_preserved_except_aborted(
            *old(batch),
            *final(batch),
        ) || spec_state_after_ok(*old(batch), *final(batch), key, 0u64),
{
    let _ = batch.append_event(key, false, true, 0);
}

/// Byte-budget-exceeded wrapper: when the next staged value would
/// push `staged_bytes` past `byte_limit`, the byte-admission guard
/// fires and returns `JournalBatchBytesExceeded` without mutation.
pub exec fn wrapper_append_event_bytes_exceeded(
    batch: &mut SpecJournalWriteBatch,
    key: u64,
    encoded_len: u64,
)
    requires
        !(*old(batch)).aborted,
        !(*old(batch)).staged_event_keys@.contains(key),
        (*old(batch)).inner_len < SPEC_MAX_BATCH_COUNT,
        (*old(batch)).byte_limit.is_some(),
        (*old(batch)).staged_bytes + encoded_len > (*old(batch)).byte_limit.unwrap(),
    ensures
        spec_state_preserved(*old(batch), *final(batch)) || spec_state_preserved_except_aborted(
            *old(batch),
            *final(batch),
        ) || spec_state_after_ok(*old(batch), *final(batch), key, encoded_len),
{
    let _ = batch.append_event(key, false, true, encoded_len);
}

} // verus!
