use vb_core::{
    CapabilitySet, CompiledNodeKind, EngineError, RunFrame, RunId, SlotIdx, SlotValue, StepBudget,
    StepIdx, Taint, ValueStore, new_run_frame,
};
use vb_runtime::engine::{EvidenceCollector, RetryPolicy, RuntimeSignal, drive_deterministic_full};
use vb_runtime::primitives::{collect::CollectStates, together::together_start};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const SOURCE: &str = r#"version: velvet-ballistics/v1
name: together_start_allocation_failure
when: {manual: {}}
steps:
  - id: seed
    set: {output: saved, value: '42'}
  - id: outer
    on_error: {handler: recovery}
    together:
      branches:
        - label: branch
          steps:
            - id: value
              set: {output: value, value: '7'}
  - id: recovery
    finish: {result: '$saved'}
"#;

#[test]
fn bounded_allocation_failure_acquires_no_together_reservation_before_recovery() -> TestResult {
    let plan = vb_compile::compile_workflow(SOURCE.as_bytes())?;
    let mut run = new_run_frame(RunId::new(1), &plan)?;
    let mut store = ValueStore::with_max_slots(1);
    let occupied =
        store.insert_list_with_taint(Box::from([SlotValue::I64(9)]), Box::from([Taint::Secret]))?;
    let mut evidence = EvidenceCollector::new();
    let mut collect = CollectStates::new();
    assert_eq!(
        drive_deterministic_full(
            &plan,
            &mut run,
            &mut StepBudget::new(1),
            &mut store,
            &[],
            RetryPolicy::NEVER,
            &mut evidence,
            &mut collect,
            &CapabilitySet::empty(),
            1
        )?,
        RuntimeSignal::StepBudgetExhausted
    );
    let start = run.pc();
    assert!(matches!(
        plan.node(start).ok_or("start")?.kind,
        CompiledNodeKind::TogetherStart { .. }
    ));
    let before = run.clone();
    let stored = store.checkpoint_v1();
    assert_eq!(
        drive_deterministic_full(
            &plan,
            &mut run,
            &mut StepBudget::new(1),
            &mut store,
            &[],
            RetryPolicy::NEVER,
            &mut evidence,
            &mut collect,
            &CapabilitySet::empty(),
            1,
        )?,
        RuntimeSignal::StepBudgetExhausted
    );
    assert_eq!(run.parallel_in_flight(), 0);
    assert_eq!(run.step_state(start)?, vb_core::StepState::Failed);
    assert_eq!(
        run.pc(),
        plan.node(start)
            .and_then(|node| node.routing.error.handler())
            .ok_or("handler")?
    );
    assert_eq!(
        run.executed(),
        before.executed().checked_add(1).ok_or("counter overflow")?
    );
    assert_eq!(run.initialized_slots()?, before.initialized_slots()?);
    assert_eq!(store.checkpoint_v1(), stored);
    assert_eq!(
        store.list_item_with_taint(occupied, 0)?,
        (SlotValue::I64(9), Taint::Secret)
    );
    let events = evidence.drain();
    let failed_output = plan
        .node(start)
        .and_then(|node| node.output.slot())
        .ok_or("output")?;
    assert_eq!(
        run.read_slot(failed_output),
        Err(EngineError::SlotUninitialized {
            slot: failed_output
        })
    );
    assert_eq!(events.iter().filter(|event| matches!(event, vb_runtime::engine::EvidenceEvent::StepFailed { step } if *step == start)).count(), 1);
    assert!(!events.iter().any(|event| matches!(event, vb_runtime::engine::EvidenceEvent::StepSucceeded { step, .. } if *step == start)));
    assert!(!events.iter().any(|event| matches!(event, vb_runtime::engine::EvidenceEvent::SlotWritten { slot, .. } if *slot == failed_output)));
    let started = events.iter().position(|event| matches!(event, vb_runtime::engine::EvidenceEvent::StepStarted { step } if *step == start)).ok_or("start evidence")?;
    let failed = events.iter().position(|event| matches!(event, vb_runtime::engine::EvidenceEvent::StepFailed { step } if *step == start)).ok_or("failed evidence")?;
    assert!(started < failed);
    assert_eq!(
        drive_deterministic_full(
            &plan,
            &mut run,
            &mut StepBudget::new(1),
            &mut store,
            &[],
            RetryPolicy::NEVER,
            &mut evidence,
            &mut collect,
            &CapabilitySet::empty(),
            1,
        )?,
        RuntimeSignal::Finished(SlotValue::I64(42))
    );
    assert_eq!(run.parallel_in_flight(), 0);
    assert_eq!(store.checkpoint_v1(), stored);
    Ok(())
}

#[test]
fn together_start_rejects_invalid_output_before_arena_or_reservation_writes() -> TestResult {
    for output in [None, Some(SlotIdx::new(1))] {
        let mut run = RunFrame::new(RunId::new(1), StepIdx::ZERO, 2, 1)?;
        let mut store = ValueStore::with_max_slots(1);
        let before = run.clone();
        let stored = store.checkpoint_v1();
        let error = if let Some(slot) = output {
            EngineError::SlotOutOfBounds { slot }
        } else {
            EngineError::MissingOutputSlot {
                step: StepIdx::ZERO,
            }
        };
        assert_eq!(
            together_start(
                &mut run,
                &mut store,
                &[StepIdx::new(1)],
                StepIdx::new(1),
                output
            ),
            Err(error)
        );
        assert_eq!(run, before);
        assert_eq!(store.checkpoint_v1(), stored);
    }
    Ok(())
}
