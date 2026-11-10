#![forbid(unsafe_code)]

use vb_core::workflow::{
    CompiledInputConstraint, CompiledInputSlot, InputSlotKind, ResourceContract, node,
};
use vb_core::{
    CapabilitySet, CompiledNodeKind as K, CompiledWorkflow, ConstIdx, ConstValue, RunFrame, RunId,
    SlotIdx, SlotValue, StepBudget, StepIdx, Taint, ValueStore, WorkflowDigest, WorkflowParts,
};
use vb_runtime::engine::{EvidenceCollector, RetryPolicy, RuntimeSignal, drive_deterministic_full};
use vb_runtime::primitives::collect::CollectStates;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn parts() -> WorkflowParts {
    let value = SlotIdx::ZERO;
    let cursor = SlotIdx::new(1);
    let result = SlotIdx::new(2);
    let body = StepIdx::new(1);
    let done = StepIdx::new(3);
    WorkflowParts {
        name: "reduce-input-accumulator-alias".into(),
        digest: WorkflowDigest::from_bytes([0; 32]),
        semantic_source_identity: None,
        source_metadata: None,
        nodes: Box::from([
            node(
                StepIdx::ZERO,
                K::ReduceStart {
                    input: value,
                    accumulator: value,
                    initial: ConstIdx::new(0),
                    limit: 2,
                    body,
                    done,
                },
                Some(cursor),
                None,
            ),
            node(body, K::Nop, None, Some(StepIdx::new(2))),
            node(
                StepIdx::new(2),
                K::ReduceNext {
                    iterator_slot: cursor,
                    accumulator: value,
                    body,
                    done,
                },
                Some(value),
                None,
            ),
            node(
                done,
                K::ReduceFinish { accumulator: value },
                Some(result),
                Some(StepIdx::new(4)),
            ),
            node(StepIdx::new(4), K::Finish { result }, None, None),
        ]),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::from([ConstValue::I64(0)]),
        slot_count: 3,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
        input_slots: Box::from([CompiledInputSlot {
            slot: value,
            kind: InputSlotKind::List,
            constraint: CompiledInputConstraint::required(),
        }]),
    }
}

fn execute(items: &[SlotValue], taint: Taint) -> TestResult {
    let parts = parts();
    let plan = CompiledWorkflow::try_from_parts(parts.clone())?;
    vb_validate::shared::validate(&parts)?;
    let mut run = RunFrame::new(
        RunId::new(1),
        plan.entry(),
        plan.node_count(),
        plan.slot_count(),
    )?;
    let mut store = ValueStore::new();
    let source = store.insert_list(items)?;
    run.write_slot_with_taint(SlotIdx::ZERO, SlotValue::List(source), taint)?;
    let mut evidence = EvidenceCollector::new();
    let mut collect = CollectStates::new();
    let mut drive = |run: &mut RunFrame, store: &mut ValueStore| {
        drive_deterministic_full(
            &plan,
            run,
            &mut StepBudget::new(1),
            store,
            &[],
            RetryPolicy::NEVER,
            &mut evidence,
            &mut collect,
            &CapabilitySet::empty(),
            1,
        )
    };
    assert_eq!(
        drive(&mut run, &mut store)?,
        RuntimeSignal::StepBudgetExhausted
    );
    if let Some(first) = items.first() {
        assert_eq!(run.pc(), StepIdx::new(1));
        assert_eq!(run.read_slot(SlotIdx::ZERO)?, first);
        assert_eq!(run.read_taint(SlotIdx::ZERO)?, taint);
    } else {
        assert_eq!(run.pc(), StepIdx::new(3));
        assert_eq!(run.read_slot(SlotIdx::ZERO)?, &SlotValue::I64(0));
        assert_eq!(run.read_taint(SlotIdx::ZERO)?, Taint::Clean);
    }
    let mut finished = None;
    let mut cursors = Vec::new();
    for _ in 0..12 {
        if let Ok(SlotValue::List(cursor)) = run.read_slot(SlotIdx::new(1)) {
            let state = store.list(*cursor)?;
            assert_eq!(
                state.first(),
                Some(&SlotValue::I64(i64::from(source.get())))
            );
            let position = state.get(1).copied().ok_or("cursor position")?;
            if cursors.last() != Some(&position) {
                cursors.push(position);
            }
            assert_eq!(run.read_taint(SlotIdx::new(1))?, taint);
        } else {
            assert!(items.is_empty());
            assert_eq!(
                run.read_slot(SlotIdx::new(1)),
                Err(vb_core::CoreError::SlotUninitialized {
                    slot: SlotIdx::new(1)
                })
            );
        }
        match drive(&mut run, &mut store)? {
            RuntimeSignal::Finished(value) => {
                finished = Some(value);
                break;
            }
            other => assert_eq!(other, RuntimeSignal::StepBudgetExhausted),
        }
    }
    let expected = items
        .last()
        .copied()
        .map_or(SlotValue::I64(0), core::convert::identity);
    let expected_taint = if items.is_empty() {
        Taint::Clean
    } else {
        taint
    };
    assert_eq!(finished, Some(expected));
    assert_eq!(run.read_slot(SlotIdx::new(2))?, &expected);
    assert_eq!(run.read_taint(SlotIdx::new(2))?, expected_taint);
    assert_eq!(store.list(source)?, items);
    let expected_cursors = if items.is_empty() {
        vec![]
    } else {
        vec![SlotValue::I64(1), SlotValue::I64(2)]
    };
    assert_eq!(cursors, expected_cursors);
    Ok(())
}

#[test]
fn empty_input_accumulator_alias_finishes_with_clean_initial_value() -> TestResult {
    execute(&[], Taint::Secret)
}

#[test]
fn nonempty_input_accumulator_alias_preserves_items_taint_and_bounded_cursor() -> TestResult {
    for taint in [
        Taint::Clean,
        Taint::DerivedFromSecret,
        Taint::Secret,
        Taint::Random,
        Taint::TimeDependent,
    ] {
        execute(&[SlotValue::I64(3), SlotValue::I64(7)], taint)?;
    }
    Ok(())
}
