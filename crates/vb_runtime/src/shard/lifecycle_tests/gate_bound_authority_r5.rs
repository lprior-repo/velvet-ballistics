#[test]
fn bound_open_rejects_binding_that_disagrees_with_live_admission()
-> Result<(), Box<dyn std::error::Error>> {
    let run = RunId::new(94100);
    let (mut shard, journal) = running_shard(run)?;
    for field in 0..4 {
        let mut gate = bound_gate_for_state(&shard, run)?;
        match field {
            0 => gate.binding.artifact_digest = digest(91),
            1 => gate.binding.policy_digest = digest(92),
            2 => gate.binding.input_digest = digest(93),
            _ => gate.binding.attempt = 2,
        }
        shard.enqueue(ShardCommand::OpenBoundGate {
            gate,
            opened_at_ms: 0,
            requester: person("bob"),
        })?;
        let err = shard
            .tick()
            .err()
            .ok_or("mismatched binding was accepted")?;
        assert!(matches!(
            err,
            RuntimeError::GateEvaluationFailed {
                source: crate::manual_gate::GateError::BoundGateBindingMismatch { .. }
            }
        ));
        assert!(journal.opened()?.is_empty());
        assert_eq!(shard.runtime_state_get(run), Some(RuntimeState::Running));
    }
    Ok(())
}

#[test]
fn bound_open_rejects_unverified_policy_identity() -> Result<(), Box<dyn std::error::Error>> {
    let run = RunId::new(94101);
    let (mut shard, journal) = running_shard(run)?;
    let state = shard.run_state_get_mut(run).ok_or("run state missing")?;
    let admission = state.admission.take().ok_or("admission missing")?;
    let input_digest = match admission.input_digest() {
        vb_storage::RunInputBinding::Digest(digest) => digest,
        vb_storage::RunInputBinding::LegacyUnbound => return Err("input identity missing".into()),
    };
    let mut gate = bound_gate(run);
    gate.binding = GateBinding {
        artifact_digest: GateDigest::from_bytes(admission.artifact_digest().as_bytes()),
        policy_digest: digest(5),
        input_digest: GateDigest::from_bytes(input_digest.as_bytes()),
        attempt: state.run_attempt.get(),
    };
    state.admission = Some(admission.with_policy_pin(vb_storage::RunPolicyPin::Unverified));
    shard.enqueue(ShardCommand::OpenBoundGate {
        gate,
        opened_at_ms: 0,
        requester: person("bob"),
    })?;
    let err = shard
        .tick()
        .err()
        .ok_or("unverified policy identity was accepted")?;
    assert!(matches!(
        err,
        RuntimeError::GateEvaluationFailed {
            source: crate::manual_gate::GateError::BoundGateBindingMismatch {
                field: "policy_digest",
                ..
            }
        }
    ));
    assert!(journal.opened()?.is_empty());
    assert_eq!(shard.runtime_state_get(run), Some(RuntimeState::Running));
    Ok(())
}
