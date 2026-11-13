#![forbid(unsafe_code)]

use vb_compile::compile_workflow;
use vb_core::engine::StepBudget;
use vb_core::frame::RunFrame;
use vb_core::ids::{RunId, StepIdx};
use vb_core::value::SlotValue;
use vb_core::value_store::ValueStore;
use vb_runtime::engine::{EvidenceCollector, RetryPolicy, RuntimeSignal, drive_deterministic_full};
use vb_runtime::primitives::collect::CollectStates;

const SOURCE: &str = "version: velvet-ballistics/v1\nname: nested_parallel_runtime\nwhen: {manual: {}}\ninputs:\n  outer_items: {is: list, of: number, max: 4}\n  inner_items: {is: list, of: number, max: 4}\nsteps:\n  - id: outer\n    for_each:\n      input: '$input.outer_items'\n      variable: entry\n      at_once: 2\n      steps:\n        - id: inner\n          for_each:\n            input: '$input.inner_items'\n            variable: sub\n            at_once: 2\n            steps:\n              - id: leaf\n                set: {output: picked, value: '$entry + $sub'}\n  - id: done\n    finish: {result: '$outer'}\n";

#[path = "nested_fixed_group_budget.rs"]
mod nested_fixed_group_budget;
#[test]
fn nested_concurrent_groups_resume_under_each_small_budget() -> Result<(), String> {
    let plan = compile_workflow(SOURCE.as_bytes()).map_err(|errors| format!("{errors:?}"))?;
    let mut run = RunFrame::new(RunId::new(1), StepIdx::new(0), plan.node_count(), plan.slot_count())
        .map_err(|error| format!("{error}"))?;
    let mut store = ValueStore::new();
    let mut evidence = EvidenceCollector::new();
    let mut collect_states = CollectStates::new();
    let granted = vb_core::capability::CapabilitySet::empty();
    let slots = plan.input_slots();
    let outer = store
        .insert_list(Box::from([SlotValue::I64(1), SlotValue::I64(2)]))
        .map_err(|error| format!("{error}"))?;
    let inner = store
        .insert_list(Box::from([SlotValue::I64(10), SlotValue::I64(20)]))
        .map_err(|error| format!("{error}"))?;
    let outer_slot = slots.first().ok_or("missing outer input")?.slot;
    let inner_slot = slots.get(1).ok_or("missing inner input")?.slot;
    run.write_slot(outer_slot, SlotValue::List(outer))
        .map_err(|error| format!("{error}"))?;
    run.write_slot(inner_slot, SlotValue::List(inner))
        .map_err(|error| format!("{error}"))?;

    let mut finished = None;
    for budget_size in [1_u64, 2, 4].into_iter().cycle().take(120) {
        let mut budget = StepBudget::new(budget_size);
        let signal = drive_deterministic_full(
            &plan,
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
        .map_err(|error| format!("{error:?}"))?;
        if let RuntimeSignal::Finished(value) = signal {
            finished = Some(value);
            break;
        }
        if signal != RuntimeSignal::StepBudgetExhausted {
            return Err(format!("unexpected nested-drive signal: {signal:?}"));
        }
    }
    let Some(SlotValue::List(outer_result)) = finished else {
        return Err("nested concurrent source did not finish with a list".into());
    };
    let outer_values = store.list(outer_result).map_err(|error| error.to_string())?;
    assert_eq!(outer_values.len(), 2);
    for (ordinal, expected) in [(0_u32, [11_i64, 21]), (1_u32, [12_i64, 22])] {
        let (outer_item, outer_taint) = store
            .list_item_with_taint(outer_result, ordinal)
            .map_err(|error| error.to_string())?;
        assert_eq!(outer_taint, vb_core::value::Taint::Clean);
        let SlotValue::List(inner_result) = outer_item else {
            return Err(format!("outer result {ordinal} was not a list: {outer_item:?}"));
        };
        let inner_values = store.list(inner_result).map_err(|error| error.to_string())?;
        assert_eq!(inner_values, &[SlotValue::I64(expected[0]), SlotValue::I64(expected[1])]);
        for inner_ordinal in [0_u32, 1] {
            let (_, inner_taint) = store
                .list_item_with_taint(inner_result, inner_ordinal)
                .map_err(|error| error.to_string())?;
            assert_eq!(inner_taint, vb_core::value::Taint::Clean);
        }
    }
    Ok(())
}
