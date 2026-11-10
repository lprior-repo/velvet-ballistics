#[path = "vb_f2_signal_body_results/support.rs"]
mod support;
use support::*;
use vb_core::SlotValue;

#[test]
fn numeric_items_keep_ancestor_reads_before_the_shadowing_result_is_published() -> TestResult {
    let nested = "- id: scope\n  repeat:\n    max_attempts: 1\n    steps:\n      - id: seed\n        for_each:\n          variable: entry\n          input: $vars.xs\n          steps:\n            - id: projected\n              set: {output: projected, value: '$entry + $step.seed - 42'}\n- id: checked\n  set: {output: checked, value: 'count($step.scope) == 3 and $step.scope.0 == 3 and $step.scope.1 == 1 and $step.scope.2 == 2 and $step.seed == 42'}\n";
    for quantum in [1, 64] {
        let outcome = execute(
            &source("vars: {xs: [3, 1, 2]}\n", nested, "$step.checked"),
            quantum,
            None,
        )?;
        assert_eq!(outcome.value, SlotValue::Bool(true));
        assert_eq!((outcome.asks, outcome.waits), (0, 0));
        assert_eq!(
            outcome
                .events
                .iter()
                .filter(|event| matches!(
                    event,
                    vb_runtime::journal::RuntimeJournalEvent::RunFinished { .. }
                ))
                .count(),
            1
        );
    }
    let declared = "version: velvet-ballistics/v1\nname: declared_numeric_items\nwhen: {manual: {}}\ninputs: {items: {is: list, of: number, max: 2}}\nsteps:\n  - id: seed\n    set: {output: saved, value: '99'}\n  - id: scope\n    repeat:\n      max_attempts: 1\n      steps:\n        - id: seed\n          for_each:\n            variable: entry\n            input: $input.items\n            steps:\n              - id: projected\n                set: {output: projected, value: '$entry + $step.seed - 99'}\n  - id: done\n    finish: {result: '$step.scope'}\n";
    vb_compile::compile_workflow(declared.as_bytes())?;
    Ok(())
}

#[test]
fn declared_number_items_accept_both_runtime_numeric_representations() -> TestResult {
    use vb_core::{CapabilitySet, FiniteF64, RunId, StepBudget, Taint, ValueStore, new_run_frame};
    use vb_runtime::engine::{
        EvidenceCollector, RetryPolicy, RuntimeSignal, drive_deterministic_full,
    };
    use vb_runtime::primitives::collect::CollectStates;
    let yaml = "version: velvet-ballistics/v1\nname: numeric_elements\nwhen: {manual: {}}\ninputs: {items: {is: list, of: number, max: 3}}\nsteps:\n  - id: loop\n    for_each:\n      variable: entry\n      input: $input.items\n      steps:\n        - id: projected\n          set: {output: projected, value: '$entry + 1'}\n  - id: done\n    finish: {result: '$step.loop'}\n";
    let plan = vb_compile::compile_workflow(yaml.as_bytes())?;
    for quantum in [1, 64] {
        let mut frame = new_run_frame(RunId::new(1), &plan)?;
        let mut store = ValueStore::new();
        let values = vec![
            SlotValue::I64(3),
            SlotValue::F64(FiniteF64::new(1.5)?),
            SlotValue::I64(2),
        ];
        let source = store.insert_list(values.clone())?;
        frame.write_slot_with_taint(
            plan.input_slots().first().ok_or("input")?.slot,
            SlotValue::List(source),
            Taint::Clean,
        )?;
        let mut evidence = EvidenceCollector::new();
        let mut collect = CollectStates::new();
        let mut observed = None;
        for _ in 0..128 {
            let signal = drive_deterministic_full(
                &plan,
                &mut frame,
                &mut StepBudget::new(quantum),
                &mut store,
                &[],
                RetryPolicy::NEVER,
                &mut evidence,
                &mut collect,
                &CapabilitySet::empty(),
                1,
            )?;
            if let RuntimeSignal::Finished(SlotValue::List(result)) = signal {
                observed = Some(store.list(result)?.to_vec());
                break;
            }
            assert_eq!(signal, RuntimeSignal::StepBudgetExhausted);
        }
        assert_eq!(
            observed,
            Some(vec![
                SlotValue::I64(4),
                SlotValue::F64(FiniteF64::new(2.5)?),
                SlotValue::I64(3)
            ])
        );
        assert_eq!(store.list(source)?, values.as_slice());
    }
    Ok(())
}

#[test]
fn result_root_retains_source_taint_for_empty_and_constant_bodies() -> TestResult {
    use vb_core::{CapabilitySet, RunId, StepBudget, Taint, ValueStore, new_run_frame};
    use vb_runtime::engine::{
        EvidenceCollector, RetryPolicy, RuntimeSignal, drive_deterministic_full,
    };
    use vb_runtime::primitives::collect::CollectStates;
    let yaml = "version: velvet-ballistics/v1\nname: source_taint\nwhen: {manual: {}}\ninputs: {items: {is: list, of: number, max: 2}}\nsteps:\n  - id: loop\n    for_each:\n      variable: entry\n      input: $input.items\n      steps:\n        - id: projected\n          set: {output: projected, value: '7'}\n  - id: done\n    finish: {result: '$step.loop'}\n";
    let plan = vb_compile::compile_workflow(yaml.as_bytes())?;
    let result = plan
        .to_parts()
        .nodes
        .iter()
        .find(|node| matches!(node.kind, vb_core::CompiledNodeKind::ForEachJoin { .. }))
        .and_then(|node| node.output.slot())
        .ok_or("result")?;
    for values in [Vec::new(), vec![SlotValue::I64(3), SlotValue::I64(5)]] {
        let mut frame = new_run_frame(RunId::new(1), &plan)?;
        let mut store = ValueStore::new();
        let input = store.insert_list(values.clone())?;
        frame.write_slot_with_taint(
            plan.input_slots().first().ok_or("input")?.slot,
            SlotValue::List(input),
            Taint::Secret,
        )?;
        let signal = drive_deterministic_full(
            &plan,
            &mut frame,
            &mut StepBudget::new(128),
            &mut store,
            &[],
            RetryPolicy::NEVER,
            &mut EvidenceCollector::new(),
            &mut CollectStates::new(),
            &CapabilitySet::empty(),
            1,
        )?;
        let RuntimeSignal::Finished(SlotValue::List(output)) = signal else {
            return Err(format!("unexpected {signal:?}").into());
        };
        assert_eq!(
            store.list(output)?,
            vec![SlotValue::I64(7); values.len()].as_slice()
        );
        assert_eq!(frame.read_taint(result)?, Taint::Secret);
        assert_eq!(store.list(input)?, values.as_slice());
    }
    Ok(())
}
