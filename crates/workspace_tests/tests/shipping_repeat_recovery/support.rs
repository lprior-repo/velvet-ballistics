//! Shared Repeat recovery fixtures: compile, live drive, journal lifecycle,
//! replay and exact-divergence helpers.

use std::{
    num::NonZeroUsize,
    path::Path,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use vb_core::{
    CapabilitySet, CompiledNodeKind as K, CompiledWorkflow, RunFrame, RunId, RuntimePolicy,
    SlotIdx, SlotValue, StepBudget, StepIdx, Taint, ValueStore, WorkflowDigest, new_run_frame,
};
use vb_runtime::{
    engine::{EvidenceCollector, RetryPolicy, RuntimeSignal, drive_deterministic_full},
    journal::StorageRuntimeJournal,
    primitives::collect::CollectStates,
    runtime::Runtime,
    shard::ShardConfig,
};
use vb_storage::{FjallJournal, JournalEvent as E, RunSubmission};

pub(crate) type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn current_time_millis() -> TestResult<u64> {
    let millis = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis();
    Ok(u64::try_from(millis)?)
}

pub(crate) const RUN: RunId = RunId::new(842);
pub(crate) const SOURCE: &str = "version: velvet-ballistics/v1\nname: shipping_repeat_recovery\nwhen: {manual: {}}\nsteps:\n  - id: seed\n    set: {output: base, value: '7'}\n  - id: r\n    repeat:\n      max_attempts: 3\n      steps:\n        - id: leaf\n          set: {output: leaf_out, value: '9'}\n  - id: done\n    finish: {result: '$step.r'}\n";
pub(crate) const TAINT_SOURCE: &str = "version: velvet-ballistics/v1\nname: shipping_repeat_taint\nwhen: {manual: {}}\ninputs: {secret: {is: number}}\nsteps:\n  - id: seed\n    set: {output: base, value: '7'}\n  - id: r\n    repeat:\n      max_attempts: 2\n      steps:\n        - id: leaf\n          set: {output: leaf_out, value: '$input.secret'}\n  - id: done\n    finish: {result: '$step.r'}\n";
pub(crate) const ASK_SOURCE: &str = "version: velvet-ballistics/v1\nname: shipping_repeat_ask\nwhen: {manual: {}}\nvars: {prompt: 'repeat recovery'}\nsteps:\n  - id: seed\n    set: {output: base, value: '7'}\n  - id: r\n    repeat:\n      max_attempts: 2\n      steps:\n        - id: answer\n          ask: {prompt: '$vars.prompt'}\n  - id: done\n    finish: {result: '$step.r'}\n";

pub(crate) struct RepeatNodes {
    pub(crate) start: StepIdx,
    pub(crate) attempt: SlotIdx,
    pub(crate) body: StepIdx,
    pub(crate) control: StepIdx,
    pub(crate) finish: StepIdx,
    pub(crate) result: SlotIdx,
    pub(crate) terminal: StepIdx,
}

pub(crate) fn repeat_nodes(workflow: &CompiledWorkflow, max: u16) -> TestResult<RepeatNodes> {
    let parts = workflow.to_parts();
    let (start, attempt, body) = parts
        .nodes
        .iter()
        .find_map(|node| match node.kind {
            K::RepeatStart {
                body, max_attempts, ..
            } if max_attempts.get() == max => Some((node.id, node.output.slot()?, body)),
            _ => None,
        })
        .ok_or("RepeatStart node")?;
    let (control, done) = parts
        .nodes
        .iter()
        .find_map(|node| match node.kind {
            K::RepeatCheck { attempt_slot, done }
                if attempt_slot == attempt && node.routing.next.step() == Some(body) =>
            {
                Some((node.id, done))
            }
            _ => None,
        })
        .ok_or("RepeatCheck node")?;
    let (finish, result) = parts
        .nodes
        .iter()
        .find_map(|node| match node.kind {
            K::RepeatFinish { result } if node.id == done && node.output.slot() == Some(result) => {
                Some((node.id, result))
            }
            _ => None,
        })
        .ok_or("RepeatFinish node")?;
    let terminal = parts
        .nodes
        .iter()
        .find(|node| matches!(node.kind, K::Finish { .. }))
        .ok_or("terminal Finish node")?
        .id;
    Ok(RepeatNodes {
        start,
        attempt,
        body,
        control,
        finish,
        result,
        terminal,
    })
}

pub(crate) fn submission(source: &str, workflow: &CompiledWorkflow) -> TestResult<RunSubmission> {
    let artifact =
        vb_storage::admission::prepare_compiled_artifact(workflow, RuntimePolicy::Strict, &[])?;
    let source_record = vb_storage::WorkflowSourceRecord {
        digest: WorkflowDigest::from_bytes(blake3::hash(source.as_bytes()).into()),
        source: source.as_bytes().to_vec(),
    };
    let inputs = vb_storage::RunInputsRecord::new(
        RUN,
        artifact.digest,
        source_record.digest,
        workflow,
        Box::new([]),
    )?;
    Ok(RunSubmission::new(
        source_record,
        artifact,
        inputs,
        vb_storage::RunHeaderRecord {
            run: RUN,
            workflow_id: vb_core::WorkflowId::new(0),
            compiled_digest: workflow.digest(),
            status: 0,
            accepted_at_ms: current_time_millis()?,
        },
    )?)
}

pub(crate) fn open(path: &Path) -> TestResult<(Arc<FjallJournal>, Runtime)> {
    let journal = Arc::new(FjallJournal::open(path, None)?);
    let runtime = Runtime::new(
        NonZeroUsize::MIN,
        ShardConfig {
            policy: RuntimePolicy::Strict,
            ..ShardConfig::default()
        },
        StorageRuntimeJournal::shared_strict(journal.clone()),
    );
    Ok((journal, runtime))
}

pub(crate) fn drive_live(
    source: &str,
    bind: Option<(SlotIdx, SlotValue, Taint)>,
    expected: i64,
) -> TestResult<(CompiledWorkflow, RunFrame)> {
    let workflow = vb_compile::compile_workflow(source.as_bytes())?;
    let mut frame = new_run_frame(RUN, &workflow)?;
    let mut store = ValueStore::new();
    if let Some((slot, value, taint)) = bind {
        frame.write_slot_with_taint(slot, value, taint)?;
    }
    let signal = drive_deterministic_full(
        &workflow,
        &mut frame,
        &mut StepBudget::new(256),
        &mut store,
        &[],
        RetryPolicy::NEVER,
        &mut EvidenceCollector::new(),
        &mut CollectStates::new(),
        &CapabilitySet::empty(),
        1,
    )?;
    assert_eq!(signal, RuntimeSignal::Finished(SlotValue::I64(expected)));
    Ok((workflow, frame))
}

pub(crate) fn persist_reopen(source: &str) -> TestResult<(CompiledWorkflow, Vec<E>)> {
    let workflow = vb_compile::compile_workflow(source.as_bytes())?;
    let submission = submission(source, &workflow)?;
    let directory = tempfile::tempdir()?;
    let (journal, mut runtime) = open(directory.path())?;
    journal.commit_run_submission(&submission, RuntimePolicy::Strict)?;
    runtime.recover_and_resume(RUN)?;
    for _ in 0..128 {
        runtime.tick_all()?;
        assert_eq!(runtime.counters_snapshot().runs_failed, 0);
        if runtime.counters_snapshot().runs_completed == 1 {
            break;
        }
    }
    assert_eq!(runtime.counters_snapshot().runs_completed, 1);
    let events = journal.events_for_run_full(RUN)?;
    drop(runtime);
    drop(journal);

    let (journal, runtime) = open(directory.path())?;
    assert_eq!(journal.events_for_run_full(RUN)?, events);
    let product = runtime.recover_product(RUN)?;
    assert_eq!(product.cannot_resume_reason(), Some("terminal_state"));
    assert!(!product.is_resumable());
    Ok((workflow, events))
}

pub(crate) fn counter_sequence(events: &[E], attempt: SlotIdx) -> TestResult<Vec<(u16, u16)>> {
    events
        .iter()
        .filter_map(|event| match event {
            E::SlotWrittenEvent {
                slot,
                value: Some(bytes),
                ..
            } if *slot == attempt => {
                postcard::from_bytes::<SlotValue>(bytes)
                    .ok()
                    .and_then(|value| match value {
                        SlotValue::I64(packed) => Some(packed),
                        _ => None,
                    })
            }
            _ => None,
        })
        .map(|packed| {
            vb_core::engine::repeat::decode_repeat_state(packed)
                .map_err(|error| format!("counter decode: {error:?}").into())
        })
        .collect()
}

pub(crate) fn started(events: &[E], step: StepIdx) -> usize {
    events
        .iter()
        .filter(|event| matches!(event, E::StepStarted { step: s, .. } if *s == step))
        .count()
}

pub(crate) fn replay(
    events: &[E],
    workflow: &CompiledWorkflow,
) -> vb_storage::recovery::RecoveryResult<
    Option<vb_storage::recovery::complete_scalar::CompleteScalarReplay>,
> {
    vb_storage::recovery::complete_scalar::replay_complete_scalar(events, workflow)
}

pub(crate) fn expect_divergence(
    events: &[E],
    workflow: &CompiledWorkflow,
    step: StepIdx,
    detail: &str,
) -> TestResult {
    match replay(events, workflow) {
        Err(vb_storage::recovery::RecoveryError::ReplayDivergence {
            step: found,
            detail: found_detail,
        }) if found == step && found_detail == detail => Ok(()),
        Err(error) => Err(format!("expected divergence at {step:?} {detail:?}: {error:?}").into()),
        Ok(_) => Err(format!("expected divergence at {step:?} {detail:?}: replay accepted").into()),
    }
}
