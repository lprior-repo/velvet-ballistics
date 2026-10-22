use super::*;

#[test]
fn allowed_policy_permits_effect_and_records_decision() -> TestResult {
    let dir = tempfile::tempdir()?;
    let run = RunId::new(9_103);
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

    let allowed_digest = PolicyDigest::from_bytes([0x33u8; 32]);
    runtime.configure_live_policy(
        allowed_digest,
        PolicyRule {
            action,
            code: DecisionCode::Allowed,
        },
    )?;

    runtime.submit_direct_with_inputs_grants_and_contracts(
        run,
        workflow,
        Box::from([(SlotIdx::ZERO, SlotValue::I64(100))]),
        action_grants(action),
        Box::from([contract]),
    )?;
    runtime.tick_all()?;

    let ticket = runtime
        .lookup_pending_action_ticket(run, u64::from(StepIdx::ZERO.get()))
        .ok_or("expected pending action ticket")?;
    let grant = catalog.issue_register_grant(&artifact, ticket, scope())?;

    let register_path = dir.path().join("register");
    let mut adapter = FjallRegister::open(&register_path, 8)?;
    let receipt = runtime.execute_registered_action(
        &mut adapter,
        run,
        u64::from(StepIdx::ZERO.get()),
        &grant,
    )?;

    // Allowed: exact receipt with value 100, effect count 1, target value 100
    assert_eq!(receipt.value(), 100);
    assert_eq!(adapter.effect_count()?, 1);
    assert_eq!(adapter.value(scope())?, Some(100));

    // Verify full DecisionRecord persisted with Allowed outcome.
    let expected_admission = admission_pin_from_artifact(artifact.policy_digest.as_bytes());
    let expected_input_digest = compute_expected_input_digest(100, Taint::Clean, &ticket)?;
    let expected_decision = make_expected_decision(
        ticket,
        expected_admission,
        current_live_configured(allowed_digest.as_bytes()),
        expected_input_digest,
        DecisionCode::Allowed,
    );

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
        "full stored DecisionRecord must match oracle for allowed policy"
    );
    // Run and ticket consistency
    assert_eq!(stored.ticket.run, run);
    assert_eq!(stored.ticket, ticket);

    Ok(())
}
