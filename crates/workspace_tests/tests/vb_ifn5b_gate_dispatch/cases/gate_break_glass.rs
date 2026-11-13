use super::*;

#[test]
fn matching_break_glass_receipt_authorizes_one_registered_effect() -> TestResult {
    let dir = storage_dir()?;
    let run = RunId::new(11_703);
    let action = ActionId::new(1);
    let workflow = workflow().map_err(|error| format!("workflow: {error}"))?;
    let contract = contract()?;
    let preceding = preceding_contract()?;
    let contracts = [preceding, contract.clone()];
    let catalog = ExecutorCatalog::run_registered_conformance()?;
    let pack = catalog.register_pack(&contract, scope(), RecoveryClass::ReconcileReceipt)?;
    let journal = Arc::new(FjallJournal::open(&dir, Some(FjallConfig::default()))?);
    let artifact = catalog.submit_artifact(
        &journal,
        &workflow,
        RuntimePolicy::Strict,
        &contracts,
        &[pack],
    )?;
    let source_digest =
        super::owned_submission::commit(&journal, &catalog, &workflow, &artifact, run)?;
    let storage = StorageRuntimeJournal::shared_strict_with_catalog(
        journal.clone(),
        Arc::new(catalog.clone()),
    );
    let shard_count = std::num::NonZeroUsize::new(1).ok_or("shard count")?;
    let mut config = crate::common::runtime_fixture::strict_config();
    config.step_budget_per_tick = 1;
    let mut runtime = Runtime::new(shard_count, config, storage);
    runtime.configure_live_policy(
        PolicyDigest::from_bytes([4; 32]),
        PolicyRule {
            action,
            code: DecisionCode::Allowed,
        },
    )?;
    runtime
        .activate_owned_submission(run, source_digest)
        .map_err(|error| format!("activate owned run: {error:?}"))?;
    runtime.tick_all()?;
    let approval = approval_for(
        &mut runtime,
        &journal,
        run,
        &artifact,
        GateId::new(72),
        GateDecision::BreakGlass,
    )?;
    let ticket = pending_ticket(&mut runtime, run)?;
    let register = catalog.issue_register_grant(&artifact, ticket, scope())?;
    let params = vb_core::ScopedGrantParams {
        id: vb_core::GrantId::new(72),
        issuer: person("grant-authority")?,
        subject: person("run-operator")?,
        capability: Capability::new("velvet.register.write".into(), action),
        scope: scope(),
        issued_at: vb_core::GrantTimeMs::new(150),
        expires_at: vb_core::GrantTimeMs::new(1_000),
        delegable: false,
        delegated_from: None,
        revocation_epoch: vb_core::RevocationEpoch::new(1),
        break_glass: true,
        approval_receipt: approval.digest,
        signature: None,
    };
    let mismatched_params = vb_core::ScopedGrantParams {
        break_glass: false,
        ..params.clone()
    };
    let mismatched = catalog.issue_scoped_grant_for_bound_gate(
        &artifact,
        &register,
        mismatched_params,
        &approval,
    );
    assert!(matches!(
        mismatched,
        Err(ExecutorError::Grant(
            vb_core::GrantError::GateReceiptMismatch
        ))
    ));
    let scoped =
        catalog.issue_scoped_grant_for_bound_gate(&artifact, &register, params, &approval)?;
    let dispatch = vb_core::GrantDispatchContext::new(
        artifact.digest,
        ticket,
        scope(),
        person("grant-authority")?,
        person("run-operator")?,
        vb_core::GrantTimeMs::new(250),
        vb_core::RevocationEpoch::new(1),
    );
    let mut adapter = FjallRegister::open(&dir.join("register"), 8)?;
    let receipt = runtime.execute_registered_action(
        &mut adapter,
        run,
        u64::from(ticket.step.get()),
        &register,
        vb_core::ScopedDispatch::new(&scoped, &dispatch, &[]).with_gate_approval(&approval),
    )?;
    assert_eq!(receipt.value(), 42);
    assert_eq!(adapter.effect_count()?, 1);
    assert_eq!(adapter.value(scope())?, Some(42));
    Ok(())
}
