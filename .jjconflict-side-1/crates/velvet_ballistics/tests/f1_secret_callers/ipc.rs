use super::support::*;
use vb_core::RunId;
use vb_ipc::server::IpcResponse;

#[test]
fn actual_socket_submit_and_inline_require_host_presence_before_admission() -> TestResult {
    for inline in [false, true] {
        for present in [false, true] {
            let directory = tempfile::tempdir()?;
            let workflow = workflow(KEY_A, false)?;
            store(directory.path(), &[&workflow])?;
            let server = Server::start(directory.path(), if present { &[KEY_A] } else { &[] })?;
            let run = RunId::new(81);
            let response = submit(&mut server.connect()?, &workflow, run, inline)?;
            assert_eq!(
                response,
                if present {
                    IpcResponse::AcceptedRun { run_id: run.get() }
                } else {
                    IpcResponse::RuntimeError {
                        message: "SECRET_UNAVAILABLE: secret requirement unavailable".into(),
                    }
                }
            );
            server.stop()?;
            let events = journal_events(directory.path(), run)?;
            if present {
                assert_eq!(
                    events
                        .iter()
                        .filter(|event| matches!(
                            event,
                            vb_storage::JournalEvent::RunFinished { .. }
                        ))
                        .count(),
                    1
                );
                assert_eq!(
                    events
                        .iter()
                        .filter(|event| matches!(
                            event,
                            vb_storage::JournalEvent::RunFailedEvent { .. }
                        ))
                        .count(),
                    0
                );
            } else {
                assert_eq!(events, vec![]);
                assert_eq!(
                    vb_storage::FjallJournal::open(directory.path(), None)?.run_header(run)?,
                    None
                );
            }
            assert_artifact(directory.path(), &workflow)?;
        }
    }
    Ok(())
}

#[test]
fn socket_inputs_cannot_replace_secret_bindings_or_omit_ordinary_cold_defaults() -> TestResult {
    let directory = tempfile::tempdir()?;
    let secret = workflow(KEY_A, false)?;
    let cold = vb_compile::compile_workflow(b"version: velvet-ballistics/v1\nname: ordinary_cold\nwhen: {manual: {}}\ninputs: {label: {is: text, default: guest}}\nsteps:\n  - id: done\n    finish: {result: '$input.label'}\n")?;
    store(directory.path(), &[&secret, &cold])?;
    let server = Server::start(directory.path(), &[KEY_A])?;
    let mut stream = server.connect()?;
    for inline in [false, true] {
        let run = vb_core::RunId::new(if inline { 89 } else { 88 });
        let payload = vb_ipc::SubmitRunPayload {
            run_id: run,
            workflow: secret.digest(),
            input: postcard::to_allocvec(&vec![vb_core::SlotValue::Symbol(
                vb_core::SymbolId::new(0),
            )])?,
        };
        let (command, payload) = if inline {
            (
                vb_ipc::IpcCommand::SubmitRunInline,
                vb_ipc::IpcPayload::SubmitRunInline(payload),
            )
        } else {
            (
                vb_ipc::IpcCommand::SubmitRun,
                vb_ipc::IpcPayload::SubmitRun(payload),
            )
        };
        assert_eq!(
            request(&mut stream, command, payload)?,
            IpcResponse::PayloadError {
                diagnostic: vb_ipc::IpcError::PayloadDecodeFailed
                    .diagnostic_code()
                    .code(),
                message: "failed to decode IPC payload".into()
            }
        );
        assert_eq!(
            submit(
                &mut stream,
                &cold,
                RunId::new(if inline { 91 } else { 90 }),
                inline
            )?,
            IpcResponse::PayloadError {
                diagnostic: vb_ipc::IpcError::PayloadDecodeFailed
                    .diagnostic_code()
                    .code(),
                message: "failed to decode IPC payload".into()
            }
        );
    }
    server.stop()?;
    for run in [88, 89, 90, 91] {
        assert_eq!(journal_events(directory.path(), RunId::new(run))?, vec![]);
    }
    Ok(())
}

#[test]
fn two_available_workflows_retain_presence_while_both_are_pending() -> TestResult {
    let directory = tempfile::tempdir()?;
    let first = workflow(KEY_A, true)?;
    let second = workflow(KEY_B, true)?;
    assert_ne!(first.digest(), second.digest());
    store(directory.path(), &[&first, &second])?;
    let server = Server::start(directory.path(), &[KEY_A, KEY_B])?;
    let mut left = server.connect()?;
    let mut right = server.connect()?;
    assert_eq!(
        submit(&mut left, &first, RunId::new(82), false)?,
        IpcResponse::AcceptedRun { run_id: 82 }
    );
    assert_eq!(
        submit(&mut right, &second, RunId::new(83), true)?,
        IpcResponse::AcceptedRun { run_id: 83 }
    );
    assert_eq!(
        submit(&mut left, &first, RunId::new(84), true)?,
        IpcResponse::AcceptedRun { run_id: 84 }
    );
    for (run, workflow) in [(82, &first), (83, &second), (84, &first)] {
        answer(&mut left, workflow, RunId::new(run), i64::try_from(run)?)?;
    }
    server.stop()?;
    for run in [82, 83, 84] {
        let events = journal_events(directory.path(), RunId::new(run))?;
        assert_answer_result(&events, i64::try_from(run)?)?;
        assert_eq!(
            events
                .iter()
                .filter(|event| matches!(event, vb_storage::JournalEvent::AskScheduledEvent { .. }))
                .count(),
            1
        );
        assert_eq!(
            events
                .iter()
                .filter(|event| matches!(event, vb_storage::JournalEvent::RunFinished { .. }))
                .count(),
            1
        );
        assert_eq!(
            events
                .iter()
                .filter(|event| matches!(event, vb_storage::JournalEvent::RunFailedEvent { .. }))
                .count(),
            0
        );
    }
    assert_artifact(directory.path(), &first)?;
    assert_artifact(directory.path(), &second)?;
    Ok(())
}
