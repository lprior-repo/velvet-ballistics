//! WaitUntil and open WaitEvent suspension scenarios.

#![forbid(unsafe_code)]

use vb_core::value::Taint;
use vb_core::{CompiledNodeKind, RunId, RuntimePolicy, SlotValue, StepState};
use vb_runtime::recovery::{RecoveredRunBoundaryKind, RuntimeRecoveryProduct};

use crate::support::{
    Durability, TestResult, assert_ordering, compile, events, open_runtime, persist_artifact,
    terminal, tick_until_terminal,
};
use crate::suspension_matrix::{tick_until_timer, wait_workflow};

#[test]
fn wait_until_recovers_from_cold_reopen_then_timer_finishes() -> TestResult {
    let yaml = wait_workflow("0");
    let workflow = compile(&yaml)?;
    let dir = tempfile::tempdir()?;
    let run = RunId::new(101);
    {
        let mut open = open_runtime(dir.path(), RuntimePolicy::Strict, Durability::Strict)?;
        persist_artifact(&open.journal, &workflow, RuntimePolicy::Strict, &[])?;
        open.runtime.submit_compiled(run, workflow.clone())?;
        let _entry = tick_until_timer(&mut open.runtime, run)?;
        let durable = events(&open.journal, run)?;
        assert_eq!(
            crate::support::count_where(&durable, |event| matches!(
                event,
                vb_storage::JournalEvent::WaitScheduledEvent { run: actual, .. } if *actual == run
            )),
            1
        );
        let state = open.runtime.inspect_run_state(run)?;
        let wait_step = workflow
            .to_parts()
            .nodes
            .iter()
            .find(|node| matches!(node.kind, CompiledNodeKind::WaitUntil { .. }))
            .map(|node| node.id)
            .ok_or("missing WaitUntil")?;
        assert_eq!(state.frame.step_state(wait_step)?, StepState::Waiting);
    }

    let mut open = open_runtime(dir.path(), RuntimePolicy::Strict, Durability::Strict)?;
    let product = open.runtime.recover_product(run)?;
    match &product {
        RuntimeRecoveryProduct::Resumable(product) => {
            assert_eq!(
                product.boundary_kind(),
                RecoveredRunBoundaryKind::WaitingTimer
            );
        }
        other => return Err(format!("wait did not recover as resumable: {other:?}").into()),
    }
    open.runtime.recover_and_resume(run)?;
    let entry = tick_until_timer(&mut open.runtime, run)?;
    open.runtime.timer_entry_fired(entry)?;
    tick_until_terminal(&mut open.runtime, run, 64)?;
    let durable = events(&open.journal, run)?;
    let result = terminal(&durable, run)?;
    assert_eq!(result.value, SlotValue::I64(42));
    assert_eq!(result.taint, Taint::Clean);
    assert_eq!(
        crate::support::count_where(&durable, |event| matches!(
            event,
            vb_storage::JournalEvent::WaitScheduledEvent { run: actual, .. } if *actual == run
        )),
        1,
        "recovery must not duplicate WaitScheduled"
    );
    assert_ordering(&durable, run)?;
    Ok(())
}

#[test]
fn wait_event_without_timeout_recovers_pending_without_redrive() -> TestResult {
    // v1 has no public event-delivery API for an untimed `wait.event`. Cold
    // recovery must restore the same pending wait, without re-driving the
    // node and without fabricating a terminal state or journal event.
    let yaml = format!(
        "{HEADER}steps:\n  - id: seed\n    set: {{output: seed, value: '42'}}\n  - id: signal\n    set: {{output: event_id, value: '1'}}\n  - id: gate\n    wait: {{event: '$step.signal'}}\n  - id: done\n    finish: {{result: '$step.seed'}}\n",
        HEADER = crate::suspension_matrix::HEADER,
    );
    let workflow = compile(&yaml)?;
    let wait_step = workflow
        .to_parts()
        .nodes
        .iter()
        .find(|node| matches!(node.kind, CompiledNodeKind::WaitEvent { .. }))
        .map(|node| node.id)
        .ok_or("missing WaitEvent")?;
    let dir = tempfile::tempdir()?;
    let run = RunId::new(102);
    {
        let mut open = open_runtime(dir.path(), RuntimePolicy::Strict, Durability::Strict)?;
        persist_artifact(&open.journal, &workflow, RuntimePolicy::Strict, &[])?;
        open.runtime.submit_compiled(run, workflow.clone())?;
        for _ in 0..64 {
            open.runtime.tick_all()?;
            if open
                .runtime
                .inspect_run_state(run)?
                .frame
                .step_state(wait_step)?
                == StepState::Waiting
            {
                break;
            }
        }
        assert_eq!(
            open.runtime
                .inspect_run_state(run)?
                .frame
                .step_state(wait_step)?,
            StepState::Waiting
        );
        assert_eq!(open.runtime.counters_snapshot().runs_completed, 0);
    }

    let mut open = open_runtime(dir.path(), RuntimePolicy::Strict, Durability::Strict)?;
    let baseline_events = events(&open.journal, run)?;
    let product = open
        .runtime
        .recover_product(run)
        .map_err(|error| format!("recover_product: {error:?}"))?;
    match &product {
        RuntimeRecoveryProduct::Resumable(product) => {
            assert_eq!(
                product.boundary_kind(),
                RecoveredRunBoundaryKind::OpenWaitEvent
            );
        }
        other => return Err(format!("open wait recovered as {other:?}").into()),
    }
    open.runtime
        .recover_and_resume(run)
        .map_err(|error| format!("recover_and_resume: {error:?}"))?;
    open.runtime
        .tick_all()
        .map_err(|error| format!("recover tick: {error:?}"))?;
    let state = open.runtime.inspect_run_state(run)?;
    assert_eq!(state.frame.pc(), wait_step);
    assert_eq!(state.frame.step_state(wait_step)?, StepState::Waiting);
    let counters = open.runtime.counters_snapshot();
    assert_eq!(counters.runs_submitted, 0, "counters: {counters:?}");
    assert_eq!(counters.runs_completed, 0, "counters: {counters:?}");
    assert_eq!(counters.runs_failed, 0, "counters: {counters:?}");
    assert_eq!(counters.steps_executed, 0, "counters: {counters:?}");
    let durable = events(&open.journal, run)?;
    assert_eq!(
        durable, baseline_events,
        "recovery must not mutate the journal"
    );

    open.runtime
        .tick_all()
        .map_err(|error| format!("tick_all: {error:?}"))?;
    let state = open.runtime.inspect_run_state(run)?;
    assert_eq!(state.frame.pc(), wait_step);
    assert_eq!(state.frame.step_state(wait_step)?, StepState::Waiting);
    let durable = events(&open.journal, run)?;
    assert_eq!(
        durable, baseline_events,
        "a pending open wait must not fabricate journal events"
    );
    assert_eq!(open.runtime.counters_snapshot().runs_failed, 0);
    assert_eq!(open.runtime.counters_snapshot().runs_completed, 0);
    Ok(())
}
