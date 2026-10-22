use super::*;

/// Test 1: Actual current Suspended policy before execute on a genuine
/// accepted artifact / pending action. Expect typed PolicyDenied with
/// full 9-field DecisionRecord, zero effects, unchanged pending state.
#[test]
fn suspended_policy_denies_execution_with_full_record() -> TestResult {
    let dir = tempfile::tempdir()?;
    let run = RunId::new(11_001);
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

    // Configure a Suspended live policy for this action.
    let suspended_digest = PolicyDigest::from_bytes([0x99u8; 32]);
    runtime.configure_live_policy(
        suspended_digest,
        PolicyRule {
            action,
            code: DecisionCode::Suspended,
        },
    )?;

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

    // Compute expected full DecisionRecord independently of returned decision.
    let expected_input_digest = compute_expected_input_digest(42, Taint::Clean, &ticket)?;
    let expected_admission = PolicyDigest::from_bytes(artifact.policy_digest.as_bytes());
    let expected = DecisionRecord {
        ticket,
        admission: PinnedAdmissionPolicy::Artifact(expected_admission),
        current_live: CurrentPolicyIdentity::Configured {
            digest: suspended_digest,
        },
        input_digest: expected_input_digest,
        taint: Taint::Clean,
        outcome: DecisionCode::Suspended,
        layer: PolicyLayer::Live,
        emergency_influence: InfluenceState::Inactive,
        compat_influence: InfluenceState::NotEvaluated,
    };

    // Save pending state before execute for fail-closed verification.
    let pending_before = runtime.pending_boundary_snapshot(8);

    // Execute with suspended policy — must be rejected before any effect.
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
            assert_eq!(decision.outcome, DecisionCode::Suspended);
            assert_eq!(decision.layer, PolicyLayer::Live);

            match decision.current_live {
                CurrentPolicyIdentity::Configured { digest } => {
                    assert_eq!(digest, suspended_digest);
                }
                other => {
                    return Err(format!(
                        "expected Configured live policy with Suspended rule, got: {other:?}"
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

            assert_eq!(decision.ticket, ticket);
            assert_eq!(decision.input_digest, expected_input_digest);
            assert_eq!(decision.taint, Taint::Clean);
            assert_eq!(decision.emergency_influence, InfluenceState::Inactive);
            assert_eq!(decision.compat_influence, InfluenceState::NotEvaluated);

            // Compare full returned record against independently computed expected.
            assert_eq!(
                decision.as_ref(),
                &expected,
                "returned decision must match full expected record"
            );
        }
        other => {
            return Err(
                format!("expected PolicyDenied with Suspended decision, got: {other:?}").into(),
            );
        }
    }

    // Real FjallRegister effect count is zero.
    assert_eq!(
        adapter.effect_count()?,
        0,
        "suspended policy must not produce any effect"
    );

    // Pending state unchanged after suspended policy rejection.
    let pending_after = runtime.pending_boundary_snapshot(8);
    assert_eq!(
        pending_after, pending_before,
        "suspended policy rejection must not alter pending state"
    );

    // Pending ticket unchanged — compare exact ticket identity.
    let ticket_after = runtime
        .lookup_pending_action_ticket(run, u64::from(StepIdx::ZERO.get()))
        .ok_or("expected pending action ticket after rejection")?;
    assert_eq!(
        Some(ticket_after),
        Some(ticket),
        "pending ticket must remain identical after suspended policy rejection"
    );

    // Exactly one stored decision event matching full expected record.
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
        1,
        "exactly one policy decision event must be stored"
    );
    assert_eq!(
        decision_events,
        vec![&expected],
        "stored decision event must exactly match expected full record"
    );

    Ok(())
}
