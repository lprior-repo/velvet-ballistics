//! for_each, together, collect, reduce, and bounded repeat scenarios.

#![forbid(unsafe_code)]

use std::path::Path;

use vb_core::value::Taint;
use vb_core::{CompiledWorkflow, RunId, RuntimePolicy, SlotValue, ValueStore};
use vb_storage::{RunSubmission, WorkflowSourceRecord};

use crate::source_matrix::{
    HEADER, assert_single_success, consecutive_unique, decoded_writes, seed_and_finish, step_id,
    succeeded_count,
};
use crate::support::{
    self, Durability, TestResult, compile, events, open_runtime, persist_artifact,
    tick_until_terminal,
};

#[test]
fn for_each_executes_body_once_per_item_matrix() -> TestResult {
    let yaml = format!(
        "{HEADER}inputs:\n  entries: {{is: list, of: number, max: 8}}\nsteps:\n  - id: seed\n    set: {{output: seed, value: '42'}}\n  - id: walk\n    for_each:\n      input: '$input.entries'\n      variable: entry\n      at_once: 1\n      steps:\n        - id: body\n          set: {{output: seen, value: '$entry'}}\n  - id: done\n    finish: {{result: '$step.seed'}}\n"
    );
    let dir = tempfile::tempdir()?;
    let run = RunId::new(3);
    let (mut open, workflow) = submit_owned_list(dir.path(), &yaml, run, &[1, 2, 3])?;
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
    let body = step_id(&workflow, "body").ok_or("missing body step")?;
    assert_eq!(succeeded_count(&durable, run, body), 3);
    let body_slot = workflow
        .node(vb_core::StepIdx::new(body))
        .and_then(|node| node.output.slot())
        .ok_or("body step has no output slot")?;
    let seen: Vec<SlotValue> = decoded_writes(&durable, run)?
        .into_iter()
        .filter(|(slot, _)| *slot == body_slot)
        .map(|(_, value)| value)
        .collect();
    assert_eq!(
        consecutive_unique(seen),
        vec![SlotValue::I64(1), SlotValue::I64(2), SlotValue::I64(3)]
    );
    Ok(())
}

fn submit_owned_list(
    path: &Path,
    yaml: &str,
    run: RunId,
    items: &[i64],
) -> TestResult<(support::OpenRuntime, CompiledWorkflow)> {
    let workflow = compile(yaml)?;
    let input = workflow
        .input_slots()
        .first()
        .ok_or("workflow declares no input")?;
    let input_slot = input.slot;
    let open = open_runtime(path, RuntimePolicy::Strict, Durability::Strict)?;
    let mut store = ValueStore::with_max_slots(workflow.resource_contract().max_slots);
    let values: Vec<SlotValue> = items.iter().copied().map(SlotValue::I64).collect();
    let taints = vec![Taint::Clean; values.len()];
    let list =
        store.insert_list_with_taint(values.into_boxed_slice(), taints.into_boxed_slice())?;
    let bundle = vb_core::root_bundle::OwnedRootBundle::new(
        Box::from([(input_slot, SlotValue::List(list), Taint::Clean)]),
        store.checkpoint_v1(),
        &workflow,
    )?;
    let artifact =
        vb_storage::admission::prepare_compiled_artifact(&workflow, RuntimePolicy::Strict, &[])?;
    let source = WorkflowSourceRecord {
        digest: vb_core::WorkflowDigest::from_bytes(blake3::hash(yaml.as_bytes()).into()),
        source: yaml.as_bytes().to_vec(),
    };
    let record = vb_storage::OwnedRunInputsRecordV1::new(
        run,
        artifact.digest,
        source.digest,
        &bundle,
        &workflow,
    )?;
    let submission = RunSubmission::owned(
        source.clone(),
        artifact,
        record,
        vb_storage::RunHeaderRecord {
            run,
            workflow_id: vb_core::WorkflowId::new(0),
            compiled_digest: workflow.digest(),
            status: 0,
            accepted_at_ms: 123,
        },
    )?;
    open.journal
        .commit_run_submission(&submission, RuntimePolicy::Strict)?;
    open.runtime.activate_owned_submission(run, source.digest)?;
    Ok((open, workflow))
}

#[test]
fn together_executes_all_branches_matrix() -> TestResult {
    let yaml = seed_and_finish(
        "  - id: both\n    together:\n      branches:\n        - label: left\n          steps:\n            - id: left_value\n              set: {output: left, value: '7'}\n        - label: right\n          steps:\n            - id: right_value\n              set: {output: right, value: '9'}\n",
        "$step.seed",
    );
    let workflow = compile(&yaml)?;
    let dir = tempfile::tempdir()?;
    let mut open = open_runtime(dir.path(), RuntimePolicy::Strict, Durability::Strict)?;
    persist_artifact(&open.journal, &workflow, RuntimePolicy::Strict, &[])?;
    let run = RunId::new(4);
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
    let left = step_id(&workflow, "left_value").ok_or("missing left_value")?;
    let right = step_id(&workflow, "right_value").ok_or("missing right_value")?;
    assert_eq!(succeeded_count(&durable, run, left), 1);
    assert_eq!(succeeded_count(&durable, run, right), 1);
    let branch_values: Vec<SlotValue> = decoded_writes(&durable, run)?
        .into_iter()
        .map(|(_, value)| value)
        .filter(|value| matches!(value, SlotValue::I64(7 | 9)))
        .collect();
    assert_eq!(
        consecutive_unique(branch_values),
        vec![SlotValue::I64(7), SlotValue::I64(9)]
    );
    Ok(())
}

#[test]
fn collect_pages_all_items_matrix() -> TestResult {
    let yaml = format!(
        "{HEADER}inputs:\n  entries: {{is: list, of: number, max: 8}}\nsteps:\n  - id: seed\n    set: {{output: seed, value: '42'}}\n  - id: gathered\n    collect:\n      variable: batch\n      source: '$input.entries'\n      pages: 2\n      items: 2\n      steps:\n        - id: body\n          set: {{output: copied, value: '$batch'}}\n  - id: total\n    set: {{output: total, value: 'sum($step.gathered)'}}\n  - id: done\n    finish: {{result: '$step.total'}}\n"
    );
    let dir = tempfile::tempdir()?;
    let run = RunId::new(5);
    let (mut open, workflow) = submit_owned_list(dir.path(), &yaml, run, &[1, 2, 3])?;
    tick_until_terminal(&mut open.runtime, run, 256)?;
    let durable = events(&open.journal, run)?;
    assert_single_success(
        &open.runtime,
        &durable,
        run,
        1,
        SlotValue::I64(6),
        Taint::Clean,
        &workflow,
    )
}

#[test]
fn reduce_accumulates_every_item_matrix() -> TestResult {
    let yaml = format!(
        "{HEADER}inputs:\n  entries: {{is: list, of: number, max: 8}}\nsteps:\n  - id: seed\n    set: {{output: seed, value: '42'}}\n  - id: folded\n    reduce:\n      variable: total\n      input: '$input.entries'\n      initial: '0'\n      steps:\n        - id: body\n          set: {{output: running, value: '$total + 1'}}\n  - id: done\n    finish: {{result: '$step.seed'}}\n"
    );
    let dir = tempfile::tempdir()?;
    let run = RunId::new(6);
    let (mut open, workflow) = submit_owned_list(dir.path(), &yaml, run, &[1, 2, 3])?;
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
    let running_writes: Vec<SlotValue> = decoded_writes(&durable, run)?
        .into_iter()
        .map(|(_, value)| value)
        .filter(|value| matches!(value, SlotValue::I64(1 | 2 | 3)))
        .collect();
    assert_eq!(
        consecutive_unique(running_writes),
        vec![SlotValue::I64(1), SlotValue::I64(2), SlotValue::I64(3)]
    );
    Ok(())
}

#[test]
fn bounded_repeat_runs_body_max_attempts_matrix() -> TestResult {
    let yaml = seed_and_finish(
        "  - id: again\n    repeat:\n      max_attempts: 2\n      steps:\n        - id: body\n          set: {output: tick, value: '7'}\n",
        "$step.seed",
    );
    let workflow = compile(&yaml)?;
    let dir = tempfile::tempdir()?;
    let mut open = open_runtime(dir.path(), RuntimePolicy::Strict, Durability::Strict)?;
    persist_artifact(&open.journal, &workflow, RuntimePolicy::Strict, &[])?;
    let run = RunId::new(7);
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
    let body = step_id(&workflow, "body").ok_or("missing body step")?;
    assert_eq!(succeeded_count(&durable, run, body), 2);
    let ticks: Vec<SlotValue> = decoded_writes(&durable, run)?
        .into_iter()
        .map(|(_, value)| value)
        .filter(|value| *value == SlotValue::I64(7))
        .collect();
    assert!(!ticks.is_empty());
    Ok(())
}
