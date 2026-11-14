#[test]
fn bound_gate_zero_global_capacity_refuses_before_journal_or_state_change()
-> Result<(), Box<dyn std::error::Error>> {
    let run = RunId::new(94120);
    let config = crate::shard::ShardConfig {
        max_open_gates: 0,
        ..small_config()
    };
    let (mut shard, journal) = running_shard_with_config(run, config)?;
    assert_eq!(shard.max_open_gates, 0);
    let before = shard
        .run_state_get(run)
        .ok_or("missing state")?
        .frame
        .checkpoint_v1();
    let gate = bound_gate_for_state(&shard, run)?;
    shard.enqueue(ShardCommand::OpenBoundGate {
        gate,
        opened_at_ms: 0,
        requester: person("bob"),
    })?;
    assert_eq!(
        shard.tick(),
        Err(RuntimeError::OpenGateCapacityExceeded { capacity: 0 })
    );
    assert!(journal.opened()?.is_empty());
    assert!(journal.decisions()?.is_empty());
    assert_eq!(
        shard
            .run_state_get(run)
            .ok_or("missing state")?
            .frame
            .checkpoint_v1(),
        before
    );
    assert_eq!(shard.runtime_state_get(run), Some(RuntimeState::Running));
    assert!(shard.bound_open_gates.is_empty());
    assert_eq!(shard.command_queue_len(), 0);
    Ok(())
}

#[test]
fn bound_and_restored_legacy_gates_share_named_capacity_independent_of_active_runs()
-> Result<(), Box<dyn std::error::Error>> {
    let run = RunId::new(94121);
    let config = crate::shard::ShardConfig {
        max_open_gates: 2,
        max_active_runs: 1,
        ..small_config()
    };
    let (mut shard, journal) = running_shard_with_config(run, config)?;
    let gate = bound_gate_for_state(&shard, run)?;
    shard.enqueue(ShardCommand::OpenBoundGate {
        gate: gate.clone(),
        opened_at_ms: 0,
        requester: person("bob"),
    })?;
    assert_eq!(shard.tick(), Ok(true));
    let mut legacy = crate::shard::types::OpenGate {
        gate: gate.gate,
        opened_at_ms: 0,
        requester: person("bob"),
        delegated_to: None,
    };
    legacy.gate.id = GateId::new(8);
    shard.restore_open_gates(run, std::slice::from_ref(&legacy))?;
    let before = shard
        .run_state_get(run)
        .ok_or("missing state")?
        .frame
        .checkpoint_v1();
    legacy.gate.id = GateId::new(9);
    assert_eq!(
        shard.restore_open_gates(run, std::slice::from_ref(&legacy)),
        Err(RuntimeError::OpenGateCapacityExceeded { capacity: 2 })
    );
    let mut restored_bound = shard
        .bound_open_gates
        .get(&GateId::new(7))
        .ok_or("bound gate missing")?
        .clone();
    restored_bound.gate.gate.id = GateId::new(10);
    assert_eq!(
        shard.restore_bound_open_gates(run, std::slice::from_ref(&restored_bound)),
        Err(RuntimeError::OpenGateCapacityExceeded { capacity: 2 })
    );
    assert_eq!(shard.open_gate_count(run), 2);
    assert_eq!(journal.opened()?.len(), 1);
    assert!(journal.decisions()?.is_empty());
    assert_eq!(
        shard
            .run_state_get(run)
            .ok_or("missing state")?
            .frame
            .checkpoint_v1(),
        before
    );
    Ok(())
}
