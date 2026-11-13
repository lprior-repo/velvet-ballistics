use vb_core::action::ActionTicket;
use vb_core::frame::RunFrameCheckpointV1;
use vb_core::root_bundle::OwnedRootBundle;
use vb_core::value::join_taint;
use vb_core::value_store::ValueStoreCheckpointV1;
use vb_core::{
    ActionContract, CompiledNodeKind as K, CompiledWorkflow, ListId, RuntimePolicy, SlotIdx,
    SlotValue, StepIdx, StepState, Taint, ValueStore, WorkflowDigest,
};
use vb_runtime::recovery::RuntimeRecoveryProduct;
use vb_storage::recovery::replay_owned_for_each;
use vb_storage::{
    DecodedSlotWrittenExtra, FjallJournal, JournalEvent, RunHeaderRecord, RunSubmission,
    WorkflowSourceRecord, decode_slot_written_extra,
};

use super::fixtures::{RUN, TestResult, runtime};

pub(super) struct Cut {
    pub(super) directory: tempfile::TempDir,
    pub(super) workflow: CompiledWorkflow,
    pub(super) roots: Box<[(SlotIdx, SlotValue, Taint)]>,
    pub(super) input_store: ValueStoreCheckpointV1,
    pub(super) live_frame: RunFrameCheckpointV1,
    pub(super) live_store: ValueStoreCheckpointV1,
    pub(super) live_deadline: Option<std::time::Instant>,
    pub(super) live_pending_boundary: vb_runtime::runtime::RuntimePendingBoundarySnapshot,
    pub(super) live_action_ticket: Option<ActionTicket>,
    pub(super) events: Vec<JournalEvent>,
}

pub(super) fn step_with(
    workflow: &CompiledWorkflow,
    predicate: impl Fn(&K) -> bool,
) -> TestResult<StepIdx> {
    workflow
        .to_parts()
        .nodes
        .iter()
        .find(|node| predicate(&node.kind))
        .map(|node| node.id)
        .ok_or_else(|| "step not found".into())
}

pub(super) fn cut_with(
    source: &str,
    stop: (StepIdx, StepState),
    build: impl FnOnce(&mut ValueStore) -> TestResult<(ListId, Taint)>,
) -> TestResult<Cut> {
    cut_with_nth(source, stop, 1, build)
}

pub(super) fn cut_with_nth(
    source: &str,
    stop: (StepIdx, StepState),
    occurrences: usize,
    build: impl FnOnce(&mut ValueStore) -> TestResult<(ListId, Taint)>,
) -> TestResult<Cut> {
    cut_with_actions_nth(source, stop, occurrences, &[], build)
}

pub(super) fn cut_with_actions_nth(
    source: &str,
    stop: (StepIdx, StepState),
    occurrences: usize,
    contracts: &[ActionContract],
    build: impl FnOnce(&mut ValueStore) -> TestResult<(ListId, Taint)>,
) -> TestResult<Cut> {
    let workflow = vb_compile::compile_workflow(source.as_bytes())?;
    let mut store = ValueStore::with_max_slots(workflow.resource_contract().max_slots);
    let (list, root_taint) = build(&mut store)?;
    let input = workflow
        .input_slots()
        .first()
        .ok_or("missing input slot")?
        .slot;
    let roots = Box::from([(input, SlotValue::List(list), root_taint)]);
    let bundle = OwnedRootBundle::new(roots, store.checkpoint_v1(), &workflow)?;
    let input_store = bundle.checkpoint().clone();
    let roots = bundle.roots().to_vec().into_boxed_slice();
    let artifact = vb_storage::admission::prepare_compiled_artifact(
        &workflow,
        RuntimePolicy::Strict,
        contracts,
    )?;
    let source_record = WorkflowSourceRecord {
        digest: WorkflowDigest::from_bytes(blake3::hash(source.as_bytes()).into()),
        source: source.as_bytes().to_vec(),
    };
    let source_digest = source_record.digest;
    let inputs = vb_storage::OwnedRunInputsRecordV1::new(
        RUN,
        artifact.digest,
        source_digest,
        &bundle,
        &workflow,
    )?;
    let header = RunHeaderRecord {
        run: RUN,
        workflow_id: vb_core::WorkflowId::new(0),
        compiled_digest: workflow.digest(),
        status: 0,
        accepted_at_ms: 123,
    };
    let submission = RunSubmission::owned(source_record, artifact, inputs, header)?;
    let directory = tempfile::tempdir()?;
    let (live_frame, live_store, live_deadline, live_pending_boundary, live_action_ticket, events) = {
        let journal = std::sync::Arc::new(FjallJournal::open(directory.path(), None)?);
        journal.commit_run_submission(&submission, RuntimePolicy::Strict)?;
        let mut runtime = runtime(journal.clone(), 1);
        runtime.activate_owned_submission(RUN, source_digest)?;
        let mut live = None;
        let mut seen = 0_usize;
        for _ in 0..256 {
            runtime.tick_all()?;
            let state = runtime
                .inspect_run_state(RUN)
                .map_err(|error| format!("live inspect: {error}"))?;
            if state.frame.step_state(stop.0)? == stop.1 {
                seen = seen.saturating_add(1);
                if seen >= occurrences {
                    live = Some((state.frame.checkpoint_v1(), state.store.checkpoint_v1()));
                    break;
                }
            }
        }
        let (live_frame, live_store) = live.ok_or("cut step state never reached")?;
        let live_action_ticket = runtime.lookup_pending_action_ticket(RUN, u64::from(stop.0.get()));
        let live_pending_boundary = runtime.pending_boundary_snapshot(64);
        let live_deadline = if stop.1 == StepState::Waiting {
            Some(runtime.capture_timer_entry(RUN)?.deadline)
        } else {
            None
        };
        (
            live_frame,
            live_store,
            live_deadline,
            live_pending_boundary,
            live_action_ticket,
            journal.events_for_run_full(RUN)?,
        )
    };
    Ok(Cut {
        directory,
        workflow,
        roots,
        input_store,
        live_frame,
        live_store,
        live_deadline,
        live_pending_boundary,
        live_action_ticket,
        events,
    })
}

pub(super) fn cut_at(
    source: &str,
    values: &[SlotValue],
    taints: &[Taint],
    stop: (StepIdx, StepState),
) -> TestResult<Cut> {
    cut_at_nth(source, values, taints, stop, 1)
}

pub(super) fn cut_at_nth(
    source: &str,
    values: &[SlotValue],
    taints: &[Taint],
    stop: (StepIdx, StepState),
    occurrences: usize,
) -> TestResult<Cut> {
    cut_with_nth(source, stop, occurrences, |store| {
        let root_taint = taints
            .iter()
            .fold(Taint::Clean, |joined, taint| join_taint(joined, *taint));
        let list = store.insert_list_with_taint(
            values.to_vec().into_boxed_slice(),
            taints.to_vec().into_boxed_slice(),
        )?;
        Ok((list, root_taint))
    })
}

pub(super) fn cut_at_actions(
    source: &str,
    values: &[SlotValue],
    taints: &[Taint],
    stop: (StepIdx, StepState),
    contracts: &[ActionContract],
) -> TestResult<Cut> {
    cut_with_actions_nth(source, stop, 1, contracts, |store| {
        let root_taint = taints
            .iter()
            .fold(Taint::Clean, |joined, taint| join_taint(joined, *taint));
        let list = store.insert_list_with_taint(
            values.to_vec().into_boxed_slice(),
            taints.to_vec().into_boxed_slice(),
        )?;
        Ok((list, root_taint))
    })
}
pub(super) struct AcceptedRun {
    pub(super) directory: tempfile::TempDir,
}

pub(super) fn accepted_run(
    source: &str,
    values: &[SlotValue],
    taints: &[Taint],
) -> TestResult<AcceptedRun> {
    if values.len() != taints.len() {
        return Err("accepted input values and taints have different lengths".into());
    }
    let workflow = vb_compile::compile_workflow(source.as_bytes())?;
    let mut store = ValueStore::with_max_slots(workflow.resource_contract().max_slots);
    let root_taint = taints
        .iter()
        .fold(Taint::Clean, |joined, taint| join_taint(joined, *taint));
    let list = store.insert_list_with_taint(
        values.to_vec().into_boxed_slice(),
        taints.to_vec().into_boxed_slice(),
    )?;
    let input = workflow
        .input_slots()
        .first()
        .ok_or("missing input slot")?
        .slot;
    let bundle = OwnedRootBundle::new(
        Box::from([(input, SlotValue::List(list), root_taint)]),
        store.checkpoint_v1(),
        &workflow,
    )?;

    let artifact =
        vb_storage::admission::prepare_compiled_artifact(&workflow, RuntimePolicy::Strict, &[])?;
    let source_record = WorkflowSourceRecord {
        digest: WorkflowDigest::from_bytes(blake3::hash(source.as_bytes()).into()),
        source: source.as_bytes().to_vec(),
    };
    let source_digest = source_record.digest;
    let inputs = vb_storage::OwnedRunInputsRecordV1::new(
        RUN,
        artifact.digest,
        source_digest,
        &bundle,
        &workflow,
    )?;
    let header = RunHeaderRecord {
        run: RUN,
        workflow_id: vb_core::WorkflowId::new(0),
        compiled_digest: workflow.digest(),
        status: 0,
        accepted_at_ms: 123,
    };
    let submission = RunSubmission::owned(source_record, artifact, inputs, header)?;
    let directory = tempfile::tempdir()?;
    FjallJournal::open(directory.path(), None)?
        .commit_run_submission(&submission, RuntimePolicy::Strict)?;
    Ok(AcceptedRun { directory })
}

pub(super) fn replay_parity(cut: &Cut) -> TestResult<vb_storage::recovery::OwnedForEachReplay> {
    let replay = replay_owned_for_each(
        &cut.events,
        &cut.workflow,
        &cut.roots,
        Some(&cut.input_store),
    )?
    .ok_or("owned ForEach replay not selected")?;
    assert_eq!(replay.frame.checkpoint_v1(), cut.live_frame);
    assert_eq!(replay.store, cut.live_store);
    Ok(replay)
}

pub(super) fn recover_and_finish(cut: &Cut) -> TestResult<Vec<JournalEvent>> {
    let journal = std::sync::Arc::new(FjallJournal::open(cut.directory.path(), None)?);
    let mut runtime = runtime(journal.clone(), 1);
    let product = runtime
        .recover_product(RUN)
        .map_err(|error| format!("recover_product: {error:?}"))?;
    match &product {
        RuntimeRecoveryProduct::Resumable(_) => {}
        other => return Err(format!("owned ForEach recovery is not resumable: {other:?}").into()),
    }
    runtime
        .recover_and_resume(RUN)
        .map_err(|error| format!("recover_and_resume: {error:?}"))?;
    for _ in 0..256 {
        runtime.tick_all()?;
        assert_eq!(runtime.counters_snapshot().runs_failed, 0);
        if runtime.counters_snapshot().runs_completed == 1 {
            break;
        }
    }
    assert_eq!(runtime.counters_snapshot().runs_completed, 1);
    Ok(journal.events_for_run_full(RUN)?)
}

pub(super) fn terminal_result(events: &[JournalEvent]) -> TestResult<(SlotValue, Taint)> {
    let result = events
        .iter()
        .rev()
        .find_map(|event| match event {
            JournalEvent::RunFinished { result, .. } => Some(*result),
            _ => None,
        })
        .ok_or("missing RunFinished")?;
    let (value, extra) = events
        .iter()
        .rev()
        .find_map(|event| match event {
            JournalEvent::SlotWrittenEvent {
                slot,
                value: Some(value),
                extra,
                ..
            } if *slot == result => Some((value, extra)),
            _ => None,
        })
        .ok_or("missing terminal slot write")?;
    let value = postcard::from_bytes(value)?;
    let extra = extra.as_deref().ok_or("missing taint extra")?;
    let taint = match decode_slot_written_extra(extra).map_err(|error| format!("{error:?}"))? {
        DecodedSlotWrittenExtra::Envelope(envelope) => envelope.taint,
        DecodedSlotWrittenExtra::LegacyFrameExtra(_) => return Err("legacy taint extra".into()),
    };
    Ok((value, taint))
}
