//!
//! Minimal Kani harness for the vb-4969v aggregate membership invariant.
//!
//! Obligation: PO-vb282my-RS-KANI-006.
//! Target: `RunAggregate::{checked_out_insert, runtime_state_insert,
//! terminal_insert, runtime_state_get, terminal_contains}`.
//!
//! Bound: symbolic `RunId` raw values are restricted to 0..=3 and symbolic
//! live runtime states cover Initial, Running, Resumable, and Resuming. The
//! harness intentionally constructs no `WorkflowParts`, `CompiledWorkflow`, or
//! `RunFrame` and therefore uses no structural workflow/frame fixtures.

#![forbid(unsafe_code)]
#![cfg(kani)]

use indexmap::IndexMap;
use vb_core::ids::RunId;

use crate::shard::types::{RunAggregate, RunState, RuntimeState};

const ACTIVE_CAPACITY: usize = 1;
const RUN_ID_DOMAIN: u8 = 4;
const LIVE_STATE_DOMAIN: u8 = 4;

fn symbolic_run_id() -> RunId {
    let raw = kani::any::<u8>() % RUN_ID_DOMAIN;
    kani::cover!(raw == 0, "aggregate invariant run id lower bound");
    kani::cover!(
        raw == RUN_ID_DOMAIN - 1,
        "aggregate invariant run id upper bound"
    );
    RunId::new(u64::from(raw))
}

fn symbolic_live_runtime_state() -> RuntimeState {
    let variant = kani::any::<u8>() % LIVE_STATE_DOMAIN;
    kani::cover!(variant == 0, "aggregate invariant Initial state covered");
    kani::cover!(variant == 1, "aggregate invariant Running state covered");
    kani::cover!(variant == 2, "aggregate invariant Resumable state covered");
    kani::cover!(variant == 3, "aggregate invariant Resuming state covered");
    match variant {
        0 => RuntimeState::Initial,
        1 => RuntimeState::Running,
        2 => RuntimeState::Resumable,
        _ => RuntimeState::Resuming,
    }
}

fn empty_run_states() -> IndexMap<RunId, RunState> {
    IndexMap::new()
}

fn result_is_ok_without_drop<T, E>(result: Result<T, E>) -> bool {
    match result {
        Ok(value) => {
            core::mem::forget(value);
            true
        }
        Err(error) => {
            core::mem::forget(error);
            false
        }
    }
}

fn result_is_err_without_drop<T, E>(result: Result<T, E>) -> bool {
    match result {
        Ok(value) => {
            core::mem::forget(value);
            false
        }
        Err(error) => {
            core::mem::forget(error);
            true
        }
    }
}

fn terminal_inserted_fresh_without_drop(result: crate::RuntimeResult<bool>) -> bool {
    match result {
        Ok(inserted) => inserted,
        Err(error) => {
            core::mem::forget(error);
            false
        }
    }
}

// PO-vb282my-RS-KANI-006 / vb-4969v: terminal aggregate membership excludes
// runtime state and rejects runtime-state recreation through the crate-private
// aggregate APIs under a tiny symbolic domain.
#[kani::proof]
#[kani::unwind(16)]
fn kani_vb4969v_terminal_membership_excludes_runtime_state_minimal() {
    let run = symbolic_run_id();
    let live_state = symbolic_live_runtime_state();
    let post_terminal_state = symbolic_live_runtime_state();
    let mut aggregate = RunAggregate::new();
    let runs = empty_run_states();

    kani::assert(
        result_is_ok_without_drop(aggregate.checked_out_insert(run, ACTIVE_CAPACITY)),
        "checked-out aggregate ownership setup must succeed",
    );
    kani::assert(
        result_is_ok_without_drop(aggregate.runtime_state_insert(
            &runs,
            run,
            live_state,
            ACTIVE_CAPACITY,
        )),
        "live runtime state setup must succeed before terminal membership",
    );
    kani::assert(
        aggregate.runtime_state_get(run) == Some(live_state),
        "setup must expose the selected live runtime state",
    );

    kani::assert(
        terminal_inserted_fresh_without_drop(aggregate.terminal_insert(
            &runs,
            run,
            ACTIVE_CAPACITY,
        )),
        "terminal insertion must succeed and be fresh",
    );

    kani::assert(
        aggregate.terminal_contains(run),
        "run must be terminal after terminal_insert",
    );
    kani::assert(
        !aggregate.checked_out_contains(run),
        "terminal membership must clear checked-out ownership",
    );
    kani::assert(
        aggregate.runtime_state_get(run).is_none(),
        "terminal membership must clear runtime state",
    );

    kani::assert(
        result_is_err_without_drop(aggregate.runtime_state_insert(
            &runs,
            run,
            post_terminal_state,
            ACTIVE_CAPACITY,
        )),
        "terminal membership must reject runtime-state recreation",
    );
    kani::assert(
        aggregate.runtime_state_get(run).is_none(),
        "terminal membership must not coexist with runtime state",
    );

    // PO-vb282my-RS-KANI-006: this harness checks aggregate mutation and
    // membership semantics, not container/error Drop implementations. Leaking
    // the local verification objects keeps the proof obligation focused on the
    // aggregate invariant and is recorded in the proof evidence.
    core::mem::forget(aggregate);
    core::mem::forget(runs);
}
