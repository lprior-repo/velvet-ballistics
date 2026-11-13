use super::*;

#[test]
fn gate_dispatch_requires_immutable_origin_and_durable_receipt() -> TestResult {
    let dir = storage_dir()?;
    let run = RunId::new(11_701);
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
        GateId::new(71),
        GateDecision::Approve,
    )?;
    let ticket = pending_ticket(&mut runtime, run)?;
    let register_grant = catalog.issue_register_grant(&artifact, ticket, scope())?;
    let params = vb_core::ScopedGrantParams {
        id: vb_core::GrantId::new(71),
        issuer: person("grant-authority")?,
        subject: person("run-operator")?,
        capability: Capability::new("velvet.register.write".into(), action),
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
    let bound = catalog.issue_scoped_grant_for_bound_gate(
        &artifact,
        &register_grant,
        params.clone(),
        &approval,
    )?;
    let dispatch = vb_core::GrantDispatchContext::new(
        artifact.digest,
        ticket,
        scope(),
        person("grant-authority")?,
        person("run-operator")?,
        vb_core::GrantTimeMs::new(250),
        vb_core::RevocationEpoch::new(1),
    );
    let register_path = dir.join("register");
    let mut adapter = FjallRegister::open(&register_path, 8)?;
    let events_before = super::DispatchSnapshot {
        run,
        ticket,
        events: journal.events_for_run_full(run)?,
    };
    let result = runtime.execute_registered_action(
        &mut adapter,
        run,
        u64::from(ticket.step.get()),
        &register_grant,
        vb_core::ScopedDispatch::new(&bound, &dispatch, &[]),
    );
    if !matches!(
        result,
        Err(RegisteredExecutionError::Executor(
            ExecutorError::Unauthorized
        ))
    ) {
        return Err(format!(
            "expected Unauthorized without presented bound receipt; got {result:?}"
        )
        .into());
    }
    super::assert_no_effect(&adapter, &mut runtime, &journal, &events_before)?;
    assert_eq!(
        runtime.lookup_pending_action_ticket(run, u64::from(ticket.step.get())),
        Some(ticket)
    );

    let mut wrong_opening = approval.clone();
    wrong_opening.opening_seq = u64::MAX;
    wrong_opening.digest = vb_runtime::manual_gate::compute_bound_receipt_digest(&wrong_opening)?;
    let wrong_opening_params = vb_core::ScopedGrantParams {
        approval_receipt: wrong_opening.digest,
        ..params.clone()
    };
    let wrong_opening_grant = catalog.issue_scoped_grant_for_bound_gate(
        &artifact,
        &register_grant,
        wrong_opening_params,
        &wrong_opening,
    )?;
    let wrong_opening_result = runtime.execute_registered_action(
        &mut adapter,
        run,
        u64::from(ticket.step.get()),
        &register_grant,
        vb_core::ScopedDispatch::new(&wrong_opening_grant, &dispatch, &[])
            .with_gate_approval(&wrong_opening),
    );
    if !matches!(
        wrong_opening_result,
        Err(RegisteredExecutionError::Executor(
            ExecutorError::Unauthorized
        ))
    ) {
        return Err(
            format!("expected out-of-order opening refusal; got {wrong_opening_result:?}").into(),
        );
    }
    super::assert_no_effect(&adapter, &mut runtime, &journal, &events_before)?;

    let mut wrong_input = approval.clone();
    wrong_input.binding.input_digest = GateDigest::from_bytes([88; 32]);
    wrong_input.digest = vb_runtime::manual_gate::compute_bound_receipt_digest(&wrong_input)?;
    let wrong_input_params = vb_core::ScopedGrantParams {
        approval_receipt: wrong_input.digest,
        ..params.clone()
    };
    let wrong_input_grant = catalog.issue_scoped_grant_for_bound_gate(
        &artifact,
        &register_grant,
        wrong_input_params,
        &wrong_input,
    )?;
    let wrong_input_result = runtime.execute_registered_action(
        &mut adapter,
        run,
        u64::from(ticket.step.get()),
        &register_grant,
        vb_core::ScopedDispatch::new(&wrong_input_grant, &dispatch, &[])
            .with_gate_approval(&wrong_input),
    );
    if !matches!(
        wrong_input_result,
        Err(RegisteredExecutionError::Executor(
            ExecutorError::Unauthorized
        ))
    ) {
        return Err(
            format!("expected current-input binding refusal; got {wrong_input_result:?}").into(),
        );
    }
    super::assert_no_effect(&adapter, &mut runtime, &journal, &events_before)?;

    let mut foreign = approval.clone();
    foreign.receipt.run = RunId::new(11_702);
    foreign.digest = vb_runtime::manual_gate::compute_bound_receipt_digest(&foreign)?;
    let foreign_params = vb_core::ScopedGrantParams {
        approval_receipt: foreign.digest,
        ..params.clone()
    };
    let foreign_issue = catalog.issue_scoped_grant_for_bound_gate(
        &artifact,
        &register_grant,
        foreign_params,
        &foreign,
    );
    if !matches!(
        foreign_issue,
        Err(ExecutorError::Grant(
            vb_core::GrantError::GateReceiptMismatch
        ))
    ) {
        return Err(
            format!("expected exact foreign-run grant refusal; got {foreign_issue:?}").into(),
        );
    }

    let mut zero_attempt = approval.clone();
    zero_attempt.binding.attempt = 0;
    zero_attempt.digest = vb_runtime::manual_gate::compute_bound_receipt_digest(&zero_attempt)?;
    let zero_attempt_params = vb_core::ScopedGrantParams {
        approval_receipt: zero_attempt.digest,
        ..params.clone()
    };
    let zero_attempt_issue = catalog.issue_scoped_grant_for_bound_gate(
        &artifact,
        &register_grant,
        zero_attempt_params,
        &zero_attempt,
    );
    if !matches!(
        zero_attempt_issue,
        Err(ExecutorError::Grant(
            vb_core::GrantError::GateReceiptMismatch
        ))
    ) {
        return Err(
            format!("expected zero-attempt grant refusal; got {zero_attempt_issue:?}").into(),
        );
    }
    super::assert_nonapproval_refusals(&catalog, &artifact, &register_grant, &params, &approval)?;
    let nongate = catalog.issue_scoped_grant(&artifact, &register_grant, params.clone())?;
    let nongate_result = runtime.execute_registered_action(
        &mut adapter,
        run,
        u64::from(ticket.step.get()),
        &register_grant,
        vb_core::ScopedDispatch::new(&nongate, &dispatch, &[]),
    );
    if !matches!(
        nongate_result,
        Err(RegisteredExecutionError::Executor(
            ExecutorError::Unauthorized
        ))
    ) {
        return Err(format!(
            "expected non-gate issuance to refuse gated action; got {nongate_result:?}"
        )
        .into());
    }
    super::assert_no_effect(&adapter, &mut runtime, &journal, &events_before)?;

    assert_eq!(
        bound.authority(),
        vb_core::ScopedGrantAuthority::BoundGate {
            receipt_digest: approval.digest
        },
    );
    let bytes = postcard::to_allocvec(&bound)?;
    let decoded: vb_core::ScopedGrant = postcard::from_bytes(&bytes)?;
    assert_eq!(
        decoded.authority(),
        vb_core::ScopedGrantAuthority::Untrusted
    );
    let decoded_result = runtime.execute_registered_action(
        &mut adapter,
        run,
        u64::from(ticket.step.get()),
        &register_grant,
        vb_core::ScopedDispatch::new(&decoded, &dispatch, &[]).with_gate_approval(&approval),
    );
    if !matches!(
        decoded_result,
        Err(RegisteredExecutionError::Executor(
            ExecutorError::Unauthorized
        ))
    ) {
        return Err(
            format!("expected deserialized grant to be untrusted; got {decoded_result:?}").into(),
        );
    }
    super::assert_no_effect(&adapter, &mut runtime, &journal, &events_before)?;
    let receipt = runtime.execute_registered_action(
        &mut adapter,
        run,
        u64::from(ticket.step.get()),
        &register_grant,
        vb_core::ScopedDispatch::new(&bound, &dispatch, &[]).with_gate_approval(&approval),
    )?;
    assert_eq!(receipt.value(), 42);
    assert_eq!(adapter.effect_count()?, 1);
    assert_eq!(adapter.value(scope())?, Some(42));
    drop(adapter);
    super::gate_reconcile::recover_and_reconcile(super::gate_reconcile::ColdReconcileProbe {
        runtime,
        journal,
        catalog,
        artifact,
        dir,
        run,
    })?;
    Ok(())
}
