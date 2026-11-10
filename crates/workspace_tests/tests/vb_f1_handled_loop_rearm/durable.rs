use super::{K, SlotValue, TestResult, repeated, source_with};
use std::{num::NonZeroUsize, path::Path, sync::Arc};
use vb_core::{RunId, RuntimePolicy, StepIdx, StepState, Taint, WorkflowDigest};
use vb_runtime::{
    journal::StorageRuntimeJournal,
    recovery::{RecoveredRunBoundaryKind, RuntimeRecoveryProduct},
    runtime::Runtime,
    shard::{AskAnswer, AskTicket, ShardConfig},
};
use vb_storage::{FjallJournal, JournalEvent as E};

pub(super) const RUN: RunId = RunId::new(514);

pub(super) fn open(path: &Path) -> TestResult<(Arc<FjallJournal>, Runtime)> {
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

pub(super) fn pending(runtime: &mut Runtime, ask: StepIdx) -> TestResult<AskTicket> {
    for _ in 0..128 {
        runtime.tick_all()?;
        assert_eq!(runtime.counters_snapshot().runs_failed, 0);
        if let Some(ticket) = runtime.lookup_pending_ask_ticket(RUN, ask) {
            return Ok(ticket);
        }
    }
    Err("Ask boundary not reached".into())
}

pub(super) fn submission(
    yaml: &str,
    workflow: &vb_core::CompiledWorkflow,
) -> TestResult<vb_storage::RunSubmission> {
    let artifact =
        vb_storage::admission::prepare_compiled_artifact(workflow, RuntimePolicy::Strict, &[])?;
    let source = vb_storage::WorkflowSourceRecord {
        digest: WorkflowDigest::from_bytes(blake3::hash(yaml.as_bytes()).into()),
        source: yaml.as_bytes().to_vec(),
    };
    let inputs = vb_storage::RunInputsRecord::new(
        RUN,
        artifact.digest,
        source.digest,
        workflow,
        Box::new([]),
    )?;
    Ok(vb_storage::RunSubmission::new(
        source,
        artifact,
        inputs,
        vb_storage::RunHeaderRecord {
            run: RUN,
            workflow_id: vb_core::WorkflowId::new(0),
            compiled_digest: workflow.digest(),
            status: 0,
            accepted_at_ms: 123,
        },
    )?)
}

#[test]
fn scalar_repeat_local_ask_recovery_survives_real_fjall_reopen_at_both_iterations() -> TestResult {
    let body = "- id: failing\n  on_error: {handler: recovered}\n  set: {output: quotient, value: '1 / $zero'}\n- id: recovered\n  ask: {prompt: '$vars.prompt'}\n";
    let yaml = source_with(
        "vars: {prompt: 'recover handled error'}\n",
        &repeated("outer", body),
        "$step.outer",
    );
    let workflow = vb_compile::compile_workflow(yaml.as_bytes())?;
    let parts = workflow.to_parts();
    let failed = parts
        .nodes
        .iter()
        .find(|node| matches!(node.kind, K::EvalExpr { .. }))
        .ok_or("failing node")?
        .id;
    let ask = workflow
        .node(failed)
        .and_then(|node| node.routing.error.handler())
        .ok_or("local handler")?;
    assert!(matches!(
        workflow.node(ask).map(|node| &node.kind),
        Some(K::Ask { .. })
    ));
    let metadata = workflow.source_metadata().ok_or("metadata")?;
    let prompt = vb_core::SymbolId::new(u32::try_from(
        metadata
            .symbols()
            .iter()
            .position(|value| value.as_ref() == "recover handled error")
            .ok_or("prompt payload")?,
    )?);
    assert_eq!(metadata.symbol(prompt), Some("recover handled error"));
    assert_eq!(workflow.input_slots().len(), 0);
    let input = match workflow.node(ask).map(|node| &node.kind) {
        Some(K::Ask { prompt, .. }) => *prompt,
        _ => return Err("Ask prompt slot".into()),
    };
    let submission = submission(&yaml, &workflow)?;
    let directory = tempfile::tempdir()?;
    let (mut journal, mut runtime) = open(directory.path())?;
    journal.commit_run_submission(&submission, RuntimePolicy::Strict)?;
    runtime.recover_and_resume(RUN)?;
    let mut tickets = Vec::new();
    for (answer, taint) in [
        (SlotValue::I64(73), Taint::DerivedFromSecret),
        (SlotValue::I64(74), Taint::Clean),
    ] {
        let ticket = pending(&mut runtime, ask)?;
        assert!(!tickets.contains(&ticket));
        tickets.push(ticket);
        let live = runtime.inspect_run_state(RUN)?;
        assert_eq!(live.frame.step_state(failed)?, StepState::Failed);
        assert_eq!(live.frame.step_state(ask)?, StepState::Asking);
        assert_eq!(live.frame.read_slot(input)?, &SlotValue::Symbol(prompt));
        assert_eq!(live.store.symbol(prompt)?, "recover handled error");
        let before = live.frame.checkpoint_v1();
        let before_store = live.store.checkpoint_v1();
        let events_before = journal.events_for_run_full(RUN)?;
        drop(runtime);
        drop(journal);
        (journal, runtime) = open(directory.path())?;
        assert_eq!(journal.events_for_run_full(RUN)?, events_before);
        let product = runtime.recover_product(RUN)?;
        let RuntimeRecoveryProduct::Resumable(product) = product else {
            return Err(format!("not resumable: {product:?}").into());
        };
        assert_eq!(product.boundary_kind(), RecoveredRunBoundaryKind::OpenAsk);
        runtime.recover_and_resume(RUN)?;
        runtime.tick_all()?;
        let restored = runtime.inspect_run_state(RUN)?;
        assert_eq!(restored.frame.step_state(failed)?, StepState::Failed);
        assert_eq!(restored.frame.step_state(ask)?, StepState::Asking);
        let after = restored.frame.checkpoint_v1();
        assert_eq!(after.executed, before.executed);
        assert_eq!(after.max_parallel_in_flight, before.max_parallel_in_flight);
        assert_eq!(after.parallel_in_flight, before.parallel_in_flight);
        assert_eq!(after.parallel_in_flight, 0);
        assert_eq!(after.pc, before.pc);
        assert_eq!(after.states, before.states);
        assert_eq!(after.slots, before.slots);
        assert_eq!(after.taint, before.taint);
        assert_eq!(after, before);
        assert_eq!(restored.store.checkpoint_v1(), before_store);
        reject_incomplete_projections(&events_before, &workflow, ask)?;
        assert_eq!(runtime.lookup_pending_ask_ticket(RUN, ask), Some(ticket));
        runtime.answer_ask(AskAnswer::new(ticket, ticket.answer_slot(), answer, taint))?;
    }
    for _ in 0..128 {
        runtime.tick_all()?;
        assert_eq!(runtime.counters_snapshot().runs_failed, 0);
        if runtime.counters_snapshot().runs_completed == 1 {
            break;
        }
    }
    assert_eq!(runtime.counters_snapshot().runs_completed, 1);
    let events = journal.events_for_run_full(RUN)?;
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, E::StepFailed { .. }))
            .count(),
        2
    );
    assert_eq!(
        events
            .iter()
            .filter(
                |event| matches!(event, E::StepFailed { step, attempt: 1, .. } if *step == failed)
            )
            .count(),
        2
    );
    assert_eq!(
        events
            .iter()
            .filter(
                |event| matches!(event, E::StepStarted { step, attempt: 1, .. } if *step == failed)
            )
            .count(),
        2
    );
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, E::AskScheduledEvent { step, .. } if *step == ask))
            .count(),
        2
    );
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, E::AskAnsweredEvent { step, .. } if *step == ask))
            .count(),
        2
    );
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, E::RunFailedEvent { .. }))
            .count(),
        0
    );
    let results = events
        .iter()
        .filter_map(|event| match event {
            E::RunFinished { result, .. } => Some(*result),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(results.len(), 1);
    let result = results.first().ok_or("finish result")?;
    let encoded = events
        .iter()
        .rev()
        .find_map(|event| match event {
            E::SlotWrittenEvent { slot, value, .. } if slot == result => Some(value),
            _ => None,
        })
        .ok_or("result write")?;
    assert_eq!(
        postcard::from_bytes::<SlotValue>(encoded.as_deref().ok_or("result payload")?)?,
        SlotValue::I64(74)
    );
    reject_completed_projections(&events, &workflow, ask, failed)?;
    Ok(())
}

fn reject_incomplete_projections(
    events: &[E],
    workflow: &vb_core::CompiledWorkflow,
    ask: StepIdx,
) -> TestResult {
    // These are read-only corruptions of the real trace, never journal writes.
    for taint in [false, true] {
        let mut projection = events.to_vec();
        let event = projection
            .iter_mut()
            .find(|event| matches!(event, E::SlotWrittenEvent { .. }))
            .ok_or("captured row")?;
        if let E::SlotWrittenEvent { value, extra, .. } = event {
            if taint {
                *extra = None;
            } else {
                *value = None;
            }
        }
        let result =
            vb_storage::recovery::complete_scalar::replay_complete_scalar(&projection, workflow);
        let expected = if taint {
            "missing captured taint"
        } else {
            "missing captured value"
        };
        reject(result, workflow.entry(), expected)?;
    }
    let mut projection = events.to_vec();
    projection.pop().ok_or("last event")?;
    reject(
        vb_storage::recovery::complete_scalar::replay_complete_scalar(&projection, workflow),
        ask,
        "incomplete step receipt",
    )?;
    for attempt in [false, true] {
        let mut projection = events.to_vec();
        let event = projection
            .iter_mut()
            .find(|event| matches!(event, E::StepStarted { .. }))
            .ok_or("started row")?;
        if let E::StepStarted {
            step,
            attempt: run_attempt,
            ..
        } = event
        {
            if attempt {
                *run_attempt = 2;
            } else {
                *step = StepIdx::new(workflow.node_count().checked_sub(1).ok_or("node count")?);
            }
        }
        let expected = if attempt {
            "unproven bound run attempt"
        } else {
            "StepStarted differs from replay control edge"
        };
        let expected_step = if attempt {
            workflow.entry()
        } else {
            StepIdx::new(workflow.node_count().checked_sub(1).ok_or("node count")?)
        };
        reject(
            vb_storage::recovery::complete_scalar::replay_complete_scalar(&projection, workflow),
            expected_step,
            expected,
        )?;
    }
    let failed = events
        .iter()
        .position(|event| matches!(event, E::StepFailed { .. }))
        .ok_or("failed receipt")?;
    let mut deleted = events.to_vec();
    deleted.remove(failed);
    reject(
        vb_storage::recovery::complete_scalar::replay_complete_scalar(&deleted, workflow),
        ask,
        "incomplete bound event sequence",
    )?;
    let mut reordered = events.to_vec();
    reordered.swap(failed, failed.checked_add(1).ok_or("event offset")?);
    reject(
        vb_storage::recovery::complete_scalar::replay_complete_scalar(&reordered, workflow),
        ask,
        "incomplete bound event sequence",
    )?;
    Ok(())
}

fn replay(
    events: &[E],
    workflow: &vb_core::CompiledWorkflow,
) -> vb_storage::recovery::RecoveryResult<
    Option<vb_storage::recovery::complete_scalar::CompleteScalarReplay>,
> {
    vb_storage::recovery::complete_scalar::replay_complete_scalar(events, workflow)
}

fn reject_completed_projections(
    events: &[E],
    workflow: &vb_core::CompiledWorkflow,
    ask: StepIdx,
    failed: StepIdx,
) -> TestResult {
    let Some(_) = replay(events, workflow)? else {
        return Err("complete final trace fell back".into());
    };
    let parts = workflow.to_parts();
    let finish = parts
        .nodes
        .iter()
        .find(|node| matches!(node.kind, K::Finish { .. }))
        .ok_or("Finish node")?
        .id;
    let zero = events
        .iter()
        .find_map(|event| match event {
            E::SlotWrittenEvent {
                slot,
                value: Some(bytes),
                ..
            } if postcard::from_bytes::<SlotValue>(bytes).ok() == Some(SlotValue::I64(0)) => {
                Some(*slot)
            }
            _ => None,
        })
        .ok_or("initialized zero slot")?;
    let mut projection = events.to_vec();
    let terminal = projection
        .iter_mut()
        .find(|event| matches!(event, E::RunFinished { .. }))
        .ok_or("terminal receipt")?;
    if let E::RunFinished { result, .. } = terminal {
        *result = zero;
    }
    reject(
        replay(&projection, workflow),
        finish,
        "terminal result differs from Finish node",
    )?;
    let mut projection = events.to_vec();
    projection.pop().ok_or("terminal row")?;
    reject(
        replay(&projection, workflow),
        finish,
        "terminal receipt missing",
    )?;

    let mut projection = events.to_vec();
    let success = projection
        .iter_mut()
        .find(|event| matches!(event, E::StepSucceeded { .. }))
        .ok_or("success receipt")?;
    let success_step = if let E::StepSucceeded { step, output, .. } = success {
        *output = zero;
        *step
    } else {
        return Err("success shape".into());
    };
    reject(
        replay(&projection, workflow),
        success_step,
        "success output receipt differs from compiled writer",
    )?;

    let answer = events
        .iter()
        .position(|event| matches!(event, E::AskAnsweredEvent { .. }))
        .ok_or("answer receipt")?;
    let payload = answer.checked_sub(1).ok_or("answer payload row")?;
    let mut projection = events.to_vec();
    if let Some(E::SlotWrittenEvent { value, .. }) = projection.get_mut(payload) {
        *value = Some(postcard::to_allocvec(&SlotValue::Symbol(
            vb_core::SymbolId::new(
                workflow
                    .source_metadata()
                    .ok_or("metadata")?
                    .symbols_count(),
            ),
        ))?);
    } else {
        return Err("actual answer payload shape".into());
    }
    reject(
        replay(&projection, workflow),
        ask,
        "Ask symbol outside replay source prefix",
    )?;
    let mut projection = events.to_vec();
    projection.remove(answer);
    reject(
        replay(&projection, workflow),
        ask,
        "incomplete bound event sequence",
    )?;
    let mut projection = events.to_vec();
    if let Some(E::AskAnsweredEvent { step, .. }) = projection.get_mut(answer) {
        *step = failed;
    }
    reject(
        replay(&projection, workflow),
        failed,
        "Ask answer authority mismatch",
    )?;

    let control = parts
        .nodes
        .iter()
        .find(|node| matches!(node.kind, K::RepeatCheck { .. }))
        .ok_or("loop latch")?
        .id;
    let latch = events
        .iter()
        .position(|event| matches!(event, E::StepSucceeded { step, .. } if *step == control))
        .ok_or("latch receipt")?;
    let body = workflow.loop_region(control).ok_or("owned region")?.body();
    let mut projection = events.to_vec();
    projection.remove(latch);
    reject(
        replay(&projection, workflow),
        body,
        "incomplete bound event sequence",
    )?;
    let mut projection = events.to_vec();
    if let Some(E::StepSucceeded { step, .. }) = projection.get_mut(latch) {
        *step = failed;
    }
    reject(
        replay(&projection, workflow),
        failed,
        "success does not match execution",
    )?;

    let counter = parts
        .nodes
        .iter()
        .find_map(|node| match node.kind {
            K::RepeatCheck { attempt_slot, .. } => Some(attempt_slot),
            _ => None,
        })
        .ok_or("counter slot")?;
    let writes = events
        .iter()
        .enumerate()
        .filter_map(|(index, event)| match event {
            E::SlotWrittenEvent {
                slot,
                value: Some(bytes),
                ..
            } if *slot == counter => Some((index, bytes)),
            _ => None,
        })
        .collect::<Vec<_>>();
    let initial = writes.first().ok_or("initial counter")?;
    let changed = writes
        .iter()
        .find(|(_, bytes)| *bytes != initial.1)
        .ok_or("changed counter")?;
    let mut projection = events.to_vec();
    if let Some(E::SlotWrittenEvent { value, .. }) = projection.get_mut(changed.0) {
        *value = Some(initial.1.clone());
    }
    reject(
        replay(&projection, workflow),
        control,
        "capture differs from artifact replay",
    )?;
    let mut projection = events.to_vec();
    if let Some(E::SlotWrittenEvent { extra, .. }) = projection.get_mut(changed.0) {
        *extra = Some(
            vb_storage::slot_extra::encode_slot_written_extra(Taint::Secret, None)
                .map_err(|error| format!("taint capture: {error:?}"))?,
        );
    }
    reject(
        replay(&projection, workflow),
        control,
        "capture differs from artifact replay",
    )?;
    let mut projection = events.to_vec();
    if let Some(E::SlotWrittenEvent { extra, .. }) = projection.get_mut(changed.0) {
        *extra = Some(
            vb_storage::slot_extra::encode_slot_written_extra(Taint::Clean, Some(vec![0]))
                .map_err(|error| format!("frame capture: {error:?}"))?,
        );
    }
    reject(
        replay(&projection, workflow),
        control,
        "unproven scalar frame component",
    )?;

    let mut projection = events.to_vec();
    let wrong = WorkflowDigest::from_bytes([0xa5; 32]);
    assert_ne!(wrong, workflow.digest());
    for event in &mut projection {
        if let E::RunAccepted { workflow, .. } = event {
            *workflow = wrong;
        }
    }
    match vb_storage::recovery::recover_runtime_frame_seed_from_events_with_workflow(
        &projection,
        workflow,
    ) {
        Err(vb_storage::recovery::RecoveryError::CompiledIrDigestMismatch { expected, found }) => {
            assert_eq!(expected, workflow.digest());
            assert_eq!(found, wrong);
        }
        _ => return Err("wrong workflow artifact was not rejected".into()),
    }
    Ok(())
}

fn reject(
    result: vb_storage::recovery::RecoveryResult<
        Option<vb_storage::recovery::complete_scalar::CompleteScalarReplay>,
    >,
    expected_step: StepIdx,
    expected: &str,
) -> TestResult {
    match result {
        Err(vb_storage::recovery::RecoveryError::ReplayDivergence { step, detail }) => {
            assert_eq!(step, expected_step);
            assert_eq!(detail, expected);
            Ok(())
        }
        _ => Err(format!("incomplete trace did not produce exact divergence: {expected}").into()),
    }
}
