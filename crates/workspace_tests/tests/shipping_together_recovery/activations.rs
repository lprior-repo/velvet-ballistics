#![forbid(unsafe_code)]
//! Activation and step-budget behaviour around Together scopes.

use super::support::*;
use vb_core::{CapabilitySet, SlotValue, StepBudget, ValueStore, new_run_frame};
use vb_runtime::engine::{EvidenceCollector, RetryPolicy, RuntimeSignal, drive_deterministic_full};
use vb_runtime::primitives::collect::CollectStates;

#[test]
fn step_budget_exhaustion_before_together_start_leaves_no_reservation() -> TestResult {
    let plan =
        vb_compile::compile_workflow(FANOUT.as_bytes()).map_err(|errors| format!("{errors:?}"))?;
    let mut run = new_run_frame(RUN, &plan)?;
    let mut store = ValueStore::new();
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
            1,
        )?,
        RuntimeSignal::StepBudgetExhausted
    );
    assert_eq!(
        run.parallel_in_flight(),
        0,
        "budget exhaustion allocates no Together reservation"
    );
    let (start, _, _) = together_start(&plan)?;
    let start_output = plan
        .node(start)
        .and_then(|node| node.output.slot())
        .ok_or("start output missing")?;
    assert_eq!(
        run.read_slot(start_output),
        Err(vb_core::EngineError::SlotUninitialized { slot: start_output }),
        "budget exhaustion writes no accumulator"
    );

    let mut evidence = EvidenceCollector::new();
    assert_eq!(
        drive_deterministic_full(
            &plan,
            &mut run,
            &mut StepBudget::new(64),
            &mut store,
            &[],
            RetryPolicy::NEVER,
            &mut evidence,
            &mut collect,
            &CapabilitySet::empty(),
            1,
        )?,
        RuntimeSignal::Finished(run.read_slot(finish_result(&plan)?)?.clone())
    );
    assert_eq!(
        list_value(&store, *run.read_slot(finish_result(&plan)?)?)?,
        vec![SlotValue::I64(7), SlotValue::I64(11), SlotValue::I64(13)],
        "resuming after exhaustion still joins in declaration order"
    );
    Ok(())
}

#[test]
fn empty_together_branches_are_rejected_at_compile_time() -> TestResult {
    let source = r#"version: velvet-ballistics/v1
name: together_empty_branches
when: {manual: {}}
steps:
  - id: fanout
    together:
      branches: []
  - id: done
    finish: {result: '$step.fanout'}
"#;
    let errors = match vb_compile::compile_workflow(source.as_bytes()) {
        Ok(_) => return Err("branchless Together source was accepted".into()),
        Err(errors) => errors,
    };
    assert!(
        errors.iter().any(|error| matches!(
            error,
            vb_compile::CompileError::StepFieldShape {
                field: "together.branches",
                expected: "at least one branch",
                ..
            }
        )),
        "branchless Together must be the typed field rejection: {errors:?}"
    );
    Ok(())
}
