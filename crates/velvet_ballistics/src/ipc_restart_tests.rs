use super::{
    IpcJournalService, STARTUP_HEADER_SCAN_LIMIT, StorageWorkflowResolver, recover_startup,
};
use std::io::{Read, Write};
use std::num::NonZeroUsize;
use std::os::unix::net::UnixStream;
use std::sync::Arc;
use std::time::Duration;
use vb_core::{CompiledNodeKind, RunId, SlotIdx, SlotValue, StepIdx};
use vb_ipc::server::{IpcResponse, IpcServer};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

struct RestartSession {
    service: IpcJournalService,
    runtime: vb_runtime::runtime::Runtime,
    resolver: StorageWorkflowResolver,
    server: IpcServer,
    client: UnixStream,
}

impl RestartSession {
    fn open(db: &std::path::Path, socket: &std::path::Path) -> TestResult<Self> {
        let journal = Arc::new(vb_storage::FjallJournal::open(db, None)?);
        let queue = Arc::new(vb_storage::JournalWriterQueue::new(
            1024,
            64,
            vb_storage::StorageLimits::DEFAULT,
        )?);
        let sink = vb_runtime::journal::RuntimeJournalConfig::new(
            vb_storage::DurabilityProfile::Journaled,
        )
        .shared_journal(Arc::clone(&journal), Arc::clone(&queue))?;
        let runtime = vb_runtime::runtime::Runtime::new(
            NonZeroUsize::MIN,
            vb_runtime::shard::ShardConfig::default(),
            sink,
        );
        let server = IpcServer::bind(socket)?;
        let client = UnixStream::connect(socket)?;
        client.set_write_timeout(Some(Duration::from_secs(1)))?;
        Ok(Self {
            resolver: StorageWorkflowResolver {
                journal: Arc::clone(&journal),
                available_names: Default::default(),
            },
            service: IpcJournalService { journal, queue },
            runtime,
            server,
            client,
        })
    }

    fn request(
        &mut self,
        command: vb_ipc::IpcCommand,
        correlation: u64,
        payload: &[u8],
    ) -> TestResult<(IpcResponse, bool)> {
        self.client.set_nonblocking(false)?;
        let header =
            vb_ipc::IpcFrameHeader::new(command, 0, correlation, u32::try_from(payload.len())?);
        self.client.write_all(&header.encode()?)?;
        self.client.write_all(payload)?;
        self.client.set_nonblocking(true)?;
        let mut received = Vec::new();
        for _ in 0..32 {
            let alive = self.service.turn(
                &mut self.server,
                &mut self.runtime,
                &mut self.resolver,
                Some(Duration::from_millis(5)),
            )?;
            let mut chunk = [0u8; 512];
            match self.client.read(&mut chunk) {
                Ok(0) => return Err("IPC closed before a complete response".into()),
                Ok(count) => {
                    let bytes = chunk.get(..count).ok_or("invalid response read count")?;
                    received.extend_from_slice(bytes);
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {}
                Err(error) => return Err(error.into()),
            }
            if received.len() > 4096 {
                return Err("unexpectedly large IPC response".into());
            }
            if received.len() >= vb_ipc::IPC_HEADER_LEN {
                let header_bytes = received
                    .get(..vb_ipc::IPC_HEADER_LEN)
                    .ok_or("response header missing")?;
                let header_bytes = <[u8; vb_ipc::IPC_HEADER_LEN]>::try_from(header_bytes)?;
                let response = vb_ipc::IpcFrameHeader::decode(
                    &header_bytes,
                    vb_ipc::MaxPayloadBytes::DEFAULT,
                )?;
                assert_eq!(response.correlation, correlation);
                let end = vb_ipc::IPC_HEADER_LEN
                    .checked_add(usize::try_from(response.payload_len)?)
                    .ok_or("response length overflow")?;
                if end > 4096 {
                    return Err("unexpectedly large response payload".into());
                }
                if received.len() >= end {
                    assert_eq!(received.len(), end);
                    let payload = received
                        .get(vb_ipc::IPC_HEADER_LEN..end)
                        .ok_or("response payload missing")?;
                    return Ok((postcard::from_bytes(payload)?, alive));
                }
            }
        }
        Err("IPC response did not arrive within bounded service turns".into())
    }

    fn shutdown(&mut self, correlation: u64) -> TestResult {
        let (response, alive) = self.request(vb_ipc::IpcCommand::Shutdown, correlation, &[])?;
        assert_eq!(response, IpcResponse::ShuttingDown);
        assert!(!alive);
        let pending = self.service.queue.pending_profile_counts()?;
        assert_eq!(pending.journaled, 0);
        assert_eq!(pending.strict, 0);
        Ok(())
    }
}

fn scalar_ask_workflow() -> TestResult<vb_core::CompiledWorkflow> {
    let mut parts = vb_core::WorkflowParts {
        name: "ipc-restart-scalar-ask".into(),
        digest: vb_core::WorkflowDigest::from_bytes([0; 32]),
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
                    timeout_slot: None,
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
    parts.digest = vb_core::WorkflowDigest::from_bytes(
        *blake3::hash(&postcard::to_allocvec(&parts)?).as_bytes(),
    );
    Ok(vb_core::CompiledWorkflow::try_from_parts(parts)?)
}

fn assert_open_ask_history(events: &[vb_storage::JournalEvent]) {
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, vb_storage::JournalEvent::AskScheduledEvent { .. }))
            .count(),
        1
    );
    assert!(!events.iter().any(|event| matches!(
        event,
        vb_storage::JournalEvent::AskAnsweredEvent { .. }
            | vb_storage::JournalEvent::AskTimedOutEvent { .. }
            | vb_storage::JournalEvent::RunFinished { .. }
            | vb_storage::JournalEvent::RunFailedEvent { .. }
            | vb_storage::JournalEvent::RunCancelled { .. }
            | vb_storage::JournalEvent::RunKilled { .. }
    )));
}

#[test]
fn socket_submitted_scalar_ask_survives_graceful_close_and_fjall_reopen() -> TestResult {
    let dir = tempfile::Builder::new()
        .prefix("vb68-")
        .tempdir_in("/tmp")?;
    let db = dir.path().join("journal");
    let first_socket = dir.path().join("first.sock");
    let second_socket = dir.path().join("second.sock");
    let run = RunId::new(86_068);
    let ask_step = StepIdx::new(1);
    let (original_ticket, original_history) = {
        let mut session = RestartSession::open(&db, &first_socket)?;
        let workflow = scalar_ask_workflow()?;
        vb_storage::submit_artifact(
            &session.service.journal,
            &workflow,
            vb_core::RuntimePolicy::Strict,
        )?;
        assert!(session.service.journal.events_for_run(run)?.is_empty());
        let payload =
            postcard::to_allocvec(&vb_ipc::IpcPayload::SubmitRun(vb_ipc::SubmitRunPayload {
                run_id: run,
                workflow: workflow.digest(),
                input: Vec::new(),
            }))?;
        let (response, alive) = session.request(vb_ipc::IpcCommand::SubmitRun, 101, &payload)?;
        assert_eq!(response, IpcResponse::AcceptedRun { run_id: run.get() });
        assert!(alive);
        let ticket = session
            .runtime
            .lookup_pending_ask_ticket(run, ask_step)
            .ok_or("socket submission did not install the untimed Ask")?;
        let history = session.service.journal.events_for_run(run)?;
        assert_open_ask_history(&history);
        session.shutdown(102)?;
        assert_eq!(
            session.runtime.lookup_pending_ask_ticket(run, ask_step),
            Some(ticket)
        );
        assert_eq!(session.service.journal.events_for_run(run)?, history);
        assert!(session.service.journal.run_header(run)?.is_none());
        (ticket, history)
    };
    let mut session = RestartSession::open(&db, &second_socket)?;
    assert!(session.service.journal.run_header(run)?.is_none());
    assert_eq!(
        session.service.journal.events_for_run(run)?,
        original_history
    );
    assert_eq!(
        session.runtime.lookup_pending_ask_ticket(run, ask_step),
        None
    );
    recover_startup(
        &session.service,
        &mut session.runtime,
        STARTUP_HEADER_SCAN_LIMIT,
    )?;
    assert_eq!(
        session.runtime.lookup_pending_ask_ticket(run, ask_step),
        Some(original_ticket)
    );
    assert_eq!(
        session.service.journal.events_for_run(run)?,
        original_history
    );
    let pending = session.service.queue.pending_profile_counts()?;
    assert_eq!(pending.journaled, 0);
    assert_eq!(pending.strict, 0);
    let payload = postcard::to_allocvec(&vb_ipc::IpcPayload::AnswerAsk {
        run_id: run,
        ticket: u64::from(ask_step.get()),
        answer: postcard::to_allocvec(&SlotValue::I64(77))?,
        taint: Some(vb_core::Taint::Clean),
    })?;
    let (response, alive) = session.request(vb_ipc::IpcCommand::AnswerAsk, 103, &payload)?;
    assert_eq!(response, IpcResponse::AcceptedRun { run_id: run.get() });
    assert!(alive);
    assert_eq!(
        session.runtime.lookup_pending_ask_ticket(run, ask_step),
        None
    );
    assert_eq!(session.runtime.counters_snapshot().runs_completed, 1);
    let completed = session.service.journal.events_for_run(run)?;
    assert!(completed.starts_with(&original_history));
    assert_eq!(
        completed
            .iter()
            .filter(|event| matches!(event, vb_storage::JournalEvent::AskScheduledEvent { .. }))
            .count(),
        1
    );
    assert_eq!(
        completed
            .iter()
            .filter(|event| matches!(event, vb_storage::JournalEvent::AskAnsweredEvent { .. }))
            .count(),
        1
    );
    assert_eq!(
        completed
            .iter()
            .filter(|event| matches!(event, vb_storage::JournalEvent::RunFinished { .. }))
            .count(),
        1
    );
    let expected_answer = postcard::to_allocvec(&SlotValue::I64(77))?;
    assert!(completed.iter().any(|event| matches!(event,
        vb_storage::JournalEvent::SlotWrittenEvent { slot, value, .. }
        if *slot == original_ticket.answer_slot()
            && value.as_deref() == Some(expected_answer.as_slice())
    )));
    let (finish_index, result_slot) = completed
        .iter()
        .enumerate()
        .find_map(|(index, event)| match event {
            vb_storage::JournalEvent::RunFinished { result, .. } => Some((index, *result)),
            _ => None,
        })
        .ok_or("run completion missing")?;
    assert_eq!(result_slot, SlotIdx::new(1));
    let final_result_write = completed
        .get(..finish_index)
        .ok_or("completion position missing")?
        .iter()
        .rev()
        .find_map(|event| match event {
            vb_storage::JournalEvent::SlotWrittenEvent { slot, value, .. }
                if *slot == result_slot =>
            {
                Some(value)
            }
            _ => None,
        })
        .ok_or("result slot was not written before completion")?;
    let final_result = final_result_write
        .as_deref()
        .ok_or("final result slot write did not capture a value")?;
    assert_eq!(final_result, expected_answer.as_slice());
    assert_eq!(
        postcard::from_bytes::<SlotValue>(final_result)?,
        SlotValue::I64(77)
    );
    session.shutdown(104)?;
    assert_eq!(session.service.journal.events_for_run(run)?, completed);
    Ok(())
}
