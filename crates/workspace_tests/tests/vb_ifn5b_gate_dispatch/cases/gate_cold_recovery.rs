use super::*;

#[test]
fn cold_resume_reissues_bound_authority_for_pending_registered_do() -> TestResult {
    let dir = storage_dir()?;
    let run = RunId::new(11_704);
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
    let shards = std::num::NonZeroUsize::new(1).ok_or("shard count")?;
    let mut config = crate::common::runtime_fixture::strict_config();
    config.step_budget_per_tick = 1;
    let mut runtime = Runtime::new(shards, config, storage);
    runtime.configure_live_policy(
        PolicyDigest::from_bytes([4; 32]),
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
        GateId::new(74),
        GateDecision::Approve,
    )?;
    let live_ticket = pending_ticket(&mut runtime, run)?;
    let live_register = catalog.issue_register_grant(&artifact, live_ticket, scope())?;
    let live_params = scoped_params(action, approval.digest, false)?;
    let live_bound = catalog.issue_scoped_grant_for_bound_gate(
        &artifact,
        &live_register,
        live_params,
        &approval,
    )?;
    let live_dispatch = dispatch_context(&artifact, live_ticket)?;
    let register_path = dir.join("register");
    let mut live_adapter = FjallRegister::open(&register_path, 8)?;
    let live_receipt = runtime.execute_registered_action(
        &mut live_adapter,
        run,
        u64::from(live_ticket.step.get()),
        &live_register,
        vb_core::ScopedDispatch::new(&live_bound, &live_dispatch, &[])
            .with_gate_approval(&approval),
    )?;
    assert_eq!(live_receipt.value(), 42);
    assert_eq!(live_adapter.effect_count()?, 1);
    drop(live_adapter);
    drop(runtime);
    drop(journal);
    drop(catalog);

    let catalog = ExecutorCatalog::run_registered_conformance()?;
    let journal = Arc::new(FjallJournal::open(&dir, Some(FjallConfig::default()))?);
    let storage = StorageRuntimeJournal::shared_strict_with_catalog(
        journal.clone(),
        Arc::new(catalog.clone()),
    );
    let mut runtime = Runtime::new(shards, config, storage);
    runtime.configure_live_policy(
        PolicyDigest::from_bytes([4; 32]),
        PolicyRule {
            action,
            code: DecisionCode::Allowed,
        },
    )?;
    runtime.recover_and_resume(run)?;
    let ticket = pending_ticket(&mut runtime, run)?;
    assert_eq!(ticket, live_ticket);
    assert_eq!(ticket.action, action);
    let durable_approval = matching_approval(&journal, run, approval.digest)?;
    let register = catalog.issue_register_grant(&artifact, ticket, scope())?;
    let params = scoped_params(action, durable_approval.digest, false)?;
    let bound = catalog
        .issue_scoped_grant_for_bound_gate(&artifact, &register, params.clone(), &durable_approval)
        .map_err(|error| format!("cold bound issuance: {error:?}"))?;
    let dispatch = dispatch_context(&artifact, ticket)?;
    let mut adapter = FjallRegister::open(&register_path, 8)?;
    let before = DispatchSnapshot {
        run,
        ticket,
        events: journal.events_for_run_full(run)?,
    };
    let missing = runtime.execute_registered_action(
        &mut adapter,
        run,
        u64::from(ticket.step.get()),
        &register,
        vb_core::ScopedDispatch::new(&bound, &dispatch, &[]),
    );
    if !matches!(
        missing,
        Err(RegisteredExecutionError::Executor(
            ExecutorError::Unauthorized
        ))
    ) {
        return Err(format!("expected missing cold receipt refusal; got {missing:?}").into());
    }
    assert_one_effect_unchanged(&adapter, &mut runtime, &journal, &before)?;
    let altered = altered_input_receipt(&durable_approval)?;
    let altered_params = vb_core::ScopedGrantParams {
        approval_receipt: altered.digest,
        ..params
    };
    let altered_grant = catalog.issue_scoped_grant_for_bound_gate(
        &artifact,
        &register,
        altered_params,
        &altered,
    )?;
    let wrong = runtime.execute_registered_action(
        &mut adapter,
        run,
        u64::from(ticket.step.get()),
        &register,
        vb_core::ScopedDispatch::new(&altered_grant, &dispatch, &[]).with_gate_approval(&altered),
    );
    if !matches!(
        wrong,
        Err(RegisteredExecutionError::Executor(
            ExecutorError::Unauthorized
        ))
    ) {
        return Err(format!("expected altered cold receipt refusal; got {wrong:?}").into());
    }
    assert_one_effect_unchanged(&adapter, &mut runtime, &journal, &before)?;
    let receipt = runtime
        .reconcile_registered_action(
            &mut adapter,
            run,
            u64::from(ticket.step.get()),
            &register,
            &bound,
            &dispatch,
        )
        .map_err(|error| format!("cold receipt reconciliation: {error:?}"))?;
    assert_eq!(receipt, live_receipt);
    assert_eq!(receipt.value(), 42);
    assert_eq!(adapter.effect_count()?, 1);
    assert_eq!(adapter.value(scope())?, Some(42));
    runtime.tick_all()?;
    assert!(journal.events_for_run_full(run)?.iter().any(|event| matches!(
        event,
        JournalEvent::ActionCompletedEnvelope { ticket: completed, .. } if *completed == ticket
    )));
    assert_eq!(adapter.effect_count()?, 1);
    Ok(())
}

fn matching_approval(
    journal: &FjallJournal,
    run: RunId,
    expected: GateDigest,
) -> Result<BoundApprovalReceipt, Box<dyn std::error::Error>> {
    journal
        .events_for_run_full(run)?
        .iter()
        .find_map(|event| match event {
            JournalEvent::GateDecisionBoundEvent { record, .. } => record.bound_receipt.clone(),
            _ => None,
        })
        .filter(|receipt| receipt.digest == expected)
        .ok_or_else(|| "durable matching gate receipt missing after cold resume".into())
}

fn assert_one_effect_unchanged(
    adapter: &FjallRegister,
    runtime: &mut Runtime,
    journal: &FjallJournal,
    before: &DispatchSnapshot,
) -> TestResult {
    assert_eq!(adapter.effect_count()?, 1);
    assert_eq!(adapter.value(scope())?, Some(42));
    assert_eq!(journal.events_for_run_full(before.run)?, before.events);
    assert_eq!(
        runtime.lookup_pending_action_ticket(before.run, 9),
        Some(before.ticket)
    );
    Ok(())
}

fn altered_input_receipt(
    receipt: &BoundApprovalReceipt,
) -> Result<BoundApprovalReceipt, Box<dyn std::error::Error>> {
    let mut altered = receipt.clone();
    altered.binding.input_digest = GateDigest::from_bytes([88; 32]);
    altered.digest = vb_runtime::manual_gate::compute_bound_receipt_digest(&altered)?;
    Ok(altered)
}

fn scoped_params(
    action: ActionId,
    receipt: GateDigest,
    break_glass: bool,
) -> Result<vb_core::ScopedGrantParams, Box<dyn std::error::Error>> {
    Ok(vb_core::ScopedGrantParams {
        id: vb_core::GrantId::new(74),
        issuer: person("grant-authority")?,
        subject: person("run-operator")?,
        capability: Capability::new("velvet.register.write".into(), action),
        scope: scope(),
        issued_at: vb_core::GrantTimeMs::new(150),
        expires_at: vb_core::GrantTimeMs::new(1_000),
        delegable: false,
        delegated_from: None,
        revocation_epoch: vb_core::RevocationEpoch::new(1),
        break_glass,
        approval_receipt: receipt,
        signature: None,
    })
}

fn dispatch_context(
    artifact: &vb_storage::AcceptedArtifact,
    ticket: vb_core::ActionTicket,
) -> Result<vb_core::GrantDispatchContext, Box<dyn std::error::Error>> {
    Ok(vb_core::GrantDispatchContext::new(
        artifact.digest,
        ticket,
        scope(),
        person("grant-authority")?,
        person("run-operator")?,
        vb_core::GrantTimeMs::new(250),
        vb_core::RevocationEpoch::new(1),
    ))
}
