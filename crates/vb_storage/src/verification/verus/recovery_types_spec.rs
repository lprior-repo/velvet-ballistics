#![forbid(unsafe_code)]
//! Verus proof artifacts for vb_storage recovery and classification types.
//!
//! This module provides:
//! - Standalone spec functions and proof lemmas (compiled standalone via --crate-type=lib)
//! - Production exec bridges (compiled as part of the crate with #[cfg(verus)])
//!
//! Production binding map:
//!   - `mrwe5_contract` functions  → `mrwe5_spec_*` / `mrwe5_exec_*` (cfg-bound)
//!   - `codec::semantic` functions → `semantic_spec_*` / `semantic_exec_*` (cfg-bound)
//!   - `codec::validation` functions → `validation_spec_*` / `validation_exec_*` (cfg-bound)
//!   - `recovery::types` invariants → `recovery_spec_*` / `recovery_exec_*` (cfg-bound)

// =========================================================================
// STANDALONE SPEC LAYER — compiled standalone for --crate-type=lib
// =========================================================================

use vstd::prelude::*;

verus! {

    // =========================================================================
    // MRWE5 SPEC LAYER
    // =========================================================================

    /// Spec: two durable kind ids are an exact match.
    #[verifier::nonlinear]
    pub open spec fn spec_mrwe5_kinds_exact_match(envelope_kind: u16, payload_kind: u16) -> bool {
        envelope_kind == payload_kind
    }

    /// Spec: kind compatibility is exact when kinds match, rejected otherwise.
    #[verifier::nonlinear]
    pub open spec fn spec_mrwe5_kind_compatibility(
        envelope_kind: u16,
        payload_kind: u16,
    ) -> int {
        if spec_mrwe5_kinds_exact_match(envelope_kind, payload_kind) {
            1 // ExactMatch
        } else {
            2 // RejectedMismatch
        }
    }

    /// Spec: semantic decode decision from envelope/payload pair and validity.
    #[verifier::nonlinear]
    pub open spec fn spec_mrwe5_semantic_decode(
        envelope_kind: u16,
        payload_kind: u16,
        event_valid: bool,
    ) -> int {
        let comp = spec_mrwe5_kind_compatibility(envelope_kind, payload_kind);
        if comp == 1int {
            if event_valid { 1int } else { 3int }
        } else {
            2int
        }
    }

    /// Spec: a kind id is a journal-event family member.
    /// Production: MRWE5_JOURNAL_MIN_KIND_ID=10, MRWE5_JOURNAL_MAX_KIND_ID=29.
    pub open spec fn spec_mrwe5_is_journal_record_kind(kind: u16) -> bool {
        10u16 <= kind && kind <= 29u16
    }

    /// Spec: record kind family acceptance for the journal-event magic.
    /// Production: MRWE5_MAGIC_JOURNAL_EVENT = 0xDEADBEEF (codec magic).
    pub open spec fn spec_mrwe5_record_kind_family(magic: u32, kind: u16) -> int {
        if magic == 0xDEADBEEFu32 && spec_mrwe5_is_journal_record_kind(kind) {
            1int // Accepted
        } else {
            2int // Rejected
        }
    }

    /// Spec: canonical kind id for MRWE5 payload classes.
    /// Production: StepSucceeded=29, SlotWrittenEvent=30.
    pub open spec fn spec_mrwe5_canonical_kind_id(class: int) -> Option<u16> {
        if class == 1 {
            Some(29u16)
        } else if class == 2 {
            Some(30u16)
        } else {
            None
        }
    }

    /// Spec: MRWE5 payload class for a given kind id.
    pub open spec fn spec_mrwe5_payload_class(kind_id: u16) -> Option<int> {
        if kind_id == 29u16 {
            Some(1)
        } else if kind_id == 30u16 {
            Some(2)
        } else {
            None
        }
    }

    /// Spec: StepSucceeded and SlotWrittenEvent have distinct kind ids.
    pub open spec fn spec_mrwe5_step_succeeded_and_slot_written_distinct() -> bool {
        29u16 != 30u16
    }

    /// Spec: MRWE5 magic is the journal-event magic constant.
    pub open spec fn spec_mrwe5_magic_journal_event() -> u32 {
        0xDEADBEEFu32
    }

    // =========================================================================
    // KEY INVARIANT LEMMAS — MRWE5 classification correctness
    // =========================================================================

    pub proof fn lemma_compatibility_exact_match_symmetric(
        envelope_kind: u16,
        payload_kind: u16,
    ) {
        assert(spec_mrwe5_kinds_exact_match(envelope_kind, payload_kind)
            == spec_mrwe5_kinds_exact_match(payload_kind, envelope_kind));
    }

    pub proof fn lemma_exact_match_implies_exact_compatibility(
        envelope_kind: u16,
        payload_kind: u16,
    ) {
        if spec_mrwe5_kinds_exact_match(envelope_kind, payload_kind) {
            assert(spec_mrwe5_kind_compatibility(envelope_kind, payload_kind) == 1);
        }
    }

    pub proof fn lemma_non_match_implies_rejected_compatibility(
        envelope_kind: u16,
        payload_kind: u16,
    ) {
        if !spec_mrwe5_kinds_exact_match(envelope_kind, payload_kind) {
            assert(spec_mrwe5_kind_compatibility(envelope_kind, payload_kind) == 2);
        }
    }

    pub proof fn lemma_semantic_decode_mismatch_when_kinds_differ(
        envelope_kind: u16,
        payload_kind: u16,
        event_valid: bool,
    ) {
        if !spec_mrwe5_kinds_exact_match(envelope_kind, payload_kind) {
            assert(spec_mrwe5_semantic_decode(envelope_kind, payload_kind, event_valid) == 2);
        }
    }

    pub proof fn lemma_semantic_decode_success_requires_match_and_valid(
        envelope_kind: u16,
        payload_kind: u16,
        event_valid: bool,
    ) {
        if spec_mrwe5_semantic_decode(envelope_kind, payload_kind, event_valid) == 1 {
            assert(spec_mrwe5_kinds_exact_match(envelope_kind, payload_kind));
            assert(event_valid);
        }
    }

    pub proof fn lemma_step_succeeded_slot_written_distinct() {
        assert(spec_mrwe5_step_succeeded_and_slot_written_distinct());
    }

    pub proof fn lemma_step_succeeded_is_journal_kind() {
        assert(spec_mrwe5_is_journal_record_kind(29));
    }

       pub proof fn lemma_slot_written_is_not_journal_kind() {
        assert(!spec_mrwe5_is_journal_record_kind(30));
    }

    pub proof fn lemma_kind_0_not_journal() {
        assert(!spec_mrwe5_is_journal_record_kind(0));
    }

    pub proof fn lemma_kind_10_is_journal() {
        assert(spec_mrwe5_is_journal_record_kind(10));
    }

    pub proof fn lemma_kind_29_is_journal() {
        assert(spec_mrwe5_is_journal_record_kind(29));
    }

    pub proof fn lemma_record_kind_family_journal_only(
        kind: u16,
    ) {
        assert(spec_mrwe5_record_kind_family(0, kind) == 2);
    }

    pub proof fn lemma_exact_match_compatibility_correspondence(
        envelope_kind: u16,
        payload_kind: u16,
    ) {
        if spec_mrwe5_kinds_exact_match(envelope_kind, payload_kind) {
            assert(spec_mrwe5_kind_compatibility(envelope_kind, payload_kind) == 1);
        } else {
            assert(spec_mrwe5_kind_compatibility(envelope_kind, payload_kind) == 2);
        }
    }

    pub proof fn lemma_semantic_decode_exact_valid_is_success(
        envelope_kind: u16,
        payload_kind: u16,
    ) {
        if spec_mrwe5_kinds_exact_match(envelope_kind, payload_kind) {
            assert(spec_mrwe5_semantic_decode(envelope_kind, payload_kind, true) == 1);
        }
    }

    pub proof fn lemma_semantic_decode_exact_invalid_is_error(
        envelope_kind: u16,
        payload_kind: u16,
    ) {
        if spec_mrwe5_kinds_exact_match(envelope_kind, payload_kind) {
            assert(spec_mrwe5_semantic_decode(envelope_kind, payload_kind, false) == 3);
        }
    }

    // =========================================================================
    // EVENT SEQ SPEC
    // =========================================================================

    pub open spec fn spec_event_seq_contiguous(prev: u64, next: u64) -> bool {
        next == prev.wrapping_add(1)
    }

    pub open spec fn spec_event_seq_validate_contiguous(seqs: Seq<u64>) -> bool {
        seqs.len() <= 1
            || (forall|i: int| 0 <= i < seqs.len() as int - 1 ==> #[trigger] seqs[i + 1] == seqs[i].wrapping_add(1))
    }

    // =========================================================================
    // ACTION REPLAY TRACKER SPEC
    // =========================================================================

    pub open spec fn spec_action_replay_is_resolved(
        completed: Set<(u64, u16)>,
        failed: Set<(u64, u16)>,
        action: u64,
        step: u16,
    ) -> bool {
        completed.contains((action, step)) || failed.contains((action, step))
    }

    pub open spec fn spec_action_replay_mark_resolved_fails(
        completed: Set<(u64, u16)>,
        failed: Set<(u64, u16)>,
        action: u64,
        step: u16,
    ) -> bool {
        spec_action_replay_is_resolved(completed, failed, action, step)
    }

    pub open spec fn spec_action_replay_apply_effect(
        completed: Set<(u64, u16)>,
        action: u64,
        step: u16,
    ) -> int {
        if spec_action_replay_is_resolved(completed, set! {}, action, step) {
            2 // Duplicate
        } else {
            1 // Apply
        }
    }

    pub open spec fn spec_action_replay_state_equivalent(
        completed_a: Set<(u64, u16)>,
        failed_a: Set<(u64, u16)>,
        completed_b: Set<(u64, u16)>,
        failed_b: Set<(u64, u16)>,
    ) -> bool {
        completed_a == completed_b && failed_a == failed_b
    }

    // =========================================================================
    // DIGEST CHECK SPEC
    // =========================================================================

    pub open spec fn spec_digest_check_hierarchy_rank(level: int) -> int {
        if level == 0 { 1 }
        else if level == 1 { 2 }
        else { 3 }
    }

    pub open spec fn spec_digest_check_strictly_weaker(level_a: int, level_b: int) -> bool {
        spec_digest_check_hierarchy_rank(level_a) < spec_digest_check_hierarchy_rank(level_b)
    }

    pub open spec fn spec_digest_check_checks_workflow_source(level: int) -> bool {
        spec_digest_check_hierarchy_rank(level) >= 1
    }

    pub open spec fn spec_digest_check_checks_compiled_ir(level: int) -> bool {
        spec_digest_check_hierarchy_rank(level) >= 2
    }

    pub open spec fn spec_digest_check_checks_full(level: int) -> bool {
        spec_digest_check_hierarchy_rank(level) == 3
    }

    // =========================================================================
    // UNSUPPORTED RECOVERY STATE SPEC
    // =========================================================================

    pub open spec fn spec_unsupported_union(
        slot_values_a: bool,
        slot_taint_a: bool,
        action_payloads_a: bool,
        slot_values_b: bool,
        slot_taint_b: bool,
        action_payloads_b: bool,
    ) -> (bool, bool, bool) {
        (
            slot_values_a || slot_values_b,
            slot_taint_a || slot_taint_b,
            action_payloads_a || action_payloads_b,
        )
    }

    pub open spec fn spec_unsupported_supported_is_clean() -> bool {
        false && false && false
    }

    pub open spec fn spec_unsupported_is_fully_supported(
        slot_values: bool,
        slot_taint: bool,
        action_payloads: bool,
    ) -> bool {
        !slot_values && !slot_taint && !action_payloads
    }

    // =========================================================================
    // REPLAY CONTIGUOUS SEQUENCE SPEC
    // =========================================================================

    pub open spec fn spec_validate_contiguous_sequences(seqs: Seq<u64>) -> bool {
        seqs.len() <= 1
            || (forall|i: int| 0 <= i < seqs.len() as int - 1 ==> #[trigger] seqs[i + 1] == seqs[i].wrapping_add(1))
    }

    pub proof fn lemma_contiguous_empty() {
        assert(spec_validate_contiguous_sequences(seq![]));
    }

    pub proof fn lemma_contiguous_single() {
        assert(spec_validate_contiguous_sequences(seq![42u64]));
    }

    pub proof fn lemma_contiguous_two() {
        assert(spec_validate_contiguous_sequences(seq![10u64, 11]));
    }

    // =========================================================================
    // DIMENSION COUNT SPEC
    // =========================================================================

    pub open spec fn spec_recovery_dimension_count_from_index(max_index: Option<u16>, _run: u64) -> (int, bool) {
        match max_index {
            None => (0, true),
            Some(idx) => {
                let result = idx as int + 1;
                if result <= u16::MAX as int {
                    (result, true)
                } else {
                    (0, false)
                }
            }
        }
    }

    // =========================================================================
    // EVENT KIND CLASS SPEC
    // =========================================================================

    pub open spec fn spec_event_kind_class_step_succeeded() -> int {
        1
    }

    pub open spec fn spec_event_kind_class_slot_written() -> int {
        2
    }

    pub open spec fn spec_event_kind_class_other() -> int {
        3
    }

    // =========================================================================
    // RECOVERY TERMINAL STATE SPEC
    // =========================================================================

    pub open spec fn spec_recovery_terminal_state_count() -> int {
        4
    }

    // =========================================================================
    // EXEC BRIDGE FOR DIGEST CHECK (standalone — no production code needed)
    // =========================================================================

    pub exec fn exec_digest_check_hierarchy_rank(level: u8) -> (result: u8)
        ensures
            spec_digest_check_hierarchy_rank(level as int) == result as int,
    {
        match level {
            0 => 1u8,
            1 => 2u8,
            _ => 3u8,
        }
    }

    pub exec fn exec_digest_check_strictly_weaker(a: u8, b: u8) -> (result: bool)
        ensures
            result == (a as int <= b as int && !(a == b)),
    {
        a < b
    }

    pub exec fn exec_unsupported_is_fully_supported(
        slot_values: bool,
        slot_taint: bool,
        action_payloads: bool,
    ) -> (result: bool)
        ensures
            result == spec_unsupported_is_fully_supported(slot_values, slot_taint, action_payloads),
    {
        !slot_values && !slot_taint && !action_payloads
    }

    // =========================================================================
    // REPLAY ATTEMPT STALENESS SPEC
    // =========================================================================

    pub open spec fn spec_replay_attempt_is_stale(attempt: u16, max_attempt: u16) -> bool {
        attempt < max_attempt
    }

    pub open spec fn spec_replay_attempt_is_current(attempt: u16, max_attempt: u16) -> bool {
        attempt == max_attempt
    }

    

    pub open spec fn spec_max_attempt(attempts: Seq<u16>) -> u16 {
        if attempts.len() == 0 {
            0
        } else {
            attempts[0]
        }
    }

    // =========================================================================
    // RECOVERY RUNTIME SUMMARY SPEC
    // =========================================================================

    pub open spec fn spec_apply_summary_step_started(steps_started: u64) -> u64 {
        steps_started.wrapping_add(1)
    }

    pub open spec fn spec_apply_summary_step_succeeded(steps_succeeded: u64) -> u64 {
        steps_succeeded.wrapping_add(1)
    }

    pub open spec fn spec_apply_summary_slot_written(slots_written: u64) -> u64 {
        slots_written.wrapping_add(1)
    }

    pub open spec fn spec_apply_summary_action_completed(actions_resolved: u64) -> u64 {
        actions_resolved.wrapping_add(1)
    }

    pub open spec fn spec_apply_summary_action_completed_envelope(
        actions_resolved: u64,
        steps_succeeded: u64,
        slots_written: u64,
    ) -> (u64, u64, u64) {
        (
            actions_resolved.wrapping_add(1),
            steps_succeeded.wrapping_add(1),
            slots_written.wrapping_add(1),
        )
    }

    // =========================================================================
    // EVENTSEQ MONOTONICITY LEMMAS
    // =========================================================================

    pub proof fn lemma_event_seq_wrap_preserves_order(a: u64, b: u64) {
        if a <= b && a < u64::MAX && b < u64::MAX {
            assert(a.wrapping_add(1) <= b.wrapping_add(1));
        }
    }

    pub proof fn lemma_event_seq_wrapping_add_well_defined(n: u64) {
        let _result = n.wrapping_add(1);
        assert(_result >= 0 && _result <= u64::MAX);
    }

} // end verus!{

// =========================================================================
// PRODUCTION EXEC BRIDGE — cfg(verus) bound, compiled as part of crate
// =========================================================================

#[cfg(verus)]
verus! {

    /// Exec bridge: mrwe5_kinds_are_exact_match matches spec.
    pub exec fn exec_mrwe5_kinds_exact_match(
        envelope_kind: u16,
        payload_kind: u16,
    ) -> bool {
        let result = crate::mrwe5_contract::mrwe5_kinds_are_exact_match(envelope_kind, payload_kind);
        assert(spec_mrwe5_kinds_exact_match(envelope_kind, payload_kind) == result);
        result
    }

    /// Exec bridge: mrwe5_classify_kind_compatibility matches spec.
    pub exec fn exec_mrwe5_kind_compatibility(
        envelope_kind: u16,
        payload_kind: u16,
    ) -> crate::mrwe5_contract::Mrwe5KindCompatibility {
        let result = crate::mrwe5_contract::mrwe5_classify_kind_compatibility(envelope_kind, payload_kind);
        let spec_rank = spec_mrwe5_kind_compatibility(envelope_kind, payload_kind);
        let prod_rank: int = match result {
            crate::mrwe5_contract::Mrwe5KindCompatibility::ExactMatch => 1,
            crate::mrwe5_contract::Mrwe5KindCompatibility::RejectedMismatch => 2,
        };
        assert(spec_rank == prod_rank);
        result
    }

    /// Exec bridge: mrwe5_classify_semantic_decode matches spec.
    pub exec fn exec_mrwe5_semantic_decode(
        envelope_kind: u16,
        payload_kind: u16,
        event_valid: bool,
    ) -> crate::mrwe5_contract::Mrwe5SemanticDecodeDecision {
        let result = crate::mrwe5_contract::mrwe5_classify_semantic_decode(envelope_kind, payload_kind, event_valid);
        let spec_result = spec_mrwe5_semantic_decode(envelope_kind, payload_kind, event_valid);
        let prod_disc: int = match result {
            crate::mrwe5_contract::Mrwe5SemanticDecodeDecision::SemanticSuccess => 1,
            crate::mrwe5_contract::Mrwe5SemanticDecodeDecision::KindPayloadMismatch => 2,
            crate::mrwe5_contract::Mrwe5SemanticDecodeDecision::InvalidEvent => 3,
        };
        assert(spec_result == prod_disc);
        result
    }

    /// Exec bridge: mrwe5_is_journal_record_kind matches spec.
    pub exec fn exec_mrwe5_is_journal_record_kind(kind: u16) -> bool {
        let result = crate::mrwe5_contract::mrwe5_is_journal_record_kind(kind);
        assert(spec_mrwe5_is_journal_record_kind(kind) == result);
        result
    }

    /// Exec bridge: mrwe5_classify_record_kind_family matches spec.
    pub exec fn exec_mrwe5_record_kind_family(
        magic: u32,
        kind: u16,
    ) -> crate::mrwe5_contract::Mrwe5RecordKindFamilyDecision {
        let result = crate::mrwe5_contract::mrwe5_classify_record_kind_family(magic, kind);
        let spec_result = spec_mrwe5_record_kind_family(magic, kind);
        let prod_disc: int = match result {
            crate::mrwe5_contract::Mrwe5RecordKindFamilyDecision::Accepted => 1,
            crate::mrwe5_contract::Mrwe5RecordKindFamilyDecision::Rejected => 2,
        };
        assert(spec_result == prod_disc);
        result
    }

} // end cfg(verus) verus!{
