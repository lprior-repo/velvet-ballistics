//! Kani harnesses for VB-REPLAY-006: a terminal run never becomes running again.
//!
//! Proved against the production `RunAggregate` membership functions in
//! `crate::shard::types`: once a run is retained as terminal, every re-entry
//! path rejects it — runtime-state insert, pending-timer insert, and
//! pending-action insert — and terminal membership sticks.
//!
//! Memory note: CBMC exhausts a 7.7GB box on both harnesses (3 OOM attempts
//! 2026-11-12); run where more memory is available.

#![cfg(kani)]
#![forbid(unsafe_code)]

use std::time::Instant;

use indexmap::IndexMap;
use vb_core::action::ActionTicket;
use vb_core::ids::{ActionId, RunId, SeqNo, StepIdx};

use crate::shard::types::{PendingTimer, PendingTimerKind, RunAggregate, RunState, RuntimeState};

fn any_runtime_state() -> RuntimeState {
    match kani::any::<u8>() % 5 {
        0 => RuntimeState::Initial,
        1 => RuntimeState::Running,
        2 => RuntimeState::Resumable,
        3 => RuntimeState::Resuming,
        _ => RuntimeState::Failed,
    }
}

fn any_pending_timer() -> PendingTimer {
    let kind = if kani::any::<bool>() {
        PendingTimerKind::Wait
    } else {
        PendingTimerKind::Ask
    };
    PendingTimer {
        step: StepIdx::new(kani::any::<u16>()),
        kind,
        generation: kani::any::<u64>(),
        deadline: Instant::now(),
    }
}

fn any_action_ticket(run: RunId) -> ActionTicket {
    ActionTicket {
        run,
        step: StepIdx::new(kani::any::<u16>()),
        seq: SeqNo::new(kani::any::<u64>()),
        action: ActionId::new(kani::any::<u16>()),
        run_attempt: kani::any::<u16>(),
        attempt: kani::any::<u16>(),
        idempotency_key: kani::any::<u128>(),
        capacity: kani::any::<u16>(),
        fencing_token: kani::any::<u64>(),
    }
}

const TERMINAL_CAPACITY: usize = 4;

fn terminal_aggregate() -> (RunId, RunAggregate) {
    let raw: u64 = kani::any();
    kani::assume(raw < 8);
    let run = RunId::new(raw);
    let runs: IndexMap<RunId, RunState> = IndexMap::new();
    let mut aggregate = RunAggregate::new();
    kani::assume(aggregate.checked_out_insert(run, TERMINAL_CAPACITY).is_ok());
    kani::assume(
        aggregate
            .terminal_insert(&runs, run, TERMINAL_CAPACITY)
            .is_ok(),
    );
    (run, aggregate)
}

#[kani::proof]
#[kani::unwind(8)]
fn kani_terminal_state_rejects_runtime_state() {
    let (run, mut aggregate) = terminal_aggregate();
    let runs: IndexMap<RunId, RunState> = IndexMap::new();
    kani::assert(
        aggregate.terminal_contains(run),
        "run is retained as terminal",
    );
    kani::assert(
        aggregate.runtime_state_get(run).is_none(),
        "terminal run has no runtime state",
    );
    kani::assert(
        !aggregate.run_is_live_for_boundary(&runs, run),
        "terminal run is not live for boundaries",
    );
    kani::assert(
        aggregate.terminal_insert(&runs, run, TERMINAL_CAPACITY) == Ok(false),
        "re-marking a terminal run is a no-op",
    );
    let state = any_runtime_state();
    kani::assert(
        aggregate
            .runtime_state_insert(&runs, run, state, TERMINAL_CAPACITY)
            .is_err(),
        "terminal run rejects runtime-state insert",
    );
    kani::assert(
        aggregate.terminal_contains(run),
        "run stays terminal after rejected insert",
    );
    kani::assert(
        aggregate.runtime_state_get(run).is_none(),
        "rejected insert leaves no runtime state",
    );
}

#[kani::proof]
#[kani::unwind(8)]
fn kani_terminal_state_rejects_pending() {
    let (run, mut aggregate) = terminal_aggregate();
    let runs: IndexMap<RunId, RunState> = IndexMap::new();
    kani::assert(
        aggregate
            .pending_timer_insert(&runs, TERMINAL_CAPACITY, run, any_pending_timer())
            .is_err(),
        "terminal run rejects pending timers",
    );
    kani::assert(
        aggregate
            .pending_action_insert(&runs, TERMINAL_CAPACITY, run, any_action_ticket(run))
            .is_err(),
        "terminal run rejects pending actions",
    );
    kani::assert(
        aggregate.pending_timer_len() == 0,
        "no pending timer recorded for terminal run",
    );
    kani::assert(
        aggregate.pending_action_len() == 0,
        "no pending action recorded for terminal run",
    );
}
