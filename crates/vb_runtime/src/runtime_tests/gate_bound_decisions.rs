fn bound_gate_fixture(runtime: &Runtime, run: RunId, id: u64) -> BoundManualGate {
    let state = runtime
        .shards
        .first()
        .expect("single-shard runtime must hold one shard")
        .run_state_get(run)
        .expect("run state must exist");
    let admission = state.admission.as_ref().expect("admission must exist");
    let input_digest = match admission.input_digest() {
        vb_storage::RunInputBinding::Digest(digest) => GateDigest::from_bytes(digest.as_bytes()),
        vb_storage::RunInputBinding::LegacyUnbound => {
            panic!("submitted run must carry an input digest")
        }
    };
    let policy_digest = match admission.policy_pin() {
        vb_storage::RunPolicyPin::Pinned(digest) => GateDigest::from_bytes(digest.as_bytes()),
        vb_storage::RunPolicyPin::Unverified => gate_digest(5),
    };
    BoundManualGate {
        gate: gate_fixture(run, id),
        binding: GateBinding {
            artifact_digest: GateDigest::from_bytes(admission.artifact_digest().as_bytes()),
            policy_digest,
            input_digest,
            attempt: state.run_attempt.get(),
        },
    }
}

fn running_runtime_with_open_bound_gate(
    run: RunId,
    gate_id: u64,
) -> (Runtime, Arc<GateDecisionTestJournal>) {
    let shard_count = NonZeroUsize::new(1).expect("one shard");
    let journal = GateDecisionTestJournal::shared();
    let mut runtime =
        Runtime::new(shard_count, runtime_config(), journal.clone()).expect("valid test configuration");
    let workflow = gate_chain_workflow().expect("chain workflow must compile");
    assert_eq!(runtime.submit_compiled(run, workflow), Ok(()));
    assert_eq!(runtime.tick_all(), Ok(true));
    let policy_digest = WorkflowDigest::from_bytes([5; 32]);
    let shard = runtime.shards.first_mut().expect("single shard");
    let state = shard.run_state_get_mut(run).expect("submitted run state");
    let admission = state.admission.take().expect("run admission");
    state.admission =
        Some(admission.with_policy_pin(vb_storage::RunPolicyPin::Pinned(policy_digest)));
    let gate = bound_gate_fixture(&runtime, run, gate_id);
    assert_eq!(
        runtime.open_bound_gate(run, gate, 0, gate_person("bob")),
        Ok(())
    );
    assert_eq!(runtime.tick_all(), Ok(true));
    (runtime, journal)
}

#[test]
fn runtime_submit_bound_gate_decision_approves_open_bound_gate()
-> Result<(), Box<dyn std::error::Error>> {
    let run = RunId::new(93101);
    let _clock = crate::shard::helpers::timer::install_wall_clock_millis_guard(1_000);
    let (mut runtime, journal) = running_runtime_with_open_bound_gate(run, 7);
    assert_eq!(
        runtime.submit_bound_gate_decision(run, gate_input_fixture(7, GateDecision::Approve),),
        Ok(())
    );
    assert_eq!(runtime.tick_all(), Ok(true));
    let shard = runtime.shards.first().expect("single shard");
    assert_eq!(shard.open_gate_for(run), None);
    assert_eq!(shard.runtime_state_get(run), Some(RuntimeState::Running));
    let decisions = journal.bound_decisions().expect("bound journal");
    assert_eq!(decisions.len(), 1);
    let decision = decisions.first().ok_or("bound decision missing")?;
    assert_eq!(decision.record.outcome, GateOutcome::Approved);
    assert_eq!(
        decision.operator_trust,
        Some(OperatorTrust::InProcessCaller)
    );
    assert!(decision.bound_receipt.is_some());
    Ok(())
}

#[test]
fn runtime_submit_bound_gate_decision_rejects_unknown_gate() {
    let run = RunId::new(93102);
    let shard_count = NonZeroUsize::new(1).expect("one shard");
    let mut runtime = Runtime::new_for_tests_and_benchmarks_only(shard_count, runtime_config())
        .expect("valid test configuration");
    let workflow = gate_chain_workflow().expect("chain workflow must compile");
    assert_eq!(runtime.submit_compiled(run, workflow), Ok(()));
    assert_eq!(runtime.tick_all(), Ok(true));
    assert_eq!(
        runtime.submit_bound_gate_decision(run, gate_input_fixture(7, GateDecision::Approve),),
        Err(RuntimeError::GateNotOpen {
            gate: GateId::new(7)
        })
    );
}

#[test]
fn runtime_submit_bound_gate_decision_rejects_run_mismatch() {
    let run = RunId::new(93103);
    let _clock = crate::shard::helpers::timer::install_wall_clock_millis_guard(1_000);
    let other = RunId::new(93104);
    let (mut runtime, _journal) = running_runtime_with_open_bound_gate(run, 7);
    let workflow = gate_chain_workflow().expect("chain workflow must compile");
    assert_eq!(runtime.submit_compiled(other, workflow), Ok(()));
    assert_eq!(runtime.tick_all(), Ok(true));
    assert_eq!(
        runtime.submit_bound_gate_decision(other, gate_input_fixture(7, GateDecision::Approve),),
        Err(RuntimeError::GateRunMismatch {
            gate: GateId::new(7),
            expected: run,
            actual: other,
        })
    );
}

#[test]
fn runtime_submit_bound_gate_decision_rejects_legacy_gate() {
    let run = RunId::new(93105);
    let (runtime, _journal) = running_runtime_with_open_gate(run, 7);
    assert_eq!(
        runtime.submit_bound_gate_decision(run, gate_input_fixture(7, GateDecision::Approve),),
        Err(RuntimeError::GateEvaluationFailed {
            source: crate::manual_gate::GateError::LegacyGateWithoutBinding { gate: 7 }
        })
    );
}

#[test]
fn runtime_submit_bound_gate_decision_request_more_evidence_keeps_gate_open() {
    let run = RunId::new(93106);
    let _clock = crate::shard::helpers::timer::install_wall_clock_millis_guard(1_000);
    let (runtime, _journal) = running_runtime_with_open_bound_gate(run, 7);
    let input = gate_input_fixture(7, GateDecision::RequestMoreEvidence);
    assert_eq!(
        runtime.submit_bound_gate_decision(run, input),
        Ok(())
    );
    let shard = runtime.shards.first().expect("single shard");
    assert_eq!(
        shard.open_gate_for(run),
        Some(GateId::new(7))
    );
}

#[test]
fn runtime_submit_bound_gate_decision_delegate_sets_delegation() {
    let run = RunId::new(93107);
    let _clock = crate::shard::helpers::timer::install_wall_clock_millis_guard(1_000);
    let (mut runtime, journal) = running_runtime_with_open_bound_gate(run, 7);
    let mut input = gate_input_fixture(7, GateDecision::Delegate);
    input.delegated_to = Some(gate_person("carol"));
    runtime
        .submit_bound_gate_decision(run, input)
        .expect("delegate should succeed");
    runtime.tick_all().expect("tick should succeed");
    let shard = runtime.shards.first().expect("single shard");
    assert_eq!(
        shard.open_gate_for(run),
        Some(GateId::new(7)),
        "gate must remain open after delegation"
    );
    let decisions = journal.bound_decisions().expect("bound journal");
    assert_eq!(decisions.len(), 1);
    assert_eq!(
        decisions[0].record.delegated_to,
        Some(gate_person("carol")),
        "journal must record the delegation target"
    );
}

#[test]
fn runtime_submit_bound_gate_decision_denies_run() {
    let run = RunId::new(93109);
    let _clock = crate::shard::helpers::timer::install_wall_clock_millis_guard(1_000);
    let (mut runtime, journal) = running_runtime_with_open_bound_gate(run, 7);
    assert_eq!(
        runtime.submit_bound_gate_decision(run, gate_input_fixture(7, GateDecision::Deny)),
        Ok(())
    );
    assert_eq!(runtime.tick_all(), Ok(true));
    let shard = runtime.shards.first().expect("single shard");
    assert!(
        shard.terminal_runs_contains(run),
        "denied run must be in terminal runs"
    );
    let decisions = journal.bound_decisions().expect("bound journal");
    assert_eq!(decisions.len(), 1);
    assert_eq!(decisions[0].record.outcome, GateOutcome::Denied);
}
