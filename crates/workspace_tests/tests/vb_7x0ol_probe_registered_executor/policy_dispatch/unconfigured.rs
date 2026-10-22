use super::*;

#[test]
fn unconfigured_policy_denies_execution() -> TestResult {
    let dir = tempfile::tempdir()?;
    let run = RunId::new(9_101);
    let action = ActionId::new(0);
    let workflow = action_then_finish_workflow()?;
    let contract = external_contract(action)?;
    let catalog = ExecutorCatalog::run_registered_conformance()?;
    let pack = catalog.register_pack(&contract, scope(), RecoveryClass::ReconcileReceipt)?;

    let journal = Arc::new(FjallJournal::open(
        dir.path(),
        Some(FjallConfig::default()),
    )?);
    let artifact = catalog.submit_artifact(
        &journal,
        &workflow,
        RuntimePolicy::Strict,
        &[contract.clone()],
        &[pack],
    )?;

    let storage = StorageRuntimeJournal::shared_strict_with_catalog(
        journal.clone(),
        Arc::new(catalog.clone()),
    );
    let mut runtime = Runtime::new(shard_count(1)?, strict_config(), storage);

    runtime.submit_direct_with_inputs_grants_and_contracts(
        run,
        workflow,
        Box::from([(SlotIdx::ZERO, SlotValue::I64(42))]),
        action_grants(action),
        Box::from([contract]),
    )?;
    runtime.tick_all()?;

    let ticket = runtime
        .lookup_pending_action_ticket(run, u64::from(StepIdx::ZERO.get()))
        .ok_or("expected pending action ticket")?;
    let grant = catalog.issue_register_grant(&artifact, ticket, scope())?;

    // No configure_live_policy call — runtime is Unconfigured.
    let register_path = dir.path().join("register");
    let mut adapter = FjallRegister::open(&register_path, 8)?;
    let result = runtime.execute_registered_action(
        &mut adapter,
        run,
        u64::from(StepIdx::ZERO.get()),
        &grant,
    );

    let expected_admission = admission_pin_from_artifact(artifact.policy_digest.as_bytes());
    let expected_input_digest = compute_expected_input_digest(42, Taint::Clean, &ticket)?;
    let expected_decision = make_expected_decision(
        ticket,
        expected_admission,
        CurrentPolicyIdentity::Unconfigured,
        expected_input_digest,
        DecisionCode::Denied,
    );

    match result {
        Err(RegisteredExecutionError::PolicyDenied(decision)) => {
            // Full 9-field DecisionRecord oracle comparison
            assert_eq!(
                *decision, expected_decision,
                "full DecisionRecord must match oracle for unconfigured deny"
            );
            // Run consistency: stored decision must reference the same run
            assert_eq!(decision.ticket.run, run);
        }
        other => return Err(format!("expected PolicyDenied, got: {other:?}").into()),
    }

    // Denial: no adapter effect, target value unchanged
    assert_eq!(adapter.effect_count()?, 0);
    assert_eq!(
        adapter.value(scope())?,
        None,
        "target value must remain unset after policy denial"
    );

    // Exact stored decision list: exactly 1 decision matching oracle
    let stored_decisions = read_journal_decision_events(&journal, run)?;
    assert_eq!(
        stored_decisions.len(),
        1,
        "exactly one policy decision must be stored"
    );
    let stored = stored_decisions
        .first()
        .ok_or("expected exactly one decision")?;
    assert_eq!(
        stored, &expected_decision,
        "stored DecisionRecord must match oracle for unconfigured deny"
    );

    // Pending ticket preserved after denial
    let after_ticket = runtime
        .lookup_pending_action_ticket(run, u64::from(StepIdx::ZERO.get()))
        .ok_or("pending ticket must remain after policy denial")?;
    assert_eq!(
        after_ticket, ticket,
        "exact full ticket must be preserved after denial"
    );
    Ok(())
}
