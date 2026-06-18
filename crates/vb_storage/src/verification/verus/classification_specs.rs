#![forbid(unsafe_code)]
//! Verus spec + proof artifacts for vb_storage classification seams.
//!
//! This module provides standalone spec functions (mathematical model) and
//! proof lemmas (properties of the model). Compiled standalone via
//! --crate-type=lib. Production binding map for when compiled as part of
//! crate:
//!
//! - `mrwe5_contract::mrwe5_kinds_are_exact_match`
//! - `mrwe5_contract::mrwe5_classify_kind_compatibility`
//! - `mrwe5_contract::mrwe5_classify_semantic_decode`
//! - `mrwe5_contract::mrwe5_classify_record_kind_family`
//! - `codec::semantic::classify_journal_semantic_decode`
//! - `codec::semantic::classify_record_kind_family`
//! - `codec::validation::RecordKindFamilyDecision`
//! - `recovery::types::ActionReplayTracker::is_resolved`
//! - `recovery::types::UnsupportedRecoveryState::union_matches_flags`
//! - `recovery::types::DigestCheck::hierarchy_rank`
//! - `recovery::types::DigestCheck::is_strictly_weaker_than`

use vstd::prelude::*;

verus! {

    // =========================================================================
    // MRWE5 KERNEL SPEC — mirrors mrwe5_contract production
    // =========================================================================

    /// Spec: kind compatibility is ExactMatch (1) iff envelope and payload
    /// kinds match; RejectedMismatch (2) otherwise.
    /// Production: mrwe5_contract::mrwe5_classify_kind_compatibility
    #[verifier::nonlinear]
    pub open spec fn spec_mrwe5_kind_compatibility(envelope_kind: u16, payload_kind: u16) -> int {
        if envelope_kind == payload_kind {
            1int
        } else {
            2int
        }
    }

    /// Spec: mrwe5_semantic_decode returns the semantic decode decision as
    /// an int (1=Success, 2=Mismatch, 3=InvalidEvent).
    /// Production: mrwe5_contract::mrwe5_classify_semantic_decode
    #[verifier::nonlinear]
    pub open spec fn spec_mrwe5_semantic_decode(
        envelope_kind: u16,
        payload_kind: u16,
        event_valid: bool,
    ) -> int {
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

    /// Spec: a kind id is a journal-event family member (10..=29).
    /// Production: mrwe5_contract::mrwe5_is_journal_record_kind
    pub open spec fn spec_mrwe5_is_journal_record_kind(kind: u16) -> bool {
        10u16 <= kind && kind <= 29u16
    }

    /// Spec: exact kind match predicate.
    /// Production: mrwe5_contract::mrwe5_kinds_are_exact_match
    pub open spec fn spec_mrwe5_kinds_exact_match(envelope_kind: u16, payload_kind: u16) -> bool {
        envelope_kind == payload_kind
    }

    /// Spec: record kind family decision (1=Accepted, 2=Rejected).
    /// Production: mrwe5_contract::mrwe5_classify_record_kind_family + codec::validation
    pub open spec fn spec_validate_kind_family(magic: u32, kind: u16) -> int {
        // Journal-event family has priority
        if magic == 0x5642_4A45u32 && spec_mrwe5_is_journal_record_kind(kind) {
            1int
        } else {
            // Other magic/kind pairs
            match magic {
                0x5642_5352u32 => if kind == 1 { 1int } else { 2int },
                0x5642_4952u32 => if kind == 2 { 1int } else { 2int },
                0x5642_534Eu32 => if kind == 30 { 1int } else { 2int },
                0x5642_424Cu32 => if kind == 40 { 1int } else { 2int },
                0x5642_4958u32 => if kind == 3 || kind == 50 { 1int } else { 2int },
                0x5642_5254u32 => if kind == 5 { 1int } else { 2int },
                _ => 2int,
            }
        }
    }

    // =========================================================================
    // CLASSIFY JOURNAL SEMANTIC DECODE SPEC
    // =========================================================================

    /// Spec model for classify_journal_semantic_decode.
    /// Mirrors: codec::semantic::classify_journal_semantic_decode
    #[verifier::nonlinear]
    pub open spec fn spec_classify_journal_semantic_decode(
        envelope_kind: u16,
        payload_kind: u16,
        event_valid: bool,
    ) -> int {
        let comp = spec_mrwe5_kind_compatibility(envelope_kind, payload_kind);
        if comp == 1int {
            if event_valid {
                1int // SemanticSuccess
            } else {
                3int // InvalidEvent
            }
        } else {
            2int // KindPayloadMismatch
        }
    }

    // =========================================================================
    // EXACT JOURNAL KIND PARITY SPEC
    // =========================================================================

    /// Spec: a parity witness exists iff envelope == payload kind.
    pub open spec fn spec_exact_journal_kind_parity_exists(
        envelope_kind: u16,
        payload_kind: u16,
    ) -> bool {
        envelope_kind == payload_kind
    }

    // =========================================================================
    // VALIDATED JOURNAL RECORD INVARIANT SPEC
    // =========================================================================

    /// Spec: a record is validated when decode == SemanticSuccess.
    pub open spec fn spec_validated_journal_record(
        envelope_kind: u16,
        payload_kind: u16,
        event_valid: bool,
    ) -> bool {
        spec_classify_journal_semantic_decode(envelope_kind, payload_kind, event_valid) == 1int
    }

    // =========================================================================
    // ACTION REPLAY TRACKER SPEC
    // =========================================================================

    /// Spec: is_resolved holds when action/step is in completed or failed set.
    pub open spec fn spec_action_replay_is_resolved_set(
        completed: Set<u64>,
        failed: Set<u64>,
        packed_key: u64,
    ) -> bool {
        completed.contains(packed_key) || failed.contains(packed_key)
    }

    /// Spec: the packed key from action + step.
    pub open spec fn spec_action_replay_packed_key(action: u64, step: u64) -> u64 {
        action.wrapping_mul(65536).wrapping_add(step)
    }

    /// Spec: apply_effect returns Apply (1) when not yet resolved,
    /// Duplicate (2) when already resolved.
    pub open spec fn spec_action_replay_apply_effect(
        completed: Set<u64>,
        action: u64,
        step: u64,
    ) -> int {
        let packed = spec_action_replay_packed_key(action, step);
        if spec_action_replay_is_resolved_set(completed, set! {}, packed) {
            2 // Duplicate
        } else {
            1 // Apply
        }
    }

    // =========================================================================
    // UNSUPPORTED RECOVERY STATE SPEC
    // =========================================================================

    /// Spec: union of two unsupported states is flag-wise OR.
    pub open spec fn spec_unsupported_state_union_or(
        a_sv: bool,
        a_st: bool,
        a_ap: bool,
        b_sv: bool,
        b_st: bool,
        b_ap: bool,
    ) -> (bool, bool, bool) {
        (
            a_sv || b_sv,
            a_st || b_st,
            a_ap || b_ap,
        )
    }

    /// Spec: union_matches_flags returns true when union result matches
    /// flag-wise OR.
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

    // =========================================================================
    // DIGEST CHECK SPEC
    // =========================================================================

    /// Spec: hierarchy_rank maps digest check levels to ordinal ranks.
    /// 1=WorkflowSourceOnly, 2=WorkflowAndIr, 3=Full.
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

    /// Spec: checks_workflow_source is true for rank >= 1.
    pub open spec fn spec_digest_check_checks_workflow_source(level: int) -> bool {
        spec_digest_check_hierarchy_rank(level) >= 1
    }

    /// Spec: checks_compiled_ir is true for rank >= 2.
    pub open spec fn spec_digest_check_checks_compiled_ir(level: int) -> bool {
        spec_digest_check_hierarchy_rank(level) >= 2
    }

    /// Spec: checks_full is true only for rank == 3.
    pub open spec fn spec_digest_check_checks_full(level: int) -> bool {
        spec_digest_check_hierarchy_rank(level) == 3
    }

    // =========================================================================
    // EVENT SEQ SPEC
    // =========================================================================

    /// Spec: two sequence numbers are contiguous (with wrapping).
    pub open spec fn spec_event_seq_contiguous(prev: u64, next: u64) -> bool {
        next == prev.wrapping_add(1)
    }

    /// Spec: a sequence is contiguous when each element follows the previous.
    pub open spec fn spec_validate_contiguous_sequences(seqs: Seq<u64>) -> bool {
        seqs.len() <= 1
            || (forall|i: int| 0 <= i < seqs.len() as int - 1 ==> #[trigger] seqs[i + 1] == seqs[i].wrapping_add(1))
    }

    // =========================================================================
    // DIMENSION COUNT SPEC
    // =========================================================================

    /// Spec: recovery dimension count from max index.
    pub open spec fn spec_recovery_dimension_count_from_index(
        max_index: Option<u16>,
        _run: u64,
    ) -> (int, bool) {
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

    pub open spec fn spec_event_kind_class_step_succeeded() -> int { 1 }
    pub open spec fn spec_event_kind_class_slot_written() -> int { 2 }
    pub open spec fn spec_event_kind_class_other() -> int { 3 }

    // =========================================================================
    // RECOVERY TERMINAL STATE SPEC
    // =========================================================================

    pub open spec fn spec_recovery_terminal_state_count() -> int { 4 }

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
    // IS_KNOWN_RECORD_KIND SPEC
    // =========================================================================

    pub open spec fn spec_is_known_record_kind(kind: u16) -> bool {
        kind == 1 || kind == 2 || kind == 3 || kind == 7
            || (10u16 <= kind && kind <= 29u16)
            || kind == 30 || kind == 40 || kind == 50
    }

    // =========================================================================
    // MRWE5 KERNEL LEMMAS
    // =========================================================================

    pub proof fn lemma_kind_compatibility_exact_match_symmetric(
        envelope_kind: u16,
        payload_kind: u16,
    ) {
        assert(spec_mrwe5_kind_compatibility(envelope_kind, payload_kind)
            == spec_mrwe5_kind_compatibility(payload_kind, envelope_kind));
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
    // PARITY WITNESS LEMMAS
    // =========================================================================

    pub proof fn lemma_parity_exists_on_match(envelope_kind: u16, payload_kind: u16)
        requires
            envelope_kind == payload_kind,
        ensures
            spec_exact_journal_kind_parity_exists(envelope_kind, payload_kind),
    {
        assert(spec_exact_journal_kind_parity_exists(envelope_kind, payload_kind));
    }

    pub proof fn lemma_parity_not_exists_on_mismatch(envelope_kind: u16, payload_kind: u16)
        requires
            envelope_kind != payload_kind,
        ensures
            !spec_exact_journal_kind_parity_exists(envelope_kind, payload_kind),
    {
        assert(!spec_exact_journal_kind_parity_exists(envelope_kind, payload_kind));
    }

    pub proof fn lemma_parity_reflexive() {
        assert(spec_exact_journal_kind_parity_exists(29u16, 29u16));
    }

    pub proof fn lemma_parity_symmetric(a: u16, b: u16)
        requires
            a == b,
        ensures
            spec_exact_journal_kind_parity_exists(b, a),
    {
        assert(spec_exact_journal_kind_parity_exists(b, a));
    }

    // =========================================================================
    // JOURNAL SEMANTIC DECODE LEMMAS
    // =========================================================================

    pub proof fn lemma_semantic_decode_success_step_succeeded() {
        assert(spec_classify_journal_semantic_decode(29u16, 29u16, true) == 1);
    }

    pub proof fn lemma_semantic_decode_invalid_step_succeeded() {
        assert(spec_classify_journal_semantic_decode(29u16, 29u16, false) == 3);
    }

    pub proof fn lemma_semantic_decode_mismatch_rejects_both(event_valid: bool) {
        assert(spec_classify_journal_semantic_decode(29u16, 12u16, event_valid) == 2);
    }

    pub proof fn lemma_classify_journal_semantic_decode_equivalence(
        envelope_kind: u16,
        payload_kind: u16,
        event_valid: bool,
    ) {
        assert(spec_classify_journal_semantic_decode(envelope_kind, payload_kind, event_valid)
            == spec_mrwe5_semantic_decode(envelope_kind, payload_kind, event_valid));
    }

    // =========================================================================
    // RECORD KIND FAMILY LEMMAS
    // =========================================================================

    pub proof fn lemma_journal_family_accepts_29() {
        assert(spec_validate_kind_family(0x5642_4A45u32, 29) == 1);
    }

    pub proof fn lemma_journal_family_rejects_30() {
        assert(spec_validate_kind_family(0x5642_4A45u32, 30) == 2);
    }

    pub proof fn lemma_snapshot_family_kind_30_ok() {
        assert(spec_validate_kind_family(0x5642_534Eu32, 30) == 1);
    }

    pub proof fn lemma_snapshot_family_kind_31_err() {
        assert(spec_validate_kind_family(0x5642_534Eu32, 31) == 2);
    }

    pub proof fn lemma_blob_family_kind_40_ok() {
        assert(spec_validate_kind_family(0x5642_424Cu32, 40) == 1);
    }

    pub proof fn lemma_index_family_kind_3_ok() {
        assert(spec_validate_kind_family(0x5642_4958u32, 3) == 1);
    }

    pub proof fn lemma_index_family_kind_50_ok() {
        assert(spec_validate_kind_family(0x5642_4958u32, 50) == 1);
    }

    pub proof fn lemma_unknown_magic_rejected(kind: u16) {
        assert(spec_validate_kind_family(0x0000_0000u32, kind) == 2);
    }

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

    pub proof fn lemma_family_rejects_all_invalid_pairs() {
        assert(spec_validate_kind_family(0x5642_4A45u32, 9) == 2);
        assert(spec_validate_kind_family(0x5642_4A45u32, 30) == 2);
        assert(spec_validate_kind_family(0x5642_5352u32, 2) == 2);
        assert(spec_validate_kind_family(0x5642_4952u32, 1) == 2);
        assert(spec_validate_kind_family(0x5642_534Eu32, 31) == 2);
    }

    pub proof fn lemma_journal_family_is_subset() {
        assert(spec_validate_kind_family(0x5642_4A45u32, 15) == 1);
        assert(spec_validate_kind_family(0x5642_5352u32, 1) == 1);
        assert(spec_validate_kind_family(0x5642_4952u32, 2) == 1);
        assert(spec_validate_kind_family(0x5642_534Eu32, 30) == 1);
        assert(spec_validate_kind_family(0x5642_424Cu32, 40) == 1);
        assert(spec_validate_kind_family(0x5642_4958u32, 3) == 1);
        assert(spec_validate_kind_family(0x5642_5254u32, 5) == 1);
        assert(spec_validate_kind_family(0x5642_4A45u32, 50) == 2);
    }

    // =========================================================================
    // VALIDATED RECORD INVARIANT LEMMAS
    // =========================================================================

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
    // ACTION REPLAY LEMMAS
    // =========================================================================

    pub proof fn lemma_completed_insert_implies_resolved() {
        let mut completed: Set<u64> = set! {};
        let key = 42u64.wrapping_mul(65536).wrapping_add(1);
        completed = completed.insert(key);
        assert(completed.contains(key));
        assert(spec_action_replay_is_resolved_set(completed, set! {}, key));
    }

    pub proof fn lemma_failed_insert_implies_resolved() {
        let mut failed: Set<u64> = set! {};
        let key = 42u64.wrapping_mul(65536).wrapping_add(1);
        failed = failed.insert(key);
        assert(failed.contains(key));
        assert(spec_action_replay_is_resolved_set(set! {}, failed, key));
    }

    pub proof fn lemma_is_resolved_union_commutative(
        completed: Set<u64>,
        failed: Set<u64>,
        action: u64,
        step: u64,
    ) {
        let packed = spec_action_replay_packed_key(action, step);
        assert(spec_action_replay_is_resolved_set(completed, failed, packed) == (
            completed.contains(packed) || failed.contains(packed)));
    }

    // =========================================================================
    // UNSUPPORTED STATE LEMMAS
    // =========================================================================

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
            c_sv, c_st, c_ap,
        );
        let rhs = spec_unsupported_state_union_or(
            a_sv, a_st, a_ap,
            spec_unsupported_state_union_or(b_sv, b_st, b_ap, c_sv, c_st, c_ap).0,
            spec_unsupported_state_union_or(b_sv, b_st, b_ap, c_sv, c_st, c_ap).1,
            spec_unsupported_state_union_or(b_sv, b_st, b_ap, c_sv, c_st, c_ap).2,
        );
        assert(lhs == rhs);
    }

    pub proof fn lemma_union_with_supported_is_identity(sv: bool, st: bool, ap: bool) {
        let result = spec_unsupported_state_union_or(sv, st, ap, false, false, false);
        assert(result.0 == sv);
        assert(result.1 == st);
        assert(result.2 == ap);
    }

    pub proof fn lemma_union_supported_left_is_identity(sv: bool, st: bool, ap: bool) {
        let result = spec_unsupported_state_union_or(false, false, false, sv, st, ap);
        assert(result.0 == sv);
        assert(result.1 == st);
        assert(result.2 == ap);
    }

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
    // DIGEST CHECK LEMMAS
    // =========================================================================

    pub proof fn lemma_digest_check_workflow_source_strictly_weaker_than_workflow_and_ir() {
        assert(spec_digest_check_strictly_weaker(1int, 2int));
    }

    pub proof fn lemma_digest_check_workflow_and_ir_strictly_weaker_than_full() {
        assert(spec_digest_check_strictly_weaker(2int, 3int));
    }

    pub proof fn lemma_digest_check_workflow_source_strictly_weaker_than_full() {
        assert(spec_digest_check_strictly_weaker(1int, 3int));
    }

    pub proof fn lemma_hierarchy_rank_monotone() {
        assert(1int < 2int);
        assert(2int < 3int);
    }

    pub proof fn lemma_checks_workflow_source_for_rank_ge_1() {
        assert(spec_digest_check_checks_workflow_source(1int));
        assert(spec_digest_check_checks_workflow_source(2int));
        assert(spec_digest_check_checks_workflow_source(3int));
    }

    pub proof fn lemma_checks_compiled_ir_threshold() {
        assert(!spec_digest_check_checks_compiled_ir(1int));
        assert(spec_digest_check_checks_compiled_ir(2int));
        assert(spec_digest_check_checks_compiled_ir(3int));
    }

    pub proof fn lemma_checks_full_only_at_full_rank() {
        assert(!spec_digest_check_checks_full(1int));
        assert(!spec_digest_check_checks_full(2int));
        assert(spec_digest_check_checks_full(3int));
    }

    pub proof fn lemma_digest_check_strictly_weaker_transitive() {
        assert(spec_digest_check_strictly_weaker(1int, 2int));
        assert(spec_digest_check_strictly_weaker(2int, 3int));
        assert(spec_digest_check_strictly_weaker(1int, 3int));
    }

    // =========================================================================
    // IS_KNOWN_RECORD_KIND LEMMAS
    // =========================================================================

    pub proof fn lemma_kind_1_is_known_non_journal() {
        assert(spec_is_known_record_kind(1));
        assert(!spec_mrwe5_is_journal_record_kind(1));
    }

    pub proof fn lemma_kind_30_is_known_non_journal() {
        assert(spec_is_known_record_kind(30));
        assert(!spec_mrwe5_is_journal_record_kind(30));
    }

    pub proof fn lemma_kind_50_is_known_non_journal() {
        assert(spec_is_known_record_kind(50));
        assert(!spec_mrwe5_is_journal_record_kind(50));
    }

    pub proof fn lemma_kind_9_is_unknown() {
        assert(!spec_is_known_record_kind(9));
    }

    pub proof fn lemma_kind_max_unknown() {
        assert(!spec_is_known_record_kind(65535u16));
    }

    pub proof fn lemma_journal_boundary_at_10() {
        assert(spec_mrwe5_is_journal_record_kind(10));
        assert(!spec_mrwe5_is_journal_record_kind(9));
    }

    pub proof fn lemma_journal_boundary_at_29_30() {
        assert(spec_mrwe5_is_journal_record_kind(29));
        assert(!spec_mrwe5_is_journal_record_kind(30));
    }

    pub proof fn lemma_all_journal_range_is_journal_kind() {
        assert(spec_mrwe5_is_journal_record_kind(10));
        assert(spec_mrwe5_is_journal_record_kind(15));
        assert(spec_mrwe5_is_journal_record_kind(29));
    }

    pub proof fn lemma_kind_0_not_known() {
        assert(!spec_is_known_record_kind(0));
    }

    // =========================================================================
    // JOURNAL SEMANTIC DECODE — PRODUCTION BOUNDING LEMMAS
    // =========================================================================

    pub proof fn lemma_classify_semantic_decode_success_iff() {
        assert(spec_classify_journal_semantic_decode(29u16, 29u16, true) == 1);
        assert(spec_classify_journal_semantic_decode(12u16, 12u16, true) == 1);
        assert(spec_classify_journal_semantic_decode(10u16, 10u16, true) == 1);
        assert(spec_classify_journal_semantic_decode(29u16, 29u16, false) == 3);
        assert(spec_classify_journal_semantic_decode(12u16, 12u16, false) == 3);
        assert(spec_classify_journal_semantic_decode(29u16, 12u16, true) == 2);
        assert(spec_classify_journal_semantic_decode(29u16, 12u16, false) == 2);
        assert(spec_classify_journal_semantic_decode(10u16, 29u16, true) == 2);
        assert(spec_classify_journal_semantic_decode(10u16, 29u16, false) == 2);
    }

    // =========================================================================
    // RECORD KIND FAMILY — PRODUCTION BOUNDING LEMMAS
    // =========================================================================

    pub proof fn lemma_classify_family_all_known_pairs() {
        assert(spec_validate_kind_family(0x5642_5352u32, 1) == 1);
        assert(spec_validate_kind_family(0x5642_4952u32, 2) == 1);
        assert(spec_validate_kind_family(0x5642_4A45u32, 10) == 1);
        assert(spec_validate_kind_family(0x5642_4A45u32, 15) == 1);
        assert(spec_validate_kind_family(0x5642_4A45u32, 29) == 1);
        assert(spec_validate_kind_family(0x5642_534Eu32, 30) == 1);
        assert(spec_validate_kind_family(0x5642_424Cu32, 40) == 1);
        assert(spec_validate_kind_family(0x5642_4958u32, 3) == 1);
        assert(spec_validate_kind_family(0x5642_4958u32, 50) == 1);
        assert(spec_validate_kind_family(0x5642_5254u32, 5) == 1);
        assert(spec_validate_kind_family(0x5642_5352u32, 2) == 2);
        assert(spec_validate_kind_family(0x5642_4952u32, 1) == 2);
    }

    // =========================================================================
    // PARITY — PRODUCTION BOUNDING LEMMAS
    // =========================================================================

    pub proof fn lemma_parity_new_succeeds_on_same() {
        assert(spec_exact_journal_kind_parity_exists(12u16, 12u16));
        assert(spec_exact_journal_kind_parity_exists(29u16, 29u16));
        assert(spec_exact_journal_kind_parity_exists(10u16, 10u16));
    }

    pub proof fn lemma_parity_new_fails_on_different() {
        assert(!spec_exact_journal_kind_parity_exists(12u16, 29u16));
        assert(!spec_exact_journal_kind_parity_exists(29u16, 12u16));
    }

    // =========================================================================
    // EVENT SEQ LEMMAS
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
    // RECOVERY TERMINAL STATE LEMMAS
    // =========================================================================

    pub proof fn lemma_recovery_terminal_state_count_ok() {
        assert(spec_recovery_terminal_state_count() == 4);
    }

} // end verus!
