#[path = "vb_f2_signal_body_results/support.rs"]
mod support;
use support::*;
use vb_core::SlotValue;
use vb_runtime::journal::RuntimeJournalEvent as E;
#[test]
fn dynamic_signal_operand_errors_reach_authored_handlers_without_suspension() -> TestResult {
    for primitive in [
        "wait: {timeout: '$input.dynamic'}",
        "wait: {event: '$input.dynamic'}",
        "ask: {prompt: '$input.dynamic'}",
        "ask: {prompt: '$seed', timeout: '$input.dynamic'}",
    ] {
        let body = format!("- id: failing\n  on_error: {{handler: done}}\n  {primitive}\n");
        let yaml = source("inputs: {dynamic: {is: any}}\n", &body, "$seed");
        for quantum in [1, 64] {
            let outcome = execute(&yaml, quantum, Some(SlotValue::Bool(true)))?;
            assert_eq!(outcome.value, SlotValue::I64(42));
            assert_eq!((outcome.asks, outcome.waits), (0, 0));
            let failed = outcome
                .events
                .iter()
                .filter_map(|event| match event {
                    E::StepFailed { step, .. } => Some(*step),
                    _ => None,
                })
                .collect::<Vec<_>>();
            assert_eq!(failed.len(), 1);
            assert!(!outcome.events.iter().any(
                |event| matches!(event, E::StepSucceeded { step, .. } if failed.contains(step))
            ));
        }
    }
    Ok(())
}
#[test]
fn ask_resume_with_handler_succeeds_once_before_a_distinct_handled_error() -> TestResult {
    let body = "- id: question\n  on_error: {handler: done}\n  ask: {prompt: '$seed'}\n- id: failing\n  on_error: {handler: done}\n  set: {output: quotient, value: '1 / $input.divisor'}\n";
    let yaml = source("inputs: {divisor: number}\n", body, "$seed");
    let plan = vb_compile::compile_workflow(yaml.as_bytes())?;
    let parts = plan.to_parts();
    let ask = parts
        .nodes
        .iter()
        .find(|node| matches!(node.kind, vb_core::CompiledNodeKind::Ask { .. }))
        .ok_or("Ask")?;
    let resume = plan
        .node(ask.routing.next.step().ok_or("resume")?)
        .ok_or("resume node")?;
    assert_eq!(resume.routing.error.handler(), ask.routing.error.handler());
    for quantum in [1, 64] {
        let outcome = execute(&yaml, quantum, Some(SlotValue::I64(0)))?;
        assert_eq!(outcome.value, SlotValue::I64(42));
        assert_eq!((outcome.asks, outcome.waits), (1, 0));
        for step in [ask.id, resume.id] {
            assert_eq!(outcome.events.iter().filter(|event| matches!(event, E::StepSucceeded { step: actual, .. } if *actual == step)).count(), 1);
            assert!(!outcome.events.iter().any(
                |event| matches!(event, E::StepFailed { step: actual, .. } if *actual == step)
            ));
        }
        assert_eq!(
            outcome
                .events
                .iter()
                .filter(|event| matches!(event, E::StepFailed { .. }))
                .count(),
            1
        );
    }
    Ok(())
}

#[test]
fn unhandled_signal_errors_preserve_typed_return_and_slot_prestate() -> TestResult {
    for (primitive, expected) in [
        ("wait: {timeout: '$input.dynamic'}", "deadline"),
        ("ask: {prompt: '$input.dynamic'}", "prompt"),
    ] {
        let yaml = source(
            "inputs: {dynamic: {is: any}}\n",
            &format!("- id: failing\n  {primitive}\n"),
            "$seed",
        );
        let plan = vb_compile::compile_workflow(yaml.as_bytes())?;
        let input = plan.input_slots().first().ok_or("input")?.slot;
        let mut frame = vb_core::new_run_frame(vb_core::RunId::new(1), &plan)?;
        frame.write_slot(input, SlotValue::Bool(true))?;
        let mut store = vb_core::ValueStore::new();
        let mut evidence = vb_runtime::engine::EvidenceCollector::new();
        let mut collect = vb_runtime::primitives::collect::CollectStates::new();
        let drive = |frame: &mut vb_core::RunFrame,
                     store: &mut vb_core::ValueStore,
                     evidence: &mut vb_runtime::engine::EvidenceCollector,
                     collect: &mut vb_runtime::primitives::collect::CollectStates,
                     quantum| {
            vb_runtime::engine::drive_deterministic_full(
                &plan,
                frame,
                &mut vb_core::StepBudget::new(quantum),
                store,
                &[],
                vb_runtime::engine::RetryPolicy::NEVER,
                evidence,
                collect,
                &vb_core::CapabilitySet::empty(),
                1,
            )
        };
        assert_eq!(
            drive(&mut frame, &mut store, &mut evidence, &mut collect, 2)?,
            vb_runtime::engine::RuntimeSignal::StepBudgetExhausted
        );
        let failed = frame.pc();
        let before = frame.initialized_slots()?;
        let stored = store.checkpoint_v1();
        assert_eq!(
            drive(&mut frame, &mut store, &mut evidence, &mut collect, 1),
            Err(vb_runtime::engine::RuntimeEngineError::Core(
                vb_core::EngineError::TypeMismatch {
                    expected,
                    found: "boolean"
                }
            ))
        );
        assert_eq!(frame.pc(), failed);
        assert_eq!(frame.step_state(failed)?, vb_core::StepState::Failed);
        assert_eq!(frame.initialized_slots()?, before);
        assert_eq!(store.checkpoint_v1(), stored);
    }
    Ok(())
}
