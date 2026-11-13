#![forbid(unsafe_code)]
//! End-to-end execution of whole-source parallel `for_each` through the runtime.

use vb_compile::compile_workflow;
use vb_core::engine::StepBudget;
use vb_core::frame::RunFrame;
use vb_core::ids::{RunId, StepIdx};
use vb_core::value::SlotValue;
use vb_core::value_store::ValueStore;
use vb_core::workflow::CompiledWorkflow;
use vb_runtime::engine::{
    EvidenceCollector, RetryPolicy, RuntimeSignal, drive_deterministic_full,
};
use vb_runtime::primitives::collect::CollectStates;

const HEADER: &str =
    "version: velvet-ballistics/v1\nname: parallel_runtime\nwhen: {manual: {}}\n";

fn source(at_once: &str) -> String {
    format!(
        "{HEADER}inputs:\n  items: {{is: list, of: any, max: 4}}\nsteps:\n  - id: seed\n    set: {{output: base, value: '1'}}\n  - id: outer\n    for_each:\n      input: '$input.items'\n      variable: entry\n      at_once: {at_once}\n      steps:\n        - id: leaf\n          set: {{output: picked, value: '$entry'}}\n  - id: done\n    finish: {{result: '$outer'}}\n"
    )
}

fn compile(at_once: &str) -> Result<CompiledWorkflow, String> {
    compile_workflow(source(at_once).as_bytes()).map_err(|errors| format!("{errors:?}"))
}

fn drive(plan: &CompiledWorkflow, items: &[i64]) -> Result<Vec<i64>, String> {
    let slot_count = plan.slot_count();
    let mut run = RunFrame::new(RunId::new(1), StepIdx::new(0), plan.node_count(), slot_count)
        .map_err(|error| format!("{error}"))?;
    let mut store = ValueStore::new();
    let mut budget = StepBudget::new(10_000);
    let mut evidence = EvidenceCollector::new();
    let mut collect_states = CollectStates::new();
    let granted = vb_core::capability::CapabilitySet::empty();

    let input = plan
        .input_slots()
        .first()
        .ok_or("missing input slot")?
        .slot;
    let values: Vec<SlotValue> = items.iter().map(|value| SlotValue::I64(*value)).collect();
    let list = store
        .insert_list(values.into_boxed_slice())
        .map_err(|error| format!("{error}"))?;
    run.write_slot(input, SlotValue::List(list))
        .map_err(|error| format!("{error}"))?;

    let signal = drive_deterministic_full(
        plan,
        &mut run,
        &mut budget,
        &mut store,
        &[],
        RetryPolicy::NEVER,
        &mut evidence,
        &mut collect_states,
        &granted,
        1,
    )
    .map_err(|error| format!("{error}"))?;
    let RuntimeSignal::Finished(SlotValue::List(results)) = signal else {
        return Err(format!("unexpected signal: {signal:?}"));
    };
    let values = store
        .list(results)
        .map_err(|error| format!("{error}"))?;
    values
        .iter()
        .map(|value| match value {
            SlotValue::I64(number) => Ok(*number),
            other => Err(format!("unexpected item: {other:?}")),
        })
        .collect()
}

#[test]
fn parallel_for_each_returns_ordered_owned_results() -> Result<(), String> {
    let plan = compile("2")?;
    assert_eq!(drive(&plan, &[10, 20, 30])?, vec![10, 20, 30]);
    Ok(())
}

#[test]
fn at_once_one_and_raw_max_produce_identical_order() -> Result<(), String> {
    let serial = compile("1")?;
    let capped = compile("4294967295")?;
    assert_eq!(drive(&serial, &[4, 5, 6])?, vec![4, 5, 6]);
    assert_eq!(drive(&capped, &[4, 5, 6])?, vec![4, 5, 6]);
    Ok(())
}

#[test]
fn zero_item_parallel_for_each_finishes_with_empty_list() -> Result<(), String> {
    let plan = compile("2")?;
    assert_eq!(drive(&plan, &[])?, Vec::<i64>::new());
    Ok(())
}

#[test]
fn repeated_activation_on_fresh_frames_is_deterministic() -> Result<(), String> {
    let plan = compile("3")?;
    let first = drive(&plan, &[7, 8])?;
    let second = drive(&plan, &[7, 8])?;
    assert_eq!(first, vec![7, 8]);
    assert_eq!(second, first);
    Ok(())
}
