#[path = "support/ipc_process_crash_recovery.rs"]
mod support;

use std::os::unix::process::ExitStatusExt;
use std::path::Path;
use support::{CrashFixture, TestResult};
use vb_core::{CompiledNodeKind, RunId, SlotIdx, SlotValue, StepIdx, WorkflowDigest};
use vb_ipc::server::IpcResponse;
use vb_ipc::{IpcCommand, IpcPayload, IpcTraceEvent, IpcTraceEventKind};
use vb_storage::{CompiledIrRecord, EventReplayLimit, EventSeq, FjallJournal, JournalEvent};

const MAX_EVENTS: usize = 64;
const MAX_EVIDENCE_BYTES: usize = 1_048_576;
const TRACE_POLLS: u64 = 16;

struct Readback {
    artifact: CompiledIrRecord,
    events: Vec<JournalEvent>,
    raw: Vec<(EventSeq, Vec<u8>)>,
}

fn scalar_ask_workflow() -> TestResult<vb_core::CompiledWorkflow> {
    let mut parts = vb_core::WorkflowParts {
        name: "ipc-process-crash-recovery".into(),
        digest: WorkflowDigest::from_bytes([0; 32]),
        semantic_source_identity: None,
        source_metadata: None,
        nodes: Box::from([
            vb_core::workflow::node(
                StepIdx::new(0),
                CompiledNodeKind::SetConst {
                    value: vb_core::ConstIdx::new(0),
                },
                Some(SlotIdx::new(0)),
                Some(StepIdx::new(1)),
            ),
            vb_core::workflow::node(
                StepIdx::new(1),
                CompiledNodeKind::Ask {
                    prompt: SlotIdx::new(0),
                    timeout_slot: None.into(),
                },
                Some(SlotIdx::new(1)),
                Some(StepIdx::new(2)),
            ),
            vb_core::workflow::node(
                StepIdx::new(2),
                CompiledNodeKind::AskResume {
                    answer: SlotIdx::new(1),
                },
                Some(SlotIdx::new(1)),
                Some(StepIdx::new(3)),
            ),
            vb_core::workflow::node(
                StepIdx::new(3),
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(1),
                },
                None,
                None,
            ),
        ]),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::from([vb_core::ConstValue::I64(7)]),
        slot_count: 2,
        symbols_count: 0,
        entry: StepIdx::new(0),
        step_names: Box::new([]),
        resource_contract: vb_core::ResourceContract::DEFAULT,
        input_slots: Box::new([]),
    };
    parts.digest =
        WorkflowDigest::from_bytes(*blake3::hash(&postcard::to_allocvec(&parts)?).as_bytes());
    Ok(vb_core::CompiledWorkflow::try_from_parts(parts)?)
}

fn seed_artifact(path: &Path, run: RunId) -> TestResult<(WorkflowDigest, CompiledIrRecord)> {
    let workflow = scalar_ask_workflow()?;
    let cache = vb_storage::FjallConfig::try_new(1_048_576).ok_or("invalid seed cache")?;
    let journal = FjallJournal::open(path, Some(cache))?;
    let accepted =
        vb_storage::submit_artifact(&journal, &workflow, vb_core::RuntimePolicy::Strict)?;
    let record = journal
        .compiled_ir(accepted.digest)?
        .ok_or("seeded artifact missing")?;
    assert!(journal.events_for_run_full(run)?.is_empty());
    assert!(journal.run_header(run)?.is_none());
    Ok((workflow.digest(), record))
}

fn readback(journal: &FjallJournal, run: RunId, digest: WorkflowDigest) -> TestResult<Readback> {
    let limit = EventReplayLimit::new(MAX_EVENTS).ok_or("invalid history bound")?;
    let events = journal.events_for_run_full_bounded(run, limit)?;
    let artifact = journal
        .compiled_ir(digest)?
        .ok_or("retained artifact missing")?;
    assert!(journal.run_header(run)?.is_none());
    let mut raw = Vec::new();
    raw.try_reserve(events.len())?;
    let mut captured = artifact.ir.len();
    if captured > MAX_EVIDENCE_BYTES {
        return Err("artifact exceeds evidence bound".into());
    }
    for event in &events {
        let bytes = journal
            .get_event_bytes(run, event.seq())?
            .ok_or("raw journal event missing")?;
        captured = captured
            .checked_add(bytes.len())
            .ok_or("evidence length overflow")?;
        if captured > MAX_EVIDENCE_BYTES {
            return Err("journal exceeds evidence bound".into());
        }
        raw.push((event.seq(), bytes));
    }
    Ok(Readback {
        artifact,
        events,
        raw,
    })
}

fn await_trace(
    fixture: &mut CrashFixture,
    run: RunId,
    correlation: u64,
    matches: impl Fn(&IpcTraceEventKind) -> bool,
) -> TestResult<Vec<IpcTraceEvent>> {
    let payload = postcard::to_allocvec(&IpcPayload::ListEvents {
        run_id: run,
        from_sequence: 0,
    })?;
    for poll in 0..TRACE_POLLS {
        let correlation = correlation
            .checked_add(poll)
            .ok_or("correlation overflow")?;
        let response = fixture.request(IpcCommand::ListEvents, correlation, &payload)?;
        let IpcResponse::Events { events } = response else {
            return Err("ListEvents did not return events".into());
        };
        assert!(events.len() <= MAX_EVENTS);
        if events.iter().any(|event| matches(&event.kind)) {
            return Ok(events);
        }
    }
    Err("trace boundary did not appear within bounded requests".into())
}

fn scheduled_ask(events: &[JournalEvent], run: RunId) -> TestResult<(EventSeq, StepIdx, u16)> {
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, JournalEvent::AskScheduledEvent { .. }))
            .count(),
        1
    );
    let (event_run, seq, step, attempt) = events
        .iter()
        .find_map(|event| match event {
            JournalEvent::AskScheduledEvent {
                run,
                seq,
                step,
                attempt,
            } => Some((*run, *seq, *step, *attempt)),
            _ => None,
        })
        .ok_or("durable Ask scheduling event missing")?;
    assert_eq!(event_run, run);
    assert_eq!(step, StepIdx::new(1));
    assert_eq!(attempt, 1);
    Ok((seq, step, attempt))
}

fn assert_single_submission_and_prefix_steps(events: &[JournalEvent]) {
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, JournalEvent::RunAccepted { .. }))
            .count(),
        1
    );
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, JournalEvent::RunAdmissionBudget { .. }))
            .count(),
        1
    );
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, JournalEvent::RunAdmission { .. }))
            .count(),
        0
    );
    for original_step in [StepIdx::new(0), StepIdx::new(1)] {
        assert_eq!(
            events
                .iter()
                .filter(|event| matches!(
                    event,
                    JournalEvent::StepStarted { step, .. } if *step == original_step
                ))
                .count(),
            1
        );
    }
}

fn assert_no_failed_outcomes(events: &[JournalEvent]) {
    assert!(!events.iter().any(|event| matches!(
        event,
        JournalEvent::RunFailedEvent { .. }
            | JournalEvent::RunCancelled { .. }
            | JournalEvent::RunKilled { .. }
            | JournalEvent::AskTimedOutEvent { .. }
            | JournalEvent::StepFailed { .. }
            | JournalEvent::ActionFailedEvent { .. }
            | JournalEvent::ActionAbandoned { .. }
    )));
}

fn assert_prompt_value(events: &[JournalEvent]) -> TestResult {
    let last_write = events
        .iter()
        .rev()
        .find_map(|event| match event {
            JournalEvent::SlotWrittenEvent { slot, value, .. } if *slot == SlotIdx::new(0) => {
                Some(value)
            }
            _ => None,
        })
        .ok_or("prompt slot has no write")?;
    let bytes = last_write
        .as_deref()
        .ok_or("prompt slot value was not captured")?;
    assert_eq!(bytes, postcard::to_allocvec(&SlotValue::I64(7))?.as_slice());
    assert_eq!(postcard::from_bytes::<SlotValue>(bytes)?, SlotValue::I64(7));
    Ok(())
}

fn assert_completed_result(
    events: &[JournalEvent],
    run: RunId,
    schedule: (EventSeq, StepIdx, u16),
) -> TestResult {
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, JournalEvent::AskAnsweredEvent { .. }))
            .count(),
        1
    );
    assert!(events.iter().any(|event| matches!(
        event,
        JournalEvent::AskAnsweredEvent { run: id, seq, step, attempt }
            if *id == run && *seq > schedule.0
                && *step == schedule.1 && *attempt == schedule.2
    )));
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, JournalEvent::RunFinished { .. }))
            .count(),
        1
    );
    let (finish_index, result) = events
        .iter()
        .enumerate()
        .find_map(|(index, event)| match event {
            JournalEvent::RunFinished {
                run: id,
                result,
                attempt,
                ..
            } if *id == run && *attempt == schedule.2 => Some((index, *result)),
            _ => None,
        })
        .ok_or("matching terminal event missing")?;
    assert_eq!(result, SlotIdx::new(1));
    let last_write = events
        .get(..finish_index)
        .ok_or("terminal position missing")?
        .iter()
        .rev()
        .find_map(|event| match event {
            JournalEvent::SlotWrittenEvent { slot, value, .. } if *slot == result => Some(value),
            _ => None,
        })
        .ok_or("result slot has no pre-terminal write")?;
    let bytes = last_write
        .as_deref()
        .ok_or("terminal result was not captured")?;
    assert_eq!(
        bytes,
        postcard::to_allocvec(&SlotValue::I64(77))?.as_slice()
    );
    assert_eq!(
        postcard::from_bytes::<SlotValue>(bytes)?,
        SlotValue::I64(77)
    );
    Ok(())
}

#[test]
fn real_ipc_process_recovers_untimed_ask_after_sigkill() -> TestResult {
    let mut fixture = CrashFixture::new()?;
    let run = RunId::new(86_069);
    let (workflow, original_artifact) = seed_artifact(fixture.db_path(), run)?;
    fixture.start_server("first.sock")?;
    let payload = postcard::to_allocvec(&IpcPayload::SubmitRun(vb_ipc::SubmitRunPayload {
        run_id: run,
        workflow,
        input: Vec::new(),
    }))?;
    assert_eq!(
        fixture.request(IpcCommand::SubmitRun, 100, &payload)?,
        IpcResponse::AcceptedRun { run_id: run.get() }
    );
    let trace = await_trace(&mut fixture, run, 200, |kind| {
        matches!(kind, IpcTraceEventKind::StepStarted { run: id, step }
            if *id == run && *step == StepIdx::new(1))
    })?;
    assert!(!trace.iter().any(|event| matches!(
        &event.kind,
        IpcTraceEventKind::RunFinished { .. }
            | IpcTraceEventKind::RunFailed { .. }
            | IpcTraceEventKind::RunCancelled { .. }
    )));
    let killed = fixture.kill_and_reap()?;
    assert_eq!(killed.signal(), Some(9));
    fixture.copy_after_exit()?;

    fixture.start_server("second.sock")?;
    let answer = postcard::to_allocvec(&IpcPayload::AnswerAsk {
        run_id: run,
        ticket: u64::from(StepIdx::new(1).get()),
        answer: postcard::to_allocvec(&SlotValue::I64(77))?,
        taint: Some(vb_core::Taint::Clean),
    })?;
    assert_eq!(
        fixture.request(IpcCommand::AnswerAsk, 300, &answer)?,
        IpcResponse::AcceptedRun { run_id: run.get() }
    );
    await_trace(
        &mut fixture,
        run,
        400,
        |kind| matches!(kind, IpcTraceEventKind::RunFinished { run: id } if *id == run),
    )?;
    assert_eq!(
        fixture.request(IpcCommand::Shutdown, 500, &[])?,
        IpcResponse::ShuttingDown
    );
    assert!(fixture.wait_for_exit()?.success());

    let crashed =
        fixture.inspect_copy(|journal| readback(journal, run, original_artifact.digest))?;
    assert_eq!(crashed.artifact, original_artifact);
    assert_single_submission_and_prefix_steps(&crashed.events);
    assert_no_failed_outcomes(&crashed.events);
    assert_prompt_value(&crashed.events)?;
    assert_eq!(
        crashed
            .events
            .iter()
            .filter(|event| matches!(event, JournalEvent::StepStarted { .. }))
            .count(),
        2
    );
    assert!(matches!(
        crashed.events.first(),
        Some(JournalEvent::RunAccepted { run: id, workflow: digest, .. })
            if *id == run && *digest == workflow
    ));
    assert!(crashed.events.iter().any(|event| matches!(
        event,
        JournalEvent::RunAdmissionBudget { artifact_digest, policy, .. }
            if *artifact_digest == original_artifact.digest
                && *policy == vb_core::RuntimePolicy::Strict
    )));
    let schedule = scheduled_ask(&crashed.events, run)?;
    assert!(!crashed.events.iter().any(|event| matches!(
        event,
        JournalEvent::AskAnsweredEvent { .. }
            | JournalEvent::AskTimedOutEvent { .. }
            | JournalEvent::RunFinished { .. }
            | JournalEvent::RunFailedEvent { .. }
            | JournalEvent::RunCancelled { .. }
            | JournalEvent::RunKilled { .. }
    )));
    fixture.delete_copy()?;
    fixture.copy_after_exit()?;
    let completed =
        fixture.inspect_copy(|journal| readback(journal, run, original_artifact.digest))?;
    assert_eq!(completed.artifact, original_artifact);
    assert_single_submission_and_prefix_steps(&completed.events);
    assert_no_failed_outcomes(&completed.events);
    assert_prompt_value(&completed.events)?;
    assert!(completed.events.starts_with(&crashed.events));
    assert!(completed.raw.starts_with(&crashed.raw));
    assert_eq!(scheduled_ask(&completed.events, run)?, schedule);
    assert_completed_result(&completed.events, run, schedule)?;
    fixture.delete_copy()?;
    fixture.finish()
}
