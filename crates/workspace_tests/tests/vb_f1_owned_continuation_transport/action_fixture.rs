//! Separate real guarded-Do fixture; the original action-free SOURCE is untouched.
use super::support::{Fixture, ROOT, TestResult};
use vb_core::root_bundle::OwnedRootBundle;
use vb_core::{InputSlotKind, SlotIdx, SlotValue, StepIdx, Taint, WorkflowDigest, new_run_frame};
use vb_runtime::engine::RuntimeSignal;
use vb_runtime::primitives::collect::CollectStates;
use vb_runtime::run_continuation::{RunContinuationError, RunContinuationV1, TransportFailure};
use vb_runtime::shard::types::{RegisteredExecutionMode, RunState};
use vb_storage::WorkflowSourceRecord;

const SOURCE: &str = r#"version: velvet-ballistics/v1
name: owned_continuation_contract_path
when:
  manual: {}
inputs:
  entries: { is: list, of: any, max: 8 }
  perform: { is: boolean }
steps:
  - id: optional
    if: "$input.perform"
    do: { action: "0", input: "$input.perform" }
  - id: bounded
    collect:
      variable: batch
      source: "$input.entries"
      pages: 3
      items: 2
      steps:
        - id: first
          set: { output: copied, value: "$batch" }
        - id: second
          set: { output: echoed, value: "$copied" }
  - id: done
    finish: { result: "$step.bounded" }
"#;

pub(super) const ACTION: StepIdx = StepIdx::new(1);
pub(super) const START: StepIdx = StepIdx::new(2);
pub(super) const FINISH: StepIdx = StepIdx::new(6);
pub(super) const EXIT: StepIdx = StepIdx::new(7);

fn fixture(perform: bool) -> TestResult<Fixture> {
    let workflow =
        vb_compile::compile_workflow(SOURCE.as_bytes()).map_err(|error| format!("{error:?}"))?;
    super::action_evidence::compiled(&workflow)?;
    let (store, input) = super::arenas::original_with_blob(&workflow, 3)?;
    let flag = guard_slot(&workflow)?;
    let bundle = OwnedRootBundle::new(
        Box::from([
            (input, SlotValue::List(ROOT), Taint::Secret),
            (flag, SlotValue::Bool(perform), Taint::Clean),
        ]),
        store.checkpoint_v1(),
        &workflow,
    )?;
    let source = WorkflowSourceRecord {
        digest: WorkflowDigest::from_bytes(blake3::hash(SOURCE.as_bytes()).into()),
        source: SOURCE.as_bytes().to_vec(),
    };
    let receipt = super::receipt::persist_owned(
        &workflow,
        &source,
        &bundle,
        &[super::contracts::accepted()?],
    )?;
    let state = initial_state(workflow, store, &receipt)?;
    Ok(Fixture {
        state,
        record: receipt.record,
        artifact: receipt.artifact,
        admission: receipt.admission,
        expected: receipt.expected,
        source,
        input,
    })
}

pub(super) fn guard_slot(workflow: &vb_core::CompiledWorkflow) -> TestResult<SlotIdx> {
    let flags: Vec<_> = workflow
        .input_slots()
        .iter()
        .filter(|input| input.kind == InputSlotKind::Bool)
        .collect();
    assert_eq!(flags.len(), 1);
    Ok(flags.first().ok_or("guard input missing")?.slot)
}

pub(super) fn at_cut(after: bool) -> TestResult<Fixture> {
    let mut fixture = fixture(false)?;
    let original_ir = fixture.state.workflow.to_parts();
    let steps = if after { 12 } else { 11 };
    let (signal, events) = super::support::drive(&mut fixture.state, steps)?;
    assert_eq!(signal, RuntimeSignal::StepBudgetExhausted);
    assert_eq!(fixture.state.frame.executed(), steps);
    assert_eq!(fixture.source.source, SOURCE.as_bytes());
    super::action_evidence::cut(&fixture, &events, after)?;
    assert_eq!(fixture.state.workflow.to_parts(), original_ir);
    Ok(fixture)
}

#[test]
fn real_guarded_do_is_reachable_and_dispatches_without_fabricated_completion() -> TestResult {
    let mut fixture = fixture(true)?;
    let original_ir = fixture.state.workflow.to_parts();
    let (signal, events) = super::support::drive(&mut fixture.state, 2)?;
    let RuntimeSignal::AwaitingAction(ticket) = signal else {
        return Err("true source branch did not dispatch its real action".into());
    };
    assert_eq!(ticket.action, vb_core::ActionId::new(0));
    assert_eq!(ticket.step, ACTION);
    assert_eq!(ticket.run, fixture.expected.run);
    assert_eq!(ticket.run_attempt, fixture.expected.run_attempt.get());
    assert_eq!(ticket.attempt, 1);
    assert_eq!(fixture.state.frame.pc(), ACTION);
    assert_eq!(events.len(), 3);
    assert_eq!(fixture.state.workflow.to_parts(), original_ir);
    let error = RunContinuationV1::capture_transport(
        &fixture.state,
        &fixture.authority()?,
        super::golden::limits(1_048_576)?,
    )
    .err()
    .ok_or("in-flight action was accepted as quiescent")?;
    assert!(matches!(
        error.failure,
        TransportFailure::Strong(RunContinuationError::InFlight)
    ));
    assert!(!error.strong_decode_started);
    Ok(())
}

#[test]
fn real_contract_fixture_matches_ordinary_transport_at_both_actual_cuts() -> TestResult {
    for after in [false, true] {
        let fixture = at_cut(after)?;
        let authority = fixture.authority()?;
        let ordinary = RunContinuationV1::capture(&fixture.state, &authority)?;
        let bounded = RunContinuationV1::capture_transport(
            &fixture.state,
            &authority,
            super::golden::limits(1_048_576)?,
        )?;
        assert_eq!(bounded.as_bytes(), ordinary.to_bytes()?);
        let (decoded, _) = RunContinuationV1::from_transport_bytes(
            bounded.as_bytes(),
            &authority,
            super::golden::limits(1_048_576)?,
        )?;
        assert_eq!(decoded.digest(), ordinary.digest());
        let restored = decoded.restore(authority)?;
        assert_eq!(restored.frame, fixture.state.frame);
        assert_eq!(restored.store, fixture.state.store);
        assert_eq!(restored.action_contracts, fixture.state.action_contracts);
    }
    Ok(())
}

fn initial_state(
    workflow: vb_core::CompiledWorkflow,
    store: vb_core::ValueStore,
    receipt: &super::receipt::Receipt,
) -> TestResult<RunState> {
    let mut frame = new_run_frame(receipt.expected.run, &workflow)?;
    let retained = receipt.record.restore(&workflow)?;
    for (slot, value, taint) in retained.roots() {
        frame.write_slot_with_taint(*slot, *value, *taint)?;
    }
    let admission = vb_runtime::recovery::hydrate_run_admission_from_events(std::slice::from_ref(
        &receipt.admission,
    ))
    .ok_or("admission missing")?;
    Ok(RunState {
        registered_execution: RegisteredExecutionMode::Live,
        frame,
        store,
        run_attempt: receipt.expected.run_attempt,
        action_attempts: vec![0; usize::from(workflow.node_count())].into_boxed_slice(),
        workflow,
        admission: Some(admission),
        collect_states: CollectStates::new(),
        action_contracts: receipt.artifact.action_contracts.clone(),
    })
}
