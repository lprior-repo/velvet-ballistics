use super::*;

fn assert_divergence(
    events: &[JournalEvent],
    workflow: &vb_core::CompiledWorkflow,
    roots: &[(vb_core::SlotIdx, SlotValue, Taint)],
    checkpoint: &vb_core::value_store::ValueStoreCheckpointV1,
    step: vb_core::StepIdx,
    detail: &str,
) -> TestResult {
    match replay_owned_for_each(events, workflow, roots, Some(checkpoint)) {
        Err(vb_storage::recovery::RecoveryError::ReplayDivergence { step: actual_step, detail: actual_detail }) => {
            assert_eq!(actual_step, step);
            assert_eq!(actual_detail, detail);
        }
        Err(error) => return Err(format!("expected exact ReplayDivergence, got {error:?}").into()),
        Ok(_) => return Err(format!("corrupt prefix unexpectedly recovered; expected ReplayDivergence {{ step: {step:?}, detail: {detail:?} }}").into()),
    }
    Ok(())
}

#[test]
fn corrupt_owned_foreach_evidence_is_rejected_typed() -> TestResult {
    let source = multi_source("count($step.loop)");
    let workflow = vb_compile::compile_workflow(source.as_bytes())?;
    let body = step_with(&workflow, |kind| matches!(kind, K::ForEachNext { .. }))?;
    let cut = cut_at(
        &source,
        &[SlotValue::I64(3), SlotValue::I64(1)],
        &[Taint::Secret, Taint::Clean],
        (body, StepState::Succeeded),
    )?;
    let before_events = cut.events.clone();
    let before_frame = cut.live_frame.clone();
    let before_store = cut.live_store.clone();

    let mut bad_value = cut.events.clone();
    let mut changed_value = false;
    for event in &mut bad_value {
        if let JournalEvent::SlotWrittenEvent {
            value: Some(bytes), ..
        } = event
        {
            bytes.clear();
            changed_value = true;
            break;
        }
    }
    assert!(changed_value, "fixture lacks value-bearing slot event");
    assert_divergence(
        &bad_value,
        &cut.workflow,
        &cut.roots,
        &cut.input_store,
        StepIdx::ZERO,
        "invalid captured value",
    )?;

    let mut missing_taint = cut.events.clone();
    let mut changed_taint = false;
    for event in &mut missing_taint {
        if let JournalEvent::SlotWrittenEvent { extra, .. } = event {
            if extra.is_some() {
                *extra = None;
                changed_taint = true;
                break;
            }
        }
    }
    assert!(changed_taint, "fixture lacks taint-bearing slot event");
    let missing_result = replay_owned_for_each(
        &missing_taint,
        &cut.workflow,
        &cut.roots,
        Some(&cut.input_store),
    );
    match missing_result {
        Err(vb_storage::recovery::RecoveryError::ReplayDivergence { step, detail }) => {
            assert_eq!(step, StepIdx::ZERO);
            assert_eq!(detail, "missing captured taint");
        }
        Err(error) => {
            return Err(
                format!("missing-taint corruption returned wrong typed error: {error:?}").into(),
            );
        }
        Ok(_) => return Err("missing taint unexpectedly recovered".into()),
    }

    let mut reordered = cut.events.clone();
    let starts: Vec<(usize, StepIdx)> = reordered
        .iter()
        .enumerate()
        .filter_map(|(index, event)| match event {
            JournalEvent::StepStarted { step, .. } => Some((index, *step)),
            _ => None,
        })
        .collect();
    let [(first_index, first_step), (second_index, _), ..] = starts.as_slice() else {
        return Err("fixture lacks two started steps".into());
    };
    reordered.swap(*first_index, *second_index);
    assert_divergence(
        &reordered,
        &cut.workflow,
        &cut.roots,
        &cut.input_store,
        *first_step,
        "incomplete bound event sequence",
    )?;

    for raw_handle in [9_999, 0] {
        let mut bad_handle = cut.events.clone();
        let bytes = postcard::to_allocvec(&SlotValue::List(ListId::new(raw_handle)))?;
        let slot = {
            let mut changed = None;
            for event in &mut bad_handle {
                if let JournalEvent::SlotWrittenEvent {
                    slot,
                    value: Some(value),
                    ..
                } = event
                {
                    *value = bytes;
                    changed = Some(*slot);
                    break;
                }
            }
            changed.ok_or("missing handle-bearing slot event")?
        };
        let result = replay_owned_for_each(
            &bad_handle,
            &cut.workflow,
            &cut.roots,
            Some(&cut.input_store),
        );
        assert!(
            matches!(result, Err(vb_storage::recovery::RecoveryError::ReplayDivergence { step, ref detail }) if step == StepIdx::ZERO && detail == "capture differs from artifact replay"),
            "invalid/foreign list handle {raw_handle} slot {slot:?}: {result:?}"
        );
    }

    assert_eq!(
        cut.events, before_events,
        "tamper vectors cannot mutate durable-prefix input"
    );
    assert_eq!(cut.live_frame, before_frame);
    assert_eq!(cut.live_store, before_store);
    let journal = Arc::new(FjallJournal::open(cut.directory.path(), None)?);
    assert_eq!(
        journal.events_for_run_full(RUN)?,
        before_events,
        "rejected prefixes leave durable journal bytes/events unchanged"
    );
    let sequence = before_events
        .iter()
        .find_map(|event| match event {
            JournalEvent::StepStarted { seq, .. } => Some(*seq),
            _ => None,
        })
        .ok_or("fixture lacks a persisted StepStarted event")?;
    journal.inject_raw_event(
        RUN,
        sequence,
        vb_storage::records::RecordKind::StepStarted,
        &[0xff],
    )?;
    let snapshot_bytes = || {
        before_events
            .iter()
            .map(|event| {
                let sequence = event.seq();
                Ok((
                    sequence,
                    journal
                        .get_event_bytes(RUN, sequence)?
                        .ok_or("durable prefix event bytes disappeared")?,
                ))
            })
            .collect::<TestResult<Vec<_>>>()
    };
    let tampered_prefix_bytes = snapshot_bytes()?;
    let runtime = runtime(journal.clone(), 1);
    assert!(
        matches!(
            runtime.inspect_run_state(RUN),
            Err(vb_runtime::RuntimeError::RunNotFound)
        ),
        "recovery product must not install a run before resume"
    );
    let recovery_result = runtime.recover_product(RUN);
    assert!(
        matches!(
            recovery_result,
            Err(vb_runtime::RuntimeError::Recovery { ref error })
                if error == "postcard payload decode failed: Serde Deserialization Error"
        ),
        "malformed persisted event must preserve its exact decode failure: {recovery_result:?}"
    );
    assert_eq!(
        snapshot_bytes()?,
        tampered_prefix_bytes,
        "public rejection must not alter any durable prefix bytes"
    );
    assert!(
        matches!(
            runtime.inspect_run_state(RUN),
            Err(vb_runtime::RuntimeError::RunNotFound)
        ),
        "rejected prefix must not install a run state"
    );
    assert_eq!(cut.live_frame, before_frame);
    assert_eq!(cut.live_store, before_store);
    Ok(())
}

#[test]
fn owned_foreach_action_ticket_corruption_is_rejected_typed() -> TestResult {
    let source = retry_action_source();
    let contracts = action_contracts()?;
    let work = control_step(&source, |kind| matches!(kind, K::Do { .. }))?;
    let cut = cut_at_actions(
        &source,
        &[SlotValue::I64(1)],
        &[Taint::Clean],
        (work, StepState::Running),
        &contracts,
    )?;
    let retry_capacity = cut
        .workflow
        .node(work)
        .and_then(|node| node.retry)
        .map(|policy| policy.max_attempts)
        .ok_or("Do node lacks compiled retry policy")?;
    assert_eq!(
        retry_capacity, 3,
        "fixture retry authority must differ from capacity one"
    );
    let before = cut.events.clone();
    for mutation in [
        "run",
        "step",
        "action",
        "attempt",
        "idempotency",
        "capacity",
        "zero_capacity",
    ] {
        let mut events = cut.events.clone();
        let ticket = events
            .iter_mut()
            .find_map(|event| match event {
                JournalEvent::ActionScheduledTicket { ticket, .. } => Some(ticket),
                _ => None,
            })
            .ok_or("missing action ticket")?;
        match mutation {
            "run" => ticket.run = RunId::new(9_999),
            "step" => ticket.step = StepIdx::new(ticket.step.get().saturating_add(1)),
            "action" => {
                ticket.action = vb_core::ActionId::new(ticket.action.get().saturating_add(1))
            }
            "attempt" => ticket.attempt = ticket.capacity.saturating_add(1),
            "capacity" => {
                assert_eq!(
                    ticket.capacity, retry_capacity,
                    "persisted ticket must match compiled policy"
                );
                ticket.capacity = retry_capacity
                    .checked_sub(1)
                    .filter(|capacity| *capacity >= ticket.attempt)
                    .ok_or("retry policy does not permit a positive unequal capacity")?;
                assert_ne!(ticket.capacity, retry_capacity);
                assert_eq!(ticket.capacity, 2);
                assert_eq!(ticket.capacity >= ticket.attempt, true);
            }
            "zero_capacity" => {
                assert_eq!(ticket.capacity, retry_capacity);
                ticket.capacity = 0;
            }
            "idempotency" => ticket.idempotency_key ^= 1,
            _ => return Err("unknown ticket corruption".into()),
        }
        let error_step = ticket.step;
        let expected = match mutation {
            "run" => "action ticket run mismatch",
            "attempt" => "action ticket attempt bounds invalid",
            "capacity" => "action ticket differs from compiled Do",
            "zero_capacity" => "action ticket attempt bounds invalid",
            "step" => "action schedule authority mismatch",
            "action" | "idempotency" => "action ticket idempotency key mismatch",
            _ => return Err("unknown ticket corruption".into()),
        };
        assert_divergence(
            &events,
            &cut.workflow,
            &cut.roots,
            &cut.input_store,
            error_step,
            expected,
        )?;
    }
    assert_eq!(cut.events, before);
    Ok(())
}

fn assert_malformed_pending_prefix_is_nonmutating(
    cut: Cut,
    pending_step: StepIdx,
    pending_action: bool,
    pending_wait: bool,
) -> TestResult {
    assert_eq!(cut.live_pending_boundary.active_run_count(), 1);
    assert_eq!(
        cut.live_pending_boundary.pending_action_count(),
        if pending_action { 1 } else { 0 }
    );
    assert_eq!(
        cut.live_pending_boundary.pending_timer_count(),
        if pending_wait { 1 } else { 0 }
    );
    assert_eq!(cut.live_action_ticket.is_some(), pending_action);
    assert_eq!(cut.live_deadline.is_some(), pending_wait);
    if let Some(ticket) = cut.live_action_ticket {
        assert_eq!(ticket.run, RUN);
        assert_eq!(ticket.step, pending_step);
    }
    let journal = Arc::new(FjallJournal::open(cut.directory.path(), None)?);
    let last_seq = cut
        .events
        .last()
        .map(JournalEvent::seq)
        .ok_or("pending prefix has no events")?;
    let malformed_seq = vb_storage::EventSeq::new(
        last_seq
            .get()
            .checked_add(1)
            .ok_or("event sequence overflow")?,
    );
    journal.inject_raw_event(
        RUN,
        malformed_seq,
        vb_storage::RecordKind::StepStarted,
        &[0xff],
    )?;
    let sequences = cut
        .events
        .iter()
        .map(JournalEvent::seq)
        .chain(std::iter::once(malformed_seq))
        .collect::<Vec<_>>();
    let raw_bytes = || {
        sequences
            .iter()
            .map(|seq| journal.get_event_bytes(RUN, *seq))
            .collect::<Result<Vec<_>, _>>()
    };
    let before_bytes = raw_bytes()?;
    assert!(
        before_bytes.iter().all(Option::is_some),
        "every prefix event must have raw bytes"
    );

    let runtime = runtime(Arc::clone(&journal), 1);
    let before_boundary = runtime.pending_boundary_snapshot(64);
    assert_eq!(before_boundary.active_run_count(), 0);
    assert_eq!(before_boundary.pending_action_count(), 0);
    assert_eq!(before_boundary.pending_timer_count(), 0);
    assert_eq!(
        runtime.lookup_pending_action_ticket(RUN, u64::from(pending_step.get())),
        None
    );
    assert_eq!(
        runtime.capture_timer_entry(RUN),
        Err(vb_runtime::RuntimeError::RunNotFound)
    );
    assert_eq!(
        runtime.inspect_run_state(RUN).map(|_| ()),
        Err(vb_runtime::RuntimeError::RunNotFound)
    );
    assert!(matches!(
        runtime.recover_product(RUN),
        Err(vb_runtime::RuntimeError::Recovery { .. })
    ));
    assert_eq!(
        raw_bytes()?,
        before_bytes,
        "rejected recovery must preserve every raw event record"
    );
    assert_eq!(runtime.pending_boundary_snapshot(64), before_boundary);
    assert_eq!(
        runtime.lookup_pending_action_ticket(RUN, u64::from(pending_step.get())),
        None
    );
    assert_eq!(
        runtime.capture_timer_entry(RUN),
        Err(vb_runtime::RuntimeError::RunNotFound)
    );
    assert_eq!(
        runtime.inspect_run_state(RUN).map(|_| ()),
        Err(vb_runtime::RuntimeError::RunNotFound)
    );
    Ok(())
}

#[test]
fn malformed_pending_action_and_wait_prefixes_reject_without_mutation() -> TestResult {
    let action_source = action_source();
    let action_step = control_step(&action_source, |kind| matches!(kind, K::Do { .. }))?;
    let action_cut = cut_at_actions(
        &action_source,
        &[SlotValue::I64(1)],
        &[Taint::Clean],
        (action_step, StepState::Running),
        &action_contracts()?,
    )?;
    assert_malformed_pending_prefix_is_nonmutating(action_cut, action_step, true, false)?;

    let wait_source = wait_source();
    let wait_step = control_step(&wait_source, |kind| matches!(kind, K::WaitUntil { .. }))?;
    let wait_cut = cut_at(
        &wait_source,
        &[SlotValue::I64(60_000)],
        &[Taint::Clean],
        (wait_step, StepState::Waiting),
    )?;
    assert_malformed_pending_prefix_is_nonmutating(wait_cut, wait_step, false, true)
}

#[test]
fn owned_foreach_limit_is_revalidated_during_replay() -> TestResult {
    let source = multi_source("count($step.loop)");
    let workflow = vb_compile::compile_workflow(source.as_bytes())?;
    let body = step_with(&workflow, |kind| matches!(kind, K::ForEachNext { .. }))?;
    let cut = cut_at(
        &source,
        &[SlotValue::I64(3), SlotValue::I64(1)],
        &[Taint::Secret, Taint::Clean],
        (body, StepState::Succeeded),
    )?;
    let mut store = ValueStore::with_max_slots(workflow.resource_contract().max_slots);
    let list = store.insert_list(Box::from([
        SlotValue::I64(3),
        SlotValue::I64(1),
        SlotValue::I64(2),
        SlotValue::I64(4),
        SlotValue::I64(5),
    ]))?;
    let input = workflow.input_slots().first().ok_or("input")?.slot;
    let roots = Box::from([(input, SlotValue::List(list), Taint::Clean)]);
    let checkpoint = store.checkpoint_v1();
    let result = replay_owned_for_each(&cut.events, &workflow, &roots, Some(&checkpoint));
    assert!(
        matches!(
            result,
            Err(vb_storage::recovery::RecoveryError::ReplayDivergence { .. })
        ),
        "over-limit source must be rejected during replay: {result:?}"
    );
    Ok(())
}
