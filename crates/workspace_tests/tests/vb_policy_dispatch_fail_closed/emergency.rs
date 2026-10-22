use super::*;

/// Test 2: Configure Allowed, submit, tick, activate emergency deny via
/// actual dedicated store persist, then execute. Expect typed Emergency
/// Denied decision with Active influence, zero effects, unchanged pending.
/// Show that configuring Allowed afterwards cannot loosen the overlay.
#[test]
fn emergency_deny_overlay_cannot_be_loosened_by_later_allowed_config() -> TestResult {
    let dir = tempfile::tempdir()?;
    let run = RunId::new(11_002);
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

    let storage = StorageRuntimeJournal::shared_strict_with_catalog(
        journal.clone(),
        Arc::new(catalog.clone()),
    );
    let mut runtime = Runtime::new(shard_count(1)?, strict_config(), storage);

    // Configure Allowed before scheduling.
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
        Box::from([(SlotIdx::ZERO, SlotValue::I64(100))]),
        action_grants(action),
        Box::from([contract]),
    )?;
    runtime.tick_all()?;

    let ticket = runtime
        .lookup_pending_action_ticket(run, u64::from(StepIdx::ZERO.get()))
        .ok_or("expected pending action ticket")?;

    // Activate emergency deny via actual dedicated store persist.
    runtime.activate_emergency_deny()?;

    // Compute expected full DecisionRecord for first emergency denial attempt.
    let expected_input_digest_first = compute_expected_input_digest(100, Taint::Clean, &ticket)?;
    let expected_admission = PolicyDigest::from_bytes(artifact.policy_digest.as_bytes());
    let expected_first = DecisionRecord {
        ticket,
        admission: PinnedAdmissionPolicy::Artifact(expected_admission),
        current_live: CurrentPolicyIdentity::Configured {
            digest: allowed_digest,
        },
        input_digest: expected_input_digest_first,
        taint: Taint::Clean,
        outcome: DecisionCode::Denied,
        layer: PolicyLayer::Emergency,
        emergency_influence: InfluenceState::Active,
        compat_influence: InfluenceState::NotEvaluated,
    };

    // Save pending state before first execute.
    let pending_before_first = runtime.pending_boundary_snapshot(8);

    // Execute — emergency overlay must reject even though live config is Allowed.
    let grant = catalog.issue_register_grant(&artifact, ticket, scope())?;
    let register_path = dir.path().join("register");
    let mut adapter = FjallRegister::open(&register_path, 8)?;
    let result = runtime.execute_registered_action(
        &mut adapter,
        run,
        u64::from(StepIdx::ZERO.get()),
        &grant,
    );

    match result {
        Err(RegisteredExecutionError::PolicyDenied(decision)) => {
            // Retain existing individual field checks.
            assert_eq!(decision.outcome, DecisionCode::Denied);
            assert_eq!(decision.layer, PolicyLayer::Emergency);
            assert_eq!(decision.emergency_influence, InfluenceState::Active);
            assert_eq!(decision.compat_influence, InfluenceState::NotEvaluated);
            assert_eq!(decision.ticket, ticket);
            assert_eq!(decision.input_digest, expected_input_digest_first);
            assert_eq!(decision.taint, Taint::Clean);

            match decision.current_live {
                CurrentPolicyIdentity::Configured { digest } => {
                    assert_eq!(digest, allowed_digest);
                }
                other => {
                    return Err(format!(
                        "expected Configured live policy (Allowed), got: {other:?}"
                    )
                    .into());
                }
            }

            match decision.admission {
                PinnedAdmissionPolicy::Artifact(admitted_digest) => {
                    assert_eq!(admitted_digest, expected_admission);
                }
                other => {
                    return Err(format!("expected Artifact admission pin, got: {other:?}").into());
                }
            }

            // Compare full returned record against independently computed expected.
            assert_eq!(
                decision.as_ref(),
                &expected_first,
                "first emergency denial record must match full expected"
            );
        }
        other => {
            return Err(
                format!("expected PolicyDenied with Emergency decision, got: {other:?}").into(),
            );
        }
    }

    // Zero effects.
    assert_eq!(
        adapter.effect_count()?,
        0,
        "emergency deny must not produce any effect"
    );

    // Pending state unchanged after first emergency deny.
    let pending_after_first = runtime.pending_boundary_snapshot(8);
    assert_eq!(
        pending_after_first, pending_before_first,
        "first emergency deny must not alter pending state"
    );

    // Pending ticket unchanged — compare exact identity.
    let ticket_after_first = runtime
        .lookup_pending_action_ticket(run, u64::from(StepIdx::ZERO.get()))
        .ok_or("expected pending action ticket after first deny")?;
    assert_eq!(
        Some(ticket_after_first),
        Some(ticket),
        "pending ticket must remain identical after first emergency deny"
    );

    // Show that configuring Allowed after emergency cannot loosen the overlay.
    let later_allowed_digest = PolicyDigest::from_bytes([0x33u8; 32]);
    runtime.configure_live_policy(
        later_allowed_digest,
        PolicyRule {
            action,
            code: DecisionCode::Allowed,
        },
    )?;

    let second_ticket = runtime
        .lookup_pending_action_ticket(run, u64::from(StepIdx::ZERO.get()))
        .ok_or("expected pending action ticket still present")?;
    let second_grant = catalog.issue_register_grant(&artifact, second_ticket, scope())?;

    // Compute expected full record for second attempt.
    let second_input_value = 100; // same workflow, same input
    let expected_input_digest_second =
        compute_expected_input_digest(second_input_value, Taint::Clean, &second_ticket)?;
    let expected_second = DecisionRecord {
        ticket: second_ticket,
        admission: PinnedAdmissionPolicy::Artifact(expected_admission),
        current_live: CurrentPolicyIdentity::Configured {
            digest: later_allowed_digest,
        },
        input_digest: expected_input_digest_second,
        taint: Taint::Clean,
        outcome: DecisionCode::Denied,
        layer: PolicyLayer::Emergency,
        emergency_influence: InfluenceState::Active,
        compat_influence: InfluenceState::NotEvaluated,
    };

    // Save pending state before second execute.
    let pending_before_second = runtime.pending_boundary_snapshot(8);

    let register_path2 = dir.path().join("register2");
    let mut adapter2 = FjallRegister::open(&register_path2, 8)?;
    let result2 = runtime.execute_registered_action(
        &mut adapter2,
        run,
        u64::from(StepIdx::ZERO.get()),
        &second_grant,
    );

    match result2 {
        Err(RegisteredExecutionError::PolicyDenied(decision)) => {
            // Retain existing individual field checks.
            assert_eq!(decision.outcome, DecisionCode::Denied);
            assert_eq!(decision.layer, PolicyLayer::Emergency);
            assert_eq!(decision.emergency_influence, InfluenceState::Active);
            assert_eq!(decision.compat_influence, InfluenceState::NotEvaluated);
            assert_eq!(decision.ticket, second_ticket);
            assert_eq!(decision.input_digest, expected_input_digest_second);
            assert_eq!(decision.taint, Taint::Clean);

            // Compare full returned record against independently computed expected.
            assert_eq!(
                decision.as_ref(),
                &expected_second,
                "second emergency denial record must match full expected"
            );
        }
        other => {
            return Err(format!(
                "expected second execute to also be denied by emergency overlay, got: {other:?}"
            )
            .into());
        }
    }
    assert_eq!(
        adapter2.effect_count()?,
        0,
        "second execute must also have zero effects"
    );

    // Pending state unchanged after second emergency deny.
    let pending_after_second = runtime.pending_boundary_snapshot(8);
    assert_eq!(
        pending_after_second, pending_before_second,
        "second emergency deny must not alter pending state"
    );

    // Both stored decision events must exactly match the two expected records in order.
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
        2,
        "exactly two policy decision events must be stored"
    );
    assert_eq!(
        decision_events,
        vec![&expected_first, &expected_second],
        "stored decision events must exactly match expected records in order"
    );

    Ok(())
}
