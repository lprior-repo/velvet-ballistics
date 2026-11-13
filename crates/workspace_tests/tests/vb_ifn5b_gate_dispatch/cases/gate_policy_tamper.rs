use super::*;
#[path = "gate_policy_tamper_journal.rs"]
mod journal;

#[test]
fn durable_gate_without_required_evidence_cannot_authorize_effect() -> TestResult {
    durable_policy_case("evidence")
}

#[test]
fn durable_gate_without_required_reason_cannot_authorize_effect() -> TestResult {
    durable_policy_case("reason")
}

#[test]
fn durable_gate_after_recorded_deadline_cannot_authorize_effect() -> TestResult {
    durable_policy_case("timeout")
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

fn durable_policy_case(mismatch: &'static str) -> TestResult {
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
    let source_storage = StorageRuntimeJournal::shared_strict_with_catalog(
        source_journal.clone(),
        Arc::new(catalog.clone()),
    );
    let mut config = crate::common::runtime_fixture::strict_config();
    config.step_budget_per_tick = 1;
    let mut source_runtime = Runtime::new(
        std::num::NonZeroUsize::new(1).ok_or("shard")?,
        config,
        Arc::new(journal::PolicyTamperJournal {
            inner: source_storage,
            mismatch,
        }),
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
    let gate_id = GateId::new(805);
    let approval = approval_for(
        &mut source_runtime,
        &source_journal,
        run,
        &artifact,
        gate_id,
        GateDecision::Approve,
    )?;

    let rebuilt_journal = source_journal;
    let mut runtime = source_runtime;
    let ticket = original_ticket;
    assert_policy_rejected(
        &rebuilt_journal.events_for_run_full(run)?,
        &approval,
        mismatch,
    )?;
    let register = catalog.issue_register_grant(&artifact, ticket, scope())?;
    let params = vb_core::ScopedGrantParams {
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
        return Err(format!("expected exact Unauthorized for {mismatch}; got {result:?}; effect_count={}; value={:?}", adapter.effect_count()?, adapter.value(scope())?).into());
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

fn assert_policy_rejected(
    events: &[JournalEvent],
    approval: &BoundApprovalReceipt,
    mismatch: &str,
) -> TestResult {
    let (gate, opened_at, requester) = events
        .iter()
        .find_map(|event| match event {
            JournalEvent::GateOpenedBoundEvent {
                gate,
                opened_at_ms,
                requester,
                ..
            } => Some((gate, *opened_at_ms, requester)),
            _ => None,
        })
        .ok_or("opening missing after codec roundtrip")?;
    let record = events
        .iter()
        .find_map(|event| match event {
            JournalEvent::GateDecisionBoundEvent { record, .. } => Some(record),
            _ => None,
        })
        .ok_or("decision missing after codec roundtrip")?;
    assert!(gate.is_structurally_valid());
    assert!(record.is_structurally_valid());
    assert_eq!(record.bound_receipt.as_ref(), Some(approval));
    assert_eq!(
        vb_runtime::manual_gate::verify_bound_receipt(approval, gate)?,
        true
    );
    let input = GateDecisionInput {
        gate: record.record.gate,
        decision: record.record.decision.ok_or("decision missing")?,
        approver: record.record.approver.clone().ok_or("approver missing")?,
        approver_role: record.record.approver_role.clone().ok_or("role missing")?,
        requester: record.record.requester.clone(),
        reason: record.record.reason.clone(),
        evidence_shown: record.record.evidence_shown,
        evidence_bundle_digest: record.record.evidence_bundle_digest,
        delegated_to: record.record.delegated_to.clone(),
        break_glass_justification: record.record.break_glass_justification.clone(),
        decided_at_ms: record.record.decided_at_ms,
    };
    use vb_runtime::manual_gate::GateError;
    let expected = match mismatch {
        "evidence" => GateError::EvidenceNotShown,
        "reason" => GateError::ReasonRequired,
        "timeout" => GateError::DecisionAfterTimeout {
            deadline_ms: 150,
            decided_at_ms: 200,
        },
        _ => return Err("unknown tamper".into()),
    };
    assert_eq!(
        vb_runtime::manual_gate::evaluate_bound_decision(
            gate,
            opened_at,
            record.opening_seq,
            requester,
            approval.operator_trust,
            &input,
        ),
        Err(expected)
    );
    Ok(())
}
