#![forbid(unsafe_code)]

use std::num::NonZeroUsize;
use std::os::unix::fs::{FileTypeExt, MetadataExt, symlink};
use std::os::unix::net::{UnixListener, UnixStream};
use std::sync::Arc;
use vb_core::{
    CompiledInputConstraint, CompiledInputSlot, CompiledNodeKind, CompiledWorkflow, InputSlotKind,
    ResourceContract, RunId, RuntimePolicy, SlotIdx, SlotValue, StepIdx, WorkflowDigest,
    WorkflowParts,
};
use vb_ipc::server::{
    IpcResponse, IpcServer, IpcServerError, WorkflowResolutionError, WorkflowResolver,
};
use vb_ipc::{IpcCommand, IpcFrameHeader, IpcPayload, SubmitRunPayload};
use vb_runtime::journal::{RuntimeJournalEvent, SharedRuntimeJournal, VolatileRuntimeJournal};
use vb_runtime::runtime::Runtime;
use vb_runtime::shard::ShardConfig;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

struct Resolver(CompiledWorkflow);
impl WorkflowResolver for Resolver {
    fn resolve_workflow(
        &mut self,
        digest: WorkflowDigest,
    ) -> Result<CompiledWorkflow, WorkflowResolutionError> {
        if digest == self.0.digest() {
            Ok(self.0.clone())
        } else {
            Err(WorkflowResolutionError::NotFound)
        }
    }
}

fn workflow(default_flag: bool) -> TestResult<CompiledWorkflow> {
    let mut number = CompiledInputConstraint::required();
    number.min = Some(1);
    number.max = Some(50);
    let mut flag = CompiledInputConstraint::required();
    if default_flag {
        flag.default_value = Some(SlotValue::Bool(true));
    }
    Ok(CompiledWorkflow::try_from_parts(WorkflowParts {
        semantic_source_identity: None,
        name: Box::from("ipc-positional-inputs"),
        digest: WorkflowDigest::from_bytes([4; 32]),
        nodes: Box::from([
            vb_core::node(
                StepIdx::ZERO,
                CompiledNodeKind::Copy {
                    source: SlotIdx::new(2),
                },
                Some(SlotIdx::new(5)),
                Some(StepIdx::new(1)),
            ),
            vb_core::node(
                StepIdx::new(1),
                CompiledNodeKind::Copy {
                    source: SlotIdx::new(4),
                },
                Some(SlotIdx::new(6)),
                Some(StepIdx::new(2)),
            ),
            vb_core::node(
                StepIdx::new(2),
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(6),
                },
                None,
                None,
            ),
        ]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([]),
        slot_count: 7,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        step_names: Box::from([]),
        resource_contract: ResourceContract::DEFAULT,
        input_slots: Box::from([
            CompiledInputSlot {
                slot: SlotIdx::new(2),
                kind: InputSlotKind::I64,
                constraint: number,
            },
            CompiledInputSlot {
                slot: SlotIdx::new(4),
                kind: InputSlotKind::Bool,
                constraint: flag,
            },
        ]),
    })?)
}

fn runtime() -> (Runtime, Arc<VolatileRuntimeJournal>) {
    let journal = Arc::new(VolatileRuntimeJournal::new());
    let shared: SharedRuntimeJournal = journal.clone();
    let config = ShardConfig {
        policy: RuntimePolicy::Relaxed,
        ..ShardConfig::default()
    };
    (Runtime::new(NonZeroUsize::MIN, config, shared), journal)
}

fn submit(
    command: IpcCommand,
    run: RunId,
    input: Vec<u8>,
    runtime: &mut Runtime,
    resolver: &mut Resolver,
) -> TestResult<IpcResponse> {
    let body = SubmitRunPayload {
        run_id: run,
        workflow: resolver.0.digest(),
        input,
    };
    let payload = match command {
        IpcCommand::SubmitRun => IpcPayload::SubmitRun(body),
        IpcCommand::SubmitRunInline => IpcPayload::SubmitRunInline(body),
        _ => return Err("submit command required".into()),
    };
    let bytes = postcard::to_allocvec(&payload)?;
    let size = u32::try_from(bytes.len())?;
    let header = IpcFrameHeader::new(command, 0, 1, size);
    Ok(vb_ipc::server::dispatch::dispatch_command_with_resolver(
        &header,
        &bytes,
        runtime,
        Some(resolver),
    ))
}

fn assert_copied_inputs(events: &[RuntimeJournalEvent], run: RunId) -> TestResult {
    for (slot, expected) in [
        (SlotIdx::new(5), SlotValue::I64(37)),
        (SlotIdx::new(6), SlotValue::Bool(true)),
    ] {
        let values = events
            .iter()
            .filter_map(|event| match event {
                RuntimeJournalEvent::SlotWritten {
                    run: actual,
                    slot: actual_slot,
                    value,
                    ..
                } if *actual == run && *actual_slot == slot => Some(value),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(values.len(), 1);
        for bytes in values {
            assert_eq!(postcard::from_bytes::<SlotValue>(bytes)?, expected);
        }
    }
    assert!(events.iter().any(|event| matches!(event,
        RuntimeJournalEvent::RunFinished { run: actual, .. } if *actual == run)));
    Ok(())
}

#[test]
fn both_submit_commands_preserve_positional_values_in_sparse_compiled_slots() -> TestResult {
    for command in [IpcCommand::SubmitRun, IpcCommand::SubmitRunInline] {
        let (mut runtime, journal) = runtime();
        let mut resolver = Resolver(workflow(false)?);
        let run = RunId::new(71);
        let values: Box<[SlotValue]> = Box::from([SlotValue::I64(37), SlotValue::Bool(true)]);
        assert_eq!(
            submit(
                command,
                run,
                postcard::to_allocvec(&values)?,
                &mut runtime,
                &mut resolver
            )?,
            IpcResponse::AcceptedRun { run_id: run.get() }
        );
        assert!(runtime.tick_all()?);
        assert_copied_inputs(&journal.snapshot()?, run)?;
    }
    Ok(())
}

#[test]
fn both_submit_commands_fill_omitted_scalar_defaults() -> TestResult {
    for command in [IpcCommand::SubmitRun, IpcCommand::SubmitRunInline] {
        let (mut runtime, journal) = runtime();
        let mut resolver = Resolver(workflow(true)?);
        let run = RunId::new(72);
        let values: Box<[SlotValue]> = Box::from([SlotValue::I64(37)]);
        assert_eq!(
            submit(
                command,
                run,
                postcard::to_allocvec(&values)?,
                &mut runtime,
                &mut resolver
            )?,
            IpcResponse::AcceptedRun { run_id: run.get() }
        );
        assert!(runtime.tick_all()?);
        assert_copied_inputs(&journal.snapshot()?, run)?;
    }
    Ok(())
}

#[test]
fn malformed_inputs_fail_before_any_submission_is_enqueued_or_journaled() -> TestResult {
    let mut cases = Vec::new();
    for values in [
        vec![],
        vec![SlotValue::I64(37)],
        vec![SlotValue::Bool(true), SlotValue::Bool(true)],
        vec![SlotValue::I64(0), SlotValue::Bool(true)],
        vec![SlotValue::I64(51), SlotValue::Bool(true)],
        vec![SlotValue::I64(37), SlotValue::Bool(true), SlotValue::Null],
        vec![
            SlotValue::List(vb_core::ListId::new(1)),
            SlotValue::Bool(true),
        ],
        vec![
            SlotValue::Symbol(vb_core::SymbolId::new(0)),
            SlotValue::Bool(true),
        ],
    ] {
        cases.push(postcard::to_allocvec(&values.into_boxed_slice())?);
    }
    let good: Box<[SlotValue]> = Box::from([SlotValue::I64(37), SlotValue::Bool(true)]);
    let mut trailing = postcard::to_allocvec(&good)?;
    trailing.push(0);
    cases.extend([
        Vec::new(),
        vec![0xff],
        trailing,
        vec![0; 65_537],
        postcard::to_allocvec(&usize::MAX)?,
    ]);
    for command in [IpcCommand::SubmitRun, IpcCommand::SubmitRunInline] {
        for bytes in &cases {
            let (mut runtime, journal) = runtime();
            let mut resolver = Resolver(workflow(false)?);
            let run = RunId::new(73);
            assert!(matches!(
                submit(command, run, bytes.clone(), &mut runtime, &mut resolver)?,
                IpcResponse::PayloadError { .. }
            ));
            assert!(journal.snapshot()?.is_empty());
            assert!(runtime.tick_all()?);
            assert!(journal.snapshot()?.is_empty());
            assert_eq!(
                submit(
                    command,
                    run,
                    postcard::to_allocvec(&good)?,
                    &mut runtime,
                    &mut resolver
                )?,
                IpcResponse::AcceptedRun { run_id: run.get() }
            );
            assert!(runtime.tick_all()?);
            assert_copied_inputs(&journal.snapshot()?, run)?;
        }
    }
    Ok(())
}

fn socket_dir() -> TestResult<tempfile::TempDir> {
    Ok(tempfile::Builder::new()
        .prefix("vb-ipc-owned-")
        .tempdir_in("/tmp")?)
}

fn assert_bind_refused(path: &std::path::Path) {
    assert!(matches!(
        IpcServer::bind(path),
        Err(IpcServerError::BindFailed { .. })
    ));
}

fn inode(path: &std::path::Path) -> TestResult<(u64, u64)> {
    let meta = std::fs::symlink_metadata(path)?;
    Ok((meta.dev(), meta.ino()))
}

#[test]
fn occupied_live_socket_preserves_inode_and_original_listener() -> TestResult {
    let dir = socket_dir()?;
    let path = dir.path().join("live.sock");
    let listener = UnixListener::bind(&path)?;
    let before = inode(&path)?;
    assert_bind_refused(&path);
    assert_eq!(inode(&path)?, before);
    assert!(std::fs::symlink_metadata(&path)?.file_type().is_socket());
    let client = UnixStream::connect(&path)?;
    let (_peer, _) = listener.accept()?;
    drop(client);
    Ok(())
}

#[test]
fn occupied_stale_socket_preserves_inode() -> TestResult {
    let dir = socket_dir()?;
    let path = dir.path().join("stale.sock");
    drop(UnixListener::bind(&path)?);
    let before = inode(&path)?;
    assert_bind_refused(&path);
    assert_eq!(inode(&path)?, before);
    assert!(std::fs::symlink_metadata(&path)?.file_type().is_socket());
    Ok(())
}

#[test]
fn occupied_symlinks_preserve_links_and_targets() -> TestResult {
    let dir = socket_dir()?;
    let target = dir.path().join("target");
    std::fs::write(&target, b"owned contents")?;
    let missing = dir.path().join("missing");
    for destination in [&target, &missing] {
        let path = dir.path().join(if destination == &target {
            "valid-link"
        } else {
            "dangling-link"
        });
        symlink(destination, &path)?;
        let before = inode(&path)?;
        assert_bind_refused(&path);
        assert_eq!(inode(&path)?, before);
        assert_eq!(std::fs::read_link(&path)?, *destination);
    }
    assert_eq!(std::fs::read(&target)?, b"owned contents");
    Ok(())
}

#[test]
fn occupied_directory_preserves_contents() -> TestResult {
    let dir = socket_dir()?;
    let path = dir.path().join("directory");
    std::fs::create_dir(&path)?;
    std::fs::write(path.join("owned"), b"directory contents")?;
    let before = inode(&path)?;
    assert_bind_refused(&path);
    assert_eq!(inode(&path)?, before);
    assert_eq!(std::fs::read(path.join("owned"))?, b"directory contents");
    Ok(())
}

#[test]
fn fresh_socket_is_created_and_only_fixture_owner_cleans_it() -> TestResult {
    let dir = socket_dir()?;
    let path = dir.path().join("fresh.sock");
    let server = IpcServer::bind(&path)?;
    assert!(std::fs::symlink_metadata(&path)?.file_type().is_socket());
    drop(server);
    assert!(std::fs::symlink_metadata(&path)?.file_type().is_socket());
    dir.close()?;
    assert!(!path.exists());
    Ok(())
}

fn transport_workflow() -> TestResult<CompiledWorkflow> {
    let mut parts = workflow(false)?.to_parts();
    parts.slot_count = 10;
    parts.input_slots = Box::from([
        CompiledInputSlot {
            slot: SlotIdx::new(2),
            kind: InputSlotKind::I64,
            constraint: CompiledInputConstraint {
                min: Some(1),
                max: Some(50),
                ..CompiledInputConstraint::required()
            },
        },
        CompiledInputSlot {
            slot: SlotIdx::new(4),
            kind: InputSlotKind::Bool,
            constraint: CompiledInputConstraint::required(),
        },
        CompiledInputSlot {
            slot: SlotIdx::new(6),
            kind: InputSlotKind::F64,
            constraint: CompiledInputConstraint::required(),
        },
    ]);
    parts.nodes = Box::from([
        vb_core::node(
            StepIdx::ZERO,
            CompiledNodeKind::Copy {
                source: SlotIdx::new(2),
            },
            Some(SlotIdx::new(7)),
            Some(StepIdx::new(1)),
        ),
        vb_core::node(
            StepIdx::new(1),
            CompiledNodeKind::Copy {
                source: SlotIdx::new(4),
            },
            Some(SlotIdx::new(8)),
            Some(StepIdx::new(2)),
        ),
        vb_core::node(
            StepIdx::new(2),
            CompiledNodeKind::Copy {
                source: SlotIdx::new(6),
            },
            Some(SlotIdx::new(9)),
            Some(StepIdx::new(3)),
        ),
        vb_core::node(
            StepIdx::new(3),
            CompiledNodeKind::Finish {
                result: SlotIdx::new(9),
            },
            None,
            None,
        ),
    ]);
    Ok(CompiledWorkflow::try_from_parts(parts)?)
}

fn socket_submit(
    command: IpcCommand,
    run: RunId,
    input: Vec<u8>,
    runtime: &mut Runtime,
    resolver: &mut Resolver,
) -> TestResult<IpcResponse> {
    use std::io::Read;
    use std::time::Duration;
    let dir = socket_dir()?;
    let path = dir.path().join("exchange.sock");
    let mut server = IpcServer::bind(&path)?;
    let mut client = UnixStream::connect(&path)?;
    client.set_read_timeout(Some(Duration::from_millis(250)))?;
    client.set_write_timeout(Some(Duration::from_millis(250)))?;
    let submit = SubmitRunPayload {
        run_id: run,
        workflow: resolver.0.digest(),
        input,
    };
    let payload = match command {
        IpcCommand::SubmitRun => IpcPayload::SubmitRun(submit),
        IpcCommand::SubmitRunInline => IpcPayload::SubmitRunInline(submit),
        _ => return Err("submit command required".into()),
    };
    let bytes = postcard::to_allocvec(&payload)?;
    vb_ipc::frame::write_frame(&mut client, command, 0, 91, &bytes)?;
    client.set_nonblocking(true)?;
    let mut received = Vec::new();
    let max_frame = vb_ipc::IPC_HEADER_LEN
        .checked_add(4096)
        .ok_or("response bound overflow")?;
    for _ in 0..256 {
        assert!(vb_ipc::server::dispatch::serve_ipc_with_resolver(
            &mut server,
            runtime,
            Some(Duration::from_millis(1)),
            Some(&mut *resolver)
        )?);
        if let Some(error) = server.take_client_error() {
            return Err(error.into());
        }
        let mut chunk = [0u8; 512];
        match client.read(&mut chunk) {
            Ok(0) => return Err("IPC response closed before complete frame".into()),
            Ok(count) => {
                let length = received
                    .len()
                    .checked_add(count)
                    .ok_or("response length overflow")?;
                if length > max_frame {
                    return Err("response exceeded fixture bound".into());
                }
                received.try_reserve(count)?;
                received.extend_from_slice(chunk.get(..count).ok_or("invalid read length")?);
            }
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::Interrupted
                ) => {}
            Err(error) => return Err(error.into()),
        }
        if let Some(bytes) = received.get(..vb_ipc::IPC_HEADER_LEN) {
            let header_bytes: [u8; vb_ipc::IPC_HEADER_LEN] = bytes.try_into()?;
            let response_header = vb_ipc::frame::decode_frame_header(&header_bytes)?;
            assert_eq!(response_header.command, command);
            assert_eq!(response_header.correlation, 91);
            let length = vb_ipc::IPC_HEADER_LEN
                .checked_add(usize::try_from(response_header.payload_len)?)
                .ok_or("response frame overflow")?;
            if length > max_frame {
                return Err("response exceeded fixture bound".into());
            }
            if received.len() >= length {
                assert_eq!(received.len(), length);
                let body = received
                    .get(vb_ipc::IPC_HEADER_LEN..length)
                    .ok_or("missing response body")?;
                let (response, remainder) = postcard::take_from_bytes::<IpcResponse>(body)?;
                assert!(remainder.is_empty());
                return Ok(response);
            }
        }
    }
    Err("IPC exchange exceeded 256 bounded service turns".into())
}

#[test]
fn real_socket_both_submit_commands_preserve_three_scalar_kinds_and_finish_output() -> TestResult {
    let float = SlotValue::F64(vb_core::FiniteF64::new(4.5)?);
    for command in [IpcCommand::SubmitRun, IpcCommand::SubmitRunInline] {
        let (mut runtime, journal) = runtime();
        let mut resolver = Resolver(transport_workflow()?);
        let run = RunId::new(81);
        let values: Box<[SlotValue]> =
            Box::from([SlotValue::I64(37), SlotValue::Bool(true), float]);
        assert_eq!(
            socket_submit(
                command,
                run,
                postcard::to_allocvec(&values)?,
                &mut runtime,
                &mut resolver
            )?,
            IpcResponse::AcceptedRun { run_id: run.get() }
        );
        assert_eq!(
            runtime.pending_boundary_snapshot(4).command_queue_depth(),
            1
        );
        assert!(journal.snapshot()?.is_empty());
        assert!(runtime.tick_all()?);
        let events = journal.snapshot()?;
        let mut outputs = Vec::new();
        for event in &events {
            if let RuntimeJournalEvent::SlotWritten {
                run: actual,
                slot,
                value,
                ..
            } = event
            {
                assert_eq!(*actual, run);
                outputs.push((*slot, postcard::from_bytes::<SlotValue>(value)?));
            }
        }
        assert_eq!(
            outputs,
            [
                (SlotIdx::new(7), SlotValue::I64(37)),
                (SlotIdx::new(8), SlotValue::Bool(true)),
                (SlotIdx::new(9), float)
            ]
        );
        let terminal = events
            .iter()
            .filter_map(|event| match event {
                RuntimeJournalEvent::RunFinished {
                    run,
                    result,
                    attempt,
                } => Some((*run, *result, *attempt)),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(terminal, [(run, SlotIdx::new(9), 1)]);
        assert_eq!(
            runtime.pending_boundary_snapshot(4).command_queue_depth(),
            0
        );
        assert!(
            !events
                .iter()
                .any(|event| matches!(event, RuntimeJournalEvent::RunFailed { .. }))
        );
    }
    Ok(())
}

#[test]
fn real_socket_rejects_malformed_trailing_type_and_range_before_admission() -> TestResult {
    let float = SlotValue::F64(vb_core::FiniteF64::new(4.5)?);
    let good: Box<[SlotValue]> = Box::from([SlotValue::I64(37), SlotValue::Bool(true), float]);
    let mut trailing = postcard::to_allocvec(&good)?;
    trailing.push(0);
    let wrong_type: Box<[SlotValue]> =
        Box::from([SlotValue::Bool(true), SlotValue::Bool(true), float]);
    let out_of_range: Box<[SlotValue]> =
        Box::from([SlotValue::I64(51), SlotValue::Bool(true), float]);
    let cases = [
        vec![0xff],
        trailing,
        postcard::to_allocvec(&wrong_type)?,
        postcard::to_allocvec(&out_of_range)?,
        postcard::to_allocvec(&usize::MAX)?,
    ];
    for command in [IpcCommand::SubmitRun, IpcCommand::SubmitRunInline] {
        for input in &cases {
            let (mut runtime, journal) = runtime();
            let mut resolver = Resolver(transport_workflow()?);
            let before = runtime.pending_boundary_snapshot(4);
            let before_events = journal.snapshot()?;
            let run = RunId::new(82);
            assert!(matches!(
                socket_submit(command, run, input.clone(), &mut runtime, &mut resolver)?,
                IpcResponse::PayloadError { .. }
            ));
            assert_eq!(runtime.pending_boundary_snapshot(4), before);
            assert_eq!(journal.snapshot()?, before_events);
            assert!(runtime.tick_all()?);
            assert_eq!(runtime.pending_boundary_snapshot(4), before);
            assert_eq!(journal.snapshot()?, before_events);
            assert_eq!(
                socket_submit(
                    command,
                    run,
                    postcard::to_allocvec(&good)?,
                    &mut runtime,
                    &mut resolver
                )?,
                IpcResponse::AcceptedRun { run_id: run.get() }
            );
            assert!(runtime.tick_all()?);
            assert!(journal.snapshot()?.iter().any(|event| matches!(event,
                RuntimeJournalEvent::RunFinished { run: actual, result, .. }
                    if *actual == run && *result == SlotIdx::new(9))));
        }
    }
    Ok(())
}
