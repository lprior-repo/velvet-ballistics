use super::*;

#[test]
fn cold_pending_without_receipt_refuses_execute_and_reconcile_without_mutation() -> TestResult {
    let dir = storage_dir()?;
    let run = RunId::new(11_705);
    let action = ActionId::new(1);
    let workflow = workflow().map_err(|error| format!("workflow: {error}"))?;
    let contract = contract()?;
    let contracts = [preceding_contract()?, contract.clone()];
    let catalog = ExecutorCatalog::run_registered_conformance()?;
    let pack = catalog.register_pack(&contract, scope(), RecoveryClass::ReconcileReceipt)?;
    let journal = Arc::new(FjallJournal::open(&dir, Some(FjallConfig::default()))?);
    let artifact = catalog.submit_artifact(
        &journal,
        &workflow,
        RuntimePolicy::Strict,
        &contracts,
        &[pack],
    )?;
    let source = owned_submission::commit(&journal, &catalog, &workflow, &artifact, run)?;
    let storage = StorageRuntimeJournal::shared_strict_with_catalog(
        journal.clone(),
        Arc::new(catalog.clone()),
    );
    let shard_count = std::num::NonZeroUsize::new(1).ok_or("shard count")?;
    let mut config = crate::common::runtime_fixture::strict_config();
    config.step_budget_per_tick = 1;
    let mut runtime = Runtime::new(shard_count, config, storage);
    let policy_digest = PolicyDigest::from_bytes([4; 32]);
    runtime.configure_live_policy(
        policy_digest,
        PolicyRule {
            action,
            code: DecisionCode::Allowed,
        },
    )?;
    runtime.activate_owned_submission(run, source)?;
    runtime.tick_all()?;
    let approval = approval_for(
        &mut runtime,
        &journal,
        run,
        &artifact,
        GateId::new(75),
        GateDecision::Approve,
    )?;
    let live_ticket = pending_ticket(&mut runtime, run)?;
    let register_path = dir.join("register");
    drop(runtime);
    drop(journal);
    drop(catalog);

    let catalog = ExecutorCatalog::run_registered_conformance()?;
    let journal = Arc::new(FjallJournal::open(&dir, Some(FjallConfig::default()))?);
    let storage = StorageRuntimeJournal::shared_strict_with_catalog(
        journal.clone(),
        Arc::new(catalog.clone()),
    );
    let mut runtime = Runtime::new(shard_count, config, storage);
    runtime.configure_live_policy(
        policy_digest,
        PolicyRule {
            action,
            code: DecisionCode::Allowed,
        },
    )?;
    runtime.recover_and_resume(run)?;
    let ticket = pending_ticket(&mut runtime, run)?;
    assert_eq!(ticket, live_ticket);
    assert_eq!(ticket.run, approval.receipt.run);
    assert_eq!(ticket.action, approval.action);
    assert_eq!(ticket.run_attempt, approval.binding.attempt);
    assert_eq!(
        approval.binding.artifact_digest.as_bytes(),
        artifact.digest.as_bytes()
    );
    let cold_events = journal.events_for_run_full(run)?;
    let opened_gate = cold_events
        .iter()
        .find_map(|event| match event {
            JournalEvent::GateOpenedBoundEvent { gate, .. } => Some(gate),
            _ => None,
        })
        .ok_or("durable gate opening missing after cold recovery")?;
    assert_eq!(opened_gate.gate.id, approval.receipt.gate);
    assert_eq!(opened_gate.binding, approval.binding);
    assert_eq!(
        approval.binding.policy_digest.as_bytes(),
        artifact.policy_digest.as_bytes()
    );
    let durable = super::gate_cold_recovery::matching_approval(&journal, run, approval.digest)?;
    assert_eq!(durable, approval);
    let register = catalog.issue_register_grant(&artifact, ticket, scope())?;
    let bound = catalog.issue_scoped_grant_for_bound_gate(
        &artifact,
        &register,
        super::gate_cold_recovery::scoped_params(action, durable.digest, false)?,
        &durable,
    )?;
    let dispatch = super::gate_cold_recovery::dispatch_context(&artifact, ticket)?;
    let mut adapter = FjallRegister::open(&register_path, 8)?;
    let before = DispatchSnapshot {
        run,
        ticket,
        events: journal.events_for_run_full(run)?,
    };

    let execute = runtime.execute_registered_action(
        &mut adapter,
        run,
        u64::from(ticket.step.get()),
        &register,
        vb_core::ScopedDispatch::new(&bound, &dispatch, &[]).with_gate_approval(&durable),
    );
    if !matches!(
        execute,
        Err(RegisteredExecutionError::Executor(
            ExecutorError::Unauthorized
        ))
    ) {
        return Err(format!("expected cold non-live execute refusal; got {execute:?}").into());
    }
    assert_no_effect(&adapter, &mut runtime, &journal, &before)?;

    let reconcile = runtime.reconcile_registered_action(
        &mut adapter,
        run,
        u64::from(ticket.step.get()),
        &register,
        &bound,
        &dispatch,
    );
    if !matches!(
        reconcile,
        Err(RegisteredExecutionError::Executor(
            ExecutorError::ReceiptMissing
        ))
    ) {
        return Err(format!("expected cold missing-receipt refusal; got {reconcile:?}").into());
    }
    assert_no_effect(&adapter, &mut runtime, &journal, &before)?;
    Ok(())
}
