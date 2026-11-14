use super::*;

#[test]
fn malformed_opening_metadata_cannot_authorize_registered_effect() -> TestResult {
    for mismatch in ["requester", "opened_at"] {
        malformed_opening_metadata_case(mismatch)?;
    }
    Ok(())
}

fn event_bytes(
    journal: &FjallJournal,
    run: RunId,
    count: usize,
) -> Result<Vec<Vec<u8>>, Box<dyn std::error::Error>> {
    let mut bytes = Vec::with_capacity(count);
    for seq in 0..count {
        bytes.push(
            journal
                .get_event_bytes(run, vb_storage::EventSeq::new(u64::try_from(seq)?))?
                .ok_or("event bytes missing")?,
        );
    }
    Ok(bytes)
}

fn malformed_opening_metadata_case(mismatch: &str) -> TestResult {
    let root = storage_dir()?;
    let run = RunId::new(11_803);
    let action = ActionId::new(1);
    let workflow =
        gate_ticket_binding::repeated_workflow().map_err(|error| format!("workflow: {error}"))?;
    let contract = contract()?;
    let contracts = [preceding_contract()?, contract.clone()];
    let catalog = ExecutorCatalog::run_registered_conformance()?;
    let pack = catalog.register_pack(&contract, scope(), RecoveryClass::ReconcileReceipt)?;
    let source_journal = Arc::new(FjallJournal::open(
        root.join("source"),
        Some(FjallConfig::default()),
    )?);
    let artifact = catalog.submit_artifact(
        &source_journal,
        &workflow,
        RuntimePolicy::Strict,
        &contracts,
        std::slice::from_ref(&pack),
    )?;
    let source = owned_submission::commit(&source_journal, &catalog, &workflow, &artifact, run)?;
    let original_header = source_journal
        .run_header(run)?
        .ok_or("original run header missing after commit")?;
    let source_storage = StorageRuntimeJournal::shared_strict_with_catalog(
        source_journal.clone(),
        Arc::new(catalog.clone()),
    );
    let mut config = crate::common::runtime_fixture::strict_config();
    config.step_budget_per_tick = 1;
    let mut source_runtime = Runtime::new(
        std::num::NonZeroUsize::new(1).ok_or("shard")?,
        config,
        source_storage,
    );
    source_runtime.configure_live_policy(
        PolicyDigest::from_bytes([4; 32]),
        PolicyRule {
            action,
            code: DecisionCode::Allowed,
        },
    )?;
    source_runtime.activate_owned_submission(run, source)?;
    source_runtime.tick_all()?;
    let original_ticket =
        gate_ticket_binding::pending_ticket_from_events(&mut source_runtime, &source_journal, run)?;
    let base_events = source_journal.events_for_run_full(run)?;
    let gate_id = GateId::new(if mismatch == "requester" { 803 } else { 804 });
    let approval = approval_for(
        &mut source_runtime,
        &source_journal,
        run,
        &artifact,
        gate_id,
        GateDecision::Approve,
    )?;

    let events = source_journal.events_for_run_full(run)?;
    let rebuilt_journal = Arc::new(FjallJournal::open(
        root.join("rebuilt"),
        Some(FjallConfig::default()),
    )?);
    let rebuilt_artifact = catalog.submit_artifact(
        &rebuilt_journal,
        &workflow,
        RuntimePolicy::Strict,
        &contracts,
        &[pack],
    )?;
    if rebuilt_artifact.digest != artifact.digest {
        return Err("rebuilt submission changed the accepted artifact".into());
    }
    let rebuilt_source = owned_submission::commit_with_header(
        &rebuilt_journal,
        &catalog,
        &workflow,
        &rebuilt_artifact,
        original_header,
    )?;
    let storage = StorageRuntimeJournal::shared_strict_with_catalog(
        rebuilt_journal.clone(),
        Arc::new(catalog.clone()),
    );
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
    runtime.activate_owned_submission(run, rebuilt_source)?;
    runtime.tick_all()?;
    let ticket =
        gate_ticket_binding::pending_ticket_from_events(&mut runtime, &rebuilt_journal, run)?;
    if ticket != original_ticket || rebuilt_journal.events_for_run_full(run)? != base_events {
        return Err("rebuilt accepted submission changed its pending invocation".into());
    }
    let mut changed_opening = false;
    for mut event in events.into_iter().skip(base_events.len()) {
        if let JournalEvent::GateOpenedBoundEvent {
            seq,
            opened_at_ms,
            requester,
            ..
        } = &mut event
            && seq.get() == approval.opening_seq
        {
            if mismatch == "requester" {
                *requester = person("different-requester")?;
            } else {
                *opened_at_ms = opened_at_ms.checked_add(1).ok_or("timestamp overflow")?;
            }
            changed_opening = true;
        }
        rebuilt_journal.append_strict(&event)?;
    }
    if !changed_opening {
        return Err("actual durable opening event was not mutated".into());
    }
    let register = catalog.issue_register_grant(&artifact, ticket, scope())?;
    let params = vb_core::ScopedGrantParams {
        mode: vb_core::GrantAdmissionMode::StrictExact,
        id: vb_core::GrantId::new(803),
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
    let bound =
        catalog.issue_scoped_grant_for_bound_gate(&artifact, &register, params, &approval)?;
    let dispatch = vb_core::GrantDispatchContext::new(
        artifact.digest,
        ticket,
        scope(),
        person("grant-authority")?,
        person("run-operator")?,
        vb_core::GrantTimeMs::new(250),
        vb_core::RevocationEpoch::new(1),
        false,
    );
    let event_snapshot = rebuilt_journal.events_for_run_full(run)?;
    let bytes_snapshot = event_bytes(&rebuilt_journal, run, event_snapshot.len())?;
    let mut adapter = FjallRegister::open(&root.join("adapter"), 8)?;
    let result = runtime.execute_registered_action(
        &mut adapter,
        run,
        u64::from(ticket.step.get()),
        &register,
        vb_core::ScopedDispatch::new(&bound, &dispatch, &[]).with_gate_approval(&approval),
    );
    if !matches!(
        result,
        Err(RegisteredExecutionError::Executor(
            ExecutorError::Unauthorized
        ))
    ) {
        return Err(format!("expected exact Unauthorized for {mismatch}; got {result:?}").into());
    }
    if adapter.effect_count()? != 0
        || adapter.value(scope())?.is_some()
        || runtime.lookup_pending_action_ticket(run, u64::from(ticket.step.get())) != Some(ticket)
        || rebuilt_journal.events_for_run_full(run)? != event_snapshot
        || event_bytes(&rebuilt_journal, run, event_snapshot.len())? != bytes_snapshot
    {
        return Err(format!(
            "rejected {mismatch} mismatch mutated effect, receipt, pending, or journal"
        )
        .into());
    }
    Ok(())
}
