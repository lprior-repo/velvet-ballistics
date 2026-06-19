#![forbid(unsafe_code)]

use crate::recovery::types::{
    ActionReplayEffect, DigestCheck, RecoveryError, UnsupportedRecoveryState,
};
use vb_core::{ActionId, ActionTicket, MockMarker, SeqNo, SlotIdx, StepIdx};

#[derive(Clone, Copy)]
pub(crate) enum ReplayEffectKind {
    Apply,
    Duplicate,
    Blocked,
    Divergence,
    OtherErr,
}

#[derive(Clone, Copy)]
pub(crate) enum ReplayUnitKind {
    Ok,
    Divergence,
    OtherErr,
}

#[derive(Clone, Copy)]
pub(crate) enum ReplayScenario {
    ScheduleDuplicate,
    CompletionDuplicateAndDivergent,
    ScheduleAfterCompleted,
    ScheduleAfterFailed,
    CompletionAfterCompleted,
    CompletionAfterFailed,
    MissingSchedule,
    MismatchedRequiredSchedule,
    MismatchedCompletionSchedule,
}

pub(crate) fn digest_check_from_symbol(symbol: u8) -> DigestCheck {
    match symbol {
        0 => DigestCheck::WorkflowSourceOnly,
        1 => DigestCheck::WorkflowAndIr,
        _ => DigestCheck::Full,
    }
}

pub(crate) fn unsupported_from_symbols(
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

pub(crate) fn generated_ticket() -> ActionTicket {
    let action_is_two: bool = kani::any();
    let step_is_one: bool = kani::any();
    let seq_is_one: bool = kani::any();
    let action_raw = if action_is_two { 2 } else { 1 };
    let step_raw = if step_is_one { 1 } else { 0 };
    let seq_raw = if seq_is_one { 1 } else { 0 };
    ticket(action_raw, step_raw, seq_raw)
}

pub(crate) fn generated_nonzero_slot() -> SlotIdx {
    let use_second_slot: bool = kani::any();
    if use_second_slot {
        SlotIdx::new(2)
    } else {
        SlotIdx::new(1)
    }
}

pub(crate) fn replay_scenario_from_symbol(symbol: u8) -> ReplayScenario {
    match symbol {
        0 => ReplayScenario::ScheduleDuplicate,
        1 => ReplayScenario::CompletionDuplicateAndDivergent,
        2 => ReplayScenario::ScheduleAfterCompleted,
        3 => ReplayScenario::ScheduleAfterFailed,
        4 => ReplayScenario::CompletionAfterCompleted,
        5 => ReplayScenario::CompletionAfterFailed,
        6 => ReplayScenario::MissingSchedule,
        7 => ReplayScenario::MismatchedRequiredSchedule,
        _ => ReplayScenario::MismatchedCompletionSchedule,
    }
}

pub(crate) fn replay_effect_kind(
    result: Result<ActionReplayEffect, RecoveryError>,
) -> ReplayEffectKind {
    let kind = match &result {
        Ok(ActionReplayEffect::Apply) => ReplayEffectKind::Apply,
        Ok(ActionReplayEffect::Duplicate) => ReplayEffectKind::Duplicate,
        Err(RecoveryError::NonIdempotentActionBlocked { .. }) => ReplayEffectKind::Blocked,
        Err(RecoveryError::ReplayDivergence { .. }) => ReplayEffectKind::Divergence,
        Err(_) => ReplayEffectKind::OtherErr,
    };
    std::mem::forget(result);
    kind
}

pub(crate) fn replay_unit_kind(result: Result<(), RecoveryError>) -> ReplayUnitKind {
    let kind = match &result {
        Ok(()) => ReplayUnitKind::Ok,
        Err(RecoveryError::ReplayDivergence { .. }) => ReplayUnitKind::Divergence,
        Err(_) => ReplayUnitKind::OtherErr,
    };
    std::mem::forget(result);
    kind
}

pub(crate) fn replay_effect_is_apply(kind: ReplayEffectKind) -> bool {
    matches!(kind, ReplayEffectKind::Apply)
}

pub(crate) fn replay_effect_is_duplicate(kind: ReplayEffectKind) -> bool {
    matches!(kind, ReplayEffectKind::Duplicate)
}

pub(crate) fn replay_effect_is_blocked(kind: ReplayEffectKind) -> bool {
    matches!(kind, ReplayEffectKind::Blocked)
}

pub(crate) fn replay_effect_is_divergence(kind: ReplayEffectKind) -> bool {
    matches!(kind, ReplayEffectKind::Divergence)
}

pub(crate) fn replay_unit_is_divergence(kind: ReplayUnitKind) -> bool {
    matches!(kind, ReplayUnitKind::Divergence)
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
