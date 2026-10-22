use super::*;

/// SCN-VT2F-006 / ERR-003: an ask ticket issued by a separate issuer runtime is rejected when
/// submitted to an unrelated receiver runtime. The receiver's suspended run must be untouched.
///
/// This test proves that AskTicket is not a cross-runtime forgery vector: the genuine ticket
/// from the issuer cannot be answered on a different receiver that has never seen the
/// issuing run. The answer_ask boundary check short-circuits before any mutation.
#[test]
fn test_direct_api_answer_ask_rejects_wrong_run_ticket_without_mutating_unrelated_run()
-> Result<(), String> {
    // Given: SCN-VT2F-006 / ERR-003 an issuer runtime with a suspended ask run and a
    // separate receiver runtime with an unrelated suspended ask run.
    let issuer_journal = Arc::new(VolatileRuntimeJournal::new());
    let receiver_journal = Arc::new(VolatileRuntimeJournal::new());
    let mut issuer = Runtime::new(shard_count(1)?, relaxed_config(), issuer_journal.clone());
    let mut receiver = Runtime::new(shard_count(1)?, relaxed_config(), receiver_journal.clone());
    let issuer_run = RunId::new(1606);
    let receiver_run = RunId::new(2606);

    // Issuer submits and suspends on ask, generating a genuine runtime-issued ticket.
    assert_eq!(
        issuer.submit_direct(issuer_run, ask_then_finish_workflow()?),
        Ok(())
    );
    run_one_tick(&mut issuer)?;
    let ticket = issuer
        .lookup_pending_ask_ticket(issuer_run, StepIdx::new(2))
        .ok_or("issuer suspended ask must have a runtime-issued ticket")?;

    // Receiver submits and suspends on an unrelated ask workflow.
    assert_eq!(
        receiver.submit_direct(receiver_run, ask_then_finish_workflow()?),
        Ok(())
    );
    run_one_tick(&mut receiver)?;
    let receiver_ticket_before = receiver
        .lookup_pending_ask_ticket(receiver_run, StepIdx::new(2))
        .ok_or("receiver must have genuine ask ticket before rejection")?;

    // Capture receiver observable state before the cross-runtime answer attempt.
    let receiver_before = receiver.snapshot_run(receiver_run, 607);
    let receiver_events_before = trace_events(&receiver, receiver_run)?;
    let receiver_active_before = receiver
        .list_active_runs(8, None)
        .iter()
        .map(|summary| summary.run_id)
        .collect::<Vec<_>>();
    let receiver_counters_before = receiver.counters_snapshot();
    let receiver_boundary_before = receiver.pending_boundary_snapshot(8);

    // Capture issuer observable state before the cross-runtime answer attempt.
    let issuer_before = issuer.snapshot_run(issuer_run, 606);
    let issuer_events_before = trace_events(&issuer, issuer_run)?;
    let issuer_active_before = issuer
        .list_active_runs(8, None)
        .iter()
        .map(|summary| summary.run_id)
        .collect::<Vec<_>>();
    let issuer_counters_before = issuer.counters_snapshot();
    let issuer_boundary_before = issuer.pending_boundary_snapshot(8);

    // Capture full journal state before the cross-runtime answer attempt.
    let receiver_journal_before = receiver_journal.snapshot().map_err(|e| e.to_string())?;
    let issuer_journal_before = issuer_journal.snapshot().map_err(|e| e.to_string())?;

    // When: the issuer's genuine ticket is answered on the receiver runtime.
    let answer = ask_answer(ticket, SlotValue::I64(42), Taint::Clean);
    let answer_result = receiver.answer_ask(answer);

    // Then: the receiver rejects with RunNotFound because the issuing run is not
    // known to it. The issuer's ticket is valid, but the receiver's boundary check
    // prevents cross-runtime mutation.
    assert_eq!(answer_result, Err(RuntimeError::RunNotFound));

    // Full journal equality: rejected cross-runtime answer must leave both journals unchanged.
    let receiver_journal_after = receiver_journal.snapshot().map_err(|e| e.to_string())?;
    let issuer_journal_after = issuer_journal.snapshot().map_err(|e| e.to_string())?;
    assert_eq!(
        receiver_journal_after, receiver_journal_before,
        "receiver journal must be unchanged after rejected wrong-run answer"
    );
    assert_eq!(
        issuer_journal_after, issuer_journal_before,
        "issuer journal must be unchanged after rejected wrong-run answer"
    );

    // Receiver: the unrelated suspended run is completely unchanged.
    assert_eq!(
        receiver.snapshot_run(receiver_run, 607),
        receiver_before,
        "receiver snapshot must be unchanged after rejected wrong-run answer"
    );
    assert_eq!(
        trace_events(&receiver, receiver_run)?,
        receiver_events_before,
        "receiver trace must be unchanged after rejected wrong-run answer"
    );
    assert_eq!(
        receiver
            .list_active_runs(8, None)
            .iter()
            .map(|summary| summary.run_id)
            .collect::<Vec<_>>(),
        receiver_active_before,
        "receiver active list must be unchanged after rejected wrong-run answer"
    );
    assert_eq!(
        receiver.counters_snapshot(),
        receiver_counters_before,
        "receiver counters must be unchanged after rejected wrong-run answer"
    );
    assert_eq!(
        receiver.pending_boundary_snapshot(8),
        receiver_boundary_before,
        "receiver pending boundary must be unchanged after rejected wrong-run answer"
    );

    // Issuer: the originating run is also completely unchanged — the answer was rejected
    // at the receiver's boundary before any propagation.
    assert_eq!(
        issuer.snapshot_run(issuer_run, 606),
        issuer_before,
        "issuer snapshot must be unchanged after rejected wrong-run answer"
    );
    assert_eq!(
        trace_events(&issuer, issuer_run)?,
        issuer_events_before,
        "issuer trace must be unchanged after rejected wrong-run answer"
    );
    assert_eq!(
        issuer
            .list_active_runs(8, None)
            .iter()
            .map(|summary| summary.run_id)
            .collect::<Vec<_>>(),
        issuer_active_before,
        "issuer active list must be unchanged after rejected wrong-run answer"
    );
    assert_eq!(
        issuer.counters_snapshot(),
        issuer_counters_before,
        "issuer counters must be unchanged after rejected wrong-run answer"
    );
    assert_eq!(
        issuer.pending_boundary_snapshot(8),
        issuer_boundary_before,
        "issuer pending boundary must be unchanged after rejected wrong-run answer"
    );

    // The receiver's run remains genuinely suspended with its own ticket unchanged.
    let receiver_ticket = receiver
        .lookup_pending_ask_ticket(receiver_run, StepIdx::new(2))
        .ok_or("receiver unrelated run must still be suspended on ask after rejection")?;
    assert_eq!(
        receiver_ticket, receiver_ticket_before,
        "receiver ticket must be unchanged after rejected wrong-run answer"
    );

    Ok(())
}
