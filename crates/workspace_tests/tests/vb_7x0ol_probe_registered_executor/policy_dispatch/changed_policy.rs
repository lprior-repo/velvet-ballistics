use super::*;
use vb_core::action_pack::ActionPack;
use vb_core::policy::DecisionRecord;
use vb_storage::AcceptedArtifact;
use vb_storage::executor::RegisterGrant;

type CaseResult<T> = Result<T, Box<dyn std::error::Error>>;

struct TestCase {
    dir: tempfile::TempDir,
    run: RunId,
    journal: Arc<FjallJournal>,
    artifact: AcceptedArtifact,
    runtime: Runtime,
    ticket: ActionTicket,
    grant: RegisterGrant,
    denied_digest: PolicyDigest,
}

fn build_test_case() -> CaseResult<TestCase> {
    let dir = tempfile::tempdir()?;
    let run = RunId::new(9_102);
    let action = ActionId::new(0);
    let (catalog, pack) = create_catalog_and_pack(action)?;
    let (journal, artifact) = create_journal_and_artifact(&dir, &catalog, &pack)?;
    let mut runtime = create_and_submit_runtime(&journal, &catalog, run)?;
    let (ticket, grant) = get_ticket_and_grant(&catalog, &artifact, &runtime, run)?;
    let denied_digest = PolicyDigest::from_bytes([0x22u8; 32]);
    runtime.configure_live_policy(
        denied_digest,
        PolicyRule {
            action,
            code: DecisionCode::Denied,
        },
    )?;
    Ok(TestCase {
        dir,
        run,
        journal,
        artifact,
        runtime,
        ticket,
        grant,
        denied_digest,
    })
}

fn create_catalog_and_pack(action: ActionId) -> CaseResult<(ExecutorCatalog, ActionPack)> {
    let contract = external_contract(action)?;
    let catalog = ExecutorCatalog::run_registered_conformance()?;
    let pack = catalog.register_pack(&contract, scope(), RecoveryClass::ReconcileReceipt)?;
    Ok((catalog, pack))
}

fn create_journal_and_artifact(
    dir: &tempfile::TempDir,
    catalog: &ExecutorCatalog,
    pack: &ActionPack,
) -> CaseResult<(Arc<FjallJournal>, AcceptedArtifact)> {
    let workflow = action_then_finish_workflow()?;
    let contract = external_contract(ActionId::new(0))?;
    let journal = Arc::new(FjallJournal::open(
        dir.path(),
        Some(FjallConfig::default()),
    )?);
    let artifact = catalog.submit_artifact(
        &journal,
        &workflow,
        RuntimePolicy::Strict,
        &[contract],
        &[pack.clone()],
    )?;
    Ok((journal, artifact))
}

fn create_and_submit_runtime(
    journal: &Arc<FjallJournal>,
    catalog: &ExecutorCatalog,
    run: RunId,
) -> CaseResult<Runtime> {
    let storage = StorageRuntimeJournal::shared_strict_with_catalog(
        journal.clone(),
        Arc::new(catalog.clone()),
    );
    let mut runtime = Runtime::new(shard_count(1)?, strict_config(), storage);
    let workflow = action_then_finish_workflow()?;
    let contract = external_contract(ActionId::new(0))?;
    runtime.submit_direct_with_inputs_grants_and_contracts(
        run,
        workflow,
        Box::from([(SlotIdx::ZERO, SlotValue::I64(99))]),
        action_grants(ActionId::new(0)),
        Box::from([contract]),
    )?;
    runtime.tick_all()?;
    Ok(runtime)
}

fn get_ticket_and_grant(
    catalog: &ExecutorCatalog,
    artifact: &AcceptedArtifact,
    runtime: &Runtime,
    run: RunId,
) -> CaseResult<(ActionTicket, RegisterGrant)> {
    let ticket = runtime
        .lookup_pending_action_ticket(run, u64::from(StepIdx::ZERO.get()))
        .ok_or("expected pending action ticket")?;
    let grant = catalog.issue_register_grant(artifact, ticket, scope())?;
    Ok((ticket, grant))
}

fn execute_deny(case: &mut TestCase) -> CaseResult<DecisionRecord> {
    let register_path = case.dir.path().join("register");
    let mut adapter = FjallRegister::open(&register_path, 8)?;
    let result = case.runtime.execute_registered_action(
        &mut adapter,
        case.run,
        u64::from(StepIdx::ZERO.get()),
        &case.grant,
    );
    match result {
        Err(RegisteredExecutionError::PolicyDenied(decision)) => Ok(*decision),
        other => Err(format!("expected PolicyDenied, got: {other:?}").into()),
    }
}

fn build_expected_decision(case: &TestCase) -> CaseResult<DecisionRecord> {
    let expected_admission = admission_pin_from_artifact(case.artifact.policy_digest.as_bytes());
    let expected_input_digest = compute_expected_input_digest(99, Taint::Clean, &case.ticket)?;
    let expected_decision = make_expected_decision(
        case.ticket,
        expected_admission,
        current_live_configured(case.denied_digest.as_bytes()),
        expected_input_digest,
        DecisionCode::Denied,
    );
    Ok(expected_decision)
}

fn assert_decision(
    case: &TestCase,
    decision: &DecisionRecord,
    expected: &DecisionRecord,
    allowed_digest: PolicyDigest,
) {
    assert_eq!(
        decision, expected,
        "full DecisionRecord must match oracle for policy-change deny"
    );
    match decision.admission {
        PinnedAdmissionPolicy::Artifact(admitted) => {
            assert_ne!(
                admitted, case.denied_digest,
                "admission must differ from current denied"
            );
            assert_ne!(
                admitted, allowed_digest,
                "admission must differ from original allowed"
            );
        }
        _ => assert!(false, "admission checked in DecisionRecord equality"),
    }
}

fn assert_adapter_effects(case: &TestCase) -> CaseResult<()> {
    let register_path = case.dir.path().join("register");
    let adapter = FjallRegister::open(&register_path, 8)?;
    assert_eq!(adapter.effect_count()?, 0);
    assert_eq!(
        adapter.value(scope())?,
        None,
        "target value must remain unset after policy denial"
    );
    Ok(())
}

fn assert_stored_decisions(case: &TestCase, expected: &DecisionRecord) -> CaseResult<()> {
    let stored_decisions = read_journal_decision_events(&case.journal, case.run)?;
    assert_eq!(
        stored_decisions.len(),
        1,
        "exactly one policy decision must be stored"
    );
    let stored = stored_decisions
        .first()
        .ok_or("expected exactly one decision")?;
    assert_eq!(
        stored, expected,
        "stored DecisionRecord must match oracle for policy-change deny"
    );
    Ok(())
}

#[test]
fn after_scheduling_policy_change_to_deny() -> TestResult {
    let mut case = build_test_case()?;
    let expected = build_expected_decision(&case)?;
    let decision = execute_deny(&mut case)?;
    let allowed_digest = PolicyDigest::from_bytes([0x11u8; 32]);
    assert_decision(&case, &decision, &expected, allowed_digest);
    assert_adapter_effects(&case)?;
    assert_stored_decisions(&case, &expected)?;
    Ok(())
}
