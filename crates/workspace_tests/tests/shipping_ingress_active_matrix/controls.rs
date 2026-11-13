use vb_core::frame::{CheckpointStepState, RunFrameCheckpointV1};
use vb_core::value_store::ValueStoreCheckpointV1;
use vb_core::{RunId, RuntimePolicy, SlotIdx, SlotValue, StepIdx, StepState, Taint};
use vb_runtime::runtime::Runtime;
use vb_storage::JournalEvent;

use super::harness::{Scratch, step};
use super::support::{self, Durability, TestResult};

const HEADER: &str = "version: velvet-ballistics/v1\nname: ingress_active_control\nwhen: {manual: {}}\nvars: {items: [1, 2, 3]}\n";

struct ControlCase {
    name: &'static str,
    body: &'static str,
    source: String,
    outputs: Vec<(&'static str, Vec<SlotValue>)>,
}

fn cases() -> Vec<ControlCase> {
    vec![
        ControlCase {
            name: "outer",
            body: "body",
            source: format!(
                "{HEADER}steps:\n  - id: seed\n    set: {{output: seed, value: '42'}}\n  - id: outer\n    for_each:\n      variable: entry\n      input: '$vars.items'\n      at_once: 1\n      steps:\n        - id: body\n          set: {{output: seen, value: '$entry'}}\n  - id: done\n    finish: {{result: '$step.seed'}}\n"
            ),
            outputs: vec![(
                "body",
                vec![SlotValue::I64(1), SlotValue::I64(2), SlotValue::I64(3)],
            )],
        },
        ControlCase {
            name: "outer",
            body: "body",
            source: format!(
                "{HEADER}steps:\n  - id: seed\n    set: {{output: seed, value: '42'}}\n  - id: outer\n    reduce:\n      variable: total\n      input: '$vars.items'\n      initial: '0'\n      steps:\n        - id: body\n          set: {{output: running, value: '$total + 1'}}\n  - id: done\n    finish: {{result: '$step.seed'}}\n"
            ),
            outputs: vec![(
                "body",
                vec![
                    SlotValue::I64(0),
                    SlotValue::I64(1),
                    SlotValue::I64(2),
                    SlotValue::I64(3),
                    SlotValue::I64(3),
                ],
            )],
        },
        ControlCase {
            name: "outer",
            body: "left_value",
            source: format!(
                "{HEADER}steps:\n  - id: seed\n    set: {{output: seed, value: '42'}}\n  - id: outer\n    together:\n      branches:\n        - label: left\n          steps:\n            - id: left_value\n              set: {{output: left, value: '7'}}\n        - label: right\n          steps:\n            - id: right_value\n              set: {{output: right, value: '9'}}\n  - id: done\n    finish: {{result: '$step.seed'}}\n"
            ),
            outputs: vec![
                ("left_value", vec![SlotValue::I64(7)]),
                ("right_value", vec![SlotValue::I64(9)]),
            ],
        },
        ControlCase {
            name: "outer",
            body: "body",
            source: format!(
                "{HEADER}steps:\n  - id: seed\n    set: {{output: seed, value: '42'}}\n  - id: outer\n    repeat:\n      max_attempts: 2\n      steps:\n        - id: body\n          set: {{output: tick, value: '7'}}\n  - id: done\n    finish: {{result: '$step.seed'}}\n"
            ),
            outputs: vec![("body", vec![SlotValue::I64(7), SlotValue::I64(7)])],
        },
        ControlCase {
            name: "outer",
            body: "body",
            source: format!(
                "{HEADER}steps:\n  - id: seed\n    set: {{output: seed, value: '42'}}\n  - id: outer\n    collect:\n      variable: batch\n      source: '$vars.items'\n      pages: 2\n      items: 2\n      steps:\n        - id: body\n          set: {{output: copied, value: '$batch'}}\n  - id: total\n    set: {{output: total, value: 'sum($step.outer)'}}\n  - id: done\n    finish: {{result: '$step.total'}}\n"
            ),
            outputs: vec![(
                "body",
                vec![SlotValue::I64(1), SlotValue::I64(2), SlotValue::I64(3)],
            )],
        },
    ]
}

#[test]
fn direct_api_active_control_cut_cold_recovery_preserves_literal_execution() -> TestResult {
    for (case_index, case) in cases().into_iter().enumerate() {
        exercise_case(case_index, case)?;
    }
    Ok(())
}

fn exercise_case(case_index: usize, case: ControlCase) -> TestResult {
    let workflow = support::compile(&case.source)?;
    let outer_index = workflow
        .to_parts()
        .nodes
        .iter()
        .position(|node| {
            matches!(
                node.kind,
                vb_core::CompiledNodeKind::ForEachStart { .. }
                    | vb_core::CompiledNodeKind::ReduceStart { .. }
                    | vb_core::CompiledNodeKind::RepeatStart { .. }
                    | vb_core::CompiledNodeKind::CollectStart { .. }
                    | vb_core::CompiledNodeKind::TogetherStart { .. }
            )
        })
        .ok_or("compiled workflow has no supported control start")?;
    let outer = StepIdx::new(u16::try_from(outer_index)?);
    let node = workflow
        .node(outer)
        .ok_or("control start node is missing")?;
    let body = match &node.kind {
        vb_core::CompiledNodeKind::ForEachStart { body, .. }
        | vb_core::CompiledNodeKind::ReduceStart { body, .. }
        | vb_core::CompiledNodeKind::RepeatStart { body, .. }
        | vb_core::CompiledNodeKind::CollectStart { body, .. } => *body,
        vb_core::CompiledNodeKind::TogetherStart { branches, .. } => {
            *branches.first().ok_or("together control has no branch")?
        }
        _ => return Err(format!("named outer step kind is {:?}", node.kind).into()),
    };
    let output_steps = match &node.kind {
        vb_core::CompiledNodeKind::TogetherStart { branches, .. } => branches
            .iter()
            .map(|branch| {
                workflow
                    .node(*branch)
                    .and_then(|node| match node.kind {
                        vb_core::CompiledNodeKind::TogetherBranch { entry, .. } => Some(entry),
                        _ => None,
                    })
                    .ok_or("together branch has no compiled entry")
            })
            .collect::<Result<Vec<_>, _>>()?,
        _ => vec![body],
    };
    let finish = step(&workflow, "done")?;
    let run = RunId::new(0xA100 + u64::try_from(case_index)?);
    let live_dir = Scratch::new("active")?;
    let mut live =
        support::open_runtime_with_config(live_dir.path(), config(), Durability::Strict)?;
    support::persist_artifact(&live.journal, &workflow, RuntimePolicy::Strict, &[])?;
    live.runtime.submit_compiled(run, workflow.clone())?;

    let (live_frame, live_store, prefix) =
        cut_inside_control(&mut live.runtime, &live.journal, run, outer, body, finish)?;
    assert_eq!(live_frame.run_id, run);
    assert!(live_frame.executed > 0);
    assert_eq!(
        live_frame.states.get(usize::from(outer.get())),
        Some(&CheckpointStepState::Succeeded)
    );
    assert_eq!(
        live_frame.states.get(usize::from(body.get())),
        Some(&CheckpointStepState::Succeeded)
    );
    assert_eq!(
        live_frame.states.get(usize::from(finish.get())),
        Some(&CheckpointStepState::Pending)
    );
    assert!(!live_store.lists.is_empty());
    let product_before = live.runtime.recover_product(run)?;
    drop(live);

    let mut reopened =
        support::open_runtime_with_config(live_dir.path(), config(), Durability::Strict)?;
    let product_after = reopened.runtime.recover_product(run)?;
    assert_eq!(
        product_after, product_before,
        "cold recovery changed the full frame/store/control product for {}",
        case.name
    );
    assert_eq!(
        support::events(&reopened.journal, run)?,
        prefix,
        "cold open mutated durable history before resume"
    );
    if case.source.contains("collect:") {
        assert!(checkpoint_has_list(
            &live_store,
            &[SlotValue::I64(1), SlotValue::I64(2)]
        ));
    }
    reopened.runtime.recover_and_resume(run)?;
    if case.source.contains("collect:") {
        let mut saw_last_page = false;
        for _ in 0..512 {
            reopened.runtime.tick_all()?;
            let state = reopened.runtime.inspect_run_state(run)?;
            if checkpoint_has_list(&state.store.checkpoint_v1(), &[SlotValue::I64(3)]) {
                saw_last_page = true;
                break;
            }
        }
        assert!(
            saw_last_page,
            "recovered Collect omitted its final [3] page"
        );
    }
    support::tick_until_terminal(&mut reopened.runtime, run, 512)?;
    let mut actual = support::events(&reopened.journal, run)?;
    if case.source.contains("collect:") {
        let slot = workflow.node(body).and_then(|node| node.output.slot()).ok_or("body slot")?;
        let wrong_index = live_store.lists.iter().position(|list| list.as_ref() == [SlotValue::I64(1), SlotValue::I64(2), SlotValue::I64(3)]).ok_or("input list")?;
        let wrong_value = SlotValue::List(vb_core::ids::ListId::new(u32::try_from(wrong_index)?));
        let mut changed = 0;
        for event in actual.iter_mut().skip(prefix.len()) {
            if let JournalEvent::SlotWrittenEvent { slot: written, value, extra, attempt, .. } = event
                && *written == slot
            {
                *value = Some(postcard::to_allocvec(&wrong_value)?);
                *extra = Some(vb_storage::slot_extra::encode_slot_written_extra(Taint::Secret, None).map_err(|error| format!("{error:?}"))?);
                *attempt = 99;
                changed += 1;
            }
        }
        assert_eq!(changed, 1, "probe must corrupt exactly the final body write");
    }
    support::assert_ordering(&actual, run)?;
    let result = support::terminal(&actual, run)?;
    let expected_result = if case.name == "outer" && case.source.contains("collect:") {
        SlotValue::I64(6)
    } else {
        SlotValue::I64(42)
    };
    assert_eq!(
        result.value, expected_result,
        "terminal value for {}",
        case.body
    );
    assert_eq!(
        result.taint,
        Taint::Clean,
        "terminal taint for {}",
        case.body
    );

    for (index, (name, expected)) in case.outputs.iter().enumerate() {
        let output_step = *output_steps
            .get(index)
            .ok_or("control output has no corresponding compiled body step")?;
        let slot = workflow
            .node(output_step)
            .and_then(|node| node.output.slot())
            .ok_or_else(|| format!("step {name} has no output slot"))?;
        let values = written_values(&actual, run, slot)?;
        if case.source.contains("collect:") {
            assert_eq!(values.len(), 2, "Collect should write one batch per page");
            assert!(
                values
                    .iter()
                    .all(|value| matches!(value, SlotValue::List(_))),
                "Collect body must write page lists: {values:?}"
            );
        } else {
            assert_eq!(values, *expected, "ordered writes for step {name}");
        }
        let successes = actual.iter().filter(|event| matches!(event, JournalEvent::StepSucceeded { run: event_run, step, .. } if *event_run == run && *step == output_step)).count();
        let expected_successes = if case.source.contains("reduce:") {
            3
        } else if case.source.contains("collect:") {
            2
        } else {
            expected.len()
        };
        assert_eq!(
            successes, expected_successes,
            "success count for step {name}"
        );
    }
    assert_eq!(
        actual.get(..prefix.len()),
        Some(prefix.as_slice()),
        "recovery changed the ordered durable event prefix"
    );
    assert_eq!(
        prefix.first(),
        actual.first(),
        "accepted event identity changed"
    );
    assert!(!prefix.iter().any(|event| matches!(event, JournalEvent::RunFinished { run: event_run, .. } if *event_run == run)), "run finished before the in-flight cut");
    Ok(())
}

fn config() -> vb_runtime::shard::ShardConfig {
    vb_runtime::shard::ShardConfig {
        command_queue_capacity: 32,
        trace_capacity: 64,
        step_budget_per_tick: 1,
        max_active_runs: 8,
        policy: RuntimePolicy::Strict,
    }
}

fn cut_inside_control(
    runtime: &mut Runtime,
    journal: &vb_storage::FjallJournal,
    run: RunId,
    outer: StepIdx,
    body: StepIdx,
    finish: StepIdx,
) -> TestResult<(
    RunFrameCheckpointV1,
    ValueStoreCheckpointV1,
    Vec<JournalEvent>,
)> {
    let mut observations = Vec::new();
    for tick in 0..512 {
        runtime.tick_all()?;
        let state = runtime.inspect_run_state(run).map_err(|error| {
            format!(
                "active cut lost run {run:?} at tick {tick}; target steps outer={outer:?} body={body:?} finish={finish:?}; frame states: {observations:?}; error: {error:?}"
            )
        })?;
        observations.push(state.frame.checkpoint_v1().states);
        if state.frame.step_state(body)? == StepState::Succeeded
            && state.frame.step_state(finish)? == StepState::Pending
            && (run != RunId::new(0xA100) || state.frame.pc() == finish)
        {
            return Ok((
                state.frame.checkpoint_v1(),
                state.store.checkpoint_v1(),
                support::events(journal, run)?,
            ));
        }
    }
    Err(format!("run {run:?} did not reach an active control cut").into())
}

fn checkpoint_has_list(checkpoint: &ValueStoreCheckpointV1, expected: &[SlotValue]) -> bool {
    checkpoint
        .lists
        .iter()
        .any(|values| values.as_ref() == expected)
}

fn written_values(
    events: &[JournalEvent],
    run: RunId,
    slot: SlotIdx,
) -> TestResult<Vec<SlotValue>> {
    events
        .iter()
        .filter_map(|event| match event {
            JournalEvent::SlotWrittenEvent {
                run: event_run,
                slot: event_slot,
                value,
                ..
            } if *event_run == run && *event_slot == slot => Some(value.as_deref()),
            _ => None,
        })
        .map(|bytes| {
            let encoded = bytes.ok_or("slot write omitted its serialized value")?;
            Ok(postcard::from_bytes::<SlotValue>(encoded)?)
        })
        .collect()
}
