use super::*;
use vb_core::action::{Idempotency, RetrySafety, SideEffect};

#[test]
fn yaml_retry_rejects_invalid_bounds_non_actions_and_unsafe_contracts() -> TestResult {
    for (attempts, delay) in [
        (0, "0ms"),
        (4, "0ms"),
        (2, "-1ms"),
        (2, "4294967296ms"),
        (2, "$seed"),
    ] {
        assert!(
            matches!(vb_compile::compile_workflow(source(attempts, delay, false).as_bytes()), Err(vb_compile::CompileErrors(errors)) if matches!(errors.as_slice(), [vb_compile::CompileError::StepFieldShape { step: 1, field: "try_again", expected: "an action with 1..=3 total attempts and a bounded constant delay (ms, s, m, h, d)" }])),
            "{attempts} {delay}"
        );
    }
    let source = source(2, "0ms", false).replace(
        "    do: {action: '0', input: '$seed'}",
        "    set: {output: other, value: '1'}",
    );
    assert!(
        matches!(vb_compile::compile_workflow(source.as_bytes()), Err(vb_compile::CompileErrors(errors)) if matches!(errors.as_slice(), [vb_compile::CompileError::UnsupportedStepControlField { step: 1, field }] if field.as_ref() == "try_again"))
    );
    let mut workflow = compile(2, "1s", false)?.to_parts();
    workflow.nodes.first_mut().ok_or("missing Set")?.retry = Some(CompiledRetryPolicy {
        max_attempts: 2,
        delay_ms: 0,
    });
    assert!(
        matches!(vb_core::CompiledWorkflow::try_from_parts(workflow), Err(vb_core::workflow::WorkflowError::RetryPolicy { step }) if step == StepIdx::ZERO)
    );
    let journal = std::sync::Arc::new(VolatileRuntimeJournal::new());
    let runtime = runtime(journal.clone())?;
    let mut unsafe_contract = contract()?;
    unsafe_contract.retry_safety = RetrySafety::Unsafe;
    unsafe_contract.idempotency = Idempotency::AtLeastOnceExternal;
    unsafe_contract.side_effect = SideEffect::Writes;
    assert!(matches!(runtime
            .submit_direct_with_inputs_grants_and_contracts(
                RUN,
                compile(2, "0ms", false)?,
                Box::new([]),
                vb_core::CapabilitySet::empty(),
                Box::from([unsafe_contract])
            ), Err(RuntimeError::ContractValidationError { action, error }) if action == vb_core::ActionId::new(0) && error == "retry requires an idempotent retry-safe action contract"));
    assert_eq!(journal.snapshot()?, Vec::new());
    Ok(())
}

#[test]
fn retry_due_schedule_capacity_failure_preserves_timer_authority() -> TestResult {
    let journal = std::sync::Arc::new(VolatileRuntimeJournal::new());
    let mut probe = runtime(journal.clone())?;
    submit(&mut probe, compile(2, "200ms", false)?)?;
    fail(&mut probe, ActionFailureCode::Timeout)?;
    let capacity = std::num::NonZeroUsize::new(
        journal
            .snapshot()?
            .len()
            .checked_add(2)
            .ok_or("capacity overflow")?,
    )
    .ok_or("zero capacity")?;
    let journal = std::sync::Arc::new(VolatileRuntimeJournal::with_capacity(capacity));
    let mut runtime = runtime(journal.clone())?;
    submit(&mut runtime, compile(2, "200ms", false)?)?;
    fail(&mut runtime, ActionFailureCode::Timeout)?;
    let timer = runtime.capture_timer_entry(RUN)?;
    let before = journal.snapshot()?;
    let stop = std::time::Instant::now()
        .checked_add(std::time::Duration::from_secs(2))
        .ok_or("clock overflow")?;
    let result = loop {
        let result = runtime.tick_all();
        if matches!(result, Err(_)) {
            break result;
        }
        if std::time::Instant::now() >= stop {
            return Err("retry timer did not become due".into());
        }
    };
    assert!(
        matches!(result, Err(RuntimeError::JournalFull { capacity: actual }) if actual == capacity.get())
    );
    assert_eq!(runtime.capture_timer_entry(RUN)?, timer);
    assert!(matches!(runtime.lookup_pending_action_ticket(RUN, 1), None));
    assert_eq!(journal.snapshot()?, before);
    assert!(
        matches!(runtime.tick_all(), Err(RuntimeError::JournalFull { capacity: actual }) if actual == capacity.get())
    );
    assert_eq!(runtime.capture_timer_entry(RUN)?, timer);
    assert_eq!(journal.snapshot()?, before);
    Ok(())
}
