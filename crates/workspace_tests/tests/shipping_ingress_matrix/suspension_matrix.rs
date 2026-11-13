//! Suspension boundaries: error handler, cancel, shutdown, durability
//! profiles, ask taint, plus wait and action submodules.

#![forbid(unsafe_code)]

use vb_core::value::Taint;
use vb_core::{CompiledNodeKind, RunId, RuntimePolicy, SlotValue};
use vb_runtime::recovery::{RecoveredRunBoundaryKind, RuntimeRecoveryProduct};
use vb_runtime::runtime::Runtime;
use vb_runtime::shard::AskAnswer;

use super::support::{
    self, Durability, TestResult, assert_ordering, compile, events, open_runtime, persist_artifact,
    terminal, tick_until_terminal,
};

#[path = "suspension_matrix/actions.rs"]
mod actions;
#[path = "suspension_matrix/timers.rs"]
mod timers;

const HEADER: &str =
    "version: velvet-ballistics/v1\nname: shipping_suspension\nwhen: {manual: {}}\n";

fn wait_workflow(timeout: &str) -> String {
    format!(
        "{HEADER}steps:\n  - id: seed\n    set: {{output: seed, value: '42'}}\n  - id: deadline\n    set: {{output: delay, value: '{timeout}'}}\n  - id: gate\n    wait: {{timeout: '$step.deadline'}}\n  - id: done\n    finish: {{result: '$step.seed'}}\n"
    )
}

fn tick_until_timer(
    runtime: &mut Runtime,
    run: RunId,
) -> TestResult<vb_runtime::shard::timer_wheel::TimerEntry> {
    for _ in 0..128 {
        runtime.tick_all()?;
        if let Ok(entry) = runtime.capture_timer_entry(run) {
            return Ok(entry);
        }
    }
    Err(format!("run {run:?} did not register a timer").into())
}

#[test]
fn on_error_handler_produces_terminal_value_after_step_failure() -> TestResult {
    let yaml = format!(
        "{HEADER}steps:\n  - id: seed\n    set: {{output: zero, value: '0'}}\n  - id: failing\n    on_error: {{handler: recovered}}\n    set: {{output: quotient, value: '1 / $step.seed'}}\n  - id: recovered\n    set: {{output: recovered, value: '7'}}\n  - id: done\n    finish: {{result: '$step.recovered'}}\n"
    );
    let workflow = compile(&yaml)?;
    let dir = tempfile::tempdir()?;
    let run = RunId::new(106);
    let mut open = open_runtime(dir.path(), RuntimePolicy::Strict, Durability::Strict)?;
    persist_artifact(&open.journal, &workflow, RuntimePolicy::Strict, &[])?;
    open.runtime.submit_compiled(run, workflow.clone())?;
    tick_until_terminal(&mut open.runtime, run, 128)?;
    let durable = events(&open.journal, run)?;
    let failing = workflow
        .to_parts()
        .nodes
        .iter()
        .find(|node| matches!(node.kind, CompiledNodeKind::EvalExpr { .. }))
        .map(|node| node.id)
        .ok_or("missing EvalExpr")?;
    let recovered = workflow
        .node(failing)
        .and_then(|node| node.routing.error.handler())
        .ok_or("missing handler")?;
    assert_eq!(
        support::count_where(&durable, |event| matches!(
            event,
            vb_storage::JournalEvent::StepFailed { step, .. } if *step == failing
        )),
        1
    );
    assert_eq!(
        support::count_where(&durable, |event| matches!(
            event,
            vb_storage::JournalEvent::StepSucceeded { step, .. } if *step == recovered
        )),
        1
    );
    assert_eq!(terminal(&durable, run)?.value, SlotValue::I64(7));
    assert_eq!(open.runtime.counters_snapshot().runs_completed, 1);
    assert_eq!(open.runtime.counters_snapshot().runs_failed, 0);
    Ok(())
}

#[test]
fn cancel_suspended_wait_is_terminal_and_cold_rejected() -> TestResult {
    let yaml = wait_workflow("60000");
    let workflow = compile(&yaml)?;
    let dir = tempfile::tempdir()?;
    let run = RunId::new(107);
    {
        let mut open = open_runtime(dir.path(), RuntimePolicy::Strict, Durability::Strict)?;
        persist_artifact(&open.journal, &workflow, RuntimePolicy::Strict, &[])?;
        open.runtime.submit_compiled(run, workflow.clone())?;
        let _entry = tick_until_timer(&mut open.runtime, run)?;
        open.runtime.cancel_run(run)?;
        open.runtime.tick_all()?;
        let durable = events(&open.journal, run)?;
        assert_eq!(
            support::count_where(&durable, |event| matches!(
                event,
                vb_storage::JournalEvent::RunCancelled { run: actual, .. } if *actual == run
            )),
            1
        );
        assert_eq!(open.runtime.counters_snapshot().runs_failed, 1);
        assert_eq!(open.runtime.counters_snapshot().runs_completed, 0);
    }
    let open = open_runtime(dir.path(), RuntimePolicy::Strict, Durability::Strict)?;
    let product = open.runtime.recover_product(run)?;
    match product {
        RuntimeRecoveryProduct::CannotResume(product) => {
            assert_eq!(product.reason(), "terminal_state");
        }
        other => return Err(format!("cancelled run recovered as {other:?}").into()),
    }
    Ok(())
}

#[test]
fn graceful_shutdown_preserves_waiting_boundary_and_recovery_finishes() -> TestResult {
    let yaml = wait_workflow("0");
    let workflow = compile(&yaml)?;
    let dir = tempfile::tempdir()?;
    let run = RunId::new(108);
    {
        let mut open = open_runtime(dir.path(), RuntimePolicy::Strict, Durability::Strict)?;
        persist_artifact(&open.journal, &workflow, RuntimePolicy::Strict, &[])?;
        open.runtime.submit_compiled(run, workflow.clone())?;
        let _entry = tick_until_timer(&mut open.runtime, run)?;
        let report = open.runtime.shutdown_graceful()?;
        assert_eq!(report.timers_released(), 1);
        assert_eq!(report.remaining().pending_timer_count(), 1);
        assert_eq!(report.remaining().active_run_count(), 1);
    }
    let mut open = open_runtime(dir.path(), RuntimePolicy::Strict, Durability::Strict)?;
    let durable = events(&open.journal, run)?;
    assert_eq!(
        support::count_where(&durable, |event| matches!(
            event,
            vb_storage::JournalEvent::WaitScheduledEvent { run: actual, .. } if *actual == run
        )),
        1
    );
    let product = open.runtime.recover_product(run)?;
    match &product {
        RuntimeRecoveryProduct::Resumable(product) => {
            assert_eq!(
                product.boundary_kind(),
                RecoveredRunBoundaryKind::WaitingTimer
            );
        }
        other => return Err(format!("shutdown wait recovered as {other:?}").into()),
    }
    open.runtime.recover_and_resume(run)?;
    let entry = tick_until_timer(&mut open.runtime, run)?;
    open.runtime.timer_entry_fired(entry)?;
    tick_until_terminal(&mut open.runtime, run, 64)?;
    let durable = events(&open.journal, run)?;
    assert_eq!(terminal(&durable, run)?.value, SlotValue::I64(42));
    Ok(())
}

#[test]
fn journaled_durability_reopen_preserves_completed_terminal() -> TestResult {
    let yaml = format!(
        "{HEADER}steps:\n  - id: seed\n    set: {{output: seed, value: '42'}}\n  - id: done\n    finish: {{result: '$step.seed'}}\n"
    );
    let workflow = compile(&yaml)?;
    let dir = tempfile::tempdir()?;
    let run = RunId::new(109);
    {
        let mut open = open_runtime(dir.path(), RuntimePolicy::Journaled, Durability::Journaled)?;
        persist_artifact(&open.journal, &workflow, RuntimePolicy::Journaled, &[])?;
        open.runtime.submit_compiled(run, workflow.clone())?;
        tick_until_terminal(&mut open.runtime, run, 64)?;
        assert_eq!(open.runtime.counters_snapshot().runs_completed, 1);
        let _report = open.runtime.shutdown_graceful()?;
    }
    let open = open_runtime(dir.path(), RuntimePolicy::Journaled, Durability::Journaled)?;
    let durable = events(&open.journal, run)?;
    let result = terminal(&durable, run)?;
    assert_eq!(result.value, SlotValue::I64(42));
    assert_eq!(result.taint, Taint::Clean);
    match open.runtime.recover_product(run)? {
        RuntimeRecoveryProduct::CannotResume(product) => {
            assert_eq!(product.reason(), "terminal_state");
        }
        other => return Err(format!("journaled terminal recovered as {other:?}").into()),
    }
    Ok(())
}

#[test]
fn ask_answer_taint_passes_through_finish() -> TestResult {
    let yaml = format!(
        "{HEADER}vars:\n  prompt_text: 'input?'\nsteps:\n  - id: question\n    ask: {{prompt: '$vars.prompt_text'}}\n  - id: done\n    finish: {{result: '$step.question'}}\n"
    );
    let workflow = compile(&yaml)?;
    let ask_step = workflow
        .to_parts()
        .nodes
        .iter()
        .find(|node| matches!(node.kind, CompiledNodeKind::Ask { .. }))
        .map(|node| node.id)
        .ok_or("missing Ask")?;
    let dir = tempfile::tempdir()?;
    let run = RunId::new(110);
    let mut open = open_runtime(dir.path(), RuntimePolicy::Strict, Durability::Strict)?;
    persist_artifact(&open.journal, &workflow, RuntimePolicy::Strict, &[])?;
    open.runtime.submit_compiled(run, workflow.clone())?;
    let ticket = support::tick_until_ask(&mut open.runtime, run, ask_step, 64)?;
    open.runtime.answer_ask(AskAnswer::new(
        ticket,
        ticket.answer_slot(),
        SlotValue::I64(5),
        Taint::DerivedFromSecret,
    ))?;
    tick_until_terminal(&mut open.runtime, run, 64)?;
    let durable = events(&open.journal, run)?;
    let result = terminal(&durable, run)?;
    assert_eq!(result.value, SlotValue::I64(5));
    assert_eq!(result.taint, Taint::DerivedFromSecret);
    assert_ordering(&durable, run)?;
    Ok(())
}
