use super::*;

#[test]
fn reconcile_ignores_policy_change_after_effect() -> TestResult {
    let dir = tempfile::tempdir()?;
    let run = RunId::new(9_104);
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

    let allowed_digest = PolicyDigest::from_bytes([0x44u8; 32]);
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
        Box::from([(SlotIdx::ZERO, SlotValue::I64(55))]),
        action_grants(action),
        Box::from([contract]),
    )?;
    runtime.tick_all()?;

    let ticket = runtime
        .lookup_pending_action_ticket(run, u64::from(StepIdx::ZERO.get()))
        .ok_or("expected pending action ticket")?;
    let grant = catalog.issue_register_grant(&artifact, ticket, scope())?;

    // Execute allowed: creates actual receipt but do not tick to completion.
    let register_path = dir.path().join("register");
    let mut adapter = FjallRegister::open(&register_path, 8)?;
    let first = runtime.execute_registered_action(
        &mut adapter,
        run,
        u64::from(StepIdx::ZERO.get()),
        &grant,
    )?;
    assert_eq!(adapter.effect_count()?, 1);

    // Verify the allowed decision was persisted with full record.
    let expected_admission = admission_pin_from_artifact(artifact.policy_digest.as_bytes());
    let expected_input_digest = compute_expected_input_digest(55, Taint::Clean, &ticket)?;
    let expected_allowed_decision = make_expected_decision(
        ticket,
        expected_admission,
        current_live_configured(allowed_digest.as_bytes()),
        expected_input_digest,
        DecisionCode::Allowed,
    );
    let decisions_before_reconcile = read_journal_decision_events(&journal, run)?;
    assert_eq!(
        decisions_before_reconcile.len(),
        1,
        "exactly one allowed decision must be stored before reconcile"
    );
    let stored = decisions_before_reconcile
        .first()
        .ok_or("expected exactly one decision")?;
    assert_eq!(
        stored, &expected_allowed_decision,
        "stored allowed DecisionRecord must match oracle"
    );

    // Change policy to deny after the effect was created.
    let denied_digest = PolicyDigest::from_bytes([0x55u8; 32]);
    runtime.configure_live_policy(
        denied_digest,
        PolicyRule {
            action,
            code: DecisionCode::Denied,
        },
    )?;

    // Reconcile must read existing receipt and return it, unaffected by new deny.
    let reconciled = runtime.reconcile_registered_action(
        &mut adapter,
        run,
        u64::from(StepIdx::ZERO.get()),
        &grant,
    )?;
    // Full receipt exact equality
    assert_eq!(
        reconciled, first,
        "reconciled receipt must exactly match original"
    );
    assert_eq!(reconciled.value(), 55);
    assert_eq!(adapter.effect_count()?, 1);

    // Reconcile must NOT create a new policy decision — exact same list.
    let decisions_after_reconcile = read_journal_decision_events(&journal, run)?;
    assert_eq!(
        decisions_after_reconcile.len(),
        1,
        "reconcile must not add a new policy decision"
    );
    let stored = decisions_after_reconcile
        .first()
        .ok_or("expected exactly one decision")?;
    assert_eq!(
        stored, &expected_allowed_decision,
        "stored decision list must be exactly the same before/after reconcile"
    );

    Ok(())
}
