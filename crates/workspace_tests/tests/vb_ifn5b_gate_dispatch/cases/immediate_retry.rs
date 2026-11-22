use super::*;
use std::path::Path;
use vb_core::{ActionFailureCode, ActionTicket, ids::SlotIdx, SlotValue};
use vb_runtime::RuntimeError;
use vb_storage::executor::RegisterReceipt;

#[path = "immediate_retry_fixture.rs"]
mod fixture;

#[test]
fn registered_immediate_retry_attempt_two_reconciles_one_effect_after_reopen() -> TestResult {
    cold_retry(2, true)
}

#[test]
fn registered_immediate_retry_attempt_three_reconciles_one_effect_after_reopen() -> TestResult {
    cold_retry(3, true)
}

#[test]
fn registered_immediate_retry_missing_receipt_stays_missing_after_reopen() -> TestResult {
    cold_retry(2, false)
}

fn cold_retry(attempt: u16, effect: bool) -> TestResult {
    let action = ActionId::new(1);
    let dir = storage_dir()?;
    let register_path = dir.join("register");
    let run = RunId::new(94_240);
    let catalog = ExecutorCatalog::run_registered_conformance()?;
    let journal = Arc::new(FjallJournal::open(&dir, None)?);
    let artifact = fixture::submit(&journal, &catalog, run)?;
    let mut runtime = fixture::runtime(journal.clone(), &catalog)?;
    runtime.activate_owned_submission(run, fixture::source_digest())?;
    let first = fixture::pending(&mut runtime, run, 1)?;
    let original_receipt = if effect {
        Some(uncertain_effect(
            &mut runtime,
            &journal,
            &catalog,
            &artifact,
            first,
            &register_path,
        )?)
    } else {
        None
    };
    let mut current = first;
    for next in 2..=attempt {
        runtime.fail_action(current, ActionFailureCode::Timeout.into())?;
        current = fixture::pending(&mut runtime, run, next)?;
        assert_eq!(current.run_attempt, 1);
        assert_eq!(current.capacity, 3);
        assert_eq!(current.seq, first.seq);
        assert_eq!(current.idempotency_key, first.idempotency_key);
        assert!(current.fencing_token > first.fencing_token);
    }
    let approval = approval_for(
        &mut runtime,
        &journal,
        run,
        &artifact,
        GateId::new(94_242),
        GateDecision::Approve,
    )?;
    let current = fixture::pending(&mut runtime, run, attempt)?;
    if let Some(receipt) = &original_receipt {
        fixture::warm_retry_grant(
            &mut runtime,
            &catalog,
            &artifact,
            current,
            &approval,
            &register_path,
            receipt,
        )?;
    }
    let live_state = runtime.inspect_run_state(run)?.clone();
    let events = journal.events_for_run_full(run)?;
    drop(runtime);
    drop(journal);
    drop(catalog);
    // Open every executor namespace before recovery so absence is factual too.
    drop(FjallRegister::open(&register_path, 8)?);
    let executor_rows = fixture::rows(&register_path)?;
    let catalog = ExecutorCatalog::run_registered_conformance()?;
    let journal = Arc::new(FjallJournal::open(&dir, None)?);
    let mut runtime = fixture::runtime(journal.clone(), &catalog)?;
    runtime.recover_and_resume(run)?;
    let ticket = fixture::pending(&mut runtime, run, attempt)?;
    assert_eq!(ticket, current);
    fixture::assert_state(&runtime, &journal, run, live_state)?;
    assert_eq!(journal.events_for_run_full(run)?, events);
    let register = catalog.issue_register_grant(&artifact, ticket, scope())?;
    let scoped = catalog.issue_scoped_grant_for_bound_gate(
        &artifact,
        &register,
        super::gate_cold_recovery::scoped_params(ticket.action, approval.digest)?,
        &approval,
    )?;
    let dispatch = super::gate_cold_recovery::dispatch_context(&artifact, ticket)?;
    scoped.verify_for_execute(&dispatch, &[])?;
    let mut adapter = FjallRegister::open(&register_path, 8)?;
    let before = runtime.inspect_run_state(run)?.clone();
    assert!(matches!(
        runtime.execute_registered_action(
            &mut adapter,
            run,
            u64::from(ticket.step.get()),
            &register,
            vb_core::ScopedDispatch::new(&scoped, &dispatch, &[]).with_gate_approval(&approval)
        ),
        Err(RegisteredExecutionError::Executor(
            ExecutorError::Unauthorized
        ))
    ));
    assert_eq!(runtime.inspect_run_state(run)?, &before);
    assert_eq!(journal.events_for_run_full(run)?, events);
    let reconciled = runtime.reconcile_registered_action(
        &mut adapter,
        run,
        u64::from(ticket.step.get()),
        &register,
        &scoped,
        &dispatch,
    );
    match original_receipt {
        Some(receipt) => {
            let recovered_receipt = receipt;
            let recovered = reconciled?;
            assert_eq!(recovered, recovered_receipt);
            assert_eq!(recovered.effect_ordinal(), 1);
            assert_eq!(recovered.value(), 42);
            runtime.tick_all()?;
            fixture::finish_tail(&mut runtime, run)?;
            assert_eq!(adapter.effect_count()?, 1);
            assert_eq!(adapter.value(scope())?, Some(42));
            let envelope_events = journal.events_for_run_full(run)?;
            let envelope_count = envelope_events.iter().filter(|event|
                matches!(event, JournalEvent::ActionCompletedEnvelope { ticket: completed, .. }
                    if *completed == ticket)).count();
            assert_eq!(envelope_count, 1);
            super::assertions::assert_action_completed_envelope(
                &envelope_events,
                run,
                ticket,
                SlotIdx::new(0),
                SlotValue::I64(recovered_receipt.value()),
                &artifact,
                action,
            )?;
            assert_eq!(
                journal
                    .events_for_run_full(run)?
                    .iter()
                    .filter(|event| matches!(event, JournalEvent::RunFinished { .. }))
                    .count(),
                1
            );
            assert_eq!(
                runtime.lookup_pending_action_ticket(run, u64::from(ticket.step.get())),
                None
            );
        }
        None => {
            assert!(matches!(
                reconciled,
                Err(RegisteredExecutionError::Executor(
                    ExecutorError::ReceiptMissing
                ))
            ));
            assert_eq!(adapter.effect_count()?, 0);
            assert_eq!(adapter.value(scope())?, None);
            assert_eq!(runtime.inspect_run_state(run)?, &before);
            assert_eq!(journal.events_for_run_full(run)?, events);
        }
    }
    drop(adapter);
    assert_eq!(fixture::rows(&register_path)?, executor_rows);
    Ok(())
}

fn uncertain_effect(
    runtime: &mut Runtime,
    journal: &FjallJournal,
    catalog: &ExecutorCatalog,
    artifact: &vb_storage::AcceptedArtifact,
    ticket: ActionTicket,
    path: &Path,
) -> Result<RegisterReceipt, Box<dyn std::error::Error>> {
    let approval = approval_for(
        runtime,
        journal,
        ticket.run,
        artifact,
        GateId::new(94_241),
        GateDecision::Approve,
    )?;
    let ticket = fixture::pending(runtime, ticket.run, 1)?;
    let register = catalog.issue_register_grant(artifact, ticket, scope())?;
    let scoped = catalog.issue_scoped_grant_for_bound_gate(
        artifact,
        &register,
        super::gate_cold_recovery::scoped_params(ticket.action, approval.digest)?,
        &approval,
    )?;
    let dispatch = super::gate_cold_recovery::dispatch_context(artifact, ticket)?;
    let mut adapter = FjallRegister::open(path, 8)?;
    runtime.inspect_run(ticket.run, 94_240)?;
    assert!(matches!(
        runtime.execute_registered_action(
            &mut adapter,
            ticket.run,
            u64::from(ticket.step.get()),
            &register,
            vb_core::ScopedDispatch::new(&scoped, &dispatch, &[]).with_gate_approval(&approval)
        ),
        Err(RegisteredExecutionError::Runtime(RuntimeError::QueueFull))
    ));
    assert_eq!(adapter.effect_count()?, 1);
    assert_eq!(adapter.value(scope())?, Some(42));
    let receipt = adapter.reconcile(&register, ticket, 42)?;
    runtime.tick_all()?;
    assert_eq!(
        runtime.lookup_pending_action_ticket(ticket.run, u64::from(ticket.step.get())),
        Some(ticket)
    );
    assert!(
        !journal
            .events_for_run_full(ticket.run)?
            .iter()
            .any(|event| matches!(event, JournalEvent::ActionCompletedEnvelope { .. }))
    );
    Ok(receipt)
}
