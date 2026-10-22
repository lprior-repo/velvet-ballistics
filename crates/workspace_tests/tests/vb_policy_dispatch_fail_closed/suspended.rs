use super::*;
use vb_core::action::ActionContract;
use vb_core::workflow::CompiledWorkflow;

type CaseResult<T> = Result<T, Box<dyn std::error::Error>>;

/// Typed case struct for suspended policy fail-closed tests.
struct SuspendedTestCase {
    dir: tempfile::TempDir,
    run: RunId,
    action: ActionId,
    catalog: ExecutorCatalog,
    workflow: CompiledWorkflow,
    contract: ActionContract,
    journal: Arc<FjallJournal>,
    artifact: vb_storage::AcceptedArtifact,
    runtime: Runtime,
    suspended_digest: PolicyDigest,
}

fn submit_case_artifact(
    journal: &Arc<FjallJournal>,
    catalog: &ExecutorCatalog,
    workflow: &CompiledWorkflow,
    contract: &ActionContract,
) -> CaseResult<vb_storage::AcceptedArtifact> {
    let pack = catalog.register_pack(contract, scope(), RecoveryClass::ReconcileReceipt)?;
    Ok(catalog.submit_artifact(
        journal,
        workflow,
        RuntimePolicy::Strict,
        &[contract.clone()],
        &[pack],
    )?)
}

fn make_case_runtime(
    journal: &Arc<FjallJournal>,
    catalog: &ExecutorCatalog,
) -> CaseResult<Runtime> {
    let storage = StorageRuntimeJournal::shared_strict_with_catalog(
        journal.clone(),
        Arc::new(catalog.clone()),
    );
    Ok(Runtime::new(shard_count(1)?, strict_config(), storage))
}

fn build_suspended_case() -> CaseResult<SuspendedTestCase> {
    let dir = tempfile::tempdir()?;
    let run = RunId::new(11_001);
    let action = ActionId::new(0);

    let catalog = ExecutorCatalog::run_registered_conformance()?;
    let workflow = action_then_finish_workflow()?;
    let contract = external_contract(action)?;

    let journal = Arc::new(FjallJournal::open(
        dir.path(),
        Some(FjallConfig::default()),
    )?);
    let artifact = submit_case_artifact(&journal, &catalog, &workflow, &contract)?;
    let runtime = make_case_runtime(&journal, &catalog)?;

    Ok(SuspendedTestCase {
        dir,
        run,
        action,
        catalog,
        workflow,
        contract,
        journal,
        artifact,
        runtime,
        suspended_digest: PolicyDigest::from_bytes([0x99u8; 32]),
    })
}

fn configure_suspended_policy(
    runtime: &mut Runtime,
    action: ActionId,
    digest: PolicyDigest,
) -> CaseResult<()> {
    runtime.configure_live_policy(
        digest,
        PolicyRule {
            action,
            code: DecisionCode::Suspended,
        },
    )?;
    Ok(())
}

fn submit_and_tick(
    runtime: &mut Runtime,
    run: RunId,
    workflow: CompiledWorkflow,
    contract: ActionContract,
) -> CaseResult<()> {
    runtime.submit_direct_with_inputs_grants_and_contracts(
        run,
        workflow,
        Box::from([(SlotIdx::ZERO, SlotValue::I64(42))]),
        action_grants(ActionId::new(0)),
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

fn compute_expected_decision(
    artifact: &vb_storage::AcceptedArtifact,
    ticket: &vb_core::action::ActionTicket,
    suspended_digest: PolicyDigest,
) -> CaseResult<DecisionRecord> {
    let expected_input_digest = compute_expected_input_digest(42, Taint::Clean, ticket)?;
    let expected_admission = PolicyDigest::from_bytes(artifact.policy_digest.as_bytes());
    let expected = DecisionRecord {
        ticket: ticket.clone(),
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
    Ok(expected)
}

fn execute_with_adapter(
    runtime: &mut Runtime,
    adapter: &mut FjallRegister,
    run: RunId,
    grant: &vb_storage::executor::RegisterGrant,
) -> Result<vb_storage::executor::RegisterReceipt, RegisteredExecutionError> {
    runtime.execute_registered_action(adapter, run, u64::from(StepIdx::ZERO.get()), grant)
}

fn assert_policy_denied_full_record(
    result: Result<vb_storage::executor::RegisterReceipt, RegisteredExecutionError>,
    expected: &DecisionRecord,
    suspended_digest: PolicyDigest,
) -> CaseResult<()> {
    let Err(RegisteredExecutionError::PolicyDenied(decision)) = result else {
        return Err(
            format!("expected PolicyDenied with Suspended decision, got: {result:?}").into(),
        );
    };
    assert_eq!(decision.outcome, DecisionCode::Suspended);
    assert_eq!(decision.layer, PolicyLayer::Live);
    match decision.current_live {
        CurrentPolicyIdentity::Configured { digest } => assert_eq!(digest, suspended_digest),
        other => return Err(format!("expected Configured live policy, got: {other:?}").into()),
    }
    assert_eq!(decision.ticket, expected.ticket);
    assert_eq!(decision.input_digest, expected.input_digest);
    assert_eq!(decision.taint, Taint::Clean);
    assert_eq!(decision.emergency_influence, InfluenceState::Inactive);
    assert_eq!(decision.compat_influence, InfluenceState::NotEvaluated);
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
        "suspended policy must not produce any effect"
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
        "suspended policy rejection must not alter pending state"
    );

    let ticket_after = runtime
        .lookup_pending_action_ticket(run, u64::from(StepIdx::ZERO.get()))
        .ok_or("expected pending action ticket after rejection")?;
    assert_eq!(
        Some(ticket_after),
        Some(ticket.clone()),
        "pending ticket must remain identical after suspended policy rejection"
    );
    Ok(())
}

fn assert_stored_decision(
    journal: &Arc<FjallJournal>,
    run: RunId,
    expected: &DecisionRecord,
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
        1,
        "exactly one policy decision event must be stored"
    );
    assert_eq!(
        decision_events,
        vec![expected],
        "stored decision event must exactly match expected full record"
    );
    Ok(())
}

/// Test 1: Actual current Suspended policy before execute on a genuine
/// accepted artifact / pending action. Expect typed PolicyDenied with
/// full 9-field DecisionRecord, zero effects, unchanged pending state.
#[test]
fn suspended_policy_denies_execution_with_full_record() -> CaseResult<()> {
    let mut case = build_suspended_case()?;
    configure_suspended_policy(&mut case.runtime, case.action, case.suspended_digest)?;
    submit_and_tick(
        &mut case.runtime,
        case.run,
        case.workflow.clone(),
        case.contract.clone(),
    )?;

    let (ticket, grant) =
        get_ticket_and_grant(&case.runtime, &case.catalog, &case.artifact, case.run)?;

    let expected = compute_expected_decision(&case.artifact, &ticket, case.suspended_digest)?;

    let pending_before = case.runtime.pending_boundary_snapshot(8);

    let register_path = case.dir.path().join("register");
    let mut adapter = FjallRegister::open(&register_path, 8)?;
    let result = execute_with_adapter(&mut case.runtime, &mut adapter, case.run, &grant);

    assert_policy_denied_full_record(result, &expected, case.suspended_digest)?;
    assert_zero_effects(&adapter)?;
    assert_pending_unchanged(&case.runtime, case.run, pending_before, &ticket)?;
    assert_stored_decision(&case.journal, case.run, &expected)?;

    Ok(())
}
