use core::num::NonZeroU16;
use vb_core::{
    ListId, RunId, RuntimePolicy, SlotIdx, SlotValue, StepBudget, StepIdx, Taint, WorkflowDigest,
    new_run_frame,
};
use vb_runtime::engine::{
    EvidenceCollector, EvidenceEvent, RetryPolicy, RuntimeSignal, drive_deterministic_full,
};
use vb_runtime::primitives::collect::CollectStates;
use vb_runtime::run_continuation::{ContinuationAuthority, ContinuationExpected};
use vb_runtime::shard::types::{RegisteredExecutionMode, RunState};
use vb_storage::{AcceptedArtifact, JournalEvent, OwnedRunInputsRecordV1, WorkflowSourceRecord};

pub(super) type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
pub(super) const SOURCE: &str = r#"version: velvet-ballistics/v1
name: owned_continuation
when:
  manual: {}
inputs:
  entries: { is: list, of: any, max: 8 }
steps:
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
pub(super) const NEXT: StepIdx = StepIdx::new(3);
pub(super) const FINISH: StepIdx = StepIdx::new(4);
pub(super) const EXIT: StepIdx = StepIdx::new(5);
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

pub(super) struct Fixture {
    pub(super) state: RunState,
    pub(super) record: OwnedRunInputsRecordV1,
    pub(super) artifact: AcceptedArtifact,
    pub(super) admission: JournalEvent,
    pub(super) expected: ContinuationExpected,
    pub(super) source: WorkflowSourceRecord,
    pub(super) input: SlotIdx,
}

impl Fixture {
    pub(super) fn authority(&self) -> TestResult<ContinuationAuthority> {
        Ok(ContinuationAuthority::from_owned_record(
            &self.record,
            &self.admission,
            &self.artifact,
            self.expected,
        )?)
    }

    pub(super) fn output(&self, step: StepIdx) -> TestResult<SlotIdx> {
        self.state
            .workflow
            .node(step)
            .and_then(|node| node.output)
            .ok_or_else(|| "compiled output missing".into())
    }
}

pub(super) fn fixture() -> TestResult<Fixture> {
    fixture_source(SOURCE)
}

pub(super) fn fixture_source(authored: &str) -> TestResult<Fixture> {
    let workflow =
        vb_compile::compile_workflow(authored.as_bytes()).map_err(|error| format!("{error:?}"))?;
    let (store, input) = super::arenas::original(&workflow)?;
    let source = WorkflowSourceRecord {
        digest: WorkflowDigest::from_bytes(blake3::hash(authored.as_bytes()).into()),
        source: authored.as_bytes().to_vec(),
    };
    let receipt = super::receipt::persist(&workflow, &store, input, &source)?;
    let state = initial_state(workflow, store, input, &receipt)?;
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

pub(super) fn drive(
    state: &mut RunState,
    count: u64,
) -> TestResult<(RuntimeSignal, Vec<EvidenceEvent>)> {
    let mut budget = StepBudget::new(count);
    let mut evidence = EvidenceCollector::new();
    let granted = state
        .admission
        .as_ref()
        .ok_or("admission missing")?
        .granted_capabilities();
    let signal = drive_deterministic_full(
        &state.workflow,
        &mut state.frame,
        &mut budget,
        &mut state.store,
        &state.action_contracts,
        RetryPolicy::NEVER,
        &mut evidence,
        &mut state.collect_states,
        granted,
        state.run_attempt.get(),
    )?;
    assert_eq!(budget.remaining(), 0);
    Ok((signal, evidence.drain()))
}

pub(super) fn at_cut(after: bool) -> TestResult<Fixture> {
    let mut fixture = fixture()?;
    let count = if after { 11 } else { 10 };
    let (signal, events) = drive(&mut fixture.state, count)?;
    assert_eq!(signal, RuntimeSignal::StepBudgetExhausted);
    assert_eq!(events.len(), if after { 33 } else { 30 });
    assert_eq!(fixture.state.frame.executed(), count);
    super::evidence::assert_pages_and_order(&fixture, &events, after)?;
    Ok(fixture)
}

pub(super) fn expected(
    artifact: &AcceptedArtifact,
    source: &WorkflowSourceRecord,
) -> TestResult<ContinuationExpected> {
    Ok(ContinuationExpected {
        run: RunId::new(17),
        artifact_digest: artifact.digest,
        source_digest: source.digest,
        policy: RuntimePolicy::Strict,
        run_attempt: NonZeroU16::new(1).ok_or("zero attempt")?,
        prior_seq: vb_storage::EventSeq::new(1),
    })
}

fn initial_state(
    workflow: vb_core::CompiledWorkflow,
    store: vb_core::ValueStore,
    input: SlotIdx,
    receipt: &super::receipt::Receipt,
) -> TestResult<RunState> {
    let mut frame = new_run_frame(receipt.expected.run, &workflow)?;
    frame.write_slot_with_taint(input, SlotValue::List(ROOT), Taint::Secret)?;
    let admission = vb_runtime::recovery::hydrate_run_admission_from_events(std::slice::from_ref(
        &receipt.admission,
    ))
    .ok_or("runtime admission missing")?;
    Ok(RunState {
        registered_execution: RegisteredExecutionMode::Live,
        frame,
        run_attempt: receipt.expected.run_attempt,
        action_attempts: vec![0; usize::from(workflow.node_count())].into_boxed_slice(),
        workflow,
        store,
        admission: Some(admission),
        collect_states: CollectStates::new(),
        action_contracts: receipt.artifact.action_contracts.clone(),
    })
}
