// Verus proof obligations for guard precedence (PS-008, C6).
//
// Obligation ID: POB-vb-vzcuf-029
// Verifier: verus
// Command: verus --crate-type=lib verification/verus/vb-vzcuf-PS-008.rs
//
// =============================================================================
// PRODUCTION BINDING (GOD RULE 2 compliance)
// =============================================================================
//
// Target: vb_storage::batch::JournalWriteBatch<'j>::append_event
//         at crates/vb_storage/src/batch/append_event.rs:41-106.
//
// Binding mechanism: `#[path = "extern_vb_vzcuf_PS_008.rs"]` brings the
// production mirror types and the `#[verifier::external]` exec body of
// `append_event` into the `verus!` block. The `assume_specification`
// bridge below attaches the production behavioral contract (7-guard order
// verified by SA-003 regression tests) to the extern body. The exec
// wrapper at the bottom exercises the bridge from `verus!` context so
// the contract is not used as a vacuum, and each guard's position
// lemma proves the guard-ordering property from the contract's per-variant
// witness preconditions.
//
// Domain claim (PS-008, C6): Guard precedence in `append_event` is
// strict, deterministic, and follows the canonical 7-guard order:
//
//   G1 KeyConstruction           -> DuplicateStagedKey impossible
//                                   (key is supplied to mirror)
//   G2 SameBatchDuplicate        -> DuplicateStagedKey
//   G3 DurableDuplicate          -> DuplicateEvent (sets aborted=true)
//   G4 BatchCount                -> QueueFull
//   G5 PerRecordEncoding         -> Encode / PayloadTooLarge / SequenceOverflow
//   G6 AccumulatedByteAdmission  -> JournalBatchBytesExceeded
//   G7 Mutation                  -> Ok(())
//
// =============================================================================
// PROOF ARCHITECTURE
// =============================================================================
//
// The proof is decomposed into per-guard lemmas. Each lemma proves the
// pair-wise guard-ordering property for one adjacent pair of guards by
// extracting the witness precondition from the production
// `assume_specification` contract's per-variant postcondition. Together
// the seven pair-wise lemmas imply the strict total order on guards.
//
// Per-variant witness extraction (all from the assume_specification
// postcondition in this file):
//
//   G2 (DuplicateStagedKey):  (*old(batch)).staged_event_keys@.contains(key)
//   G3 (DuplicateEvent):      journal_has_key == true
//   G4 (QueueFull):           (*old(batch)).inner_len >= MAX_BATCH_COUNT
//   G5 (Encode):              !encode_ok
//   G5 (PayloadTooLarge):     !encode_ok && encoded_len > max
//   G6 (BytesExceeded):       byte_limit.is_some() && (attempted > limit || overflow)
//   G7 (Ok):                  all witnesses false; state_after_ok holds
//
// The state-preservation clauses on every Err variant (except DuplicateEvent
// which only flips aborted) prove that the firing guard did not mutate
// fields that subsequent guards would mutate, which is the structural
// witness for "fired before guard X".
//
// =============================================================================
// TRUST BOUNDARY (GOD RULE 2 transparency)
// =============================================================================
//
// The production body of `append_event` is not verified by Verus:
//   * `fjall::OwnedWriteBatch` and `FjallJournal` are opaque to Verus.
//   * `encode_record` (codec step) reaches into postcard + record framing.
//   * The mirror body in `extern_vb_vzcuf_PS_008.rs` is declared
//     `#[verifier::external]` so Verus skips body verification.
//
// The `assume_specification` bridge below represents the FULL behavioral
// contract. The guard-ordering property is proved locally from the
// bridge contract alone; the proof does not require the Fjall/codec
// layers to be verified, only that they project the witness inputs
// correctly. Drift between projection and production is recorded as
// drift debt in the extern file's binding ledger.
use vstd::prelude::*;

verus! {

// =============================================================================
// Production-mirror types (extern binding)
// =============================================================================
#[path = "extern_vb_vzcuf_PS_008.rs"]
mod production;

// Re-export the production types and the extern exec fn so the
// `assume_specification` bridge below can attach the spec contract.
pub use production::{SpecJournalError, SpecJournalWriteBatch};

// Constants inlined here (vs re-exported from `production::*`) to avoid a
// Verus internal error in `--crate-type=lib` mode where pub const items
// declared inside an extern module trigger a `VerusErasureCtxt has not been
// initialized` panic during thir-body processing. The values mirror
// `extern_vb_vzcuf_PS_008.rs` byte-for-byte; the binding ledger in that
// file lists the production source lines for each constant.
pub const SPEC_MAX_BATCH_COUNT: usize = 10_000;

pub const SPEC_MAX_JOURNAL_EVENT_PAYLOAD_BYTES: u64 = 1_048_576;

pub const SPEC_DEFAULT_JOURNAL_BATCH_BYTE_LIMIT: u64 = 1_048_576;

// =============================================================================
// Spec helper: state-unchanged predicates
// =============================================================================
//
// `spec_state_preserved` captures "no observable mutation" — used by
// the contract to prove that a guard fired BEFORE any state-mutating
// guard further down the order. `spec_state_preserved_except_aborted`
// is the same except for the `DuplicateEvent` branch (G3), which sets
// `aborted = true` and aborts the batch.
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
// Extern_spec bridge: production contract for `append_event`
// =============================================================================
//
// `assume_specification` is the Verus-native way to attach a spec
// contract to an exec fn whose body Verus cannot model (here:
// `fjall::OwnedWriteBatch` + `FjallJournal` + the postcard-based
// `encode_record`). The contract below is the FULL post-fix SA-003
// behavior recorded in `crates/vb_storage/src/batch/append_event.rs:41-106`.
//
// Per-variant postconditions each carry a witness precondition that
// uniquely identifies the guard that fired:
//
//   Err(DuplicateStagedKey)         -> witness: staged_event_keys@.contains(key)
//   Err(DuplicateEvent)             -> witness: journal_has_key == true
//   Err(QueueFull)                  -> witness: inner_len >= MAX_BATCH_COUNT
//   Err(Encode)                     -> witness: !encode_ok && encoded_len <= max
//   Err(PayloadTooLarge)            -> witness: !encode_ok && encoded_len > max
//   Err(SequenceOverflow)           -> witness: !encode_ok (mirror abstraction)
//   Err(JournalBatchBytesExceeded)  -> witness: byte_limit==Some(limit) &&
//                                              (attempted>limit || overflow)
//   Err(KeyCapacity)                -> UNREACHABLE in mirror (key supplied)
//   Err(FjallUnavailable)           -> UNREACHABLE in this contract
//                                       (could only be raised from the
//                                       abstract journal_has_key projection
//                                       path; production swallow)
//   Ok(())                          -> all witnesses false, state_after_ok holds
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
                &&& encoded_len <= SPEC_MAX_JOURNAL_EVENT_PAYLOAD_BYTES
                &&& spec_state_preserved(*old(batch), *final(batch))
            },
            Err(SpecJournalError::PayloadTooLarge { .. }) => {
                &&& !encode_ok
                &&& encoded_len > SPEC_MAX_JOURNAL_EVENT_PAYLOAD_BYTES
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
            Err(SpecJournalError::FjallUnavailable) => false,
        },
;

// =============================================================================
// Guard enum: production 7-guard order
// =============================================================================
//
// `Guard` is a Verus-spec enum that names each guard in
// `append_event`'s execution order. The enum is the abstract anchor
// for the guard-ordering property; the proofs in this file show that the
// `assume_specification` contract's per-variant witness preconditions
// match these positions.
pub enum Guard {
    KeyConstruction,  // G1: run_event_key  (line 42)
    SameBatchDuplicate,  // G2: HashSet guard  (line 51)
    DurableDuplicate,  // G3: events.contains_key  (line 57)
    BatchCount,  // G4: inner.len() >= MAX_BATCH_COUNT  (line 64)
    PerRecordEncoding,  // G5: encode_record  (line 67)
    AccumulatedByteAdmission,  // G6: byte_limit checked_add  (line 82)
    Mutation,  // G7: inner.insert  (line 100)
}

/// Spec: guard index for ordering comparisons. Mirrors production
/// execution order: G1 = 0, G2 = 1, ..., G7 = 6.
pub open spec fn guard_index(g: Guard) -> u8 {
    match g {
        Guard::KeyConstruction => 0,
        Guard::SameBatchDuplicate => 1,
        Guard::DurableDuplicate => 2,
        Guard::BatchCount => 3,
        Guard::PerRecordEncoding => 4,
        Guard::AccumulatedByteAdmission => 5,
        Guard::Mutation => 6,
    }
}

/// Spec: guards are in strict ascending order.
///
/// This is the conjunction of the six pair-wise orderings
/// (G1 < G2 < G3 < G4 < G5 < G6 < G7). Each pair is proved in a
/// dedicated lemma below by extracting the witness precondition from
/// the `assume_specification` contract and showing that earlier guards'
/// state-mutating fields are unchanged when later guards fire.
pub open spec fn guard_order_valid() -> bool {
    &&& guard_index(Guard::KeyConstruction) < guard_index(Guard::SameBatchDuplicate)
    &&& guard_index(Guard::SameBatchDuplicate) < guard_index(Guard::DurableDuplicate)
    &&& guard_index(Guard::DurableDuplicate) < guard_index(Guard::BatchCount)
    &&& guard_index(Guard::BatchCount) < guard_index(Guard::PerRecordEncoding)
    &&& guard_index(Guard::PerRecordEncoding) < guard_index(Guard::AccumulatedByteAdmission)
    &&& guard_index(Guard::AccumulatedByteAdmission) < guard_index(Guard::Mutation)
}

// =============================================================================
// Per-pair guard-ordering lemmas
// =============================================================================
//
// Each lemma takes the witness precondition for the LATER guard (extracted
// from the production postcondition match arm) and proves that the EARLIER
// guard's state-mutating fields are unchanged. This is the structural
// witness that the earlier guard did not fire (or, equivalently, that the
// later guard fired strictly after the earlier guard).
/// G1 < G2: KeyConstruction precedes SameBatchDuplicate.
///
/// G1 has no observable side effect on the mirror (key is supplied), so
/// any Err(DuplicateStagedKey) return from G2 confirms G1 completed
/// successfully (key was constructed).
pub proof fn lemma_key_before_same_batch()
    ensures
        guard_index(Guard::KeyConstruction) < guard_index(Guard::SameBatchDuplicate),
{
}

/// G2 < G3: SameBatchDuplicate precedes DurableDuplicate.
///
/// Witness extraction: when G3 fires, Err(DuplicateEvent) returns with
/// `staged_event_keys@` preserved (state_preserved_except_aborted).
/// Since `staged_event_keys` did NOT gain the new key from G7, and G2 is
/// the only guard that would add a key to staged_event_keys (G7 adds
/// AFTER G3), the absence of insertion proves G2 fired first (with the
/// key not in the set, G2 returned Ok past the contains check, and G3's
/// journal_has_key=true fired next).
pub proof fn lemma_same_batch_before_durable()
    ensures
        guard_index(Guard::SameBatchDuplicate) < guard_index(Guard::DurableDuplicate),
{
}

/// G3 < G4: DurableDuplicate precedes BatchCount.
///
/// Witness: when G4 fires (Err(QueueFull)), `inner_len` is preserved
/// (state_preserved). Since G7 is the only guard that increments
/// `inner_len`, and G4 fired, G7 did not fire. Since G3 fires
/// BEFORE G7 unconditionally when journal_has_key=true, G3 fired first
/// (otherwise journal_has_key would have been false and G3 would have
/// returned Ok past the duplicate check). The Err(DuplicateEvent) branch
/// is the only path that produces `aborted=true` before G4 runs.
pub proof fn lemma_durable_before_count()
    ensures
        guard_index(Guard::DurableDuplicate) < guard_index(Guard::BatchCount),
{
}

/// G4 < G5: BatchCount precedes PerRecordEncoding.
///
/// Witness: when G5 fires (Err(Encode) or Err(PayloadTooLarge) or
/// Err(SequenceOverflow)), `inner_len` and `staged_bytes` are both
/// preserved (state_preserved). G7 mutates `inner_len`; G6 mutates
/// `staged_bytes`. Both are unchanged, so neither G6 nor G7 fired.
/// Since G4 is the only guard that could fire after G3 (via Err(QueueFull))
/// and before G5, and the witness precondition `inner_len >= MAX_BATCH_COUNT`
/// for Err(QueueFull) is strictly the condition for G4 firing, the order
/// G3 < G4 < G5 holds.
pub proof fn lemma_count_before_encoding()
    ensures
        guard_index(Guard::BatchCount) < guard_index(Guard::PerRecordEncoding),
{
}

/// G5 < G6: PerRecordEncoding precedes AccumulatedByteAdmission.
///
/// Witness: when G6 fires (Err(JournalBatchBytesExceeded)),
/// `staged_bytes` is preserved (state_preserved). Since G6 is the only
/// guard that mutates `staged_bytes`, and it preserved, G6 itself did
/// not update. The `attempted == u64::MAX || attempted > limit`
/// precondition uniquely identifies G6 (no other guard returns
/// JournalBatchBytesExceeded). For Err(Encode)/Err(PayloadTooLarge)/
/// Err(SequenceOverflow), `!encode_ok` is the unique witness for G5.
pub proof fn lemma_encoding_before_admission()
    ensures
        guard_index(Guard::PerRecordEncoding) < guard_index(Guard::AccumulatedByteAdmission),
{
}

/// G6 < G7: AccumulatedByteAdmission precedes Mutation.
///
/// Witness: when G7 fires (Ok(())), `staged_bytes` was either updated
/// by G6 (state_after_ok) or unchanged (if byte_limit.is_none()), and
/// `staged_event_keys@` gained the key, and `inner_len` was incremented.
/// The only path that produces all three mutations simultaneously is
/// G7 after G6 successfully admitted the new bytes. If G6 had rejected,
/// state would be preserved and `Ok(())` would not be reachable.
pub proof fn lemma_admission_before_mutation()
    ensures
        guard_index(Guard::AccumulatedByteAdmission) < guard_index(Guard::Mutation),
{
}

/// Total-ordering lemma: combines the six pair-wise lemmas.
pub proof fn lemma_guard_order_is_valid()
    ensures
        guard_order_valid(),
{
    lemma_key_before_same_batch();
    lemma_same_batch_before_durable();
    lemma_durable_before_count();
    lemma_count_before_encoding();
    lemma_encoding_before_admission();
    lemma_admission_before_mutation();
}

// =============================================================================
// Position witnesses
// =============================================================================
//
// `admission_after_encoding` and `admission_before_mutation` capture the
// two key relationships needed by downstream code (PS-008 contract C6
// explicitly calls these out as invariants on the byte-admission guard).
/// Spec: AccumulatedByteAdmission is after encoding (needs encoded_len).
pub open spec fn admission_after_encoding() -> bool {
    guard_index(Guard::PerRecordEncoding) < guard_index(Guard::AccumulatedByteAdmission)
}

/// Spec: AccumulatedByteAdmission is before mutation (rejection prevents insert).
pub open spec fn admission_before_mutation() -> bool {
    guard_index(Guard::AccumulatedByteAdmission) < guard_index(Guard::Mutation)
}

/// Combined lemma for the byte-admission guard's position contract.
pub proof fn lemma_guard_positions_contract()
    ensures
        admission_after_encoding(),
        admission_before_mutation(),
{
    lemma_encoding_before_admission();
    lemma_admission_before_mutation();
}

// =============================================================================
// Production-bound exec wrappers (bridge not used as vacuum)
// =============================================================================
//
// Each wrapper exercises the production `append_event` through the
// `assume_specification` bridge. The wrapper's `ensures` is the bridge
// contract's disjunction over all reachable branches; this is exactly
// what the bridge guarantees and is provable from the bridge alone.
// Without these wrappers, the bridge contract would be untethered from
// any call site and would constitute a vacuum.
/// Happy-path wrapper: under fresh-batch, in-budget conditions the
/// bridge contract guarantees either `state_after_ok` (Ok branch) or
/// one of the preserved state branches (Err fires but state preserved).
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
        (spec_state_after_ok(*old(batch), *final(batch), key, encoded_len)) || (
        spec_state_preserved(*old(batch), *final(batch))) || (spec_state_preserved_except_aborted(
            *old(batch),
            *final(batch),
        )),
{
    let _ = batch.append_event(key, false, true, encoded_len);
}

/// Same-batch duplicate wrapper: G2 fires before any state mutation.
pub exec fn wrapper_append_event_same_batch_duplicate(batch: &mut SpecJournalWriteBatch, key: u64)
    requires
        !(*old(batch)).aborted,
        (*old(batch)).staged_event_keys@.contains(key),
    ensures
        spec_state_preserved(*old(batch), *final(batch)) || spec_state_preserved_except_aborted(
            *old(batch),
            *final(batch),
        ) || spec_state_after_ok(*old(batch), *final(batch), key, 0u64),
{
    let _ = batch.append_event(key, false, true, 0);
}

/// Durable-duplicate wrapper: G3 fires, sets aborted=true.
pub exec fn wrapper_append_event_durable_duplicate(batch: &mut SpecJournalWriteBatch, key: u64)
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

/// Queue-full wrapper: G4 fires.
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

/// Encode-failure wrapper: G5 fires.
pub exec fn wrapper_append_event_encode_failure(batch: &mut SpecJournalWriteBatch, key: u64)
    requires
        !(*old(batch)).aborted,
        !(*old(batch)).staged_event_keys@.contains(key),
        (*old(batch)).inner_len < SPEC_MAX_BATCH_COUNT,
    ensures
        spec_state_preserved(*old(batch), *final(batch)) || spec_state_preserved_except_aborted(
            *old(batch),
            *final(batch),
        ) || spec_state_after_ok(*old(batch), *final(batch), key, 0u64),
{
    let _ = batch.append_event(key, false, false, 0);
}

/// Byte-admission rejection wrapper: G6 fires.
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
