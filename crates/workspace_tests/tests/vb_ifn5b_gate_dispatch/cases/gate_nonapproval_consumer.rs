use super::*;

#[test]
fn durable_nonapproval_decisions_refuse_approved_caller_receipts_before_effects() -> TestResult {
    for (offset, decision) in [
        GateDecision::Deny,
        GateDecision::Delegate,
        GateDecision::RequestMoreEvidence,
    ]
    .into_iter()
    .enumerate()
    {
        refuse_nonapproval(11_710 + u64::try_from(offset)?, decision)?;
    }
    Ok(())
}

fn refuse_nonapproval(run_number: u64, decision: GateDecision) -> TestResult {
    let dir = storage_dir()?;
    let run = RunId::new(run_number);
    let action = ActionId::new(1);
    let workflow = workflow().map_err(|error| format!("workflow: {error}"))?;
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
    runtime.activate_owned_submission(run, source)?;
    runtime.tick_all()?;
    let input = journal
        .events_for_run_full(run)?
        .iter()
        .find_map(|event| match event {
            JournalEvent::RunAdmission {
                input_digest: vb_storage::RunInputBinding::Digest(digest),
                ..
            }
            | JournalEvent::RunAdmissionBudget {
                input_digest: vb_storage::RunInputBinding::Digest(digest),
                ..
            } => Some(*digest),
            _ => None,
        })
        .ok_or("durable input binding missing")?;
    let requester = person("requester")?;
    let gate_id = GateId::new(run_number);
    let gate = BoundManualGate {
        gate: ManualGate {
            id: gate_id,
            run,
            action,
            risk: RiskClass::High,
            compensatable: false,
            required_role: role("deploy-approver")?,
            separation_of_duties: SeparationOfDuties::DistinctFromRequester,
            evidence_bundle_digest: GateDigest::from_bytes([7; 32]),
            prompt_digest: GateDigest::from_bytes([8; 32]),
            timeout_ms: 5_000,
            escalation_policy_digest: GateDigest::from_bytes([9; 32]),
            break_glass_allowed: true,
            approval_reason_required: true,
        },
        binding: GateBinding {
            artifact_digest: GateDigest::from_bytes(artifact.digest.as_bytes()),
            policy_digest: GateDigest::from_bytes(artifact.policy_digest.as_bytes()),
            input_digest: GateDigest::from_bytes(input.as_bytes()),
            attempt: 1,
        },
    };
    let ticket = pending_ticket(&mut runtime, run)?;
    let seq = journal
        .events_for_run_full(run)?
        .last()
        .map(|event| event.seq().get())
        .unwrap_or(0)
        .checked_add(1)
        .ok_or("gate opening sequence overflow")?;
    let opening_seq = vb_storage::EventSeq::new(seq);
    let decision_seq = vb_storage::EventSeq::new(
        seq.checked_add(1)
            .ok_or("gate decision sequence overflow")?,
    );
    let approver = person("approver")?;
    let approver_role = role("deploy-approver")?;
    let delegate = (decision == GateDecision::Delegate)
        .then(|| person("delegate"))
        .transpose()?;
    let denial = vb_core::manual_gate::BoundGateOutcomeRecord {
        record: vb_core::manual_gate::GateOutcomeRecord {
            gate: gate_id,
            run,
            action,
            decision: Some(decision),
            outcome: vb_core::manual_gate::GateOutcome::from_decision(decision),
            approver: Some(approver.clone()),
            approver_role: Some(approver_role.clone()),
            requester: requester.clone(),
            reason: Some(String::from("reviewed deployment evidence")),
            evidence_shown: true,
            evidence_bundle_digest: gate.gate.evidence_bundle_digest,
            prompt_digest: gate.gate.prompt_digest,
            escalation_policy_digest: gate.gate.escalation_policy_digest,
            delegated_to: delegate,
            break_glass: false,
            break_glass_justification: None,
            opened_at_ms: 100,
            decided_at_ms: 200,
            receipt: None,
        },
        binding: gate.binding,
        opening_seq: seq,
        operator_trust: Some(vb_core::manual_gate::OperatorTrust::InProcessCaller),
        bound_receipt: None,
    };
    assert!(denial.is_structurally_valid());
    journal.append_strict(&JournalEvent::GateOpenedBoundEvent {
        run,
        seq: opening_seq,
        gate: gate.clone(),
        opened_at_ms: 100,
        requester: requester.clone(),
    })?;
    journal.append_strict(&JournalEvent::GateDecisionBoundEvent {
        run,
        seq: decision_seq,
        record: Box::new(denial.clone()),
    })?;
    let events = journal.events_for_run_full(run)?;
    let opening = events
        .iter()
        .find_map(|event| match event {
            JournalEvent::GateOpenedBoundEvent { gate, seq, .. } => Some((gate, *seq)),
            _ => None,
        })
        .ok_or("durable gate opening missing")?;
    assert_eq!(opening.0, &gate);
    assert_eq!(opening.1, opening_seq);
    assert_eq!(denial.record.decision, Some(decision));
    assert_eq!(
        denial.record.outcome,
        vb_core::manual_gate::GateOutcome::from_decision(decision)
    );

    let approver = denial
        .record
        .approver
        .clone()
        .ok_or("non-approval record missing approver")?;
    let approver_role = denial
        .record
        .approver_role
        .clone()
        .ok_or("non-approval record missing role")?;
    let mut receipt = vb_core::manual_gate::ApprovalReceipt {
        gate: gate_id,
        run,
        decision: GateDecision::Approve,
        approver,
        evidence_bundle_digest: denial.record.evidence_bundle_digest,
        prompt_digest: denial.record.prompt_digest,
        decided_at_ms: denial.record.decided_at_ms,
        digest: GateDigest::from_bytes([0; 32]),
    };
    receipt.digest = vb_runtime::manual_gate::compute_receipt_digest(
        receipt.gate.get(),
        receipt.run,
        receipt.decision,
        &receipt.approver,
        receipt.evidence_bundle_digest,
        receipt.prompt_digest,
        receipt.decided_at_ms,
    )?;
    let mut approval = BoundApprovalReceipt {
        receipt,
        action,
        requester: denial.record.requester.clone(),
        approver_role,
        risk: RiskClass::High,
        separation: SeparationOfDuties::DistinctFromRequester,
        binding: denial.binding,
        opening_seq: opening.1.get(),
        operator_trust: denial
            .operator_trust
            .ok_or("non-approval record missing trust provenance")?,
        digest: GateDigest::from_bytes([0; 32]),
    };
    approval.digest = vb_runtime::manual_gate::compute_bound_receipt_digest(&approval)?;
    assert!(vb_runtime::manual_gate::verify_bound_receipt(
        &approval, opening.0
    )?);

    let register = catalog.issue_register_grant(&artifact, ticket, scope())?;
    let params = super::gate_cold_recovery::scoped_params(action, approval.digest, false)?;
    let bound =
        catalog.issue_scoped_grant_for_bound_gate(&artifact, &register, params, &approval)?;
    assert_eq!(
        bound.authority(),
        vb_core::ScopedGrantAuthority::BoundGate {
            receipt_digest: approval.digest
        }
    );
    let dispatch = super::gate_cold_recovery::dispatch_context(&artifact, ticket)?;
    let mut adapter = FjallRegister::open(&dir.join("register"), 8)?;
    let before = DispatchSnapshot {
        run,
        ticket,
        events,
    };
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
        return Err(format!(
            "expected durable {decision:?} consumer refusal for correctly-digested Approved grant; got {result:?}"
        )
        .into());
    }
    assert_no_effect(&adapter, &mut runtime, &journal, &before)?;
    Ok(())
}
