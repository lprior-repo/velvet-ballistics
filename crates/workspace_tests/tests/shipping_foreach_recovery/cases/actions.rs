use super::*;

#[test]
fn owned_foreach_action_recovers_recorded_completion_without_reissue() -> TestResult {
    let source = action_source();
    let contracts = action_contracts()?;
    let work = control_step(&source, |kind| matches!(kind, K::Do { .. }))?;
    let cut = cut_at_actions(&source, &[SlotValue::I64(1), SlotValue::I64(2)], &[Taint::Clean, Taint::Clean], (work, StepState::Running), &contracts)?;
    let replay = replay_parity(&cut)?;
    assert!(replay.product.seed().pending_actions.iter().any(|entry| entry.step == work), "pending action must be retained before completion");
    let output = cut.workflow.node(work).and_then(|node| node.output.slot()).ok_or("missing Do output")?;
    let journal = Arc::new(FjallJournal::open(cut.directory.path(), None)?);
    let mut runtime = runtime(journal.clone(), 1);
    assert!(matches!(runtime.recover_product(RUN)?, RuntimeRecoveryProduct::Resumable(_)));
    runtime.recover_and_resume(RUN)?;
    let mut values = [41_i64, 42_i64].into_iter();
    for _ in 0..256 {
        if let Some(ticket) = runtime.lookup_pending_action_ticket(RUN, u64::from(work.get())) {
            let value = values.next().ok_or("more action tickets than outputs")?;
            runtime.complete_action_with_output(ticket, action_output(output, SlotValue::I64(value))?)?;
        }
        runtime.tick_all()?;
        assert_eq!(runtime.counters_snapshot().runs_failed, 0);
        if runtime.counters_snapshot().runs_completed == 1 { break; }
    }
    assert_eq!(runtime.counters_snapshot().runs_completed, 1);
    assert!(values.next().is_none(), "action output list not consumed");
    let events = journal.events_for_run_full(RUN)?;
    assert_eq!(
        events.iter().filter_map(|event| match event {
            JournalEvent::ActionCompletedEnvelope { outcome, .. } => Some(*outcome),
            _ => None,
        }).collect::<Vec<_>>(),
        [vb_storage::DurableActionOutcome::Ready; 2],
        "every valid completion envelope has the only durable outcome"
    );
    assert_eq!(events.iter().filter(|event| matches!(event, JournalEvent::ActionScheduledTicket { .. })).count(), 2, "recovery must not reissue a recorded action");
    let replay = replay_owned_for_each(&events, &cut.workflow, &cut.roots, Some(&cut.input_store))?.ok_or("owned action replay not selected")?;
    let result = events.iter().rev().find_map(|event| match event { JournalEvent::RunFinished { result, .. } => Some(*result), _ => None }).ok_or("missing RunFinished")?;
    let SlotValue::List(list) = *replay.frame.read_slot(result)? else { return Err("action result is not a list".into()); };
    let store = ValueStore::from_checkpoint_v1(replay.store.clone())?;
    assert_eq!(store.list(list)?, [SlotValue::I64(41), SlotValue::I64(42)]);
    Ok(())
}

#[test]
fn durable_action_outcome_codec_rejects_unknown_discriminant() -> TestResult {
    let ready = vb_storage::DurableActionOutcome::Ready;
    let encoded = postcard::to_allocvec(&ready)?;
    assert_eq!(postcard::from_bytes::<vb_storage::DurableActionOutcome>(&encoded)?, ready);
    assert!(postcard::from_bytes::<vb_storage::DurableActionOutcome>(&[1]).is_err());
    Ok(())
}

#[test]
fn completed_same_step_actions_recover_through_public_runtime() -> TestResult {
    let source = action_source();
    let contracts = action_contracts()?;
    let work = control_step(&source, |kind| matches!(kind, K::Do { .. }))?;
    let initial = cut_at_actions(&source, &[SlotValue::I64(1), SlotValue::I64(2), SlotValue::I64(3)], &[Taint::Clean; 3], (work, StepState::Running), &contracts)?;
    let Cut { directory, workflow, roots, input_store, .. } = initial;
    let output = workflow.node(work).and_then(|node| node.output.slot()).ok_or("missing Do output")?;
    let (live_frame, live_store, live_pending_boundary, live_action_ticket, events) = {
        let journal = Arc::new(FjallJournal::open(directory.path(), None)?);
        let mut runtime = runtime(journal.clone(), 1);
        assert!(matches!(runtime.recover_product(RUN)?, RuntimeRecoveryProduct::Resumable(_)));
        runtime.recover_and_resume(RUN)?;
        let mut values = [41_i64, 42_i64].into_iter();
        let mut captured = None;
        for _ in 0..256 {
            if let Some(ticket) = runtime.lookup_pending_action_ticket(RUN, u64::from(work.get())) {
                let value = values.next().ok_or("more pending actions than outputs")?;
                runtime.complete_action_with_output(ticket, action_output(output, SlotValue::I64(value))?)?;
            }
            runtime.tick_all()?;
            assert_eq!(runtime.counters_snapshot().runs_failed, 0);
            let events = journal.events_for_run_full(RUN)?;
            if events.iter().filter(|event| matches!(event, JournalEvent::ActionCompletedEnvelope { .. })).count() >= 2 {
                let state = runtime.inspect_run_state(RUN)?;
                captured = Some((
                    state.frame.checkpoint_v1(),
                    state.store.checkpoint_v1(),
                    runtime.pending_boundary_snapshot(64),
                    runtime.lookup_pending_action_ticket(RUN, u64::from(work.get())),
                    events,
                ));
                break;
            }
        }
        captured.ok_or("second completed same-step action never reached")?
    };
    let cut = Cut { directory, workflow, roots, input_store, live_frame, live_store, live_deadline: None, live_pending_boundary, live_action_ticket, events };
    let action = match cut.workflow.node(work).map(|node| &node.kind) {
        Some(K::Do { action, .. }) => *action,
        _ => return Err("Do step has no action identity".into()),
    };
    let scheduled_tickets = cut.events.iter().filter_map(|event| match event {
        JournalEvent::ActionScheduledTicket { ticket, .. } => Some(*ticket),
        _ => None,
    }).collect::<Vec<_>>();
    let completed = cut.events.iter().filter_map(|event| match event {
        JournalEvent::ActionCompletedEnvelope { run, ticket, output: completed_output, outcome, taint, value, .. } => {
            Some((*run, *ticket, *completed_output, *outcome, *taint, value.as_slice()))
        }
        _ => None,
    }).collect::<Vec<_>>();
    assert_eq!(scheduled_tickets.len(), 2);
    assert_eq!(completed.len(), 2);
    assert_eq!(scheduled_tickets, completed.iter().map(|(_, ticket, _, _, _, _)| *ticket).collect::<Vec<_>>());
    for ((run, ticket, completed_output, outcome, taint, bytes), expected) in completed.into_iter().zip([41_i64, 42_i64]) {
        assert_eq!(run, RUN);
        assert_eq!(ticket.run, RUN);
        assert_eq!(ticket.step, work);
        assert_eq!(ticket.action, action);
        assert_eq!(ticket.run_attempt, 1);
        assert_eq!(ticket.attempt, 1);
        assert_eq!(outcome, vb_storage::DurableActionOutcome::Ready);
        assert!(ticket.capacity > 0);
        assert_eq!(completed_output, output);
        assert_eq!(taint, Taint::Clean);
        assert_eq!(postcard::from_bytes::<SlotValue>(bytes)?, SlotValue::I64(expected));
    }
    replay_parity(&cut)?;
    for mutation in ["abi", "value_digest", "encoded_len", "value", "output"] {
        let mut corrupted = cut.events.clone();
        let mut changed = false;
        for event in &mut corrupted {
            if let JournalEvent::ActionCompletedEnvelope { action_abi_digest, value_digest, encoded_len, value, output, .. } = event {
                match mutation {
                    "abi" => *action_abi_digest = WorkflowDigest::from_bytes([0; 32]),
                    "value_digest" => *value_digest = [0; 32],
                    "encoded_len" => *encoded_len = encoded_len.saturating_add(1),
                    "value" => {
                        let byte = value.first_mut().ok_or("completed action value bytes are empty")?;
                        *byte ^= 1;
                    }
                    "output" => *output = vb_core::SlotIdx::new(output.get().saturating_add(1)),
                    _ => return Err("unknown completion mutation".into()),
                }
                changed = true;
                break;
            }
        }
        assert!(changed, "fixture lacks completed action envelope");
        let expected_detail = match mutation {
            "abi" | "output" => "action completion evidence mismatch",
            "value_digest" | "value" => "action completion value digest mismatch",
            "encoded_len" => "action completion encoded length mismatch",
            _ => return Err("unknown completion mutation".into()),
        };
        match replay_owned_for_each(&corrupted, &cut.workflow, &cut.roots, Some(&cut.input_store)) {
            Err(vb_storage::recovery::RecoveryError::ReplayDivergence { step, detail }) => {
                assert_eq!(step, work);
                assert_eq!(detail, expected_detail);
            }
            Err(error) => return Err(format!("expected exact action replay divergence, got {error:?}").into()),
            Ok(_) => return Err("corrupt action completion unexpectedly recovered".into()),
        }
    }

    // The runtime that observed two completions is gone; recovery below reopens the same Fjall database.
    let journal = Arc::new(FjallJournal::open(cut.directory.path(), None)?);
    let mut runtime = runtime(journal.clone(), 1);
    assert!(matches!(runtime.recover_product(RUN)?, RuntimeRecoveryProduct::Resumable(_)));
    runtime.recover_and_resume(RUN)?;
    let mut values = [43_i64].into_iter();
    for _ in 0..256 {
        if let Some(ticket) = runtime.lookup_pending_action_ticket(RUN, u64::from(work.get())) {
            let value = values.next().ok_or("more pending actions than outputs")?;
            runtime.complete_action_with_output(ticket, action_output(output, SlotValue::I64(value))?)?;
        }
        runtime.tick_all()?;
        assert_eq!(runtime.counters_snapshot().runs_failed, 0);
        if runtime.counters_snapshot().runs_completed == 1 { break; }
    }
    assert_eq!(runtime.counters_snapshot().runs_completed, 1);
    assert!(values.next().is_none(), "unexpected extra action request");
    let events = journal.events_for_run_full(RUN)?;
    assert_eq!(events.iter().filter(|event| matches!(event, JournalEvent::ActionScheduledTicket { .. })).count(), 3, "recovery must not reissue completed activations");
    let replay = replay_owned_for_each(&events, &cut.workflow, &cut.roots, Some(&cut.input_store))?.ok_or("completed action replay not selected")?;
    let result = events.iter().rev().find_map(|event| match event { JournalEvent::RunFinished { result, .. } => Some(*result), _ => None }).ok_or("missing RunFinished")?;
    let SlotValue::List(list) = *replay.frame.read_slot(result)? else { return Err("completed action result is not a list".into()); };
    let store = ValueStore::from_checkpoint_v1(replay.store.clone())?;
    let outputs = store.list_items_with_taint(list)?.collect::<Vec<_>>();
    assert_eq!(outputs, [(SlotValue::I64(41), Taint::Clean), (SlotValue::I64(42), Taint::Clean), (SlotValue::I64(43), Taint::Clean)]);
    assert_eq!(terminal_result(&events)?, (SlotValue::List(list), Taint::Clean));
    Ok(())
}

#[test]
fn failed_owned_action_recovers_as_terminal_after_reopen() -> TestResult {
    let source = action_source();
    let contracts = action_contracts()?;
    let work = control_step(&source, |kind| matches!(kind, K::Do { .. }))?;
    let cut = cut_at_actions(&source, &[SlotValue::I64(1)], &[Taint::Clean], (work, StepState::Running), &contracts)?;
    {
        let journal = Arc::new(FjallJournal::open(cut.directory.path(), None)?);
        let mut runtime = runtime(journal, 1);
        assert!(matches!(runtime.recover_product(RUN)?, RuntimeRecoveryProduct::Resumable(_)));
        runtime.recover_and_resume(RUN)?;
        runtime.tick_all()?;
        let ticket = runtime.lookup_pending_action_ticket(RUN, u64::from(work.get())).ok_or("missing action ticket")?;
        runtime.fail_action(ticket, ActionFailure::from(ActionFailureCode::Rejected))?;
        for _ in 0..256 { runtime.tick_all()?; if runtime.counters_snapshot().runs_failed == 1 { break; } }
        assert_eq!(runtime.counters_snapshot().runs_failed, 1);
    }
    let journal = Arc::new(FjallJournal::open(cut.directory.path(), None)?);
    let runtime = runtime(journal, 1);
    let product = runtime.recover_product(RUN)?;
    assert_eq!(product.cannot_resume_reason(), Some("terminal_state"));
    assert_eq!(product.summary().actions_resolved, 1);
    Ok(())
}
