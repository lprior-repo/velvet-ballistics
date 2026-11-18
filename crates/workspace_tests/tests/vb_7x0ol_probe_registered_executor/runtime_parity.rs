//! Runtime parity checks for the registered executor: duplicate dispatch and
//! cold journal reopen must both resolve to one durable effect and one receipt.

use super::*;

type FixtureResult = Result<Fixture, Box<dyn std::error::Error>>;

struct Fixture {
    dir: tempfile::TempDir,
    run: RunId,
    catalog: ExecutorCatalog,
    artifact: vb_storage::AcceptedArtifact,
    grant: vb_storage::executor::RegisterGrant,
    scoped: ScopedGrant,
    dispatch: GrantDispatchContext,
    register_path: std::path::PathBuf,
    runtime: Runtime,
    adapter: FjallRegister,
}

fn build_fixture(run_id: u64, input: i64) -> FixtureResult {
    let dir = tempfile::tempdir()?;
    let run = RunId::new(run_id);
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
    let storage =
        StorageRuntimeJournal::shared_strict_with_catalog(journal, Arc::new(catalog.clone()));
    let mut runtime = Runtime::new(shard_count(1)?, strict_config(), storage);
    runtime.configure_live_policy(
        PolicyDigest::from_bytes([0x5au8; 32]),
        PolicyRule {
            action,
            code: DecisionCode::Allowed,
        },
    )?;
    runtime.submit_direct_with_inputs_grants_and_contracts(
        run,
        workflow,
        Box::from([(SlotIdx::ZERO, SlotValue::I64(input))]),
        action_grants(action),
        Box::from([contract]),
    )?;
    runtime.tick_all()?;
    let ticket = runtime
        .lookup_pending_action_ticket(run, u64::from(StepIdx::ZERO.get()))
        .ok_or("expected pending action ticket")?;
    let grant = catalog.issue_register_grant(&artifact, ticket, scope())?;
    let (scoped, dispatch) =
        scoped_grant_fixture::issue_scoped(&catalog, &artifact, &grant, ticket, scope(), 30)?;
    let register_path = dir.path().join("register");
    let adapter = FjallRegister::open(&register_path, 8)?;
    Ok(Fixture {
        dir,
        run,
        catalog,
        artifact,
        grant,
        scoped,
        dispatch,
        register_path,
        runtime,
        adapter,
    })
}

impl Fixture {
    fn execute(
        &mut self,
    ) -> Result<vb_storage::executor::RegisterReceipt, Box<dyn std::error::Error>> {
        Ok(self.runtime.execute_registered_action(
            &mut self.adapter,
            self.run,
            u64::from(StepIdx::ZERO.get()),
            &self.grant,
            vb_core::ScopedDispatch::new(&self.scoped, &self.dispatch, &[]),
        )?)
    }
}

#[test]
fn duplicate_runtime_dispatch_returns_same_receipt_and_one_effect() -> TestResult {
    let mut fixture = build_fixture(8_201, 42)?;
    let first = fixture.execute()?;
    let duplicate = fixture.execute()?;
    assert_eq!(duplicate, first);
    assert_eq!(fixture.adapter.effect_count()?, 1);
    assert_eq!(fixture.adapter.value(scope())?, Some(42));
    fixture.runtime.tick_all()?;
    assert!(
        fixture
            .runtime
            .lookup_pending_action_ticket(fixture.run, u64::from(StepIdx::ZERO.get()))
            .is_none()
    );
    assert_eq!(fixture.adapter.effect_count()?, 1);
    Ok(())
}

#[test]
fn cold_reopen_reads_durable_receipt_without_new_effect() -> TestResult {
    let mut fixture = build_fixture(8_202, 42)?;
    let first = fixture.execute()?;
    assert_eq!(fixture.adapter.effect_count()?, 1);
    let Fixture {
        dir,
        run,
        catalog,
        artifact,
        register_path,
        runtime,
        adapter,
        ..
    } = fixture;
    drop(runtime);
    drop(adapter);
    let journal = Arc::new(FjallJournal::open(
        dir.path(),
        Some(FjallConfig::default()),
    )?);
    let storage =
        StorageRuntimeJournal::shared_strict_with_catalog(journal, Arc::new(catalog.clone()));
    let mut runtime = Runtime::new(shard_count(1)?, strict_config(), storage);
    runtime.recover_and_resume(run)?;
    runtime.tick_all()?;
    let ticket = runtime
        .lookup_pending_action_ticket(run, u64::from(StepIdx::ZERO.get()))
        .ok_or("expected pending action after cold recovery")?;
    assert!(
        runtime.inspect_run_state(run)?.admission.is_some(),
        "cold recovery must restore the admitted artifact pin"
    );
    let grant = catalog.issue_register_grant(&artifact, ticket, scope())?;
    let mut adapter = FjallRegister::open(&register_path, 8)?;
    let reconciled = adapter.reconcile(&grant, ticket, 42)?;
    assert_eq!(reconciled, first);
    assert_eq!(adapter.effect_count()?, 1);
    assert_eq!(adapter.value(scope())?, Some(42));
    Ok(())
}
