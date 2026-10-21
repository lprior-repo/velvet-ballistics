fn review_shutdown_with_queue_wiring(shared: bool) -> TestResult {
    use std::io::{Read, Write};
    let dir = tempfile::tempdir()?;
    let service = service(&dir)?;
    let run = vb_core::RunId::new(77);
    for seq in 0..65 { service.queue.enqueue_journaled(event(run, seq, 7))?; }
    let socket = dir.path().join("review.sock");
    let mut server = vb_ipc::server::IpcServer::bind(&socket)?;
    let mut client = std::os::unix::net::UnixStream::connect(&socket)?;
    client.set_read_timeout(Some(std::time::Duration::from_millis(200)))?;
    let config = vb_runtime::shard::ShardConfig::default();
    let mut runtime = if shared {
        let adapter = vb_runtime::journal::RuntimeJournalConfig::new(vb_storage::DurabilityProfile::Journaled)
            .shared_journal(Arc::clone(&service.journal), Arc::clone(&service.queue))?;
        vb_runtime::runtime::Runtime::new(NonZeroUsize::MIN, config, adapter)
    } else {
        vb_runtime::runtime::Runtime::new_for_tests_and_benchmarks_only(NonZeroUsize::MIN, config)
    };
    let mut resolver = StorageWorkflowResolver { journal: Arc::clone(&service.journal) };
    let request = vb_ipc::IpcFrameHeader::new(vb_ipc::IpcCommand::Shutdown, 0, 777, 0);
    client.write_all(&request.encode()?)?;
    assert!(!service.turn(&mut server, &mut runtime, &mut resolver, Some(std::time::Duration::from_millis(20)))?);
    let mut bytes = [0; vb_ipc::IPC_HEADER_LEN];
    client.read_exact(&mut bytes)?;
    let header = vb_ipc::IpcFrameHeader::decode(&bytes, vb_ipc::MaxPayloadBytes::DEFAULT)?;
    let mut payload = vec![0; usize::try_from(header.payload_len)?];
    client.read_exact(&mut payload)?;
    assert_eq!(header.correlation, 777);
    assert_eq!(postcard::from_bytes::<vb_ipc::server::IpcResponse>(&payload)?, vb_ipc::server::IpcResponse::ShuttingDown);
    assert_eq!(service.queue.pending_profile_counts()?.journaled, if shared {0} else {1});
    assert_eq!(service.journal.events_for_run(run)?.len(), if shared {65} else {64});
    if shared {
        drop(runtime); drop(resolver); drop(service);
        let reopened = vb_storage::FjallJournal::open(dir.path(), None)?;
        assert_eq!(reopened.events_for_run(run)?.len(), 65);
    }
    Ok(())
}
#[test]
fn review_production_shared_queue_drains_all_65_before_ack() -> TestResult { review_shutdown_with_queue_wiring(true) }
#[test]
fn review_disconnected_test_runtime_leaves_one_of_65() -> TestResult { review_shutdown_with_queue_wiring(false) }
