use super::*;
use vb_core::action_pack::ActionPack;
use vb_core::policy::DecisionRecord;
use vb_storage::AcceptedArtifact;
use vb_storage::executor::{RegisterGrant, RegisterReceipt};

struct TestCase {
    dir: tempfile::TempDir,
    run: RunId,
    journal: Arc<FjallJournal>,
    artifact: AcceptedArtifact,
    runtime: Runtime,
    ticket: ActionTicket,
    grant: RegisterGrant,
    allowed_digest: PolicyDigest,
}

fn build_test_case() -> Result<TestCase, Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let run = RunId::new(9_103);
    let action = ActionId::new(0);
    let (catalog, pack) = create_catalog_and_pack(action)?;
    let (journal, artifact) = create_journal_and_artifact(&dir, &catalog, &pack)?;
    let mut runtime = create_and_submit_runtime(&journal, &catalog, run)?;
    let allowed_digest = PolicyDigest::from_bytes([0x33u8; 32]);
    runtime.configure_live_policy(
        allowed_digest,
        PolicyRule {
            action,
            code: DecisionCode::Allowed,
        },
    )?;
    let (ticket, grant) = get_ticket_and_grant(&catalog, &artifact, &runtime, run)?;
    Ok(TestCase {
        dir,
        run,
        journal,
        artifact,
        runtime,
        ticket,
        grant,
        allowed_digest,
    })
}

fn create_catalog_and_pack(
    action: ActionId,
) -> Result<(ExecutorCatalog, ActionPack), Box<dyn std::error::Error>> {
    let contract = external_contract(action)?;
    let catalog = ExecutorCatalog::run_registered_conformance()?;
    let pack = catalog.register_pack(&contract, scope(), RecoveryClass::ReconcileReceipt)?;
    Ok((catalog, pack))
}

fn create_journal_and_artifact(
    dir: &tempfile::TempDir,
    catalog: &ExecutorCatalog,
    pack: &ActionPack,
) -> Result<(Arc<FjallJournal>, AcceptedArtifact), Box<dyn std::error::Error>> {
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
) -> Result<Runtime, Box<dyn std::error::Error>> {
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
        Box::from([(SlotIdx::ZERO, SlotValue::I64(100))]),
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
) -> Result<(ActionTicket, RegisterGrant), Box<dyn std::error::Error>> {
    let ticket = runtime
        .lookup_pending_action_ticket(run, u64::from(StepIdx::ZERO.get()))
        .ok_or("expected pending action ticket")?;
    let grant = catalog.issue_register_grant(artifact, ticket, scope())?;
    Ok((ticket, grant))
}

fn execute_action(case: &mut TestCase) -> Result<RegisterReceipt, Box<dyn std::error::Error>> {
    let register_path = case.dir.path().join("register");
    let mut adapter = FjallRegister::open(&register_path, 8)?;
    let receipt = case.runtime.execute_registered_action(
        &mut adapter,
        case.run,
        u64::from(StepIdx::ZERO.get()),
        &case.grant,
    )?;
    Ok(receipt)
}

fn build_expected_decision(case: &TestCase) -> Result<DecisionRecord, Box<dyn std::error::Error>> {
    let expected_admission = admission_pin_from_artifact(case.artifact.policy_digest.as_bytes());
    let expected_input_digest = compute_expected_input_digest(100, Taint::Clean, &case.ticket)?;
    let expected_decision = make_expected_decision(
        case.ticket,
        expected_admission,
        current_live_configured(case.allowed_digest.as_bytes()),
        expected_input_digest,
        DecisionCode::Allowed,
    );
    Ok(expected_decision)
}

fn assert_receipt(receipt: &RegisterReceipt) {
    assert_eq!(receipt.value(), 100);
}

fn assert_adapter_effects(case: &TestCase) -> Result<(), Box<dyn std::error::Error>> {
    let register_path = case.dir.path().join("register");
    let adapter = FjallRegister::open(&register_path, 8)?;
    assert_eq!(adapter.effect_count()?, 1);
    assert_eq!(adapter.value(scope())?, Some(100));
    Ok(())
}

fn assert_stored_decisions(
    case: &TestCase,
    expected: &DecisionRecord,
) -> Result<(), Box<dyn std::error::Error>> {
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
        "full stored DecisionRecord must match oracle for allowed policy"
    );
    assert_eq!(stored.ticket.run, case.run);
    assert_eq!(stored.ticket, case.ticket);
    Ok(())
}

#[test]
fn allowed_policy_permits_effect_and_records_decision() -> TestResult {
    let mut case = build_test_case()?;
    let expected = build_expected_decision(&case)?;
    let receipt = execute_action(&mut case)?;
    assert_receipt(&receipt);
    assert_adapter_effects(&case)?;
    assert_stored_decisions(&case, &expected)?;
    Ok(())
}
