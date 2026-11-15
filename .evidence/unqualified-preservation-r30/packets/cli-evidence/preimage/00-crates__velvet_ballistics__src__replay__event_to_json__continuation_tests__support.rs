#![cfg(test)]

use core::num::NonZeroU16;
use vb_core::{ListId, RunId, RuntimePolicy, SlotValue, StepBudget, Taint, new_run_frame};
use vb_runtime::{
    engine::{EvidenceCollector, RetryPolicy, RuntimeSignal, drive_deterministic_full},
    primitives::collect::CollectStates,
    run_continuation::{ContinuationAuthority, ContinuationExpected},
    shard::types::{RegisteredExecutionMode, RunState},
};
use vb_storage::{AcceptedArtifact, WorkflowSourceRecord};

pub(super) type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
pub(super) const ROOT: ListId = ListId::new(2);
pub(super) const VALUES: [SlotValue; 5] = [
    SlotValue::I64(7),
    SlotValue::Object(vb_core::ObjectId::new(0)),
    SlotValue::List(ListId::new(1)),
    SlotValue::I64(7),
    SlotValue::I64(1),
];
pub(super) const TAINTS: [Taint; 5] = [
    Taint::Clean,
    Taint::Secret,
    Taint::Clean,
    Taint::DerivedFromSecret,
    Taint::Clean,
];

pub(super) fn expected(
    artifact: &AcceptedArtifact,
    source: &WorkflowSourceRecord,
) -> TestResult<ContinuationExpected> {
    Ok(ContinuationExpected {
        run: RunId::new(17),
        artifact_digest: artifact.digest,
        source_digest: source.digest,
        policy: RuntimePolicy::Strict,
        run_attempt: NonZeroU16::MIN,
        prior_seq: vb_storage::EventSeq::new(1),
    })
}

pub(super) fn at_cut() -> TestResult<(RunState, ContinuationAuthority)> {
    let source_text = b"version: velvet-ballistics/v1\nname: cli_continuation_projection\nwhen: {manual: {}}\ninputs: {entries: {is: list, of: any, max: 8}}\nsteps:\n  - id: bounded\n    collect:\n      variable: batch\n      source: '$input.entries'\n      pages: 3\n      items: 2\n      steps:\n        - id: first\n          set: {output: copied, value: '$batch'}\n        - id: second\n          set: {output: echoed, value: '$copied'}\n  - id: done\n    finish: {result: '$step.bounded'}\n";
    let workflow = vb_compile::compile_workflow(source_text)?;
    let (store, input) = super::arenas::original(&workflow)?;
    let source = WorkflowSourceRecord {
        digest: vb_core::WorkflowDigest::from_bytes(blake3::hash(source_text).into()),
        source: source_text.to_vec(),
    };
    let receipt = super::receipt::persist(&workflow, &store, input, &source)?;
    let authority = ContinuationAuthority::from_owned_record(
        &receipt.record,
        &receipt.admission,
        &receipt.artifact,
        receipt.expected,
    )?
    .with_run_started_millis(receipt.started_millis);
    let mut frame = new_run_frame(receipt.expected.run, &workflow)?;
    frame.write_slot_with_taint(input, SlotValue::List(ROOT), Taint::Secret)?;
    let admission = vb_runtime::recovery::hydrate_run_admission_from_events(std::slice::from_ref(
        &receipt.admission,
    ))
    .ok_or("current admission missing")?;
    let mut state = RunState {
        registered_execution: RegisteredExecutionMode::Live,
        frame,
        run_attempt: receipt.expected.run_attempt,
        action_attempts: vec![0; usize::from(workflow.node_count())].into_boxed_slice(),
        workflow,
        store,
        admission: Some(admission),
        run_started_millis: Some(receipt.started_millis),
        collect_states: CollectStates::new(),
        journal_bytes_written: 0,
        action_contracts: receipt.artifact.action_contracts.clone(),
    };
    let mut budget = StepBudget::new(10);
    let mut evidence = EvidenceCollector::new();
    let signal = drive_deterministic_full(
        &state.workflow,
        &mut state.frame,
        &mut budget,
        &mut state.store,
        &state.action_contracts,
        RetryPolicy::NEVER,
        &mut evidence,
        &mut state.collect_states,
        state
            .admission
            .as_ref()
            .ok_or("state admission missing")?
            .granted_capabilities(),
        state.run_attempt.get(),
    )?;
    assert_eq!(signal, RuntimeSignal::StepBudgetExhausted);
    assert_eq!(state.frame.executed(), 10);
    assert_eq!(budget.remaining(), 0);
    Ok((state, authority))
}
