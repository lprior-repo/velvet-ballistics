use super::*;

/// Test 3: Strict-port typed failure via test-only RuntimeJournal wrapper.
/// Real store, journal, catalog, grants, register remain actual. Override
/// only append_policy_decision_strict to inject a strict port failure.
#[test]
fn strict_port_failure_returns_typed_runtime_error_with_zero_effects() -> TestResult {
    let dir = tempfile::tempdir()?;
    let run = RunId::new(11_003);
    let action = ActionId::new(0);

    let catalog = ExecutorCatalog::run_registered_conformance()?;
    let workflow = action_then_finish_workflow()?;
    let contract = external_contract(action)?;
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

    // Use a test wrapper that overrides only append_policy_decision_strict.
    let wrapped = Arc::new(FailingPolicyPortJournal::new(
        journal.clone(),
        catalog.clone(),
    ));
    let mut runtime = Runtime::new(shard_count(1)?, strict_config(), wrapped.clone());

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
        Box::from([(SlotIdx::ZERO, SlotValue::I64(77))]),
        action_grants(action),
        Box::from([contract]),
    )?;
    runtime.tick_all()?;

    let ticket = runtime
        .lookup_pending_action_ticket(run, u64::from(StepIdx::ZERO.get()))
        .ok_or("expected pending action ticket")?;
    let grant = catalog.issue_register_grant(&artifact, ticket, scope())?;

    // Save pending state before execute for fail-closed verification.
    let pending_before = runtime.pending_boundary_snapshot(8);

    let register_path = dir.path().join("register");
    let mut adapter = FjallRegister::open(&register_path, 8)?;
    let result = runtime.execute_registered_action(
        &mut adapter,
        run,
        u64::from(StepIdx::ZERO.get()),
        &grant,
    );

    // Expect exact typed error propagation.
    match result {
        Err(RegisteredExecutionError::Runtime(runtime_err)) => {
            assert!(
                matches!(
                    runtime_err,
                    RuntimeError::UnsupportedOperation { operation }
                        if operation == "test_policy_decision_failure"
                ),
                "expected exact UnsupportedOperation error, got: {runtime_err:?}"
            );
        }
        other => {
            return Err(format!(
                "expected RegisteredExecutionError::Runtime with UnsupportedOperation, got: {other:?}"
            )
            .into());
        }
    }

    // Zero effects on the real FjallRegister.
    assert_eq!(
        adapter.effect_count()?,
        0,
        "strict port failure must not produce any effect"
    );

    // Pending state unchanged after strict port failure.
    let pending_after = runtime.pending_boundary_snapshot(8);
    assert_eq!(
        pending_after, pending_before,
        "strict port failure must not alter pending state"
    );

    // Pending ticket unchanged — compare exact identity.
    let ticket_after = runtime
        .lookup_pending_action_ticket(run, u64::from(StepIdx::ZERO.get()))
        .ok_or("expected pending action ticket after failure")?;
    assert_eq!(
        Some(ticket_after),
        Some(ticket),
        "pending ticket must remain identical after strict port failure"
    );

    // No stored decisions (strict port failed before any event).
    let events = vb_storage::read_run_events(&journal, run)?;
    let decision_events: Vec<_> = events
        .iter()
        .filter_map(|e| match e {
            vb_storage::JournalEvent::PolicyDecisionEvent { decision, .. } => Some(decision),
            _ => None,
        })
        .collect();
    assert_eq!(
        decision_events.len(),
        0,
        "strict port failure must not store any policy decision"
    );

    // Verify the port was called exactly once with the expected decision
    // and the actual current sequence number.
    let expected_seq = {
        let events = vb_storage::read_run_events(&journal, run)?;
        let last = events
            .last()
            .ok_or("expected at least one prior event for seq computation")?;
        let next = last
            .seq()
            .get()
            .checked_add(1)
            .ok_or("expected seq arithmetic to stay within u64")?;
        EventSeq::new(next)
    };

    let expected_input_digest = compute_expected_input_digest(77, Taint::Clean, &ticket)?;
    let expected_record = DecisionRecord {
        ticket,
        admission: PinnedAdmissionPolicy::Artifact(PolicyDigest::from_bytes(
            artifact.policy_digest.as_bytes(),
        )),
        current_live: CurrentPolicyIdentity::Configured {
            digest: allowed_digest,
        },
        input_digest: expected_input_digest,
        taint: Taint::Clean,
        outcome: DecisionCode::Allowed,
        layer: PolicyLayer::Live,
        emergency_influence: InfluenceState::Inactive,
        compat_influence: InfluenceState::NotEvaluated,
    };

    assert!(
        wrapped.policy_port_called(),
        "strict port must have been called"
    );
    let captured = wrapped.captured_decision()?.ok_or("no decision captured")?;
    assert_eq!(
        captured, expected_record,
        "single-decision view must match full oracle"
    );
    assert_eq!(
        wrapped.captured_calls()?,
        vec![(expected_record, expected_seq)],
        "exact single call with full record and correct seq must be captured"
    );

    Ok(())
}
