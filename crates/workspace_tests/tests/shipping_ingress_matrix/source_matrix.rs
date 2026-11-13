//! Source-primitive terminal matrix: compile, accepted artifact, direct
//! submission, real journal, exact terminal result and taint.

#![forbid(unsafe_code)]

use vb_core::value::Taint;
use vb_core::{CompiledNodeKind, CompiledWorkflow, RunId, RuntimePolicy, SlotValue};
use vb_storage::JournalEvent;

use super::support::{
    self, Durability, TestResult, assert_ordering, compile, events, open_runtime, persist_artifact,
    terminal, tick_until_terminal,
};

#[path = "source_matrix/loops.rs"]
mod loops;
#[path = "source_matrix/replay.rs"]
mod replay;

const HEADER: &str = "version: velvet-ballistics/v1\nname: shipping_matrix\nwhen: {manual: {}}\n";

fn seed_and_finish(body: &str, result: &str) -> String {
    format!(
        "{HEADER}steps:\n  - id: seed\n    set: {{output: seed, value: '42'}}\n{body}  - id: done\n    finish: {{result: '{result}'}}\n"
    )
}

fn finish_pc(workflow: &CompiledWorkflow) -> TestResult<vb_core::StepIdx> {
    workflow
        .to_parts()
        .nodes
        .iter()
        .find(|node| matches!(node.kind, CompiledNodeKind::Finish { .. }))
        .map(|node| node.id)
        .ok_or_else(|| "compiled workflow has no Finish node".into())
}

fn assert_single_success(
    runtime: &vb_runtime::runtime::Runtime,
    events: &[JournalEvent],
    run: RunId,
    expected_submitted: u64,
    expected: SlotValue,
    taint: Taint,
    workflow: &CompiledWorkflow,
) -> TestResult {
    let counters = runtime.counters_snapshot();
    let failed_events = support::count_where(events, |event| {
        matches!(
            event,
            JournalEvent::RunFailedEvent { run: actual, .. } if *actual == run
        )
    });
    assert_eq!(
        counters.runs_submitted, expected_submitted,
        "counters: {counters:?}"
    );
    assert_eq!(
        counters.runs_completed, 1,
        "counters: {counters:?}, RunFailed events: {failed_events}"
    );
    assert_eq!(counters.runs_failed, 0, "counters: {counters:?}");
    let started = support::count_where(
        events,
        |event| matches!(event, JournalEvent::StepStarted { run: actual, .. } if *actual == run),
    );
    assert_eq!(
        counters.steps_executed,
        u64::try_from(started).unwrap_or(u64::MAX),
        "steps_executed must equal durable StepStarted count"
    );
    let result = terminal(events, run)?;
    assert_eq!(result.value, expected);
    assert_eq!(result.taint, taint);
    let finish = finish_pc(workflow)?;
    let finish_result = workflow
        .node(finish)
        .and_then(|node| match node.kind {
            CompiledNodeKind::Finish { result } => Some(result),
            _ => None,
        })
        .ok_or("Finish node has no result slot")?;
    assert_eq!(result.slot, finish_result);
    let last_succeeded = events.iter().rev().find_map(|event| match event {
        JournalEvent::StepSucceeded {
            run: actual, step, ..
        } if *actual == run => Some(*step),
        _ => None,
    });
    assert_eq!(last_succeeded, Some(finish));
    assert_ordering(events, run)?;
    Ok(())
}

fn step_id(workflow: &CompiledWorkflow, name: &str) -> Option<u16> {
    workflow
        .to_parts()
        .step_names
        .iter()
        .position(|candidate| candidate.as_ref() == name)
        .and_then(|index| u16::try_from(index).ok())
}

fn succeeded_count(events: &[JournalEvent], run: RunId, step: u16) -> usize {
    support::count_where(events, |event| {
        matches!(
            event,
            JournalEvent::StepSucceeded { run: actual, step: actual_step, .. }
                if *actual == run && actual_step.get() == step
        )
    })
}

fn decoded_writes(
    events: &[JournalEvent],
    run: RunId,
) -> TestResult<Vec<(vb_core::SlotIdx, SlotValue)>> {
    let mut writes = Vec::new();
    for event in events {
        if let JournalEvent::SlotWrittenEvent {
            run: actual,
            slot,
            value: Some(bytes),
            ..
        } = event
            && *actual == run
        {
            writes.push((*slot, postcard::from_bytes::<SlotValue>(bytes)?));
        }
    }
    Ok(writes)
}

fn consecutive_unique(values: Vec<SlotValue>) -> Vec<SlotValue> {
    let mut unique = Vec::new();
    for value in values {
        if unique.last() != Some(&value) {
            unique.push(value);
        }
    }
    unique
}

#[test]
fn set_const_copy_expression_list_object_finish_matrix() -> TestResult {
    let yaml = format!(
        "{HEADER}vars:\n  items: [1, 2, 3]\n  record: {{left: 1, right: 2}}\nsteps:\n  - id: number\n    set: {{output: value, value: '40'}}\n  - id: copied\n    set: {{output: copied, value: '$step.number'}}\n  - id: items\n    set: {{output: items, value: '$vars.items'}}\n  - id: record\n    set: {{output: record, value: '$vars.record'}}\n  - id: total\n    set: {{output: total, value: '$step.copied + 2'}}\n  - id: done\n    finish: {{result: '$step.total'}}\n"
    );
    let workflow = compile(&yaml)?;
    let parts = workflow.to_parts();
    let kinds: Vec<&CompiledNodeKind> = parts.nodes.iter().map(|node| &node.kind).collect();
    assert!(
        kinds
            .iter()
            .any(|kind| matches!(kind, CompiledNodeKind::SetConst { .. }))
    );
    assert!(
        kinds
            .iter()
            .any(|kind| matches!(kind, CompiledNodeKind::EvalExpr { .. }))
    );
    assert!(
        kinds
            .iter()
            .any(|kind| matches!(kind, CompiledNodeKind::BuildList { .. }))
    );
    assert!(
        kinds
            .iter()
            .any(|kind| matches!(kind, CompiledNodeKind::BuildObject { .. }))
    );
    assert!(
        kinds
            .iter()
            .any(|kind| matches!(kind, CompiledNodeKind::Finish { .. }))
    );

    let dir = tempfile::tempdir()?;
    let mut open = open_runtime(dir.path(), RuntimePolicy::Strict, Durability::Strict)?;
    persist_artifact(&open.journal, &workflow, RuntimePolicy::Strict, &[])?;
    open.runtime
        .submit_compiled(RunId::new(1), workflow.clone())?;
    tick_until_terminal(&mut open.runtime, RunId::new(1), 256)?;
    let durable = events(&open.journal, RunId::new(1))?;
    assert_single_success(
        &open.runtime,
        &durable,
        RunId::new(1),
        1,
        SlotValue::I64(42),
        Taint::Clean,
        &workflow,
    )
}

fn choose_source(flag: &str) -> String {
    format!(
        "{HEADER}steps:\n  - id: seed\n    set: {{output: seed, value: '42'}}\n  - id: flag\n    set: {{output: go, value: '{flag}'}}\n  - id: route\n    choose:\n      branches:\n        - when: '$step.flag'\n          steps:\n            - id: took_a\n              set: {{output: a, value: '7'}}\n      otherwise: done\n  - id: done\n    finish: {{result: '$step.seed'}}\n"
    )
}

fn run_choose(flag: &str, run: RunId) -> TestResult<bool> {
    let workflow = compile(&choose_source(flag))?;
    let dir = tempfile::tempdir()?;
    let mut open = open_runtime(dir.path(), RuntimePolicy::Strict, Durability::Strict)?;
    persist_artifact(&open.journal, &workflow, RuntimePolicy::Strict, &[])?;
    open.runtime.submit_compiled(run, workflow.clone())?;
    tick_until_terminal(&mut open.runtime, run, 256)?;
    let durable = events(&open.journal, run)?;
    assert_single_success(
        &open.runtime,
        &durable,
        run,
        1,
        SlotValue::I64(42),
        Taint::Clean,
        &workflow,
    )?;
    let branch = workflow
        .to_parts()
        .step_names
        .iter()
        .position(|name| name.as_ref() == "took_a")
        .and_then(|index| u16::try_from(index).ok())
        .ok_or("missing took_a")?;
    Ok(durable.iter().any(|event| {
        matches!(
            event,
            JournalEvent::StepSucceeded { run: actual, step, .. }
                if *actual == run && step.get() == branch
        )
    }))
}

#[test]
fn choose_executes_only_the_selected_branch_matrix() -> TestResult {
    assert!(run_choose("true", RunId::new(2))?);
    assert!(!run_choose("false", RunId::new(2))?);
    Ok(())
}
