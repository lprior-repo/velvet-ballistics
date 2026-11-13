#[path = "controls/cases.rs"]
mod cases;
#[path = "controls/counterfactual.rs"]
mod counterfactual;
#[path = "controls/helpers.rs"]
mod helpers;
#[path = "controls/write_checks.rs"]
mod write_checks;

use super::harness::{Scratch, step};
use super::support::{self, Durability, TestResult};
use cases::{ControlCase, cases};
use counterfactual::assert_completed_control_rejected;
use helpers::{assert_remaining_control_work, config, cut_inside_control, written_values};
use vb_core::frame::CheckpointStepState;
use vb_core::{RunId, RuntimePolicy, SlotValue, StepIdx, Taint};
use vb_storage::JournalEvent;
use write_checks::{assert_body_write_provenance, assert_collect_pages, capture_slot_write_stores};

struct RestoreCollectClock;

impl Drop for RestoreCollectClock {
    fn drop(&mut self) {
        vb_runtime::primitives::collect::restore_real_time_source_for_test();
    }
}

#[test]
fn direct_api_active_control_cut_cold_recovery_preserves_literal_execution() -> TestResult {
    vb_runtime::primitives::collect::install_deterministic_time_source_for_test(
        vb_core::time::DeterministicTimeSource::new(1_700_000_000_000),
    );
    let _restore_clock = RestoreCollectClock;
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
    assert_remaining_control_work(&workflow, outer, &live_frame, &live_store)?;
    assert_completed_control_rejected(&workflow, outer, &live_frame, &live_store)?;
    let product_before = live.runtime.recover_product(run)?;
    let oracle_dir = Scratch::new("oracle")?;
    let mut oracle =
        support::open_runtime_with_config(oracle_dir.path(), config(), Durability::Strict)?;
    support::persist_artifact(&oracle.journal, &workflow, RuntimePolicy::Strict, &[])?;
    oracle.runtime.submit_compiled(run, workflow.clone())?;
    let (oracle_frame, oracle_store, oracle_prefix) = cut_inside_control(
        &mut oracle.runtime,
        &oracle.journal,
        run,
        outer,
        body,
        finish,
    )?;
    assert_eq!(
        oracle_frame, live_frame,
        "independent live frame differed at cut"
    );
    assert_eq!(
        oracle_store, live_store,
        "independent live store differed at cut"
    );
    assert_eq!(
        oracle_prefix, prefix,
        "independent live event prefix differed at cut"
    );
    oracle.runtime.tick_all()?;
    let oracle_state = oracle.runtime.inspect_run_state(run)?;
    let expected_frame = oracle_state.frame.checkpoint_v1();
    let expected_store = oracle_state.store.checkpoint_v1();
    let expected_events = support::events(&oracle.journal, run)?;
    drop(oracle);
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
    reopened.runtime.recover_and_resume(run)?;
    reopened.runtime.tick_all()?;
    let recovered = reopened.runtime.inspect_run_state(run)?;
    assert_eq!(
        recovered.frame.checkpoint_v1(),
        expected_frame,
        "recovered frame diverged from uninterrupted live execution for {}",
        case.name
    );
    assert_eq!(
        recovered.store.checkpoint_v1(),
        expected_store,
        "recovered store diverged from uninterrupted live execution for {}",
        case.name
    );
    assert_eq!(
        support::events(&reopened.journal, run)?,
        expected_events,
        "recovered event sequence diverged from uninterrupted execution for {}",
        case.name
    );
    let collect_slot = if case.source.contains("collect:") {
        let output = *output_steps.first().ok_or("Collect has no body producer")?;
        Some(
            workflow
                .node(output)
                .and_then(|node| node.output.slot())
                .ok_or("Collect body has no output slot")?,
        )
    } else {
        None
    };
    let mut collect_observations = Vec::new();
    if let Some(slot) = collect_slot {
        capture_slot_write_stores(&prefix, run, slot, &live_store, &mut collect_observations);
        for _ in 0..512 {
            reopened.runtime.tick_all()?;
            let state = reopened.runtime.inspect_run_state(run)?;
            let checkpoint = state.store.checkpoint_v1();
            let events = support::events(&reopened.journal, run)?;
            capture_slot_write_stores(&events, run, slot, &checkpoint, &mut collect_observations);
            if collect_observations.len() == 2 {
                break;
            }
        }
        assert_eq!(
            collect_observations.len(),
            2,
            "Collect did not capture one store checkpoint per body write"
        );
    }
    support::tick_until_terminal(&mut reopened.runtime, run, 512)?;
    let actual = support::events(&reopened.journal, run)?;
    support::assert_ordering(&actual, run)?;
    let result = support::terminal(&actual, run)?;
    let expected_result = if case.source.contains("collect:") {
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
        let writes = written_values(&actual, run, slot)?;
        let producers = match &node.kind {
            vb_core::CompiledNodeKind::ReduceStart { done, .. } => {
                vec![outer, body, body, body, *done]
            }
            _ => vec![output_step; writes.len()],
        };
        assert_body_write_provenance(&workflow, &actual, &producers, run, slot, &writes)?;
        if case.source.contains("collect:") {
            assert_collect_pages(&collect_observations, &writes)?;
        } else {
            let values = writes
                .iter()
                .map(|(_, value, _, _)| *value)
                .collect::<Vec<_>>();
            assert_eq!(values, *expected, "ordered writes for step {name}");
            assert!(
                writes
                    .iter()
                    .all(|(_, _, taint, attempt)| *taint == Taint::Clean && *attempt == 1),
                "body writes must carry literal Clean taint and attempt 1: {writes:?}"
            );
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
