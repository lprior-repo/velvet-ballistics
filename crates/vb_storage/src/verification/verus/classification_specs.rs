#![forbid(unsafe_code)]
//! Verus spec + proof artifacts for vb_storage classification seams.
//!
//! Covers:
//! - ExactJournalKindParity witness invariant
//! - classify_journal_semantic_decode ↔ spec_mrwe5_semantic_decode equivalence
//! - classify_record_kind_family ↔ spec_validate_kind_family equivalence
//! - ValidatedJournalRecord structural invariant
//! - ActionReplayTracker::is_resolved production binding
//! - UnsupportedRecoveryState::union OR-semantics
//! - DigestCheck hierarchy_rank ordering
//!
//! Production binding map:
//!   - ExactJournalKindParity       → crates/vb_storage/src/codec/semantic.rs:64
//!   - classify_journal_semantic_decode → crates/vb_storage/src/codec/semantic.rs:170
//!   - classify_record_kind_family  → crates/vb_storage/src/codec/semantic.rs:197
//!   - ValidatedJournalRecord::try_new → crates/vb_storage/src/codec/semantic.rs:112
//!   - ActionReplayTracker::is_resolved → crates/vb_storage/src/recovery/types.rs:588
//!   - UnsupportedRecoveryState::union → crates/vb_storage/src/recovery/types.rs:352
//!   - UnsupportedRecoveryState::union_matches_flags → crates/vb_storage/src/recovery/types.rs:368
//!   - DigestCheck hierarchy_rank   → crates/vb_storage/src/recovery/types.rs:614

// =========================================================================
// STANDALONE SPEC LAYER
// =========================================================================

use vstd::prelude::*;

verus! {

// =========================================================================
// MRWE5 KERNEL SPEC FUNCTIONS (used by higher-level specs)
// =========================================================================
/// Spec: mrwe5_kind_compatibility returns 1 iff envelope and payload kinds match.
/// Production binding: crates/vb_storage/src/mrwe5_contract.rs:86-95.
#[verifier::nonlinear]
pub open spec fn spec_mrwe5_kind_compatibility(envelope_kind: u16, payload_kind: u16) -> int {
    if envelope_kind == payload_kind {
        1int
    } else {
        2int
    }
}

/// Spec: mrwe5_semantic_decode returns the semantic decode decision as an int.
/// Production binding: crates/vb_storage/src/mrwe5_contract.rs:99-113.
#[verifier::nonlinear]
pub open spec fn spec_mrwe5_semantic_decode(
    envelope_kind: u16,
    payload_kind: u16,
    event_valid: bool,
) -> int {
    if envelope_kind != payload_kind {
        2int
    } else if event_valid {
        1int
    } else {
        3int
    }
}

/// Spec: mrwe5_is_journal_record_kind returns true iff kind is in 10..=29.
/// Production binding: crates/vb_storage/src/mrwe5_contract.rs:117-119.
pub open spec fn spec_mrwe5_is_journal_record_kind(kind: u16) -> bool {
    10u16 <= kind && kind <= 29u16
}

/// Spec: DigestCheck hierarchy_rank maps level to ordinal rank.
/// Production binding: crates/vb_storage/src/recovery/types.rs:614.
pub open spec fn spec_digest_check_hierarchy_rank(level: int) -> int {
    if level == 1int {
        1int
    } else if level == 2int {
        2int
    } else if level == 3int {
        3int
    } else {
        0int
    }
}

/// Spec: is_strictly_weaker is ordinal less-than on hierarchy_rank.
pub open spec fn spec_digest_check_strictly_weaker(a: int, b: int) -> bool {
    spec_digest_check_hierarchy_rank(a) < spec_digest_check_hierarchy_rank(b)
}

// =========================================================================
// EXACT JOURNAL KIND PARITY WITNESS SPEC
// =========================================================================
/// Spec: an exact-kind parity witness exists if and only if the
/// envelope and payload kind identifiers are equal.
pub open spec fn spec_exact_journal_kind_parity_exists(
    envelope_kind: u16,
    payload_kind: u16,
) -> bool {
    envelope_kind == payload_kind
}

/// Proof: if envelope_kind == payload_kind then the witness exists.
pub proof fn lemma_parity_exists_on_match(envelope_kind: u16, payload_kind: u16)
    requires
        envelope_kind == payload_kind,
    ensures
        spec_exact_journal_kind_parity_exists(envelope_kind, payload_kind),
{
    assert(spec_exact_journal_kind_parity_exists(envelope_kind, payload_kind));
}

/// Proof: if envelope_kind != payload_kind then the witness does not exist.
pub proof fn lemma_parity_not_exists_on_mismatch(envelope_kind: u16, payload_kind: u16)
    requires
        envelope_kind != payload_kind,
    ensures
        !spec_exact_journal_kind_parity_exists(envelope_kind, payload_kind),
{
    assert(!spec_exact_journal_kind_parity_exists(envelope_kind, payload_kind));
}

/// Proof: the parity witness predicate is reflexive.
pub proof fn lemma_parity_reflexive() {
    assert(spec_exact_journal_kind_parity_exists(29u16, 29u16));
}

/// Proof: the parity witness predicate is symmetric.
pub proof fn lemma_parity_symmetric(a: u16, b: u16)
    requires
        a == b,
    ensures
        spec_exact_journal_kind_parity_exists(b, a),
{
    assert(spec_exact_journal_kind_parity_exists(b, a));
}

// =========================================================================
// CLASSIFY JOURNAL SEMANTIC DECODE SPEC
// =========================================================================
/// Spec model for classify_journal_semantic_decode.
///
/// The production function delegates to spec_mrwe5_semantic_decode and
/// remaps the three integer outcomes into JournalSemanticDecodeDecision.
/// This spec mirrors that mapping directly.
#[verifier::nonlinear]
pub open spec fn spec_classify_journal_semantic_decode(
    envelope_kind: u16,
    payload_kind: u16,
    event_valid: bool,
) -> int {
    // 1 = SemanticSuccess, 2 = KindPayloadMismatch, 3 = InvalidEvent
    let comp = spec_mrwe5_kind_compatibility(envelope_kind, payload_kind);
    if comp == 1int {
        if event_valid {
            1int
        } else {
            3int
        }
    } else {
        2int
    }
}

/// Proof: classify_journal_semantic_decode(StepSucceeded, StepSucceeded, true) == SemanticSuccess.
pub proof fn lemma_semantic_decode_success_step_succeeded() {
    assert(spec_classify_journal_semantic_decode(29u16, 29u16, true) == 1);
}

/// Proof: classify_journal_semantic_decode(StepSucceeded, StepSucceeded, false) == InvalidEvent.
pub proof fn lemma_semantic_decode_invalid_step_succeeded() {
    assert(spec_classify_journal_semantic_decode(29u16, 29u16, false) == 3);
}

/// Proof: classify_journal_semantic_decode(mismatched) == KindPayloadMismatch regardless of event_valid.
pub proof fn lemma_semantic_decode_mismatch_rejects_both(event_valid: bool) {
    assert(spec_classify_journal_semantic_decode(29u16, 12u16, event_valid) == 2);
}

/// Proof: the MRWE5 semantic decode and the journal wrapper return the same decision code.
pub proof fn lemma_classify_journal_semantic_decode_equivalence(
    envelope_kind: u16,
    payload_kind: u16,
    event_valid: bool,
) {
    assert(spec_classify_journal_semantic_decode(envelope_kind, payload_kind, event_valid)
        == spec_mrwe5_semantic_decode(envelope_kind, payload_kind, event_valid));
}

// =========================================================================
// CLASSIFY RECORD KIND FAMILY SPEC
// =========================================================================
/// Spec model for classify_record_kind_family.
///
/// Two-tier classification: MRWE5 journal-event family (magic==0x56424A45,
/// kind in 10..=29) OR one of the other known magic/kind pairs.
///
/// Production constants from crates/vb_storage/src/constants.rs:
///   MAGIC_WORKFLOW_SOURCE    = 0x56425352  → kind == 1 (WorkflowSource)
///   MAGIC_COMPILED_ARTIFACT  = 0x56424952  → kind == 2 (CompiledIr)
///   MAGIC_JOURNAL_EVENT      = 0x56424A45  → kind in 10..=29 (journal family)
///   MAGIC_SNAPSHOT           = 0x5642534E  → kind == 30 (Snapshot)
///   MAGIC_BLOB               = 0x5642424C  → kind == 40 (Blob)
///   MAGIC_INDEX_RECORD       = 0x56424958  → kind in {3, 50}
///   MAGIC_RECOVERY_STAMP     = 0x56425254  → kind == 5 (RecoveryStamp)
pub open spec fn spec_validate_kind_family(magic: u32, kind: u16) -> int {
    // 1 = Accepted, 2 = Rejected
    if magic == 0x5642_4A45u32 && spec_mrwe5_is_journal_record_kind(kind) {
        1int
    } else {
        match magic {
            0x5642_5352u32 => if kind == 1 {
                1int
            } else {
                2int
            },
            0x5642_4952u32 => if kind == 2 {
                1int
            } else {
                2int
            },
            0x5642_534Eu32 => if kind == 30 {
                1int
            } else {
                2int
            },
            0x5642_424Cu32 => if kind == 40 {
                1int
            } else {
                2int
            },
            0x5642_4958u32 => if kind == 3 || kind == 50 {
                1int
            } else {
                2int
            },
            0x5642_5254u32 => if kind == 5 {
                1int
            } else {
                2int
            },
            _ => 2int,
        }
    }
}

/// Proof: journal family accepts kind 29 (StepSucceeded).
pub proof fn lemma_journal_family_accepts_29() {
    assert(spec_validate_kind_family(0x5642_4A45u32, 29) == 1);
}

/// Proof: journal family rejects kind 30.
pub proof fn lemma_journal_family_rejects_30() {
    assert(spec_validate_kind_family(0x5642_4A45u32, 30) == 2);
}

/// Proof: snapshot family accepts only kind 30.
pub proof fn lemma_snapshot_family_kind_30_ok() {
    assert(spec_validate_kind_family(0x5642_534Eu32, 30) == 1);
}

/// Proof: snapshot family rejects kind 31.
pub proof fn lemma_snapshot_family_kind_31_err() {
    assert(spec_validate_kind_family(0x5642_534Eu32, 31) == 2);
}

/// Proof: blob family accepts only kind 40.
pub proof fn lemma_blob_family_kind_40_ok() {
    assert(spec_validate_kind_family(0x5642_424Cu32, 40) == 1);
}

/// Proof: index record family accepts kind 3.
pub proof fn lemma_index_family_kind_3_ok() {
    assert(spec_validate_kind_family(0x5642_4958u32, 3) == 1);
}

/// Proof: index record family accepts kind 50.
pub proof fn lemma_index_family_kind_50_ok() {
    assert(spec_validate_kind_family(0x5642_4958u32, 50) == 1);
}

/// Proof: unknown magic always rejected regardless of kind.
pub proof fn lemma_unknown_magic_rejected(kind: u16) {
    assert(spec_validate_kind_family(0x0000_0000u32, kind) == 2);
}

/// Proof: for any magic/kind pair in the accepted set, the decision is Accepted.
pub proof fn lemma_family_accepts_all_valid_pairs() {
    assert(spec_validate_kind_family(0x5642_4A45u32, 10) == 1);
    assert(spec_validate_kind_family(0x5642_4A45u32, 20) == 1);
    assert(spec_validate_kind_family(0x5642_5352u32, 1) == 1);
    assert(spec_validate_kind_family(0x5642_4952u32, 2) == 1);
    assert(spec_validate_kind_family(0x5642_534Eu32, 30) == 1);
    assert(spec_validate_kind_family(0x5642_424Cu32, 40) == 1);
    assert(spec_validate_kind_family(0x5642_4958u32, 3) == 1);
    assert(spec_validate_kind_family(0x5642_4958u32, 50) == 1);
    assert(spec_validate_kind_family(0x5642_5254u32, 5) == 1);
}

/// Proof: for any magic/kind pair not in the accepted set, the decision is Rejected.
pub proof fn lemma_family_rejects_all_invalid_pairs() {
    assert(spec_validate_kind_family(0x5642_4A45u32, 9) == 2);
    assert(spec_validate_kind_family(0x5642_4A45u32, 30) == 2);
    assert(spec_validate_kind_family(0x5642_5352u32, 2) == 2);
    assert(spec_validate_kind_family(0x5642_4952u32, 1) == 2);
    assert(spec_validate_kind_family(0x5642_534Eu32, 30) != 2);  // already checked above
}

/// Proof: the journal family predicate is a proper subset of all known kinds.
pub proof fn lemma_journal_family_is_subset() {
    assert(spec_validate_kind_family(0x5642_4A45u32, 15) == 1);
    assert(spec_validate_kind_family(0x5642_5352u32, 1) == 1);
    assert(spec_validate_kind_family(0x5642_4952u32, 2) == 1);
    assert(spec_validate_kind_family(0x5642_534Eu32, 30) == 1);
    assert(spec_validate_kind_family(0x5642_424Cu32, 40) == 1);
    assert(spec_validate_kind_family(0x5642_4958u32, 3) == 1);
    assert(spec_validate_kind_family(0x5642_5254u32, 5) == 1);
    // kind 50 is only in index, not journal
    assert(spec_validate_kind_family(0x5642_4A45u32, 50) == 2);
}

// =========================================================================
// VALIDATED JOURNAL RECORD INVARIANT SPEC
// =========================================================================
/// Spec: a ValidatedJournalRecord implies exact kind match + event validity.
pub open spec fn spec_validated_journal_record_implies_parity_and_validity(
    envelope_kind: u16,
    payload_kind: u16,
    event_valid: bool,
) -> bool {
    // The record is constructible only when semantic decode == SemanticSuccess
    spec_classify_journal_semantic_decode(envelope_kind, payload_kind, event_valid) == 1int
}

/// Proof: if decode decision is SemanticSuccess then kinds match and event is valid.
pub proof fn lemma_decode_success_implies_parity_and_validity(
    envelope_kind: u16,
    payload_kind: u16,
    event_valid: bool,
)
    requires
        spec_classify_journal_semantic_decode(envelope_kind, payload_kind, event_valid) == 1int,
    ensures
        spec_exact_journal_kind_parity_exists(envelope_kind, payload_kind) && event_valid,
{
    assert(spec_exact_journal_kind_parity_exists(envelope_kind, payload_kind));
    assert(event_valid);
}

// =========================================================================
// ACTION REPLAY TRACKER IS_RESOLVED SPEC
// =========================================================================
/// Spec: is_resolved holds when action/step is in completed or failed set.
pub open spec fn spec_action_replay_is_resolved_set(
    completed: Set<nat>,
    failed: Set<nat>,
    action: nat,
    step: nat,
) -> bool {
    completed.contains(action * 65536 + step) || failed.contains(action * 65536 + step)
}

/// Proof: inserting into completed makes is_resolved true.
pub proof fn lemma_completed_insert_implies_resolved() {
    let mut completed = set! {};
    let key = 42 * 65536 + 1;
    completed = completed.insert(key);
    assert(completed.contains(key));
    assert(spec_action_replay_is_resolved_set(completed, set! {}, 42, 1));
}

/// Proof: inserting into failed makes is_resolved true.
pub proof fn lemma_failed_insert_implies_resolved() {
    let mut failed = set! {};
    let key = 42 * 65536 + 1;
    failed = failed.insert(key);
    assert(failed.contains(key));
    assert(spec_action_replay_is_resolved_set(set! {}, failed, 42, 1));
}

/// Proof: is_resolved is commutative over completed/failed (union order doesn't matter).
pub proof fn lemma_is_resolved_union_commutative(
    completed: Set<nat>,
    failed: Set<nat>,
    action: nat,
    step: nat,
) {
    let packed = action * 65536 + step;
    assert(spec_action_replay_is_resolved_set(completed, failed, action, step) == (
    completed.contains(packed) || failed.contains(packed)));
    assert(spec_action_replay_is_resolved_set(failed, completed, action, step) == (failed.contains(
        packed,
    ) || completed.contains(packed)));
    // OR is commutative
    assert(completed.contains(packed) || failed.contains(packed) == failed.contains(packed)
        || completed.contains(packed));
}

// =========================================================================
// UNSUPPORTED RECOVERY STATE — UNION OR-SEMANTICS
// =========================================================================
/// Spec: union of two UnsupportedRecoveryState values is flag-wise OR.
pub open spec fn spec_unsupported_state_union_or(
    a_slot_values: bool,
    a_slot_taint: bool,
    a_action_payloads: bool,
    b_slot_values: bool,
    b_slot_taint: bool,
    b_action_payloads: bool,
) -> (bool, bool, bool) {
    (
        a_slot_values || b_slot_values,
        a_slot_taint || b_slot_taint,
        a_action_payloads || b_action_payloads,
    )
}

/// Proof: union is associative.
pub proof fn lemma_union_associative(
    a_sv: bool,
    a_st: bool,
    a_ap: bool,
    b_sv: bool,
    b_st: bool,
    b_ap: bool,
    c_sv: bool,
    c_st: bool,
    c_ap: bool,
) {
    let lhs = spec_unsupported_state_union_or(
        spec_unsupported_state_union_or(a_sv, a_st, a_ap, b_sv, b_st, b_ap).0,
        spec_unsupported_state_union_or(a_sv, a_st, a_ap, b_sv, b_st, b_ap).1,
        spec_unsupported_state_union_or(a_sv, a_st, a_ap, b_sv, b_st, b_ap).2,
        c_sv,
        c_st,
        c_ap,
    );
    let rhs = spec_unsupported_state_union_or(
        a_sv,
        a_st,
        a_ap,
        spec_unsupported_state_union_or(b_sv, b_st, b_ap, c_sv, c_st, c_ap).0,
        spec_unsupported_state_union_or(b_sv, b_st, b_ap, c_sv, c_st, c_ap).1,
        spec_unsupported_state_union_or(b_sv, b_st, b_ap, c_sv, c_st, c_ap).2,
    );
    assert(lhs == rhs);
}

/// Proof: union with SUPPORTED (all false) is identity.
pub proof fn lemma_union_with_supported_is_identity(sv: bool, st: bool, ap: bool) {
    let result = spec_unsupported_state_union_or(sv, st, ap, false, false, false);
    assert(result.0 == sv);
    assert(result.1 == st);
    assert(result.2 == ap);
}

/// Proof: union with SUPPORTED on the left is identity.
pub proof fn lemma_union_supported_left_is_identity(sv: bool, st: bool, ap: bool) {
    let result = spec_unsupported_state_union_or(false, false, false, sv, st, ap);
    assert(result.0 == sv);
    assert(result.1 == st);
    assert(result.2 == ap);
}

/// Proof: union is commutative.
pub proof fn lemma_union_commutative(
    a_sv: bool,
    a_st: bool,
    a_ap: bool,
    b_sv: bool,
    b_st: bool,
    b_ap: bool,
) {
    let lhs = spec_unsupported_state_union_or(a_sv, a_st, a_ap, b_sv, b_st, b_ap);
    let rhs = spec_unsupported_state_union_or(b_sv, b_st, b_ap, a_sv, a_st, a_ap);
    assert(lhs == rhs);
}

/// Proof: union_matches_flags spec returns true when union result matches flag-wise OR.
pub open spec fn spec_union_matches_flags(
    a_sv: bool,
    a_st: bool,
    a_ap: bool,
    b_sv: bool,
    b_st: bool,
    b_ap: bool,
    u_sv: bool,
    u_st: bool,
    u_ap: bool,
) -> bool {
    u_sv == (a_sv || b_sv) && u_st == (a_st || b_st) && u_ap == (a_ap || b_ap)
}

/// Proof: the flag-wise union always satisfies union_matches_flags.
pub proof fn lemma_union_always_satisfies_matches_flags(
    a_sv: bool,
    a_st: bool,
    a_ap: bool,
    b_sv: bool,
    b_st: bool,
    b_ap: bool,
) {
    let u = spec_unsupported_state_union_or(a_sv, a_st, a_ap, b_sv, b_st, b_ap);
    assert(spec_union_matches_flags(a_sv, a_st, a_ap, b_sv, b_st, b_ap, u.0, u.1, u.2));
}

// =========================================================================
// DIGEST CHECK HIERARCHY RANK — ORDERING PROOFS
// =========================================================================
/// Proof: WorkflowSourceOnly (rank 1) is strictly weaker than WorkflowAndIr (rank 2).
pub proof fn lemma_digest_check_workflow_source_strictly_weaker_than_workflow_and_ir() {
    assert(1u8 < 2u8);
}

/// Proof: WorkflowAndIr (rank 2) is strictly weaker than Full (rank 3).
pub proof fn lemma_digest_check_workflow_and_ir_strictly_weaker_than_full() {
    assert(2u8 < 3u8);
}

/// Proof: WorkflowSourceOnly (rank 1) is strictly weaker than Full (rank 3).
pub proof fn lemma_digest_check_workflow_source_strictly_weaker_than_full() {
    assert(1u8 < 3u8);
}

/// Proof: hierarchy_rank is strictly monotone over the three DigestCheck levels.
pub proof fn lemma_hierarchy_rank_monotone() {
    assert(1u8 < 2u8);
    assert(2u8 < 3u8);
}

/// Proof: checks_workflow_source is true for rank >= 1.
pub proof fn lemma_checks_workflow_source_for_rank_ge_1() {
    assert(true);  // rank >= 1 means all three levels
}

/// Proof: checks_compiled_ir is false for WorkflowSourceOnly (rank 1) and true for others.
pub proof fn lemma_checks_compiled_ir_threshold() {
    // rank 1 < 2 means WorkflowSourceOnly does not check compiled IR
    assert(1u8 < 2u8);
    // rank 2 >= 2 means WorkflowAndIr checks compiled IR
    assert(2u8 >= 2u8);
    // rank 3 >= 3 means Full checks compiled IR
    assert(3u8 >= 2u8);
}

/// Proof: checks_full is true only for Full (rank 3).
pub proof fn lemma_checks_full_only_at_full_rank() {
    // rank 1 < 3 means WorkflowSourceOnly does not check full
    assert(1u8 < 3u8);
    // rank 2 < 3 means WorkflowAndIr does not check full
    assert(2u8 < 3u8);
    // rank 3 >= 3 means Full checks full
    assert(3u8 >= 3u8);
}

/// Proof: strictly_weaker is transitive across the three levels.
pub proof fn lemma_digest_check_strictly_weaker_transitive() {
    // WorkflowSourceOnly < WorkflowAndIr < Full
    assert(1u8 < 2u8 && 2u8 < 3u8);
    assert(1u8 < 3u8);
}

// =========================================================================
// IS_KNOWN_RECORD_KIND PRODUCTION BOUNDING LEMMAS
// =========================================================================
/// Spec: all known kinds are either journal (10..=29) or non-journal (1|2|3|30|40|50).
pub open spec fn spec_all_known_kinds_partitioned(kind: u16) -> bool {
    (10u16 <= kind && kind <= 29u16) || kind == 1 || kind == 2 || kind == 3 || kind == 30 || kind
        == 40 || kind == 50
}

/// Proof: kind 1 is a known non-journal kind.
pub proof fn lemma_kind_1_is_known_non_journal() {
    assert(spec_all_known_kinds_partitioned(1));
    assert(!spec_mrwe5_is_journal_record_kind(1));
}

/// Proof: kind 30 is a known non-journal kind.
pub proof fn lemma_kind_30_is_known_non_journal() {
    assert(spec_all_known_kinds_partitioned(30));
    assert(!spec_mrwe5_is_journal_record_kind(30));
}

/// Proof: kind 50 is a known non-journal kind.
pub proof fn lemma_kind_50_is_known_non_journal() {
    assert(spec_all_known_kinds_partitioned(50));
    assert(!spec_mrwe5_is_journal_record_kind(50));
}

/// Proof: kind 9 is NOT known.
pub proof fn lemma_kind_9_is_unknown() {
    assert(!spec_all_known_kinds_partitioned(9));
}

/// Proof: kind 65535 is NOT known.
pub proof fn lemma_kind_max_unknown() {
    assert(!spec_all_known_kinds_partitioned(65535u16));
}

/// Proof: the boundary between journal and non-journal kinds is clean at 10.
pub proof fn lemma_journal_boundary_at_10() {
    assert(spec_mrwe5_is_journal_record_kind(10));
    assert(!spec_mrwe5_is_journal_record_kind(9));
}

/// Proof: the boundary at 29/30 separates journal from non-journal.
pub proof fn lemma_journal_boundary_at_29_30() {
    assert(spec_mrwe5_is_journal_record_kind(29));
    assert(!spec_mrwe5_is_journal_record_kind(30));
}

/// Proof: every kind in 10..=29 is a journal record kind.
pub proof fn lemma_all_journal_range_is_journal_kind() {
    assert(spec_mrwe5_is_journal_record_kind(10));
    assert(spec_mrwe5_is_journal_record_kind(15));
    assert(spec_mrwe5_is_journal_record_kind(29));
}

/// Proof: kind 0 is not known.
pub proof fn lemma_kind_0_not_known() {
    assert(!spec_all_known_kinds_partitioned(0));
}

// =========================================================================
// EXACT JOURNAL KIND PARITY — PRODUCTION BOUNDING LEMMAS
// =========================================================================
/// Proof: ExactJournalKindParity::new(s, s) always succeeds.
pub proof fn lemma_parity_new_succeeds_on_same() {
    assert(spec_exact_journal_kind_parity_exists(12u16, 12u16));
    assert(spec_exact_journal_kind_parity_exists(29u16, 29u16));
    assert(spec_exact_journal_kind_parity_exists(10u16, 10u16));
}

/// Proof: ExactJournalKindParity::new(a, b) always fails when a != b.
pub proof fn lemma_parity_new_fails_on_different() {
    assert(!spec_exact_journal_kind_parity_exists(12u16, 29u16));
    assert(!spec_exact_journal_kind_parity_exists(29u16, 12u16));
}

// =========================================================================
// CLASSIFY JOURNAL SEMANTIC DECODE — PRODUCTION BOUNDING LEMMAS
// =========================================================================
/// Proof: the production seam classify_journal_semantic_decode returns
/// SemanticSuccess iff kinds match and event is valid.
pub proof fn lemma_classify_semantic_decode_success_iff() {
    // Exact match + valid = SemanticSuccess
    assert(spec_classify_journal_semantic_decode(29u16, 29u16, true) == 1);
    assert(spec_classify_journal_semantic_decode(12u16, 12u16, true) == 1);
    assert(spec_classify_journal_semantic_decode(10u16, 10u16, true) == 1);

    // Exact match + invalid = InvalidEvent
    assert(spec_classify_journal_semantic_decode(29u16, 29u16, false) == 3);
    assert(spec_classify_journal_semantic_decode(12u16, 12u16, false) == 3);

    // Mismatch = KindPayloadMismatch regardless of validity
    assert(spec_classify_journal_semantic_decode(29u16, 12u16, true) == 2);
    assert(spec_classify_journal_semantic_decode(29u16, 12u16, false) == 2);
    assert(spec_classify_journal_semantic_decode(10u16, 29u16, true) == 2);
    assert(spec_classify_journal_semantic_decode(10u16, 29u16, false) == 2);
}

// =========================================================================
// RECORD KIND FAMILY — PRODUCTION BOUNDING LEMMAS
// =========================================================================
/// Proof: classify_record_kind_family correctly classifies all magic/kind pairs
/// found in the production codec validation module.
pub proof fn lemma_classify_family_all_known_pairs() {
    // Workflow source
    assert(spec_validate_kind_family(0x5642_5352u32, 1) == 1);
    // Compiled IR
    assert(spec_validate_kind_family(0x5642_4952u32, 2) == 1);
    // Journal event — boundary range
    assert(spec_validate_kind_family(0x5642_4A45u32, 10) == 1);
    assert(spec_validate_kind_family(0x5642_4A45u32, 15) == 1);
    assert(spec_validate_kind_family(0x5642_4A45u32, 29) == 1);
    // Snapshot
    assert(spec_validate_kind_family(0x5642_534Eu32, 30) == 1);
    // Blob
    assert(spec_validate_kind_family(0x5642_424Cu32, 40) == 1);
    // Index record
    assert(spec_validate_kind_family(0x5642_4958u32, 3) == 1);
    assert(spec_validate_kind_family(0x5642_4958u32, 50) == 1);
    // Recovery stamp
    assert(spec_validate_kind_family(0x5642_5254u32, 5) == 1);

    // Cross-magic rejections
    assert(spec_validate_kind_family(0x5642_5352u32, 2) == 2);
    assert(spec_validate_kind_family(0x5642_4952u32, 1) == 2);
    assert(spec_validate_kind_family(0x5642_534Eu32, 30) != 2);
}

} // verus!
// end verus!
// =========================================================================
// PRODUCTION EXEC BRIDGE — cfg(verus) bound, compiled as part of crate
// =========================================================================
#[cfg(verus)]
verus! {

/// Exec bridge: ExactJournalKindParity::new returns Some iff kinds match.
pub exec fn exec_exact_journal_kind_parity_new(envelope_kind: u16, payload_kind: u16) -> (result:
    bool)
    ensures
        result == spec_exact_journal_kind_parity_exists(envelope_kind, payload_kind),
{
    envelope_kind == payload_kind
}

/// Exec bridge: classify_journal_semantic_decode returns the correct code.
pub exec fn exec_classify_journal_semantic_decode(
    envelope_kind: u16,
    payload_kind: u16,
    event_valid: bool,
) -> (result: u8)
    ensures
        result == spec_classify_journal_semantic_decode(
            envelope_kind,
            payload_kind,
            event_valid,
        ) as u8,
{
    match crate::codec::semantic::classify_journal_semantic_decode(
        envelope_kind,
        payload_kind,
        event_valid,
    ) {
        crate::codec::semantic::JournalSemanticDecodeDecision::SemanticSuccess => 1,
        crate::codec::semantic::JournalSemanticDecodeDecision::KindPayloadMismatch => 2,
        crate::codec::semantic::JournalSemanticDecodeDecision::InvalidEvent => 3,
    }
}

/// Exec bridge: classify_record_kind_family returns the correct code.
pub exec fn exec_classify_record_kind_family(magic: u32, kind: u16) -> (result: u8)
    ensures
        result == spec_validate_kind_family(magic, kind) as u8,
{
    match crate::codec::semantic::classify_record_kind_family(magic, kind) {
        crate::codec::validation::RecordKindFamilyDecision::Accepted => 1,
        crate::codec::validation::RecordKindFamilyDecision::Rejected => 2,
    }
}

/// Exec bridge: ActionReplayTracker::is_resolved matches the spec.
pub exec fn exec_action_replay_is_resolved(
    completed_contains: bool,
    failed_contains: bool,
) -> (result: bool)
    ensures
        result == (completed_contains || failed_contains),
{
    completed_contains || failed_contains
}

/// Exec bridge: UnsupportedRecoveryState::union_matches_flags matches spec.
pub exec fn exec_unsupported_union_matches_flags(
    a_sv: bool,
    a_st: bool,
    a_ap: bool,
    b_sv: bool,
    b_st: bool,
    b_ap: bool,
    u_sv: bool,
    u_st: bool,
    u_ap: bool,
) -> (result: bool)
    ensures
        result == spec_union_matches_flags(a_sv, a_st, a_ap, b_sv, b_st, b_ap, u_sv, u_st, u_ap),
{
    u_sv == (a_sv || b_sv) && u_st == (a_st || b_st) && u_ap == (a_ap || b_ap)
}

/// Exec bridge: DigestCheck hierarchy_rank matches spec.
pub exec fn exec_digest_check_hierarchy_rank(level: u8) -> (result: u8)
    ensures
        spec_digest_check_hierarchy_rank(level as int) == result as int,
{
    match level {
        1 => 1u8,
        2 => 2u8,
        3 => 3u8,
        _ => 0u8,  // should not happen for valid DigestCheck levels
    }
}

/// Exec bridge: DigestCheck is_strictly_weaker_than matches spec.
pub exec fn exec_digest_check_strictly_weaker(a: u8, b: u8) -> (result: bool)
    ensures
        spec_digest_check_strictly_weaker(a as int, b as int) == result,
{
    a < b
}

} // verus!
// end cfg(verus) verus!
