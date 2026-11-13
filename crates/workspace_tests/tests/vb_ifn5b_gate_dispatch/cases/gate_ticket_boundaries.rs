use super::*;

#[test]
fn foreign_run_and_ticket_attempt_boundary_refuse_without_mutation() -> TestResult {
    let dir = storage_dir()?;
    let run = RunId::new(11_702);
    let action = ActionId::new(1);
    let workflow = workflow().map_err(|error| format!("workflow: {error}"))?;
    let contract = contract()?;
    let preceding = preceding_contract()?;
    let contracts = [preceding, contract.clone()];
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
    let source_digest =
        super::owned_submission::commit(&journal, &catalog, &workflow, &artifact, run)?;
    let storage = StorageRuntimeJournal::shared_strict_with_catalog(
        journal.clone(),
        Arc::new(catalog.clone()),
    );
    let shard_count = std::num::NonZeroUsize::new(1).ok_or("shard count")?;
    let mut config = crate::common::runtime_fixture::strict_config();
    config.step_budget_per_tick = 1;
    let mut runtime = Runtime::new(shard_count, config, storage);
    runtime.configure_live_policy(
        PolicyDigest::from_bytes([4; 32]),
        PolicyRule {
            action,
            code: DecisionCode::Allowed,
        },
    )?;
    runtime
        .activate_owned_submission(run, source_digest)
        .map_err(|error| format!("activate owned run: {error:?}"))?;
    runtime.tick_all()?;
    let approval = approval_for(
        &mut runtime,
        &journal,
        run,
        &artifact,
        GateId::new(72),
        GateDecision::Approve,
    )?;
    let ticket = pending_ticket(&mut runtime, run)?;
    let register = catalog.issue_register_grant(&artifact, ticket, scope())?;
    let params = super::gate_cold_recovery::scoped_params(action, approval.digest, false)?;
    let bound =
        catalog.issue_scoped_grant_for_bound_gate(&artifact, &register, params, &approval)?;
    let dispatch = super::gate_cold_recovery::dispatch_context(&artifact, ticket)?;
    let mut adapter = FjallRegister::open(&dir.join("register"), 8)?;
    let before = super::DispatchSnapshot {
        run,
        ticket,
        events: journal.events_for_run_full(run)?,
    };

    let foreign_run = RunId::new(11_703);
    let foreign_result = runtime.execute_registered_action(
        &mut adapter,
        foreign_run,
        u64::from(ticket.step.get()),
        &register,
        vb_core::ScopedDispatch::new(&bound, &dispatch, &[]).with_gate_approval(&approval),
    );
    if !matches!(
        foreign_result,
        Err(RegisteredExecutionError::Runtime(
            vb_runtime::RuntimeError::InvalidActionCompletion
        ))
    ) {
        return Err(
            format!("expected foreign-run consumer refusal; got {foreign_result:?}").into(),
        );
    }
    super::assert_no_effect(&adapter, &mut runtime, &journal, &before)?;
    assert_eq!(
        runtime.lookup_pending_action_ticket(foreign_run, u64::from(ticket.step.get())),
        None
    );

    let boundary_ticket = vb_core::ActionTicket {
        run_attempt: u16::MAX,
        attempt: u16::MAX,
        ..ticket
    };
    let boundary_dispatch = vb_core::GrantDispatchContext::new(
        artifact.digest,
        boundary_ticket,
        scope(),
        person("grant-authority")?,
        person("run-operator")?,
        vb_core::GrantTimeMs::new(250),
        vb_core::RevocationEpoch::new(1),
    );
    let boundary_result = runtime.execute_registered_action(
        &mut adapter,
        run,
        u64::from(ticket.step.get()),
        &register,
        vb_core::ScopedDispatch::new(&bound, &boundary_dispatch, &[]).with_gate_approval(&approval),
    );
    if !matches!(
        boundary_result,
        Err(RegisteredExecutionError::Executor(ExecutorError::Grant(
            vb_core::GrantError::TicketMismatch
        )))
    ) {
        return Err(format!(
            "expected run/action-attempt boundary refusal; got {boundary_result:?}"
        )
        .into());
    }
    super::assert_no_effect(&adapter, &mut runtime, &journal, &before)?;
    assert_eq!(
        runtime.lookup_pending_action_ticket(run, u64::from(ticket.step.get())),
        Some(ticket)
    );
    Ok(())
}
