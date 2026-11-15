use super::*;

#[test]
fn owned_foreach_recovered_ask_accepts_exact_slot_answer() -> TestResult {
    let source = ask_source();
    let ask = control_step(&source, |kind| matches!(kind, K::Ask { .. }))?;
    let cut = cut_at(
        &source,
        &[SlotValue::I64(1), SlotValue::I64(2)],
        &[Taint::Clean; 2],
        (ask, StepState::Asking),
    )?;
    replay_parity(&cut)?;
    let journal = Arc::new(FjallJournal::open(cut.directory.path(), None)?);
    let mut runtime = runtime(journal.clone(), 1);
    runtime.recover_and_resume(RUN)?;
    let ticket = wait_for_pending_ask(&mut runtime, ask)?;
    assert_eq!(
        runtime.pending_boundary_snapshot(64).command_queue_depth(),
        0
    );
    assert_foreign_run_is_read_only(&mut runtime, &journal, ticket)?;
    assert_eq!(
        answer_by_slot(&mut runtime, RUN, ticket, SlotValue::I64(41))?,
        vb_ipc::server::IpcResponse::AcceptedRun { run_id: RUN.get() }
    );
    assert_eq!(
        runtime.pending_boundary_snapshot(64).command_queue_depth(),
        1
    );
    runtime.tick_all()?;
    assert_eq!(
        runtime.pending_boundary_snapshot(64).command_queue_depth(),
        0
    );
    assert_ne!(runtime.lookup_pending_ask_ticket(RUN, ask), Some(ticket));
    assert_answer_evidence(
        &journal.events_for_run_full(RUN)?,
        RUN,
        ticket,
        SlotValue::I64(41),
        Taint::Clean,
    )?;
    let second = wait_for_pending_ask(&mut runtime, ask)?;
    assert_ne!(second, ticket);
    assert_eq!(
        answer_by_slot(&mut runtime, RUN, second, SlotValue::I64(42))?,
        vb_ipc::server::IpcResponse::AcceptedRun { run_id: RUN.get() }
    );
    for _ in 0..256 {
        runtime.tick_all()?;
        if runtime.counters_snapshot().runs_completed == 1 {
            break;
        }
    }
    assert_eq!(runtime.counters_snapshot().runs_completed, 1);
    assert_stale_slot_is_read_only(&mut runtime, &journal, RUN, ticket)?;
    Ok(())
}

fn assert_foreign_run_is_read_only(
    runtime: &mut vb_runtime::runtime::Runtime,
    journal: &FjallJournal,
    ticket: vb_runtime::shard::AskTicket,
) -> TestResult {
    let state = runtime.inspect_run_state(RUN)?.clone();
    let events = journal.events_for_run_full(RUN)?;
    let trace = runtime.list_events(RUN)?;
    let counters = runtime.counters_snapshot();
    assert_eq!(
        answer_by_slot(runtime, RunId::new(99), ticket, SlotValue::I64(41))?,
        vb_ipc::server::IpcResponse::BadRequest
    );
    runtime.tick_all()?;
    assert_eq!(runtime.inspect_run_state(RUN)?, &state);
    assert_eq!(journal.events_for_run_full(RUN)?, events);
    assert_eq!(runtime.list_events(RUN)?, trace);
    assert_eq!(runtime.counters_snapshot(), counters);
    assert_eq!(
        runtime.pending_boundary_snapshot(64).command_queue_depth(),
        0
    );
    Ok(())
}

fn wait_for_pending_ask(
    runtime: &mut vb_runtime::runtime::Runtime,
    ask: StepIdx,
) -> TestResult<vb_runtime::shard::AskTicket> {
    for _ in 0..256 {
        runtime.tick_all()?;
        if let Some(ticket) = runtime.lookup_pending_ask_ticket(RUN, ask) {
            return Ok(ticket);
        }
    }
    Err("owned ForEach Ask did not become pending".into())
}

fn answer_by_slot(
    runtime: &mut vb_runtime::runtime::Runtime,
    run: RunId,
    ticket: vb_runtime::shard::AskTicket,
    value: SlotValue,
) -> TestResult<vb_ipc::server::IpcResponse> {
    let wire_ticket = 65_536_u64
        .checked_add(u64::from(ticket.answer_slot().get()))
        .ok_or("slot ticket overflow")?;
    let payload = postcard::to_allocvec(&vb_ipc::IpcPayload::AnswerAsk {
        run_id: run,
        ticket: wire_ticket,
        answer: postcard::to_allocvec(&value)?,
        taint: Some(Taint::Clean),
    })?;
    Ok(vb_ipc::server::handlers::handle_answer_ask(
        &payload, runtime,
    ))
}

fn assert_stale_slot_is_read_only(
    runtime: &mut vb_runtime::runtime::Runtime,
    journal: &FjallJournal,
    run: RunId,
    ticket: vb_runtime::shard::AskTicket,
) -> TestResult {
    let events = journal.events_for_run_full(run)?;
    let trace = runtime.list_events(run)?;
    let counters = runtime.counters_snapshot();
    let queue_depth = runtime.pending_boundary_snapshot(64).command_queue_depth();
    assert_eq!(
        answer_by_slot(runtime, run, ticket, SlotValue::I64(9))?,
        vb_ipc::server::IpcResponse::BadRequest
    );
    runtime.tick_all()?;
    assert_eq!(journal.events_for_run_full(run)?, events);
    assert_eq!(runtime.list_events(run)?, trace);
    assert_eq!(runtime.counters_snapshot(), counters);
    assert_eq!(
        runtime.pending_boundary_snapshot(64).command_queue_depth(),
        queue_depth
    );
    Ok(())
}

fn assert_answer_evidence(
    events: &[JournalEvent],
    run: RunId,
    ticket: vb_runtime::shard::AskTicket,
    expected: SlotValue,
    expected_taint: Taint,
) -> TestResult {
    let answers = events.iter().filter(|event| matches!(event, JournalEvent::AskAnsweredEvent { run: actual_run, step, .. } if *actual_run == run && *step == ticket.ask_step())).count();
    assert_eq!(answers, 1);
    let writes = events
        .iter()
        .filter_map(|event| match event {
            JournalEvent::SlotWrittenEvent {
                run: actual_run,
                slot,
                value: Some(value),
                extra: Some(extra),
                ..
            } if *actual_run == run && *slot == ticket.answer_slot() => Some((value, extra)),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert!(!writes.is_empty());
    for (value, extra) in writes {
        assert_eq!(postcard::from_bytes::<SlotValue>(value)?, expected);
        let decoded =
            vb_storage::decode_slot_written_extra(extra).map_err(|error| format!("{error:?}"))?;
        assert!(
            matches!(decoded, vb_storage::DecodedSlotWrittenExtra::Envelope(ref envelope) if envelope.taint == expected_taint)
        );
    }
    Ok(())
}
