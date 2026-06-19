#![forbid(unsafe_code)]

use crate::kani_vb_god2f_classification_recovery::replay_model::{
    ReplayScenario, digest_check_from_symbol, generated_nonzero_slot, generated_ticket,
    replay_effect_is_apply, replay_effect_is_blocked, replay_effect_is_divergence,
    replay_effect_is_duplicate, replay_effect_kind, replay_scenario_from_symbol,
    replay_unit_is_divergence, replay_unit_kind, unsupported_from_symbols,
};
use crate::recovery::types::{ActionReplayTracker, DigestCheck};
use vb_core::{SlotIdx, Taint};

#[kani::proof]
#[kani::unwind(40)]
fn vb_god2f_recovery_digest_and_action_replay_state() {
    let digest_symbol: u8 = kani::any();
    kani::assume(digest_symbol <= 2);
    let digest = digest_check_from_symbol(digest_symbol);
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

    let first = generated_ticket();
    let mut tracker = ActionReplayTracker::new();
    let scenario_symbol: u8 = kani::any();
    kani::assume(scenario_symbol <= 8);
    let scenario = replay_scenario_from_symbol(scenario_symbol);
    let alternate_output = generated_nonzero_slot();

    match scenario {
        ReplayScenario::ScheduleDuplicate => {
            let scheduled = replay_effect_kind(tracker.mark_scheduled_ticket_effect(
                first,
                SlotIdx::ZERO,
                SlotIdx::ZERO,
            ));
            let duplicate = replay_effect_kind(tracker.mark_scheduled_ticket_effect(
                first,
                SlotIdx::ZERO,
                SlotIdx::ZERO,
            ));
            kani::cover!(
                replay_effect_is_apply(scheduled) && replay_effect_is_duplicate(duplicate),
                "scheduled action applies once then duplicates"
            );
            kani::assert(
                replay_effect_is_apply(scheduled),
                "first scheduled ticket applies",
            );
            kani::assert(
                replay_effect_is_duplicate(duplicate),
                "identical scheduled ticket is duplicate",
            );
        }
        ReplayScenario::CompletionDuplicateAndDivergent => {
            let completion = replay_effect_kind(tracker.mark_completed_envelope_effect(
                first,
                SlotIdx::ZERO,
                1,
                Taint::Clean,
                [0; 32],
            ));
            let duplicate_completion = replay_effect_kind(tracker.mark_completed_envelope_effect(
                first,
                SlotIdx::ZERO,
                1,
                Taint::Clean,
                [0; 32],
            ));
            let divergent_completion = replay_effect_kind(tracker.mark_completed_envelope_effect(
                first,
                SlotIdx::ZERO,
                2,
                Taint::Clean,
                [0; 32],
            ));
            kani::cover!(
                replay_effect_is_apply(completion)
                    && replay_effect_is_duplicate(duplicate_completion)
                    && replay_effect_is_divergence(divergent_completion),
                "completion apply duplicate and divergence branches covered"
            );
            kani::assert(
                replay_effect_is_apply(completion),
                "first completion envelope applies",
            );
            kani::assert(
                replay_effect_is_duplicate(duplicate_completion),
                "identical completion envelope is duplicate",
            );
            kani::assert(
                replay_effect_is_divergence(divergent_completion),
                "divergent completion envelope is rejected",
            );
        }
        ReplayScenario::ScheduleAfterCompleted => {
            tracker.mark_completed(first.action, first.step);
            let scheduled = replay_effect_kind(tracker.mark_scheduled_ticket_effect(
                first,
                SlotIdx::ZERO,
                SlotIdx::ZERO,
            ));
            kani::cover!(
                replay_effect_is_blocked(scheduled),
                "completed action blocks scheduled replay branch covered"
            );
            kani::assert(
                replay_effect_is_blocked(scheduled),
                "pre-completed action blocks non-idempotent scheduled replay",
            );
        }
        ReplayScenario::ScheduleAfterFailed => {
            tracker.mark_failed(first.action, first.step);
            let scheduled = replay_effect_kind(tracker.mark_scheduled_ticket_effect(
                first,
                SlotIdx::ZERO,
                SlotIdx::ZERO,
            ));
            kani::cover!(
                replay_effect_is_blocked(scheduled),
                "failed action blocks scheduled replay branch covered"
            );
            kani::assert(
                replay_effect_is_blocked(scheduled),
                "pre-failed action blocks non-idempotent scheduled replay",
            );
        }
        ReplayScenario::CompletionAfterCompleted => {
            tracker.mark_completed(first.action, first.step);
            let completion = replay_effect_kind(tracker.mark_completed_envelope_effect(
                first,
                SlotIdx::ZERO,
                1,
                Taint::Clean,
                [0; 32],
            ));
            kani::cover!(
                replay_effect_is_blocked(completion),
                "completed action blocks completion replay branch covered"
            );
            kani::assert(
                replay_effect_is_blocked(completion),
                "pre-completed action blocks non-idempotent completion replay",
            );
        }
        ReplayScenario::CompletionAfterFailed => {
            tracker.mark_failed(first.action, first.step);
            let completion = replay_effect_kind(tracker.mark_completed_envelope_effect(
                first,
                SlotIdx::ZERO,
                1,
                Taint::Clean,
                [0; 32],
            ));
            kani::cover!(
                replay_effect_is_blocked(completion),
                "failed action blocks completion replay branch covered"
            );
            kani::assert(
                replay_effect_is_blocked(completion),
                "pre-failed action blocks non-idempotent completion replay",
            );
        }
        ReplayScenario::MissingSchedule => {
            let missing_schedule =
                replay_unit_kind(tracker.require_scheduled_ticket(first, SlotIdx::ZERO));
            kani::cover!(
                replay_unit_is_divergence(missing_schedule),
                "missing schedule ticket rejection branch covered"
            );
            kani::assert(
                replay_unit_is_divergence(missing_schedule),
                "completion without schedule is ReplayDivergence",
            );
        }
        ReplayScenario::MismatchedRequiredSchedule => {
            let scheduled = replay_effect_kind(tracker.mark_scheduled_ticket_effect(
                first,
                SlotIdx::ZERO,
                SlotIdx::ZERO,
            ));
            let mismatched_required =
                replay_unit_kind(tracker.require_scheduled_ticket(first, alternate_output));
            kani::cover!(
                replay_effect_is_apply(scheduled) && replay_unit_is_divergence(mismatched_required),
                "mismatched required schedule rejection branch covered"
            );
            kani::assert(
                replay_effect_is_apply(scheduled),
                "mismatch tracker schedules first ticket",
            );
            kani::assert(
                replay_unit_is_divergence(mismatched_required),
                "mismatched required schedule is ReplayDivergence",
            );
        }
        ReplayScenario::MismatchedCompletionSchedule => {
            let scheduled = replay_effect_kind(tracker.mark_scheduled_ticket_effect(
                first,
                SlotIdx::ZERO,
                SlotIdx::ZERO,
            ));
            let mismatched_completion = replay_effect_kind(tracker.mark_completed_envelope_effect(
                first,
                alternate_output,
                1,
                Taint::Clean,
                [0; 32],
            ));
            kani::cover!(
                replay_effect_is_apply(scheduled)
                    && replay_effect_is_divergence(mismatched_completion),
                "mismatched completion schedule rejection branch covered"
            );
            kani::assert(
                replay_effect_is_apply(scheduled),
                "mismatched completion tracker schedules first ticket",
            );
            kani::assert(
                replay_effect_is_divergence(mismatched_completion),
                "completion envelope with mismatched schedule is ReplayDivergence",
            );
        }
    }
    std::mem::forget(tracker);
}
