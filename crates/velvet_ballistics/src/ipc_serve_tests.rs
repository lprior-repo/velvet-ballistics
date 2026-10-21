//! Production service-turn durability regressions: vb-fnugl.
use super::*;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn event(run: vb_core::RunId, seq: u64, digest: u8) -> vb_storage::JournalEvent {
    vb_storage::JournalEvent::RunAccepted {
        run,
        seq: vb_storage::EventSeq::new(seq),
        workflow: vb_core::WorkflowDigest::from_bytes([digest; 32]),
    }
}

fn service(temp: &tempfile::TempDir) -> Result<IpcJournalService, vb_storage::JournalError> {
    Ok(IpcJournalService {
        journal: Arc::new(vb_storage::FjallJournal::open(temp.path(), None)?),
        queue: Arc::new(vb_storage::JournalWriterQueue::new(
            128,
            64,
            vb_storage::StorageLimits::DEFAULT,
        )?),
    })
}

#[test]
fn service_turn_commits_accepted_journal_work_before_shutdown() -> TestResult {
    let dir = tempfile::tempdir()?;
    let service = service(&dir)?;
    let run = vb_core::RunId::new(42);
    let accepted = event(run, 0, 7);
    service.queue.enqueue_journaled(accepted.clone())?;
    let socket =
        std::path::PathBuf::from(format!("/tmp/vb_fnugl_visible_{}.sock", std::process::id()));
    let mut server = vb_ipc::server::IpcServer::bind(&socket)?;
    let mut runtime = vb_runtime::runtime::Runtime::new_for_tests_and_benchmarks_only(
        NonZeroUsize::MIN,
        vb_runtime::shard::ShardConfig::default(),
    );
    let mut resolver = StorageWorkflowResolver {
        journal: Arc::clone(&service.journal),
    };
    assert_eq!(
        service.turn(
            &mut server,
            &mut runtime,
            &mut resolver,
            Some(std::time::Duration::ZERO)
        )?,
        true
    );
    assert_eq!(service.journal.events_for_run(run)?, vec![accepted]);
    assert_eq!(service.queue.pending_profile_counts()?.journaled, 0);
    std::fs::remove_file(socket)?;
    Ok(())
}

#[test]
fn service_turn_preserves_and_reports_failed_journal_staging() -> TestResult {
    let dir = tempfile::tempdir()?;
    let service = service(&dir)?;
    let run = vb_core::RunId::new(43);
    service.journal.append_journaled(&event(run, 0, 1))?;
    service.queue.enqueue_journaled(event(run, 0, 2))?;
    let socket =
        std::path::PathBuf::from(format!("/tmp/vb_fnugl_error_{}.sock", std::process::id()));
    let mut server = vb_ipc::server::IpcServer::bind(&socket)?;
    let mut runtime = vb_runtime::runtime::Runtime::new_for_tests_and_benchmarks_only(
        NonZeroUsize::MIN,
        vb_runtime::shard::ShardConfig::default(),
    );
    let mut resolver = StorageWorkflowResolver {
        journal: Arc::clone(&service.journal),
    };
    let result = service.turn(
        &mut server,
        &mut runtime,
        &mut resolver,
        Some(std::time::Duration::ZERO),
    );
    assert!(
        matches!(result, Err(IpcServiceError::Storage(vb_storage::JournalError::DuplicateEvent { run: found, seq })) if found == run && seq == vb_storage::EventSeq::new(0))
    );
    assert_eq!(service.queue.pending_profile_counts()?.journaled, 1);
    assert_eq!(service.journal.events_for_run(run)?, vec![event(run, 0, 1)]);
    std::fs::remove_file(socket)?;
    Ok(())
}

#[test]
fn shutdown_acknowledgement_follows_all_65_owned_queue_events_before_exit() -> TestResult {
    use std::io::{Read, Write};
    let dir = tempfile::tempdir()?;
    let service = service(&dir)?;
    let socket = std::path::PathBuf::from(format!(
        "/tmp/vb_fnugl_shutdown_{}.sock",
        std::process::id()
    ));
    let mut server = vb_ipc::server::IpcServer::bind(&socket)?;
    let mut client = std::os::unix::net::UnixStream::connect(&socket)?;
    client.set_read_timeout(Some(std::time::Duration::from_millis(200)))?;
    let runtime_journal =
        vb_runtime::journal::RuntimeJournalConfig::new(vb_storage::DurabilityProfile::Journaled)
            .shared_journal(Arc::clone(&service.journal), Arc::clone(&service.queue))?;
    let mut runtime = vb_runtime::runtime::Runtime::new(
        NonZeroUsize::MIN,
        vb_runtime::shard::ShardConfig::default(),
        runtime_journal,
    );
    let mut resolver = StorageWorkflowResolver {
        journal: Arc::clone(&service.journal),
    };
    assert!(service.turn(
        &mut server,
        &mut runtime,
        &mut resolver,
        Some(std::time::Duration::ZERO)
    )?);
    let run = vb_core::RunId::new(504);
    let accepted: Vec<_> = (0..65).map(|seq| event(run, seq, 7)).collect();
    for item in &accepted {
        service.queue.enqueue_journaled(item.clone())?;
    }
    let header = vb_ipc::IpcFrameHeader::new(vb_ipc::IpcCommand::Shutdown, 0, 501, 0);
    client.write_all(&header.encode()?)?;
    let alive = service.turn(
        &mut server,
        &mut runtime,
        &mut resolver,
        Some(std::time::Duration::from_millis(10)),
    )?;
    let mut bytes = [0; vb_ipc::IPC_HEADER_LEN];
    client.read_exact(&mut bytes)?;
    let response = vb_ipc::IpcFrameHeader::decode(&bytes, vb_ipc::MaxPayloadBytes::DEFAULT)?;
    let mut payload = vec![0; usize::try_from(response.payload_len)?];
    client.read_exact(&mut payload)?;
    assert_eq!(response.correlation, 501);
    assert_eq!(
        postcard::from_bytes::<vb_ipc::server::IpcResponse>(&payload)?,
        vb_ipc::server::IpcResponse::ShuttingDown
    );
    assert!(!alive);
    assert_eq!(service.queue.pending_profile_counts()?.journaled, 0);
    assert_eq!(service.journal.events_for_run(run)?, accepted);
    std::fs::remove_file(socket)?;
    Ok(())
}

#[test]
fn failed_owned_shutdown_reports_wire_error_and_typed_storage_failure() -> TestResult {
    use std::io::{Read, Write};
    let dir = tempfile::tempdir()?;
    let service = service(&dir)?;
    let socket_dir = tempfile::tempdir()?;
    let socket = socket_dir.path().join("shutdown.sock");
    let mut server = vb_ipc::server::IpcServer::bind(&socket)?;
    let mut client = std::os::unix::net::UnixStream::connect(&socket)?;
    client.set_read_timeout(Some(std::time::Duration::from_millis(200)))?;
    let runtime_journal =
        vb_runtime::journal::RuntimeJournalConfig::new(vb_storage::DurabilityProfile::Journaled)
            .shared_journal(Arc::clone(&service.journal), Arc::clone(&service.queue))?;
    let mut runtime = vb_runtime::runtime::Runtime::new(
        NonZeroUsize::MIN,
        vb_runtime::shard::ShardConfig::default(),
        runtime_journal,
    );
    let mut resolver = StorageWorkflowResolver {
        journal: Arc::clone(&service.journal),
    };
    assert!(service.turn(
        &mut server,
        &mut runtime,
        &mut resolver,
        Some(std::time::Duration::ZERO)
    )?);
    let run = vb_core::RunId::new(505);
    service.journal.append_journaled(&event(run, 64, 9))?;
    for seq in 0..65 {
        service.queue.enqueue_journaled(event(run, seq, 7))?;
    }
    client.write_all(
        &vb_ipc::IpcFrameHeader::new(vb_ipc::IpcCommand::Shutdown, 0, 502, 0).encode()?,
    )?;
    let result = service.turn(
        &mut server,
        &mut runtime,
        &mut resolver,
        Some(std::time::Duration::from_millis(10)),
    );
    assert!(
        matches!(result, Err(IpcServiceError::Storage(vb_storage::JournalError::DuplicateEvent { run: found, seq })) if found == run && seq == vb_storage::EventSeq::new(64))
    );
    let mut bytes = [0; vb_ipc::IPC_HEADER_LEN];
    client.read_exact(&mut bytes)?;
    let header = vb_ipc::IpcFrameHeader::decode(&bytes, vb_ipc::MaxPayloadBytes::DEFAULT)?;
    let mut payload = vec![0; usize::try_from(header.payload_len)?];
    client.read_exact(&mut payload)?;
    assert_eq!(header.correlation, 502);
    assert!(matches!(
        postcard::from_bytes::<vb_ipc::server::IpcResponse>(&payload)?,
        vb_ipc::server::IpcResponse::RuntimeError { .. }
    ));
    assert_eq!(service.queue.pending_profile_counts()?.journaled, 1);
    Ok(())
}
