use super::*;

/// Reconciles a bound gate grant with no durable receipt on a pack of the
/// given recovery class. Returns the executor refusal after proving it had no
/// effect on the adapter, runtime, or journal.
fn refused_reconcile_without_receipt(
    recovery: RecoveryClass,
) -> Result<ExecutorError, Box<dyn std::error::Error>> {
    let dir = storage_dir()?;
    let run = RunId::new(11_706);
    let action = ActionId::new(1);
    let workflow = workflow().map_err(|error| format!("workflow: {error}"))?;
    let contract = contract()?;
    let contracts = [preceding_contract()?, contract.clone()];
    let catalog = ExecutorCatalog::run_registered_conformance()?;
    let pack = catalog.register_pack(&contract, scope(), recovery)?;
    let journal = Arc::new(FjallJournal::open(&dir, Some(FjallConfig::default()))?);
    let artifact = catalog.submit_artifact(
        &journal,
        &workflow,
        RuntimePolicy::Strict,
        &contracts,
        &[pack],
    )?;
    let source = owned_submission::commit(&journal, &catalog, &workflow, &artifact, run)?;
    let artifact = crate::issued_run_artifact::read(&journal, run, &artifact)?;
    let storage = StorageRuntimeJournal::shared_strict_with_catalog(
        journal.clone(),
        Arc::new(catalog.clone()),
    );
    let shard_count = std::num::NonZeroUsize::new(1).ok_or("shard count")?;
    let mut config = crate::common::runtime_fixture::strict_config();
    config.step_budget_per_tick = 1;
    let mut runtime = Runtime::new(shard_count, config, storage)
        .map_err(|error| format!("runtime: {error:?}"))?;
    let policy_digest = PolicyDigest::from_bytes([4; 32]);
    runtime.configure_live_policy(
        policy_digest,
        PolicyRule {
            action,
            code: DecisionCode::Allowed,
        },
    )?;
    runtime.activate_owned_submission(run, source)?;
    runtime.tick_all()?;
    let approval = approval_for(
        &mut runtime,
        &journal,
        run,
        &artifact,
        GateId::new(76),
        GateDecision::Approve,
    )?;
    let ticket = pending_ticket(&mut runtime, run)?;
    let register = catalog.issue_register_grant(&artifact, ticket, scope())?;
    let bound = catalog.issue_scoped_grant_for_bound_gate(
        &artifact,
        &register,
        super::gate_cold_recovery::scoped_params(action, approval.digest)?,
        &approval,
    )?;
    let dispatch = super::gate_cold_recovery::dispatch_context(&artifact, ticket)?;
    let register_path = dir.join("register");
    let mut adapter = FjallRegister::open(&register_path, 8)?;
    let before = DispatchSnapshot {
        run,
        ticket,
        events: journal.events_for_run_full(run)?,
    };

    let reconcile = runtime.reconcile_registered_action(
        &mut adapter,
        run,
        u64::from(ticket.step.get()),
        &register,
        &bound,
        &dispatch,
    );
    let refusal = match reconcile {
        Err(RegisteredExecutionError::Executor(error)) => error,
        other => {
            return Err(format!("expected executor refusal without receipt; got {other:?}").into());
        }
    };
    assert_no_effect(&adapter, &mut runtime, &journal, &before)?;
    Ok(refusal)
}

#[test]
fn blocked_pack_reconcile_refuses_pack_mismatch_before_receipt_lookup_without_effect() -> TestResult
{
    // No receipt exists. The control below differs only in recovery class and
    // reaches ReceiptMissing, so this PackMismatch comes from the Blocked-pack
    // guard before the receipt lookup.
    match refused_reconcile_without_receipt(RecoveryClass::Blocked)? {
        ExecutorError::PackMismatch => Ok(()),
        other => Err(format!("expected blocked-pack PackMismatch refusal; got {other:?}").into()),
    }
}

#[test]
fn reconcile_receipt_pack_without_receipt_refuses_with_receipt_missing() -> TestResult {
    // Control: the same setup on a non-Blocked pack reaches the receipt lookup
    // and reports ReceiptMissing, so the Blocked test's PackMismatch is caused
    // by the Blocked class. Removing that guard fails the Blocked test, not this one.
    match refused_reconcile_without_receipt(RecoveryClass::ReconcileReceipt)? {
        ExecutorError::ReceiptMissing => Ok(()),
        other => Err(format!("expected ReceiptMissing control refusal; got {other:?}").into()),
    }
}
