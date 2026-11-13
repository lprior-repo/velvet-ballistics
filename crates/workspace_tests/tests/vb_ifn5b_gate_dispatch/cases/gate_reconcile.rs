use super::*;

pub(super) struct ColdReconcileProbe {
    pub runtime: Runtime,
    pub journal: Arc<FjallJournal>,
    pub catalog: ExecutorCatalog,
    pub artifact: vb_storage::AcceptedArtifact,
    pub dir: std::path::PathBuf,
    pub run: RunId,
}

pub(super) fn recover_and_reconcile(probe: ColdReconcileProbe) -> TestResult {
    drop(probe.runtime);
    let storage = StorageRuntimeJournal::shared_strict_with_catalog(
        probe.journal.clone(),
        Arc::new(probe.catalog.clone()),
    );
    let shard_count = std::num::NonZeroUsize::new(1).ok_or("shard count")?;
    let mut runtime = Runtime::new(
        shard_count,
        crate::common::runtime_fixture::strict_config(),
        storage,
    );
    runtime.recover_and_resume(probe.run)?;
    runtime.tick_all()?;
    let ticket = super::pending_ticket(&mut runtime, probe.run)?;
    let approval = probe
        .journal
        .events_for_run_full(probe.run)?
        .iter()
        .find_map(|event| match event {
            JournalEvent::GateDecisionBoundEvent { record, .. } => record.bound_receipt.clone(),
            _ => None,
        })
        .ok_or("durable gate receipt missing after recovery")?;
    let register = probe
        .catalog
        .issue_register_grant(&probe.artifact, ticket, scope())?;
    let params = vb_core::ScopedGrantParams {
        mode: vb_core::GrantAdmissionMode::StrictExact,
        id: vb_core::GrantId::new(73),
        issuer: person("grant-authority")?,
        subject: person("run-operator")?,
        capability: Capability::new("velvet.register.write".into(), ticket.action),
        scope: scope(),
        issued_at: vb_core::GrantTimeMs::new(150),
        expires_at: vb_core::GrantTimeMs::new(1_000),
        delegable: false,
        delegated_from: None,
        revocation_epoch: vb_core::RevocationEpoch::new(1),
        break_glass: false,
        approval_receipt: approval.digest,
        signature: None,
    };
    let scoped = probe.catalog.issue_scoped_grant_for_bound_gate(
        &probe.artifact,
        &register,
        params,
        &approval,
    )?;
    let dispatch = vb_core::GrantDispatchContext::new(
        probe.artifact.digest,
        ticket,
        scope(),
        person("grant-authority")?,
        person("run-operator")?,
        vb_core::GrantTimeMs::new(250),
        vb_core::RevocationEpoch::new(1),
        false,
    );
    let mut adapter = FjallRegister::open(&probe.dir.join("register"), 8)?;
    let receipt = runtime.reconcile_registered_action(
        &mut adapter,
        probe.run,
        u64::from(ticket.step.get()),
        &register,
        &scoped,
        &dispatch,
    )?;
    assert_eq!(receipt.value(), 42);
    assert_eq!(adapter.effect_count()?, 1);
    Ok(())
}
