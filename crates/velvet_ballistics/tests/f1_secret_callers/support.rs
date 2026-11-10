use std::{
    io::{Read, Write},
    os::unix::net::UnixStream,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};
use vb_core::{CompiledWorkflow, RunId};
use vb_ipc::{IpcCommand, IpcFrameHeader, IpcPayload, SubmitRunPayload, server::IpcResponse};

pub(super) type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
pub(super) const KEY_A: &str = "VB_F1_CALLER_SECRET_A_20261110";
pub(super) const KEY_B: &str = "VB_F1_CALLER_SECRET_B_20261110";
pub(super) const VALUE: &str = "fixture-only-caller-value-699d40ac-3417-43f7-b4ae-bd5b432af0b2";

pub(super) fn command() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_velvet-ballistics"));
    command.env_remove(KEY_A).env_remove(KEY_B);
    command
}
pub(super) fn workflow(key: &str, pending: bool) -> TestResult<CompiledWorkflow> {
    let steps = if pending {
        "  - id: request\n    ask: {prompt: '$secrets.token'}\n  - id: done\n    finish: {result: '$step.request'}\n"
    } else {
        "  - id: done\n    finish: {result: '$secrets.token'}\n"
    };
    Ok(vb_compile::compile_workflow(format!("version: velvet-ballistics/v1\nname: caller_secret\nwhen: {{manual: {{}}}}\nsecrets: {{token: {key}}}\nsteps:\n{steps}").as_bytes())?)
}
pub(super) fn store(db: &Path, workflows: &[&CompiledWorkflow]) -> TestResult {
    let journal = vb_storage::FjallJournal::open(db, None)?;
    for workflow in workflows {
        vb_storage::submit_artifact_with_contracts(
            &journal,
            workflow,
            vb_core::RuntimePolicy::Strict,
            &[],
        )?;
    }
    Ok(())
}
pub(super) fn no_value(bytes: &[u8]) {
    assert!(
        !bytes
            .windows(VALUE.len())
            .any(|part| part == VALUE.as_bytes())
    );
}
pub(super) fn journal_events(db: &Path, run: RunId) -> TestResult<Vec<vb_storage::JournalEvent>> {
    let journal = vb_storage::FjallJournal::open(db, None)?;
    let events = journal.events_for_run_full(run)?;
    no_value(&postcard::to_allocvec(&events)?);
    Ok(events)
}
pub(super) fn assert_artifact(db: &Path, workflow: &CompiledWorkflow) -> TestResult {
    let journal = vb_storage::FjallJournal::open(db, None)?;
    let artifact = journal
        .compiled_ir_for_source_digest(workflow.digest())?
        .ok_or("artifact")?;
    no_value(&artifact.ir);
    Ok(())
}

pub(super) fn assert_answer_result(
    events: &[vb_storage::JournalEvent],
    expected: i64,
) -> TestResult {
    let results = events
        .iter()
        .filter_map(|event| match event {
            vb_storage::JournalEvent::RunFinished { result, .. } => Some(*result),
            _ => None,
        })
        .collect::<Vec<_>>();
    let [result] = results.as_slice() else {
        return Err(format!("expected one completion: {results:?}").into());
    };
    let payload = events
        .iter()
        .rev()
        .find_map(|event| match event {
            vb_storage::JournalEvent::SlotWrittenEvent {
                slot,
                value: Some(bytes),
                ..
            } if slot == result => Some(bytes),
            _ => None,
        })
        .ok_or("result evidence")?;
    assert_eq!(
        postcard::from_bytes::<vb_core::SlotValue>(payload)?,
        vb_core::SlotValue::I64(expected)
    );
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, vb_storage::JournalEvent::RunFailedEvent { .. }))
            .count(),
        0
    );
    Ok(())
}

pub(super) struct Server {
    child: Child,
    socket: PathBuf,
    _socket_directory: tempfile::TempDir,
}
impl Server {
    pub(super) fn start(db: &Path, keys: &[&str]) -> TestResult<Self> {
        let socket_directory = tempfile::tempdir_in("/tmp")?;
        let socket = socket_directory.path().join("ipc.sock");
        let mut command = command();
        command
            .args(["ipc-serve", "--db"])
            .arg(db)
            .arg("--socket")
            .arg(&socket)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        for key in keys {
            command.env(key, VALUE);
        }
        let mut server = Self {
            child: command.spawn()?,
            socket,
            _socket_directory: socket_directory,
        };
        let start = Instant::now();
        loop {
            if let Some(status) = server.child.try_wait()? {
                let mut error = String::new();
                if let Some(mut stderr) = server.child.stderr.take() {
                    stderr.read_to_string(&mut error)?;
                }
                return Err(format!("IPC exited {status}: {error}").into());
            }
            if server.socket.exists() {
                return Ok(server);
            }
            if start.elapsed() > Duration::from_secs(10) {
                return Err("IPC readiness deadline".into());
            }
            std::thread::sleep(Duration::from_millis(5));
        }
    }
    pub(super) fn connect(&self) -> TestResult<UnixStream> {
        let stream = UnixStream::connect(&self.socket)?;
        stream.set_read_timeout(Some(Duration::from_secs(5)))?;
        stream.set_write_timeout(Some(Duration::from_secs(5)))?;
        Ok(stream)
    }
    pub(super) fn stop(mut self) -> TestResult {
        let mut stream = self.connect()?;
        assert_eq!(
            request(&mut stream, IpcCommand::Shutdown, IpcPayload::Shutdown)?,
            IpcResponse::ShuttingDown
        );
        let start = Instant::now();
        loop {
            if let Some(status) = self.child.try_wait()? {
                assert_eq!(status.code(), Some(0));
                break;
            }
            if start.elapsed() > Duration::from_secs(10) {
                return Err("IPC shutdown deadline".into());
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        if let Some(mut pipe) = self.child.stdout.take() {
            pipe.read_to_end(&mut stdout)?;
        }
        if let Some(mut pipe) = self.child.stderr.take() {
            pipe.read_to_end(&mut stderr)?;
        }
        no_value(&stdout);
        no_value(&stderr);
        assert_eq!(stderr, b"");
        Ok(())
    }
}
impl Drop for Server {
    fn drop(&mut self) {
        if matches!(self.child.try_wait(), Ok(None)) {
            let _ = self.child.kill();
        }
        let _ = self.child.wait();
    }
}

pub(super) fn request(
    stream: &mut UnixStream,
    command: IpcCommand,
    payload: IpcPayload,
) -> TestResult<IpcResponse> {
    send(stream, command, payload)?;
    receive(stream)
}
pub(super) fn send(
    stream: &mut UnixStream,
    command: IpcCommand,
    payload: IpcPayload,
) -> TestResult {
    let payload = postcard::to_allocvec(&payload)?;
    let header = IpcFrameHeader::new(command, 0, 99, u32::try_from(payload.len())?);
    stream.write_all(&header.encode()?)?;
    stream.write_all(&payload)?;
    Ok(())
}
pub(super) fn receive(stream: &mut UnixStream) -> TestResult<IpcResponse> {
    let mut bytes = [0; vb_ipc::IPC_HEADER_LEN];
    stream.read_exact(&mut bytes)?;
    let header = IpcFrameHeader::decode(&bytes, vb_ipc::MaxPayloadBytes::DEFAULT)?;
    assert_eq!(header.correlation, 99);
    let mut payload = vec![0; usize::try_from(header.payload_len)?];
    stream.read_exact(&mut payload)?;
    no_value(&payload);
    Ok(postcard::from_bytes(&payload)?)
}
pub(super) fn submit(
    stream: &mut UnixStream,
    workflow: &CompiledWorkflow,
    run: RunId,
    inline: bool,
) -> TestResult<IpcResponse> {
    let submit = SubmitRunPayload {
        run_id: run,
        workflow: workflow.digest(),
        input: Vec::new(),
    };
    if inline {
        request(
            stream,
            IpcCommand::SubmitRunInline,
            IpcPayload::SubmitRunInline(submit),
        )
    } else {
        request(stream, IpcCommand::SubmitRun, IpcPayload::SubmitRun(submit))
    }
}
pub(super) fn answer(
    stream: &mut UnixStream,
    workflow: &CompiledWorkflow,
    run: RunId,
    value: i64,
) -> TestResult {
    let step = workflow
        .to_parts()
        .nodes
        .iter()
        .find(|node| matches!(node.kind, vb_core::CompiledNodeKind::Ask { .. }))
        .ok_or("ask")?
        .id;
    assert_eq!(
        request(
            stream,
            IpcCommand::AnswerAsk,
            IpcPayload::AnswerAsk {
                run_id: run,
                ticket: u64::from(step.get()),
                answer: postcard::to_allocvec(&vb_core::SlotValue::I64(value))?,
                taint: Some(vb_core::Taint::Clean)
            }
        )?,
        IpcResponse::AcceptedRun { run_id: run.get() }
    );
    Ok(())
}
