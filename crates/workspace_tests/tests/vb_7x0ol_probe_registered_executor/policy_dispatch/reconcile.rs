use super::*;
use vb_core::action_pack::ActionPack;
use vb_core::policy::DecisionRecord;
use vb_storage::AcceptedArtifact;
use vb_storage::executor::{RegisterGrant, RegisterReceipt};

type CaseResult<T> = Result<T, Box<dyn std::error::Error>>;

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

fn build_test_case() -> CaseResult<TestCase> {
    let dir = tempfile::tempdir()?;
    let run = RunId::new(9_104);
    let action = ActionId::new(0);
    let (catalog, pack) = create_catalog_and_pack(action)?;
    let (journal, artifact) = create_journal_and_artifact(&dir, &catalog, &pack)?;
    let mut runtime = create_and_submit_runtime(&journal, &catalog, run)?;
    let allowed_digest = PolicyDigest::from_bytes([0x44u8; 32]);
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
        Box::from([(SlotIdx::ZERO, SlotValue::I64(55))]),
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

fn execute_allowed(case: &mut TestCase) -> CaseResult<RegisterReceipt> {
    let register_path = case.dir.path().join("register");
    let mut adapter = FjallRegister::open(&register_path, 8)?;
    let receipt = case.runtime.execute_registered_action(
        &mut adapter,
        case.run,
        u64::from(StepIdx::ZERO.get()),
        &case.grant,
    )?;
    assert_eq!(adapter.effect_count()?, 1);
    Ok(receipt)
}

fn build_expected_allowed_decision(case: &TestCase) -> CaseResult<DecisionRecord> {
    let expected_admission = admission_pin_from_artifact(case.artifact.policy_digest.as_bytes());
    let expected_input_digest = compute_expected_input_digest(55, Taint::Clean, &case.ticket)?;
    let expected = make_expected_decision(
        case.ticket,
        expected_admission,
        current_live_configured(case.allowed_digest.as_bytes()),
        expected_input_digest,
        DecisionCode::Allowed,
    );
    Ok(expected)
}

fn assert_allowed_decision_stored(case: &TestCase, expected: &DecisionRecord) -> CaseResult<()> {
    let decisions = read_journal_decision_events(&case.journal, case.run)?;
    assert_eq!(
        decisions.len(),
        1,
        "exactly one allowed decision must be stored before reconcile"
    );
    let stored = decisions.first().ok_or("expected exactly one decision")?;
    assert_eq!(
        stored, expected,
        "stored allowed DecisionRecord must match oracle"
    );
    Ok(())
}

fn change_policy_to_deny(case: &mut TestCase) -> CaseResult<()> {
    let denied_digest = PolicyDigest::from_bytes([0x55u8; 32]);
    case.runtime.configure_live_policy(
        denied_digest,
        PolicyRule {
            action: ActionId::new(0),
            code: DecisionCode::Denied,
        },
    )?;
    Ok(())
}

fn reconcile_action(case: &mut TestCase) -> CaseResult<RegisterReceipt> {
    let register_path = case.dir.path().join("register");
    let mut adapter = FjallRegister::open(&register_path, 8)?;
    let reconciled = case.runtime.reconcile_registered_action(
        &mut adapter,
        case.run,
        u64::from(StepIdx::ZERO.get()),
        &case.grant,
    )?;
    Ok(reconciled)
}

fn assert_reconciled_receipt(original: &RegisterReceipt, reconciled: &RegisterReceipt) {
    assert_eq!(
        reconciled, original,
        "reconciled receipt must exactly match original"
    );
    assert_eq!(reconciled.value(), 55);
}

fn assert_adapter_effects(case: &TestCase) -> CaseResult<()> {
    let register_path = case.dir.path().join("register");
    let adapter = FjallRegister::open(&register_path, 8)?;
    assert_eq!(adapter.effect_count()?, 1);
    Ok(())
}

fn assert_no_new_decisions(case: &TestCase, expected: &DecisionRecord) -> CaseResult<()> {
    let decisions = read_journal_decision_events(&case.journal, case.run)?;
    assert_eq!(
        decisions.len(),
        1,
        "reconcile must not add a new policy decision"
    );
    let stored = decisions.first().ok_or("expected exactly one decision")?;
    assert_eq!(
        stored, expected,
        "stored decision list must be exactly the same before/after reconcile"
    );
    Ok(())
}

#[test]
fn reconcile_ignores_policy_change_after_effect() -> TestResult {
    let mut case = build_test_case()?;
    let expected = build_expected_allowed_decision(&case)?;
    let first = execute_allowed(&mut case)?;
    assert_allowed_decision_stored(&case, &expected)?;
    change_policy_to_deny(&mut case)?;
    let reconciled = reconcile_action(&mut case)?;
    assert_reconciled_receipt(&first, &reconciled);
    assert_adapter_effects(&case)?;
    assert_no_new_decisions(&case, &expected)?;
    Ok(())
}
