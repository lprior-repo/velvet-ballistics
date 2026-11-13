use super::gate_ticket_binding::{pending_ticket_from_events, repeated_retry_workflow};
use super::*;

fn retryable_failure() -> vb_core::action::ActionFailure {
    vb_core::action::ActionFailure {
        code: vb_core::action::ActionFailureCode::Unknown,
        retry_policy: vb_core::action::RetryPolicy::Retryable,
        taint: vb_core::value::Taint::Clean,
        detail: None,
        encoded_len: 0,
    }
}

#[test]
fn fresh_approval_authorizes_real_retry_but_old_approval_does_not() -> TestResult {
    let dir = storage_dir()?;
    let run = RunId::new(11_800);
    let action = ActionId::new(1);
    let workflow = repeated_retry_workflow().map_err(|error| format!("workflow: {error}"))?;
    let contract = contract()?;
    let contracts = [preceding_contract()?, contract.clone()];
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
    let source = owned_submission::commit(&journal, &catalog, &workflow, &artifact, run)?;
    let storage = StorageRuntimeJournal::shared_strict_with_catalog(
        journal.clone(),
        Arc::new(catalog.clone()),
    );
    let mut config = crate::common::runtime_fixture::strict_config();
    config.step_budget_per_tick = 1;
    let mut runtime = Runtime::new(
        std::num::NonZeroUsize::new(1).ok_or("shard")?,
        config,
        storage,
    );
    runtime.configure_live_policy(
        PolicyDigest::from_bytes([4; 32]),
        PolicyRule {
            action,
            code: DecisionCode::Allowed,
        },
    )?;
    runtime
        .activate_owned_submission(run, source)
        .map_err(|error| format!("activate: {error:?}"))?;
    runtime.tick_all()?;
    let first = pending_ticket_from_events(&mut runtime, &journal, run)?;
    let old_approval = approval_for(
        &mut runtime,
        &journal,
        run,
        &artifact,
        GateId::new(793),
        GateDecision::Approve,
    )?;
    runtime.fail_action(first, retryable_failure())?;
    runtime.tick_all()?;
    let mut retried = None;
    for _ in 0..16 {
        runtime.tick_all()?;
        if let Some(ticket) = runtime.lookup_pending_action_ticket(run, 9)
            && ticket != first
        {
            retried = Some(ticket);
            break;
        }
    }
    let second = retried.ok_or("failed action did not become a retried pending ticket")?;
    if second.run != first.run
        || second.step != first.step
        || second.action != first.action
        || second.run_attempt != first.run_attempt
        || second.attempt.checked_sub(first.attempt) != Some(1)
    {
        return Err(format!("retry ticket identity mismatch: {first:?} -> {second:?}").into());
    }

    let register = catalog.issue_register_grant(&artifact, second, scope())?;
    let params = vb_core::ScopedGrantParams {
        mode: vb_core::GrantAdmissionMode::StrictExact,
        id: vb_core::GrantId::new(793),
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
        approval_receipt: old_approval.digest,
        signature: None,
    };
    let old_bound = catalog.issue_scoped_grant_for_bound_gate(
        &artifact,
        &register,
        params.clone(),
        &old_approval,
    )?;
    let dispatch = vb_core::GrantDispatchContext::new(
        artifact.digest,
        second,
        scope(),
        person("grant-authority")?,
        person("run-operator")?,
        vb_core::GrantTimeMs::new(250),
        vb_core::RevocationEpoch::new(1),
        false,
    );
    let mut adapter = FjallRegister::open(&dir.join("register"), 8)?;
    let events_before = journal.events_for_run_full(run)?;
    let result = runtime.execute_registered_action(
        &mut adapter,
        run,
        u64::from(second.step.get()),
        &register,
        vb_core::ScopedDispatch::new(&old_bound, &dispatch, &[]).with_gate_approval(&old_approval),
    );
    if !matches!(
        result,
        Err(RegisteredExecutionError::Executor(
            ExecutorError::Unauthorized
        ))
    ) {
        return Err(
            format!("expected exact Unauthorized for old retry approval: {result:?}").into(),
        );
    }
    if adapter.effect_count()? != 0
        || adapter.value(scope())?.is_some()
        || journal.events_for_run_full(run)? != events_before
        || runtime.lookup_pending_action_ticket(run, 9) != Some(second)
    {
        return Err(
            "old retry approval mutated adapter, receipt journal, or pending ticket".into(),
        );
    }

    let fresh_approval = approval_for(
        &mut runtime,
        &journal,
        run,
        &artifact,
        GateId::new(794),
        GateDecision::Approve,
    )?;
    let fresh_params = vb_core::ScopedGrantParams {
        approval_receipt: fresh_approval.digest,
        ..params
    };
    let fresh_bound = catalog.issue_scoped_grant_for_bound_gate(
        &artifact,
        &register,
        fresh_params,
        &fresh_approval,
    )?;
    runtime.execute_registered_action(
        &mut adapter,
        run,
        u64::from(second.step.get()),
        &register,
        vb_core::ScopedDispatch::new(&fresh_bound, &dispatch, &[])
            .with_gate_approval(&fresh_approval),
    )?;
    if adapter.effect_count()? != 1 || adapter.value(scope())? != Some(42) {
        return Err("fresh retry-ticket approval did not authorize exactly one effect".into());
    }
    Ok(())
}
