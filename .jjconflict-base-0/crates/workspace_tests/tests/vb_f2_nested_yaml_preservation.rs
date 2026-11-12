#![forbid(unsafe_code)]
//! Source-compiled nested activation across one-step budget boundaries.

use vb_compile::compile_workflow;
use vb_core::{
    CapabilitySet, CompiledNodeKind, ConstValue, RunId, SlotValue, StepBudget, Taint, ValueStore,
    new_run_frame,
};
use vb_runtime::engine::{
    EvidenceCollector, EvidenceEvent, RetryPolicy, RuntimeSignal, drive_deterministic_full,
};
use vb_runtime::primitives::collect::CollectStates;

const SOURCE: &str = r#"version: velvet-ballistics/v1
name: nested_preservation
when: {manual: {}}
inputs:
  items: {is: list, of: any, max: 2}
steps:
  - id: first
    set: {output: answer, value: '42'}
  - id: second
    set: {output: sentinel, value: '99'}
  - id: outer
    for_each:
      variable: entry
      input: $input.items
      at_once: 1
      steps:
        - id: inner
          repeat:
            max_attempts: 2
            steps:
              - id: inner_value
                set: {output: inside, value: '7'}
              - id: inner_next
                set: {output: inside_next, value: '$inside + 1'}
  - id: done
    finish: {result: answer}
"#;

#[test]
fn nested_yaml_preserves_inputs_and_authored_slots_across_budget_cuts()
-> Result<(), Box<dyn std::error::Error>> {
    for values in [
        vec![],
        vec![SlotValue::I64(3)],
        vec![SlotValue::I64(3), SlotValue::I64(5)],
    ] {
        let workflow = compile_workflow(SOURCE.as_bytes())?;
        let original = workflow.to_parts();
        let input = workflow
            .input_slots()
            .first()
            .ok_or("missing declared input")?
            .slot;
        let sentinel = original
            .nodes
            .iter()
            .find_map(|node| match node.kind {
                CompiledNodeKind::SetConst { value }
                    if workflow.constant(value) == Some(&ConstValue::I64(99)) =>
                {
                    node.output.slot()
                }
                _ => None,
            })
            .ok_or("missing authored sentinel")?;
        let body = original
            .nodes
            .iter()
            .find_map(|node| match node.kind {
                CompiledNodeKind::SetConst { value }
                    if workflow.constant(value) == Some(&ConstValue::I64(7)) =>
                {
                    Some(node.id)
                }
                _ => None,
            })
            .ok_or("missing nested body")?;
        let mut store = ValueStore::new();
        let taints = vec![Taint::DerivedFromSecret; values.len()];
        let list = store
            .insert_list_with_taint(values.clone().into_boxed_slice(), taints.into_boxed_slice())?;
        let mut frame = new_run_frame(RunId::new(1), &workflow)?;
        frame.write_slot_with_taint(input, SlotValue::List(list), Taint::Secret)?;
        let mut evidence = EvidenceCollector::new();
        let mut collect = CollectStates::new();
        let mut finished = false;
        for _ in 0..128 {
            let signal = drive_deterministic_full(
                &workflow,
                &mut frame,
                &mut StepBudget::new(1),
                &mut store,
                &[],
                RetryPolicy::NEVER,
                &mut evidence,
                &mut collect,
                &CapabilitySet::empty(),
                1,
            )?;
            assert_eq!(*frame.read_slot(input)?, SlotValue::List(list));
            assert_eq!(frame.read_taint(input)?, Taint::Secret);
            assert_eq!(store.list(list)?, values.as_slice());
            for (index, value) in values.iter().enumerate() {
                let index = u32::try_from(index)?;
                assert_eq!(
                    store.list_item_with_taint(list, index)?,
                    (*value, Taint::DerivedFromSecret)
                );
            }
            if let RuntimeSignal::Finished(value) = signal {
                assert_eq!(value, SlotValue::I64(42));
                finished = true;
                break;
            }
            assert_eq!(signal, RuntimeSignal::StepBudgetExhausted);
        }
        assert!(
            finished,
            "nested workflow exceeded its bounded execution budget"
        );
        assert_eq!(*frame.read_slot(sentinel)?, SlotValue::I64(99));
        assert_eq!(frame.read_taint(sentinel)?, Taint::Clean);
        let count = evidence
            .drain()
            .iter()
            .filter(|event| {
                matches!(
                    event, EvidenceEvent::StepSucceeded { step, .. } if *step == body
                )
            })
            .count();
        assert_eq!(
            count,
            values.len().checked_mul(2).ok_or("body count overflow")?
        );
        assert_eq!(workflow.to_parts(), original);
    }
    Ok(())
}
