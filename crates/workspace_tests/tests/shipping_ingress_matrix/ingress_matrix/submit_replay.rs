use super::*;
#[test]
fn binary_ipc_submit_health_shutdown_and_cold_replay() -> TestResult {
    let workflow = compile(&source())?;
    let dir = tempfile::tempdir()?;
    let run_a = RunId::new(301);
    let run_b = RunId::new(302);
    let socket = socket_path("submit")?;
    let _cleanup = Cleanup(socket.clone());
    let mut open = open_runtime(dir.path(), RuntimePolicy::Strict, Durability::Strict)?;
    persist_artifact(&open.journal, &workflow, RuntimePolicy::Strict, &[])?;
    let mut server = IpcServer::bind(&socket)?;
    let mut resolver = Resolver::new(workflow.clone());
    let mut client = std::os::unix::net::UnixStream::connect(&socket)?;
    client.set_read_timeout(Some(Duration::from_millis(100)))?;
    server.poll_once_with_resolver(
        &mut open.runtime,
        Some(Duration::from_millis(100)),
        Some(&mut resolver),
    )?;

    for (index, command) in [IpcCommand::SubmitRun, IpcCommand::SubmitRunInline]
        .into_iter()
        .enumerate()
    {
        let run = if index == 0 { run_a } else { run_b };
        let correlation = 0x5150 + u64::try_from(index)?;
        let body = SubmitRunPayload {
            run_id: run,
            workflow: workflow.digest(),
            input: Vec::new(),
        };
        let payload = match command {
            IpcCommand::SubmitRunInline => IpcPayload::SubmitRunInline(body),
            _ => IpcPayload::SubmitRun(body),
        };
        let bytes = postcard::to_allocvec(&payload)?;
        client.write_all(&encode_request(command, correlation, &bytes)?)?;
        client.flush()?;
        let (header, response) =
            read_response(&mut server, &mut open.runtime, &mut resolver, &mut client)?;
        assert_eq!(header.correlation, correlation);
        assert_eq!(header.command, command);
        assert_eq!(
            response,
            Some(IpcResponse::AcceptedRun { run_id: run.get() })
        );
        tick_until_terminal(&mut open.runtime, run, 64)?;
    }
    assert_eq!(resolver.calls, 2);
    assert_eq!(open.runtime.counters_snapshot().runs_completed, 2);

    for (command, correlation, expected) in [
        (IpcCommand::Health, 0x9001, IpcResponse::Healthy),
        (IpcCommand::Shutdown, 0x9002, IpcResponse::ShuttingDown),
    ] {
        client.write_all(&encode_request(command, correlation, &[])?)?;
        client.flush()?;
        let (header, response) =
            read_response(&mut server, &mut open.runtime, &mut resolver, &mut client)?;
        assert_eq!(header.correlation, correlation);
        assert_eq!(response, Some(expected));
    }

    drop(client);
    drop(server);
    drop(open);

    let open = open_runtime(dir.path(), RuntimePolicy::Strict, Durability::Strict)?;
    for run in [run_a, run_b] {
        let durable = events(&open.journal, run)?;
        let result = terminal(&durable, run)?;
        assert_eq!(result.value, SlotValue::I64(42));
        assert_eq!(result.taint, Taint::Clean);
        match open.runtime.recover_product(run)? {
            vb_runtime::recovery::RuntimeRecoveryProduct::CannotResume(product) => {
                assert_eq!(product.reason(), "terminal_state");
            }
            other => return Err(format!("completed IPC run recovered as {other:?}").into()),
        }
        assert_eq!(
            open.runtime.recover_and_resume(run),
            Err(vb_runtime::RuntimeError::RecoveryCannotResume {
                reason: String::from("terminal_state"),
            })
        );
    }
    Ok(())
}
