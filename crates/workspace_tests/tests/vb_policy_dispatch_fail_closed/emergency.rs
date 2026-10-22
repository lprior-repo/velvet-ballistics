use super::*;
use vb_core::action::ActionContract;
use vb_core::workflow::CompiledWorkflow;

type CaseResult<T> = Result<T, Box<dyn std::error::Error>>;

/// Typed case struct for emergency deny overlay fail-closed tests.
struct EmergencyTestCase {
    dir: tempfile::TempDir,
    run: RunId,
    action: ActionId,
    catalog: ExecutorCatalog,
    workflow: CompiledWorkflow,
    contract: ActionContract,
    journal: Arc<FjallJournal>,
    artifact: vb_storage::AcceptedArtifact,
    runtime: Runtime,
}

fn build_emergency_case() -> CaseResult<EmergencyTestCase> {
    let dir = tempfile::tempdir()?;
    let run = RunId::new(11_002);
    let action = ActionId::new(0);

    let catalog = ExecutorCatalog::run_registered_conformance()?;
    let workflow = action_then_finish_workflow()?;
    let contract = external_contract(action)?;

    let journal = Arc::new(FjallJournal::open(
        dir.path(),
        Some(FjallConfig::default()),
    )?);

    let pack = catalog.register_pack(&contract, scope(), RecoveryClass::ReconcileReceipt)?;
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
    let runtime = Runtime::new(shard_count(1)?, strict_config(), storage);

    Ok(EmergencyTestCase {
        dir,
        run,
        action,
        catalog,
        workflow,
        contract,
        journal,
        artifact,
        runtime,
    })
}

fn configure_allowed_policy(
    runtime: &mut Runtime,
    action: ActionId,
    digest: PolicyDigest,
) -> CaseResult<()> {
    runtime.configure_live_policy(
        digest,
        PolicyRule {
            action,
            code: DecisionCode::Allowed,
        },
    )?;
    Ok(())
}

fn submit_and_tick(
    runtime: &mut Runtime,
    run: RunId,
    workflow: CompiledWorkflow,
    action: ActionId,
    contract: ActionContract,
) -> CaseResult<()> {
    runtime.submit_direct_with_inputs_grants_and_contracts(
        run,
        workflow,
        Box::from([(SlotIdx::ZERO, SlotValue::I64(100))]),
        action_grants(action),
        Box::from([contract]),
    )?;
    runtime.tick_all()?;
    Ok(())
}

fn get_ticket_and_grant(
    runtime: &Runtime,
    catalog: &ExecutorCatalog,
    artifact: &vb_storage::AcceptedArtifact,
    run: RunId,
) -> CaseResult<(
    vb_core::action::ActionTicket,
    vb_storage::executor::RegisterGrant,
)> {
    let ticket = runtime
        .lookup_pending_action_ticket(run, u64::from(StepIdx::ZERO.get()))
        .ok_or("expected pending action ticket")?;
    let grant = catalog.issue_register_grant(artifact, ticket, scope())?;
    Ok((ticket, grant))
}

fn compute_expected_emergency_decision(
    artifact: &vb_storage::AcceptedArtifact,
    ticket: &vb_core::action::ActionTicket,
    current_live_digest: PolicyDigest,
) -> CaseResult<DecisionRecord> {
    let expected_input_digest = compute_expected_input_digest(100, Taint::Clean, ticket)?;
    let expected_admission = PolicyDigest::from_bytes(artifact.policy_digest.as_bytes());
    Ok(DecisionRecord {
        ticket: ticket.clone(),
        admission: PinnedAdmissionPolicy::Artifact(expected_admission),
        current_live: CurrentPolicyIdentity::Configured {
            digest: current_live_digest,
        },
        input_digest: expected_input_digest,
        taint: Taint::Clean,
        outcome: DecisionCode::Denied,
        layer: PolicyLayer::Emergency,
        emergency_influence: InfluenceState::Active,
        compat_influence: InfluenceState::NotEvaluated,
    })
}

fn execute_with_adapter(
    runtime: &mut Runtime,
    adapter: &mut FjallRegister,
    run: RunId,
    grant: &vb_storage::executor::RegisterGrant,
) -> Result<vb_storage::executor::RegisterReceipt, RegisteredExecutionError> {
    runtime.execute_registered_action(adapter, run, u64::from(StepIdx::ZERO.get()), grant)
}

fn assert_emergency_denied_full_record(
    result: Result<vb_storage::executor::RegisterReceipt, RegisteredExecutionError>,
    expected: &DecisionRecord,
) -> CaseResult<()> {
    let Err(RegisteredExecutionError::PolicyDenied(decision)) = result else {
        return Err(
            format!("expected PolicyDenied with Emergency decision, got: {result:?}").into(),
        );
    };
    assert_eq!(decision.outcome, DecisionCode::Denied);
    assert_eq!(decision.layer, PolicyLayer::Emergency);
    assert_eq!(decision.emergency_influence, InfluenceState::Active);
    assert_eq!(decision.compat_influence, InfluenceState::NotEvaluated);
    assert_eq!(decision.ticket, expected.ticket);
    assert_eq!(decision.input_digest, expected.input_digest);
    assert_eq!(decision.taint, Taint::Clean);
    assert_eq!(
        decision.as_ref(),
        expected,
        "returned decision must match full expected record"
    );
    Ok(())
}

fn assert_zero_effects(adapter: &FjallRegister) -> CaseResult<()> {
    assert_eq!(
        adapter.effect_count()?,
        0,
        "emergency deny must not produce any effect"
    );
    Ok(())
}

fn assert_pending_unchanged(
    runtime: &Runtime,
    run: RunId,
    pending_before: vb_runtime::runtime::RuntimePendingBoundarySnapshot,
    ticket: &vb_core::action::ActionTicket,
) -> CaseResult<()> {
    let pending_after = runtime.pending_boundary_snapshot(8);
    assert_eq!(
        pending_after, pending_before,
        "emergency deny must not alter pending state"
    );
    let ticket_after = runtime
        .lookup_pending_action_ticket(run, u64::from(StepIdx::ZERO.get()))
        .ok_or("expected pending action ticket after deny")?;
    assert_eq!(
        Some(ticket_after),
        Some(ticket.clone()),
        "pending ticket must remain identical after emergency deny"
    );
    Ok(())
}

fn assert_stored_emergency_decisions(
    journal: &Arc<FjallJournal>,
    run: RunId,
    expected_first: &DecisionRecord,
    expected_second: &DecisionRecord,
) -> CaseResult<()> {
    let events = vb_storage::read_run_events(journal, run)?;
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
        vec![expected_first, expected_second],
        "stored decision events must exactly match expected records in order"
    );
    Ok(())
}

fn run_first_emergency_attempt(
    case: &mut EmergencyTestCase,
    allowed_digest: PolicyDigest,
) -> CaseResult<DecisionRecord> {
    let (ticket, grant) =
        get_ticket_and_grant(&case.runtime, &case.catalog, &case.artifact, case.run)?;
    case.runtime.activate_emergency_deny()?;
    let expected = compute_expected_emergency_decision(&case.artifact, &ticket, allowed_digest)?;
    let pending_before = case.runtime.pending_boundary_snapshot(8);
    let register_path = case.dir.path().join("register");
    let mut adapter = FjallRegister::open(&register_path, 8)?;
    let result = execute_with_adapter(&mut case.runtime, &mut adapter, case.run, &grant);
    assert_emergency_denied_full_record(result, &expected)?;
    assert_zero_effects(&adapter)?;
    assert_pending_unchanged(&case.runtime, case.run, pending_before, &ticket)?;
    Ok(expected)
}

fn run_second_emergency_attempt(
    case: &mut EmergencyTestCase,
    later_allowed_digest: PolicyDigest,
) -> CaseResult<DecisionRecord> {
    configure_allowed_policy(&mut case.runtime, case.action, later_allowed_digest)?;
    let (ticket, grant) =
        get_ticket_and_grant(&case.runtime, &case.catalog, &case.artifact, case.run)?;
    let expected =
        compute_expected_emergency_decision(&case.artifact, &ticket, later_allowed_digest)?;
    let pending_before = case.runtime.pending_boundary_snapshot(8);
    let register_path = case.dir.path().join("register2");
    let mut adapter = FjallRegister::open(&register_path, 8)?;
    let result = execute_with_adapter(&mut case.runtime, &mut adapter, case.run, &grant);
    assert_emergency_denied_full_record(result, &expected)?;
    assert_zero_effects(&adapter)?;
    assert_pending_unchanged(&case.runtime, case.run, pending_before, &ticket)?;
    Ok(expected)
}

/// Test 2: Configure Allowed, submit, tick, activate emergency deny via
/// actual dedicated store persist, then execute. Expect typed Emergency
/// Denied decision with Active influence, zero effects, unchanged pending.
/// Show that configuring Allowed afterwards cannot loosen the overlay.
#[test]
fn emergency_deny_overlay_cannot_be_loosened_by_later_allowed_config() -> CaseResult<()> {
    let mut case = build_emergency_case()?;

    let allowed_digest = PolicyDigest::from_bytes([0x11u8; 32]);
    configure_allowed_policy(&mut case.runtime, case.action, allowed_digest)?;

    submit_and_tick(
        &mut case.runtime,
        case.run,
        case.workflow.clone(),
        case.action,
        case.contract.clone(),
    )?;

    let expected_first = run_first_emergency_attempt(&mut case, allowed_digest)?;

    let later_allowed_digest = PolicyDigest::from_bytes([0x33u8; 32]);
    let expected_second = run_second_emergency_attempt(&mut case, later_allowed_digest)?;

    assert_stored_emergency_decisions(&case.journal, case.run, &expected_first, &expected_second)?;

    Ok(())
}
