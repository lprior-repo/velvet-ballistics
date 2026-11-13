#![forbid(unsafe_code)]
//! Cell D: scoped action retry and error handler inside an owned Repeat body.

use std::num::NonZeroUsize;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use vb_core::action::ActionOutputReady;
use vb_core::root_bundle::OwnedRootBundle;
use vb_core::{
    ActionId, CompiledNodeKind, RunId, RuntimePolicy, SlotIdx, SlotValue, StepIdx, Taint,
    ValueStore, WorkflowDigest,
    action::{ActionContract, ActionName, Idempotency, RetrySafety, SideEffect},
};
use vb_runtime::journal::StorageRuntimeJournal;
use vb_runtime::runtime::Runtime;
use vb_runtime::shard::ShardConfig;
use vb_storage::{
    FjallJournal, JournalEvent, OwnedRunInputsRecordV1, RunHeaderRecord, RunSubmission,
    WorkflowSourceRecord, retry_timer::RetryTimerV1,
};

use super::super::support::TestResult;

pub(super) const RUN: RunId = RunId::new(4_713_031);
pub(super) const OTHER_RUN: RunId = RunId::new(4_713_032);
const SOURCE: &str = "version: velvet-ballistics/v1\nname: repeat_retry_handler\nwhen: {manual: {}}\ninputs: {seed: {is: number, min: 0, max: 100}}\nsteps:\n  - id: outer\n    repeat:\n      max_attempts: 2\n      steps:\n        - id: work\n          try_again: {max_attempts: 2, delay: '25ms'}\n          on_error: {handler: recovered}\n          then: success\n          do: {action: '0', input: '$input.seed'}\n        - id: recovered\n          set: {output: recovery_signal, value: '99'}\n          then: join\n        - id: success\n          set: {output: continuation, value: '$step.work + 1'}\n          then: join\n        - id: join\n          set: {output: body_result, value: '42'}\n  - id: done\n    finish: {result: '$step.outer'}\n";
pub(super) const RETRY_ONLY_SOURCE: &str = "version: velvet-ballistics/v1\nname: repeat_retry_only_control\nwhen: {manual: {}}\ninputs: {seed: {is: number, min: 0, max: 100}}\nsteps:\n  - id: outer\n    repeat:\n      max_attempts: 2\n      steps:\n        - id: work\n          try_again: {max_attempts: 2, delay: '25ms'}\n          do: {action: '0', input: '$input.seed'}\n  - id: done\n    finish: {result: '$step.outer'}\n";

pub(super) struct OwnedCase {
    directory: tempfile::TempDir,
    pub(super) workflow: vb_core::CompiledWorkflow,
    pub(super) source_digest: WorkflowDigest,
    pub(super) run: RunId,
}

impl OwnedCase {
    pub(super) fn new(run: RunId) -> TestResult<Self> {
        Self::new_from_source(run, SOURCE)
    }

    pub(super) fn new_from_source(run: RunId, source: &str) -> TestResult<Self> {
        let workflow = vb_compile::compile_workflow(source.as_bytes())?;
        vb_validate::shared::validate(&workflow.to_parts())?;
        let source_digest = WorkflowDigest::from_bytes(blake3::hash(source.as_bytes()).into());
        let input = workflow
            .input_slots()
            .first()
            .ok_or("input slot missing")?
            .slot;
        let store = ValueStore::with_max_slots(workflow.resource_contract().max_slots);
        let bundle = OwnedRootBundle::new(
            Box::from([(input, SlotValue::I64(41), Taint::Clean)]),
            store.checkpoint_v1(),
            &workflow,
        )?;
        let artifact = vb_storage::admission::prepare_compiled_artifact(
            &workflow,
            RuntimePolicy::Strict,
            &[contract()?],
        )?;
        let inputs =
            OwnedRunInputsRecordV1::new(run, artifact.digest, source_digest, &bundle, &workflow)?;
        let submission = RunSubmission::owned(
            WorkflowSourceRecord {
                digest: source_digest,
                source: source.as_bytes().to_vec(),
            },
            artifact,
            inputs,
            RunHeaderRecord {
                run,
                workflow_id: vb_core::WorkflowId::new(0),
                compiled_digest: workflow.digest(),
                status: 0,
                accepted_at_ms: now_ms()?,
            },
        )?;
        let directory = tempfile::tempdir()?;
        let journal = FjallJournal::open(directory.path(), None)?;
        journal.commit_run_submission(&submission, RuntimePolicy::Strict)?;
        Ok(Self {
            directory,
            workflow,
            source_digest,
            run,
        })
    }

    fn open(&self) -> TestResult<(Arc<FjallJournal>, Runtime)> {
        let journal = Arc::new(FjallJournal::open(self.directory.path(), None)?);
        let runtime = Runtime::new(
            NonZeroUsize::MIN,
            ShardConfig {
                policy: RuntimePolicy::Strict,
                step_budget_per_tick: 1,
                ..ShardConfig::default()
            },
            StorageRuntimeJournal::shared_strict(journal.clone()),
        );
        Ok((journal, runtime))
    }

    pub(super) fn activate(&self) -> TestResult<(Arc<FjallJournal>, Runtime)> {
        let (journal, runtime) = self.open()?;
        runtime.activate_owned_submission(self.run, self.source_digest)?;
        Ok((journal, runtime))
    }

    pub(super) fn recover(&self) -> TestResult<(Arc<FjallJournal>, Runtime)> {
        let (journal, runtime) = self.open()?;
        match runtime.recover_product(self.run)? {
            vb_runtime::recovery::RuntimeRecoveryProduct::Resumable(_) => {}
            other => return Err(format!("retry prefix not resumable: {other:?}").into()),
        }
        runtime.recover_and_resume(self.run)?;
        Ok((journal, runtime))
    }
}

fn now_ms() -> TestResult<u64> {
    Ok(u64::try_from(
        SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis(),
    )?)
}

fn contract() -> TestResult<ActionContract> {
    Ok(ActionContract {
        id: ActionId::new(0),
        name: ActionName::new("nested-retry-handler")?,
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 5_000,
        idempotency: Idempotency::DeterministicPure,
        side_effect: SideEffect::None,
        retry_safety: RetrySafety::Safe,
        required_capabilities: Box::new([]),
    })
}

pub(super) fn action_step(case: &OwnedCase) -> TestResult<StepIdx> {
    case.workflow
        .to_parts()
        .nodes
        .iter()
        .find(|node| matches!(node.kind, CompiledNodeKind::Do { .. }))
        .map(|node| node.id)
        .ok_or_else(|| "nested action step missing".into())
}

pub(super) fn action_output(case: &OwnedCase) -> TestResult<SlotIdx> {
    case.workflow
        .to_parts()
        .nodes
        .iter()
        .find(|node| matches!(node.kind, CompiledNodeKind::Do { .. }))
        .and_then(|node| node.output.slot())
        .ok_or_else(|| "nested action output missing".into())
}

pub(super) fn pending_action(
    runtime: &mut Runtime,
    journal: &FjallJournal,
    case: &OwnedCase,
    step: StepIdx,
    bound: usize,
) -> TestResult<vb_core::ActionTicket> {
    for _ in 0..bound {
        runtime.tick_all()?;
        if let Some(ticket) = runtime.lookup_pending_action_ticket(case.run, u64::from(step.get()))
        {
            return Ok(ticket);
        }
    }
    Err(format!(
        "bounded drive did not expose the nested action ticket: failed={}, trace={:?}, state={:?}, events={:?}, nodes={:?}",
        runtime.counters_snapshot().runs_failed,
        runtime.list_events(case.run)?,
        runtime.inspect_run_state(case.run).ok().map(|state| (state.frame.pc(), state.frame.executed())),
        journal.events_for_run_full(case.run)?,
        case.workflow.to_parts().nodes
    ).into())
}

pub(super) fn retry_timer(
    events: &[JournalEvent],
    ticket: vb_core::ActionTicket,
) -> TestResult<RetryTimerV1> {
    let timer = events
        .iter()
        .find_map(|event| match event {
            JournalEvent::RetryTimerScheduled { timer, .. } if timer.failed_ticket == ticket => {
                Some(timer.clone())
            }
            _ => None,
        })
        .ok_or_else(|| {
            format!("durable retry timer missing for failed ticket; events={events:?}")
        })?;
    timer.validate()?;
    assert_eq!(
        timer.next_attempt,
        ticket.attempt.checked_add(1).ok_or("attempt overflow")?
    );
    assert!(timer.generation > 0);
    Ok(timer)
}

pub(super) fn wait_for_retry(
    runtime: &mut Runtime,
    journal: &FjallJournal,
    case: &OwnedCase,
    step: StepIdx,
    timer: &RetryTimerV1,
) -> TestResult<vb_core::ActionTicket> {
    let now = now_ms()?;
    if timer.deadline_unix_ms.saturating_sub(now) > 1_000 {
        return Err("fixture retry deadline exceeded its bounded delay".into());
    }
    let entry = runtime
        .capture_timer_entry(case.run)
        .map_err(|error| format!("capture retry timer: {error:?}"))?;
    if entry.generation != timer.generation {
        return Err("captured retry timer generation differs from the journal".into());
    }
    let delay = entry.deadline.saturating_duration_since(Instant::now());
    if delay > Duration::from_secs(1) {
        return Err("captured retry deadline exceeded its bounded delay".into());
    }
    std::thread::sleep(delay.saturating_add(Duration::from_millis(1)));
    runtime
        .timer_entry_fired(entry)
        .map_err(|error| format!("fire retry timer: {error:?}"))?;
    runtime
        .tick_all()
        .map_err(|error| format!("process retry timer: {error:?}"))?;
    let bound = usize::from(case.workflow.node_count())
        .checked_mul(64)
        .ok_or("drive bound overflow")?;
    let ticket = pending_action(runtime, journal, case, step, bound)?;
    assert_eq!(ticket.attempt, timer.next_attempt);
    assert_eq!(ticket.run_attempt, timer.failed_ticket.run_attempt);
    assert_eq!(ticket.capacity, timer.failed_ticket.capacity);
    assert_eq!(ticket.seq, timer.failed_ticket.seq);
    assert_ne!(ticket, timer.failed_ticket);
    Ok(ticket)
}

pub(super) fn output_for(slot: SlotIdx, value: i64) -> TestResult<ActionOutputReady> {
    let encoded = postcard::to_allocvec(&SlotValue::I64(value))?;
    Ok(ActionOutputReady {
        output_slot: slot,
        value: SlotValue::I64(value),
        taint: Taint::Clean,
        encoded_len: u32::try_from(encoded.len())?,
    })
}

pub(super) fn count(events: &[JournalEvent], predicate: impl Fn(&JournalEvent) -> bool) -> usize {
    events.iter().filter(|event| predicate(event)).count()
}
