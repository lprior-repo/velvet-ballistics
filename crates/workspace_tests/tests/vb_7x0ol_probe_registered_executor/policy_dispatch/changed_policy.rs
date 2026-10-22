use super::*;

#[test]
fn after_scheduling_policy_change_to_deny() -> TestResult {
    let dir = tempfile::tempdir()?;
    let run = RunId::new(9_102);
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

    // Configure allowed before scheduling.
    let allowed_digest = PolicyDigest::from_bytes([0x11u8; 32]);
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
        Box::from([(SlotIdx::ZERO, SlotValue::I64(99))]),
        action_grants(action),
        Box::from([contract]),
    )?;
    runtime.tick_all()?;

    let ticket = runtime
        .lookup_pending_action_ticket(run, u64::from(StepIdx::ZERO.get()))
        .ok_or("expected pending action ticket")?;

    // Change policy to deny with a different digest before execute.
    let denied_digest = PolicyDigest::from_bytes([0x22u8; 32]);
    runtime.configure_live_policy(
        denied_digest,
        PolicyRule {
            action,
            code: DecisionCode::Denied,
        },
    )?;

    let grant = catalog.issue_register_grant(&artifact, ticket, scope())?;
    let register_path = dir.path().join("register");
    let mut adapter = FjallRegister::open(&register_path, 8)?;
    let result = runtime.execute_registered_action(
        &mut adapter,
        run,
        u64::from(StepIdx::ZERO.get()),
        &grant,
    );

    let expected_admission = admission_pin_from_artifact(artifact.policy_digest.as_bytes());
    let expected_input_digest = compute_expected_input_digest(99, Taint::Clean, &ticket)?;
    let expected_decision = make_expected_decision(
        ticket,
        expected_admission,
        current_live_configured(denied_digest.as_bytes()),
        expected_input_digest,
        DecisionCode::Denied,
    );

    match result {
        Err(RegisteredExecutionError::PolicyDenied(decision)) => {
            // Full 9-field DecisionRecord oracle comparison
            assert_eq!(
                *decision, expected_decision,
                "full DecisionRecord must match oracle for policy-change deny"
            );
            // Distinct digests: admission differs from both manual live digests
            match decision.admission {
                PinnedAdmissionPolicy::Artifact(admitted) => {
                    assert_ne!(
                        admitted, denied_digest,
                        "admission must differ from current denied"
                    );
                    assert_ne!(
                        admitted, allowed_digest,
                        "admission must differ from original allowed"
                    );
                }
                _ => unreachable!("admission checked in DecisionRecord equality"),
            }
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
        "stored DecisionRecord must match oracle for policy-change deny"
    );
    Ok(())
}
