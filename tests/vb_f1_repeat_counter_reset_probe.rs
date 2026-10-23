#![forbid(unsafe_code)]
//! Repeat counter slot reset probe (vb-8eocj.3.2).
//!
//! Test 1: Baseline unmutated source — Repeat max 3 with body Set; expect
//!         Finished(42) within 64 steps with 3 body completions.
//! Test 2: Malformed-IR negative — mutate body.output to the attempt counter
//!         slot; expect constructor reject or, if accepted, bounded run with
//!         actual observed slot values to document the defect. RED is preserved.

use std::num::NonZeroUsize;
use std::sync::Arc;

use vb_compile::compile_workflow;
use vb_core::ids::{RunId, SlotIdx, StepIdx};
use vb_core::policy::RuntimePolicy;
use vb_core::value::SlotValue;
use vb_core::workflow::{CompiledNodeKind, CompiledWorkflow, WorkflowParts};
use vb_runtime::journal::StorageRuntimeJournal;
use vb_runtime::runtime::Runtime;
use vb_runtime::shard::ShardConfig;
use vb_storage::{FjallJournal, JournalEvent};

fn shard_count() -> Result<NonZeroUsize, String> {
    NonZeroUsize::new(1)
        .ok_or_else(|| "shard count must be non-zero".to_string())
}

const BASELINE_SOURCE: &str = "version: velvet-ballistics/v1\n\
    name: counter_reset_probe\n\
    when:\n  manual: {}\n\
    steps:\n\
      - id: set_answer\n        set:\n          output: answer\n          value: 42\n\
      - id: repeat_body\n        repeat:\n          max_attempts: 3\n          steps:\n            - id: body_step\n              set:\n                output: body_result\n                value: \"12884901888\"\n\
      - id: finish_step\n        finish:\n          result: answer\n";

fn count_step_succeeded(events: &[JournalEvent], step: StepIdx) -> Result<u64, String> {
    let mut count = 0;
    for event in events {
        if let JournalEvent::StepSucceeded { step_id, .. } = event {
            if *step_id == step {
                count = count
                    .checked_add(1)
                    .ok_or_else(|| "body count overflow".to_string())?;
            }
        }
    }
    Ok(count)
}

fn count_slot_written(events: &[JournalEvent], slot: SlotIdx) -> Result<u64, String> {
    let mut count = 0;
    for event in events {
        if let JournalEvent::SlotWritten { slot_id, .. } = event {
            if *slot_id == slot {
                count = count
                    .checked_add(1)
                    .ok_or_else(|| "slot write count overflow".to_string())?;
            }
        }
    }
    Ok(count)
}

fn find_repeat_body_step(wf: &CompiledWorkflow) -> Result<StepIdx, String> {
    for node in wf.iter_nodes() {
        if let CompiledNodeKind::RepeatStart { body, .. } = &node.kind {
            return Ok(*body);
        }
    }
    Err("RepeatStart node not found".to_string())
}

#[test]
fn baseline_unmutated_source_control() -> Result<(), String> {
    let dir = tempfile::tempdir().map_err(|e| format!("tempdir: {e}"))?;
    let path = dir.path().to_path_buf();
    let run = RunId::new(50001);

    let workflow = compile_workflow(BASELINE_SOURCE.as_bytes())
        .map_err(|errors| format!("compile: {}", errors.iter().map(|e| e.to_string()).collect::<Vec<_>>().join("; ")))?;

    let journal = Arc::new(FjallJournal::open(&path, None).map_err(|e| format!("open journal: {e}"))?);
    vb_storage::submit_artifact(&journal, &workflow, RuntimePolicy::Strict)
        .map_err(|e| format!("submit artifact: {e}"))?;

    let shared = StorageRuntimeJournal::shared_journaled(journal.clone());
    let mut runtime = Runtime::new(shard_count()?, ShardConfig::default(), shared);
    runtime
        .submit_direct(run, workflow, Box::from([]))
        .map_err(|e| format!("submit: {e}"))?;

    let result = runtime
        .advance_bounded(run, 64)
        .map_err(|e| format!("advance_bounded: {e}"))?;

    let events = journal
        .events_for_run(run)
        .map_err(|e| format!("events: {e}"))?;

    eprintln!("baseline signal: {result:?}");
    eprintln!("baseline events: {}", events.len());

    let body_step = find_repeat_body_step(&workflow)?;
    let body_count = count_step_succeeded(&events, body_step)?;
    eprintln!("body count: {body_count}");

    match result {
        vb_runtime::RuntimeSignal::Finished(SlotValue::I64(42)) => {
            if body_count != 3 {
                return Err(format!("expected 3 body completions, got {body_count}"));
            }
            eprintln!("baseline: OK — Finished(42), body count 3");
            Ok(())
        }
        other => Err(format!("unexpected signal: {other:?}")),
    }
}

#[test]
fn malformed_ir_body_output_aliased_to_attempt_slot() -> Result<(), String> {
    let workflow = compile_workflow(BASELINE_SOURCE.as_bytes())
        .map_err(|errors| format!("compile: {}", errors.iter().map(|e| e.to_string()).collect::<Vec<_>>().join("; ")))?;

    let body_step = find_repeat_body_step(&workflow)?;

    let mut parts = workflow.to_parts();

    let repeat_node = parts
        .nodes
        .iter()
        .find(|n| matches!(n.kind, CompiledNodeKind::RepeatStart { .. }))
        .ok_or_else(|| "RepeatStart node not found".to_string())?;

    let CompiledNodeKind::RepeatStart { body, .. } = &repeat_node.kind else {
        return Err("not a RepeatStart node".to_string());
    };
    let attempt_slot = repeat_node.output.ok_or_else(|| "no attempt slot".to_string())?;

    let body_node = parts
        .nodes
        .iter_mut()
        .find(|n| n.id == *body)
        .ok_or_else(|| format!("body step {body_step:?} not found"))?;
    let original_body_output = body_node.output;
    body_node.output = Some(attempt_slot);

    let cloned_parts = parts.clone();
    match CompiledWorkflow::try_from_parts(cloned_parts) {
        Err(e) => {
            eprintln!("constructor reject: {e:?}");
            eprintln!("repeat node output: {attempt_slot:?}");
            eprintln!("body original output: {original_body_output:?}");
            Ok(())
        }
        Ok(mutant) => {
            let dir = tempfile::tempdir().map_err(|e| format!("tempdir: {e}"))?;
            let path = dir.path().to_path_buf();
            let run = RunId::new(50002);

            let journal = Arc::new(FjallJournal::open(&path, None).map_err(|e| format!("open journal: {e}"))?);
            vb_storage::submit_artifact(&journal, &mutant, RuntimePolicy::Strict)
                .map_err(|e| format!("submit artifact: {e}"))?;

            let shared = StorageRuntimeJournal::shared_journaled(journal.clone());
            let mut runtime = Runtime::new(shard_count()?, ShardConfig::default(), shared);
            runtime
                .submit_direct(run, mutant, Box::from([]))
                .map_err(|e| format!("submit: {e}"))?;

            let result = runtime
                .advance_bounded(run, 64)
                .map_err(|e| format!("advance_bounded: {e}"))?;

            let events = journal
                .events_for_run(run)
                .map_err(|e| format!("events: {e}"))?;

            let body_count = count_step_succeeded(&events, body_step)?;
            let attempt_writes = count_slot_written(&events, attempt_slot)?;
            let attempt_value = match runtime.read_slot(run, attempt_slot) {
                Ok(SlotValue::U8(v)) => format!("U8({v})"),
                Ok(SlotValue::U16(v)) => format!("U16({v})"),
                Ok(SlotValue::U32(v)) => format!("U32({v})"),
                Ok(SlotValue::U64(v)) => format!("U64({v})"),
                Ok(SlotValue::I8(v)) => format!("I8({v})"),
                Ok(SlotValue::I16(v)) => format!("I16({v})"),
                Ok(SlotValue::I32(v)) => format!("I32({v})"),
                Ok(SlotValue::I64(v)) => format!("I64({v})"),
                Ok(v) => format!("SlotValue: {v:?}"),
                Err(e) => format!("error: {e}"),
            };

            eprintln!("malformed-IR acceptance observed");
            eprintln!("  signal: {result:?}");
            eprintln!("  body count: {body_count}");
            eprintln!("  attempt slot writes: {attempt_writes}");
            eprintln!("  attempt slot final: {attempt_value}");

            Err(format!(
                "OBSERVED: malformed-IR accepted and executed; signal={result:?}, body_count={body_count}, attempt_writes={attempt_writes}, attempt_final={attempt_value}"
            ))
        }
    }
}
