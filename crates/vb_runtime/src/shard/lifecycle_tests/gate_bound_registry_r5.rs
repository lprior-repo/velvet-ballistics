#[test]
fn cancel_retires_bound_registry_without_erasing_gate_history()
-> Result<(), Box<dyn std::error::Error>> {
    let run = RunId::new(94110);
    let (mut shard, journal) = make_shard(run)?;
    assert!(shard.bound_open_gates.contains_key(&GateId::new(7)));
    shard.handle_cancel(run, Some("operator cancelled".to_string()))?;
    assert!(!shard.bound_open_gates.contains_key(&GateId::new(7)));
    assert_eq!(journal.opened()?.len(), 1);
    assert!(journal.decisions()?.is_empty());
    Ok(())
}

#[test]
fn kill_retires_bound_registry_without_erasing_gate_history()
-> Result<(), Box<dyn std::error::Error>> {
    let run = RunId::new(94111);
    let (mut shard, journal) = make_shard(run)?;
    assert!(shard.bound_open_gates.contains_key(&GateId::new(7)));
    shard.handle_kill(run, None)?;
    assert!(!shard.bound_open_gates.contains_key(&GateId::new(7)));
    assert_eq!(journal.opened()?.len(), 1);
    assert!(journal.decisions()?.is_empty());
    Ok(())
}
