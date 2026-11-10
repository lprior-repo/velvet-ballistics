use super::{K, RECOVERED_BODY, SlotValue, TestResult, indent, repeated, source_with};
use std::{num::NonZeroUsize, sync::Arc};
use vb_core::{RunId, RuntimePolicy, Taint, ValueStore, WorkflowDigest};
use vb_runtime::{journal::StorageRuntimeJournal, runtime::Runtime, shard::ShardConfig};
use vb_storage::{FjallJournal, JournalEvent as E, RunSubmission};

const DECLARATIONS: &str = "inputs: {items: {is: list, of: number, max: 2}}\n";

#[derive(Clone, Copy)]
enum Family {
    Repeat,
    ForEach,
    Collect,
    Reduce,
}
const FAMILIES: [Family; 4] = [
    Family::Repeat,
    Family::ForEach,
    Family::Collect,
    Family::Reduce,
];

fn loop_source(family: Family, id: &str, body: &str) -> String {
    let header = match family {
        Family::Repeat => return repeated(id, body),
        Family::ForEach => {
            format!("for_each:\n  variable: {id}_item\n  input: $input.items\n  at_once: 1")
        }
        Family::Collect => format!(
            "collect:\n  variable: {id}_item\n  source: $input.items\n  pages: 2\n  items: 1"
        ),
        Family::Reduce => {
            format!("reduce:\n  variable: {id}_accumulator\n  input: $input.items\n  initial: '0'")
        }
    };
    format!(
        "- id: {id}\n{}    steps:\n{}",
        indent(&header, 2),
        indent(body, 6)
    )
}

fn exercise(yaml: &str, quantum: u64, expected_iterations: usize) -> TestResult {
    exercise_with_capacity(yaml, quantum, expected_iterations, false)
}

fn exercise_with_capacity(
    yaml: &str,
    quantum: u64,
    expected_iterations: usize,
    full_arena: bool,
) -> TestResult {
    let workflow = vb_compile::compile_workflow(yaml.as_bytes())?;
    let parts = workflow.to_parts();
    let budget =
        vb_core::WholeWorkflowBudget::compute(&parts.nodes, parts.entry, &parts.resource_contract)?;
    let failing = parts
        .nodes
        .iter()
        .find(|node| {
            if full_arena {
                matches!(&node.kind, K::BuildList { items } if items.is_empty())
            } else {
                matches!(node.kind, K::EvalExpr { expr }
                    if workflow.expression(expr).is_some_and(|program| program.ops.contains(&vb_core::ExprOp::Div)))
            }
        })
        .ok_or("failing node")?;
    let failed = failing.id;
    let handler = failing.routing.error.handler().ok_or("handler")?;
    let handler_output = workflow
        .node(handler)
        .and_then(|node| node.output.slot())
        .ok_or("handler output")?;
    let input = workflow.input_slots().first().ok_or("input")?.slot;
    let mut store = ValueStore::with_max_slots(workflow.resource_contract().max_slots);
    let list = if full_arena {
        // Every arena entry belongs to the actual input graph. Admission fits
        // exactly; only the future iterator allocation is refused at runtime.
        let mut leaves = Vec::new();
        for _ in 2..workflow.resource_contract().max_slots {
            leaves.push(SlotValue::List(
                store.insert_list(Box::<[SlotValue]>::default())?,
            ));
        }
        let child = store.insert_list(leaves.into_boxed_slice())?;
        store.insert_list(Box::from([SlotValue::List(child)]))?
    } else {
        store.insert_list(Box::from([SlotValue::I64(3), SlotValue::I64(5)]))?
    };
    if full_arena {
        let start = parts
            .nodes
            .iter()
            .find(|node| matches!(node.kind, K::ForEachStart { .. }))
            .ok_or("child start")?;
        let K::ForEachStart {
            item_slot,
            limit,
            body,
            done,
            ..
        } = start.kind
        else {
            return Err("child start kind".into());
        };
        let mut frame = vb_core::new_run_frame(RunId::new(2), &workflow)?;
        frame.set_pc(start.id)?;
        frame.write_slot_with_taint(input, SlotValue::List(list), Taint::Clean)?;
        let before = frame.clone();
        let checkpoint = store.checkpoint_v1();
        assert_eq!(
            vb_runtime::primitives::for_each::for_each_start(
                &mut frame,
                &mut store,
                input,
                item_slot,
                vb_core::ids::FanoutLimit::new(limit),
                body,
                done,
                start.output.slot(),
            ),
            Err(vb_core::CoreError::BudgetExceeded {
                budget: "max_slots",
                limit: u64::from(workflow.resource_contract().max_slots),
            })
        );
        assert_eq!(frame, before);
        assert_eq!(store.checkpoint_v1(), checkpoint);
    }
    let bundle = vb_core::root_bundle::OwnedRootBundle::new(
        Box::from([(input, SlotValue::List(list), Taint::Clean)]),
        store.checkpoint_v1(),
        &workflow,
    )?;
    if full_arena {
        assert_eq!(
            bundle.checkpoint().lists.len(),
            usize::try_from(workflow.resource_contract().max_slots)?
        );
    }
    let artifact =
        vb_storage::admission::prepare_compiled_artifact(&workflow, RuntimePolicy::Strict, &[])?;
    let source = vb_storage::WorkflowSourceRecord {
        digest: WorkflowDigest::from_bytes(*blake3::hash(yaml.as_bytes()).as_bytes()),
        source: yaml.as_bytes().to_vec(),
    };
    let run = RunId::new(1);
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
    let directory = tempfile::tempdir()?;
    let journal = Arc::new(FjallJournal::open(directory.path(), None)?);
    journal.commit_run_submission(&submission, RuntimePolicy::Strict)?;
    let mut runtime = Runtime::new(
        NonZeroUsize::MIN,
        ShardConfig {
            step_budget_per_tick: quantum,
            ..ShardConfig::default()
        },
        StorageRuntimeJournal::shared_strict(journal.clone()),
    );
    runtime.activate_owned_submission(run, source.digest)?;
    let mut observed_failed_starts = 0_usize;
    for _ in 0..256 {
        runtime.tick_all()?;
        assert_eq!(runtime.counters_snapshot().runs_failed, 0, "{yaml}");
        if runtime.counters_snapshot().runs_completed == 1 {
            break;
        }
        if full_arena {
            let state = runtime.inspect_run_state(run)?;
            if state.frame.pc() == handler {
                assert_eq!(state.frame.step_state(failed)?, vb_core::StepState::Failed);
                observed_failed_starts = observed_failed_starts
                    .checked_add(1)
                    .ok_or("observation overflow")?;
                let start = parts
                    .nodes
                    .iter()
                    .find(|node| matches!(node.kind, K::ForEachStart { .. }))
                    .ok_or("child start")?;
                let K::ForEachStart { body, .. } = start.kind else {
                    return Err("child start kind".into());
                };
                assert_eq!(
                    state.frame.step_state(start.id)?,
                    vb_core::StepState::Pending
                );
                assert_eq!(state.frame.step_state(body)?, vb_core::StepState::Pending);
                assert_eq!(
                    state.store.checkpoint_v1().lists.len(),
                    usize::try_from(parts.resource_contract.max_slots)?
                );
            }
        }
    }
    assert_eq!(runtime.counters_snapshot().runs_completed, 1, "{yaml}");
    if full_arena && quantum == 1 {
        assert_eq!(observed_failed_starts, expected_iterations);
    }
    let events = journal.events_for_run_full(run)?;
    let executed = u64::try_from(
        events
            .iter()
            .filter(|event| matches!(event, E::StepStarted { .. }))
            .count(),
    )?;
    assert!(
        budget.max_total_steps >= executed,
        "budget {} < actual {executed}: {yaml}",
        budget.max_total_steps
    );
    assert!(budget.max_transitions >= executed);
    assert!(budget.max_trace_events >= executed);
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, E::StepFailed { step, .. } if *step == failed))
            .count(),
        expected_iterations,
        "{yaml}"
    );
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, E::StepSucceeded { step, .. } if *step == handler))
            .count(),
        expected_iterations,
        "{yaml}"
    );
    let writes = events
        .iter()
        .filter_map(|event| match event {
            E::SlotWrittenEvent { slot, value, .. } if *slot == handler_output => Some(value),
            _ => None,
        })
        .map(|value| -> TestResult<SlotValue> {
            Ok(postcard::from_bytes(
                value.as_deref().ok_or("missing recovery payload")?,
            )?)
        })
        .collect::<TestResult<Vec<_>>>()?;
    assert_eq!(
        writes,
        vec![SlotValue::I64(7); expected_iterations],
        "{yaml}"
    );
    let results = events
        .iter()
        .filter_map(|event| match event {
            E::RunFinished { result, .. } => Some(*result),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(results.len(), 1);
    let result = results.first().ok_or("result")?;
    let encoded = events
        .iter()
        .rev()
        .find_map(|event| match event {
            E::SlotWrittenEvent { slot, value, .. } if slot == result => Some(value),
            _ => None,
        })
        .ok_or("result write")?;
    assert_eq!(
        postcard::from_bytes::<SlotValue>(encoded.as_deref().ok_or("missing result payload")?)?,
        SlotValue::I64(0)
    );
    Ok(())
}

#[test]
fn yaml_every_nested_loop_pair_reexecutes_actual_owned_input_recovery() -> TestResult {
    for outer in FAMILIES {
        for inner in FAMILIES {
            let inner = loop_source(inner, "inner", RECOVERED_BODY);
            let body = loop_source(
                outer,
                "outer",
                &(inner + "- id: outer_tail\n  set: {output: outer_value, value: '7'}\n"),
            );
            let yaml = source_with(DECLARATIONS, &body, "$zero");
            for quantum in [1, 64] {
                exercise(&yaml, quantum, 4)?;
            }
        }
    }
    Ok(())
}

fn abandoned_inner(family: Family) -> TestResult {
    let failing = "- id: failing\n  set: {output: quotient, value: '1 / $zero'}\n";
    let child = loop_source(family, "inner", failing).replacen(
        "- id: inner\n",
        "- id: inner\n  on_error: {handler: recovered}\n",
        1,
    );
    let body = repeated(
        "outer",
        &(child + "- id: recovered\n  set: {output: recovered, value: '7'}\n"),
    );
    let yaml = source_with(DECLARATIONS, &body, "$zero");
    for quantum in [1, 64] {
        exercise(&yaml, quantum, 2)?;
    }
    Ok(())
}

#[test]
fn yaml_parent_repeat_rearms_abandoned_for_each() -> TestResult {
    abandoned_inner(Family::ForEach)
}
#[test]
fn yaml_parent_repeat_rearms_abandoned_collect() -> TestResult {
    abandoned_inner(Family::Collect)
}
#[test]
fn yaml_parent_repeat_rearms_abandoned_reduce() -> TestResult {
    abandoned_inner(Family::Reduce)
}
#[test]
fn yaml_parent_repeat_rearms_abandoned_repeat() -> TestResult {
    abandoned_inner(Family::Repeat)
}

#[test]
fn yaml_parent_repeat_recovers_child_start_capacity_failure_with_admissible_owned_list()
-> TestResult {
    let child = "- id: inner\n  on_error: {handler: recovered}\n  for_each:\n    variable: entry_value\n    input: $input.items\n    at_once: 1\n    steps:\n      - id: bypassed\n        set: {output: bypassed, value: '11'}\n- id: recovered\n  set: {output: recovered, value: '7'}\n";
    let yaml = source_with(
        "inputs: {items: {is: list, of: any, max: 2}}\n",
        &repeated("outer", child),
        "$zero",
    );
    for quantum in [1, 64] {
        exercise_with_capacity(&yaml, quantum, 2, true)?;
    }
    Ok(())
}
