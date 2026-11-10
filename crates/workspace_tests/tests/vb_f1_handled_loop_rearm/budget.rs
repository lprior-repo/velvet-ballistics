use super::{K, SlotValue, TestResult, repeated, source};
use vb_core::{
    CapabilitySet, CompiledWorkflow, RunId, StepBudget, ValueStore, WholeWorkflowBudget,
    workflow::ErrorRouting,
};
use vb_runtime::engine::{
    EvidenceCollector, EvidenceEvent, RetryPolicy, RuntimeSignal, drive_deterministic_full,
};
use vb_runtime::primitives::collect::CollectStates;

#[test]
fn descendant_only_error_exit_counts_expensive_handler_skipped_by_normal_done() -> TestResult {
    let recovery = repeated("recovery", "- id: recovery_value\n  set: {output: recovery_value, value: '7'}\n- id: recovery_tail\n  set: {output: recovery_tail, value: '11'}\n").replacen("max_attempts: 2", "max_attempts: 7", 1);
    let inner = "- id: inner\n  on_error: {handler: recovery}\n  then: end_iteration\n  repeat:\n    max_attempts: 2\n    steps:\n      - id: failing\n        set: {output: quotient, value: '1 / $zero'}\n";
    let yaml = source(&repeated(
        "outer",
        &(inner.to_owned()
            + &recovery
            + "- id: end_iteration\n  set: {output: end_iteration, value: '7'}\n"),
    ));
    let mut parts = vb_compile::compile_workflow(yaml.as_bytes())?.to_parts();
    // Valid hand IR can declare recovery only on the failing descendant;
    // its ancestor header must not accidentally supply budget reachability.
    for node in &mut parts.nodes {
        if !matches!(node.kind, K::EvalExpr { .. }) {
            node.routing.error = ErrorRouting::None;
        }
    }
    let workflow = CompiledWorkflow::try_from_parts(parts)?;
    let parts = workflow.to_parts();
    let budget = WholeWorkflowBudget::compute(&parts.nodes, parts.entry, &parts.resource_contract)?;
    let mut frame = vb_core::new_run_frame(RunId::new(1), &workflow)?;
    let mut evidence = EvidenceCollector::new();
    let signal = drive_deterministic_full(
        &workflow,
        &mut frame,
        &mut StepBudget::new(512),
        &mut ValueStore::new(),
        &[],
        RetryPolicy::NEVER,
        &mut evidence,
        &mut CollectStates::new(),
        &CapabilitySet::empty(),
        1,
    )?;
    assert_eq!(signal, RuntimeSignal::Finished(SlotValue::I64(7)));
    let events = evidence.drain();
    let executed = u64::try_from(
        events
            .iter()
            .filter(|event| matches!(event, EvidenceEvent::StepStarted { .. }))
            .count(),
    )?;
    assert!(
        budget.max_total_steps >= executed,
        "budget {} < actual {executed}",
        budget.max_total_steps
    );
    assert!(budget.max_transitions >= frame.executed());
    assert!(budget.max_trace_events >= executed);
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, EvidenceEvent::StepFailed { .. }))
            .count(),
        2
    );
    let recovery = parts.nodes.iter().find(|node| matches!(node.kind, K::SetConst { value } if workflow.constant(value) == Some(&vb_core::ConstValue::I64(11)))).ok_or("recovery leaf")?.id;
    assert_eq!(events.iter().filter(|event| matches!(event, EvidenceEvent::StepSucceeded { step, .. } if *step == recovery)).count(), 14);
    Ok(())
}
