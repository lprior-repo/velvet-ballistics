#![cfg(all(kani, feature = "kani-vb-god2f-hard-verus"))]
#![forbid(unsafe_code)]

//! HVR-PO-STORAGE-{002,005}: production storage classification/recovery harnesses.

use crate::codec::{
    JournalSemanticDecodeDecision, RecordKindFamilyDecision, classify_journal_semantic_decode,
    classify_record_kind_family,
};
use crate::constants::MAGIC_JOURNAL_EVENT;
use crate::mrwe5_contract::{
    MRWE5_JOURNAL_MAX_KIND_ID, MRWE5_JOURNAL_MIN_KIND_ID, Mrwe5RecordKindFamilyDecision,
    mrwe5_classify_record_kind_family,
};
use crate::recovery::types::{
    ActionReplayEffect, ActionReplayTracker, DigestCheck, RecoveryError, UnsupportedRecoveryState,
};
use vb_core::{ActionId, ActionTicket, MockMarker, SeqNo, SlotIdx, StepIdx, Taint};

fn digest_check_from_symbol(symbol: u8) -> DigestCheck {
    match symbol % 3 {
        0 => DigestCheck::WorkflowSourceOnly,
        1 => DigestCheck::WorkflowAndIr,
        _ => DigestCheck::Full,
    }
}

fn unsupported_from_symbols(
    slot_values: bool,
    slot_taint: bool,
    action_payloads: bool,
) -> UnsupportedRecoveryState {
    UnsupportedRecoveryState {
        slot_values,
        slot_taint,
        action_payloads,
    }
}

fn ticket(action_raw: u16, step_raw: u16, seq_raw: u64) -> ActionTicket {
    ActionTicket {
        run: vb_core::RunId::new(1),
        step: StepIdx::new(step_raw),
        seq: SeqNo::new(seq_raw),
        action: ActionId::new(action_raw),
        attempt: 1,
        idempotency_key: 0xA5A5,
        capacity: 1,
        mock: MockMarker::HttpGet,
    }
}

fn nonzero_slot_from_symbol(symbol: u8) -> SlotIdx {
    match symbol % 4 {
        0 => SlotIdx::new(1),
        1 => SlotIdx::new(2),
        2 => SlotIdx::new(3),
        _ => SlotIdx::new(4),
    }
}

fn replay_effect_is_blocked(result: Result<ActionReplayEffect, RecoveryError>) -> bool {
    matches!(
        result,
        Err(RecoveryError::NonIdempotentActionBlocked { .. })
    )
}

fn replay_effect_is_divergence(result: Result<ActionReplayEffect, RecoveryError>) -> bool {
    matches!(result, Err(RecoveryError::ReplayDivergence { .. }))
}

fn replay_unit_is_divergence(result: Result<(), RecoveryError>) -> bool {
    matches!(result, Err(RecoveryError::ReplayDivergence { .. }))
}

#[kani::proof]
#[kani::unwind(8)]
fn vb_god2f_storage_kind_family_and_semantic_decode() {
    let magic: u32 = kani::any();
    let kind: u16 = kani::any();
    let payload_kind: u16 = kani::any();
    let event_valid: bool = kani::any();

    let family = classify_record_kind_family(magic, kind);
    let mrwe5_family = mrwe5_classify_record_kind_family(magic, kind);
    kani::cover!(
        magic == MAGIC_JOURNAL_EVENT
            && kind >= MRWE5_JOURNAL_MIN_KIND_ID
            && kind <= MRWE5_JOURNAL_MAX_KIND_ID,
        "journal family accepted branch covered"
    );
    kani::cover!(
        magic != MAGIC_JOURNAL_EVENT,
        "non-journal magic branch covered"
    );
    if matches!(mrwe5_family, Mrwe5RecordKindFamilyDecision::Accepted) {
        kani::assert(
            family == RecordKindFamilyDecision::Accepted,
            "codec accepts MRWE5 journal-family kind",
        );
    }
    if magic == MAGIC_JOURNAL_EVENT
        && (kind < MRWE5_JOURNAL_MIN_KIND_ID || kind > MRWE5_JOURNAL_MAX_KIND_ID)
    {
        kani::assert(
            family == RecordKindFamilyDecision::Rejected,
            "codec rejects out-of-family journal kind",
        );
    }

    let semantic = classify_journal_semantic_decode(kind, payload_kind, event_valid);
    kani::cover!(
        kind == payload_kind && event_valid,
        "semantic success branch covered"
    );
    kani::cover!(
        kind == payload_kind && !event_valid,
        "invalid event branch covered"
    );
    kani::cover!(kind != payload_kind, "kind mismatch branch covered");
    if kind != payload_kind {
        kani::assert(
            semantic == JournalSemanticDecodeDecision::KindPayloadMismatch,
            "kind mismatch is classified before event validity",
        );
    } else if event_valid {
        kani::assert(
            semantic == JournalSemanticDecodeDecision::SemanticSuccess,
            "matching valid event is semantic success",
        );
    } else {
        kani::assert(
            semantic == JournalSemanticDecodeDecision::InvalidEvent,
            "matching invalid event is InvalidEvent",
        );
    }
}

#[kani::proof]
#[kani::unwind(10)]
fn vb_god2f_recovery_digest_and_action_replay_state() {
    let digest = digest_check_from_symbol(kani::any());
    kani::cover!(
        digest == DigestCheck::WorkflowSourceOnly,
        "digest domain covers WorkflowSourceOnly"
    );
    kani::cover!(
        digest == DigestCheck::WorkflowAndIr,
        "digest domain covers WorkflowAndIr"
    );
    kani::cover!(digest == DigestCheck::Full, "digest domain covers Full");
    kani::assert(
        digest.checks_workflow_source(),
        "all digest levels check workflow source",
    );
    kani::assert(
        digest.checks_compiled_ir()
            == (digest.hierarchy_rank() >= DigestCheck::WorkflowAndIr.hierarchy_rank()),
        "compiled-IR check follows digest hierarchy",
    );
    kani::assert(
        digest.checks_full() == (digest == DigestCheck::Full),
        "full check is exact to Full level",
    );

    let left = unsupported_from_symbols(kani::any(), kani::any(), kani::any());
    let right = unsupported_from_symbols(kani::any(), kani::any(), kani::any());
    let union = left.union(right);
    kani::cover!(
        left.is_fully_supported(),
        "fully supported left branch covered"
    );
    kani::cover!(
        !right.is_fully_supported(),
        "unsupported right branch covered"
    );
    kani::assert(
        left.union_matches_flags(right, union),
        "unsupported-state union matches flag-wise OR",
    );
    kani::assert(
        union.is_fully_supported()
            == !(union.slot_values || union.slot_taint || union.action_payloads),
        "fully-supported predicate matches unsupported flags",
    );

    let first = ticket(kani::any(), kani::any(), kani::any());
    let mut tracker = ActionReplayTracker::new();
    let scheduled = tracker.mark_scheduled_ticket_effect(first, SlotIdx::ZERO, SlotIdx::ZERO);
    kani::assert(
        matches!(scheduled, Ok(ActionReplayEffect::Apply)),
        "first scheduled ticket applies",
    );
    let duplicate = tracker.mark_scheduled_ticket_effect(first, SlotIdx::ZERO, SlotIdx::ZERO);
    kani::assert(
        matches!(duplicate, Ok(ActionReplayEffect::Duplicate)),
        "identical scheduled ticket is duplicate",
    );

    let mut completed_tracker = ActionReplayTracker::new();
    let scheduled_for_completion =
        completed_tracker.mark_scheduled_ticket_effect(first, SlotIdx::ZERO, SlotIdx::ZERO);
    kani::assert(
        matches!(scheduled_for_completion, Ok(ActionReplayEffect::Apply)),
        "completion tracker schedules first ticket",
    );
    let completion = completed_tracker.mark_completed_envelope_effect(
        first,
        SlotIdx::ZERO,
        1,
        Taint::Clean,
        [0; 32],
    );
    kani::assert(
        matches!(completion, Ok(ActionReplayEffect::Apply)),
        "first completion envelope applies",
    );
    let duplicate_completion = completed_tracker.mark_completed_envelope_effect(
        first,
        SlotIdx::ZERO,
        1,
        Taint::Clean,
        [0; 32],
    );
    kani::assert(
        matches!(duplicate_completion, Ok(ActionReplayEffect::Duplicate)),
        "identical completion envelope is duplicate",
    );
    let divergent_completion = completed_tracker.mark_completed_envelope_effect(
        first,
        SlotIdx::ZERO,
        2,
        Taint::Clean,
        [0; 32],
    );
    kani::assert(
        matches!(
            divergent_completion,
            Err(RecoveryError::ReplayDivergence { .. })
        ),
        "divergent completion envelope is rejected",
    );

    let mut completed_resolution_tracker = ActionReplayTracker::new();
    completed_resolution_tracker.mark_completed(first.action, first.step);
    let scheduled_after_completed = completed_resolution_tracker.mark_scheduled_ticket_effect(
        first,
        SlotIdx::ZERO,
        SlotIdx::ZERO,
    );
    let scheduled_after_completed_blocked = replay_effect_is_blocked(scheduled_after_completed);
    kani::cover!(
        scheduled_after_completed_blocked,
        "completed action blocks scheduled replay branch covered"
    );
    kani::assert(
        scheduled_after_completed_blocked,
        "pre-completed action blocks non-idempotent scheduled replay",
    );

    let mut failed_resolution_tracker = ActionReplayTracker::new();
    failed_resolution_tracker.mark_failed(first.action, first.step);
    let scheduled_after_failed =
        failed_resolution_tracker.mark_scheduled_ticket_effect(first, SlotIdx::ZERO, SlotIdx::ZERO);
    let scheduled_after_failed_blocked = replay_effect_is_blocked(scheduled_after_failed);
    kani::cover!(
        scheduled_after_failed_blocked,
        "failed action blocks scheduled replay branch covered"
    );
    kani::assert(
        scheduled_after_failed_blocked,
        "pre-failed action blocks non-idempotent scheduled replay",
    );

    let mut completed_envelope_block_tracker = ActionReplayTracker::new();
    completed_envelope_block_tracker.mark_completed(first.action, first.step);
    let completion_after_completed = completed_envelope_block_tracker
        .mark_completed_envelope_effect(first, SlotIdx::ZERO, 1, Taint::Clean, [0; 32]);
    let completion_after_completed_blocked = replay_effect_is_blocked(completion_after_completed);
    kani::cover!(
        completion_after_completed_blocked,
        "completed action blocks completion replay branch covered"
    );
    kani::assert(
        completion_after_completed_blocked,
        "pre-completed action blocks non-idempotent completion replay",
    );

    let mut failed_envelope_block_tracker = ActionReplayTracker::new();
    failed_envelope_block_tracker.mark_failed(first.action, first.step);
    let completion_after_failed = failed_envelope_block_tracker.mark_completed_envelope_effect(
        first,
        SlotIdx::ZERO,
        1,
        Taint::Clean,
        [0; 32],
    );
    let completion_after_failed_blocked = replay_effect_is_blocked(completion_after_failed);
    kani::cover!(
        completion_after_failed_blocked,
        "failed action blocks completion replay branch covered"
    );
    kani::assert(
        completion_after_failed_blocked,
        "pre-failed action blocks non-idempotent completion replay",
    );

    let missing_schedule_tracker = ActionReplayTracker::new();
    let missing_schedule = missing_schedule_tracker.require_scheduled_ticket(first, SlotIdx::ZERO);
    let missing_schedule_rejected = replay_unit_is_divergence(missing_schedule);
    kani::cover!(
        missing_schedule_rejected,
        "missing schedule ticket rejection branch covered"
    );
    kani::assert(
        missing_schedule_rejected,
        "completion without schedule is ReplayDivergence",
    );

    let alternate_output = nonzero_slot_from_symbol(kani::any());
    let mut mismatched_schedule_tracker = ActionReplayTracker::new();
    let scheduled_for_mismatch = mismatched_schedule_tracker.mark_scheduled_ticket_effect(
        first,
        SlotIdx::ZERO,
        SlotIdx::ZERO,
    );
    kani::assert(
        matches!(scheduled_for_mismatch, Ok(ActionReplayEffect::Apply)),
        "mismatch tracker schedules first ticket",
    );
    let mismatched_required_schedule =
        mismatched_schedule_tracker.require_scheduled_ticket(first, alternate_output);
    let mismatched_required_schedule_rejected =
        replay_unit_is_divergence(mismatched_required_schedule);
    kani::cover!(
        mismatched_required_schedule_rejected,
        "mismatched required schedule rejection branch covered"
    );
    kani::assert(
        mismatched_required_schedule_rejected,
        "mismatched required schedule is ReplayDivergence",
    );

    let mismatched_completion = mismatched_schedule_tracker.mark_completed_envelope_effect(
        first,
        alternate_output,
        1,
        Taint::Clean,
        [0; 32],
    );
    let mismatched_completion_rejected = replay_effect_is_divergence(mismatched_completion);
    kani::cover!(
        mismatched_completion_rejected,
        "mismatched completion schedule rejection branch covered"
    );
    kani::assert(
        mismatched_completion_rejected,
        "completion envelope with mismatched schedule is ReplayDivergence",
    );
}
