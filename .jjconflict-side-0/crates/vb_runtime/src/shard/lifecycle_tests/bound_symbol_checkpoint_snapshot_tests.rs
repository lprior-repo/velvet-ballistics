fn prefix_run_guard(shard: &Shard) -> impl FnOnce(&Shard) + use<> {
    let runs = shard.runs.clone();
    let membership = shard.run_aggregate.recovery_membership_snapshot();
    let sequences = shard.journal_sequences.clone();
    let counters = shard.counters().snapshot();
    let accounted = shard.accounted_executed_steps.clone();
    move |after: &Shard| {
        assert_eq!(after.runs, runs);
        assert_eq!(
            after.run_aggregate.recovery_membership_snapshot(),
            membership
        );
        assert_eq!(after.journal_sequences, sequences);
        assert_eq!(after.counters().snapshot(), counters);
        assert_eq!(after.accounted_executed_steps, accounted);
    }
}

fn prefix_authority_guard(shard: &Shard) -> impl FnOnce(&Shard) + use<> {
    let boundaries = shard.pending_boundary_snapshot(0, 64);
    let timers = shard.pending_timer_clone();
    let actions = shard.run_aggregate.pending_action_clone();
    let asks = shard.pending_ask_authorities.clone();
    let gates = shard.open_gates.clone();
    let fencing = shard.fencing_tokens.recovery_snapshot();
    move |after: &Shard| {
        assert_eq!(after.pending_boundary_snapshot(0, 64), boundaries);
        assert_eq!(after.pending_timer_clone(), timers);
        assert_eq!(after.run_aggregate.pending_action_clone(), actions);
        assert_eq!(after.pending_ask_authorities, asks);
        assert_eq!(after.open_gates, gates);
        assert_eq!(after.fencing_tokens.recovery_snapshot(), fencing);
    }
}

fn prefix_pool_snapshot(shard: &Shard) -> Vec<((u16, u16), usize, usize)> {
    shard
        .frame_pools
        .iter()
        .map(|(key, pool)| (*key, pool.available(), pool.capacity()))
        .collect()
}

fn prefix_trace_snapshot(shard: &Shard) -> (usize, usize, u64, Vec<crate::trace::TraceEvent>) {
    (
        shard.trace_ring.pending_len(),
        shard.trace_ring.history_len(),
        shard.trace_ring.dropped(),
        shard.trace_ring.snapshot_for_run(RunId::new(71), 64),
    )
}

fn prefix_resource_guard(shard: &Shard) -> impl FnOnce(&Shard) + use<> {
    let pools = prefix_pool_snapshot(shard);
    let trace = prefix_trace_snapshot(shard);
    let queue = (shard.command_queue.len(), shard.command_queue.capacity());
    move |after: &Shard| {
        assert_eq!(prefix_pool_snapshot(after), pools);
        assert_eq!(prefix_trace_snapshot(after), trace);
        assert_eq!(
            (after.command_queue.len(), after.command_queue.capacity()),
            queue
        );
    }
}

fn prefix_actor_guard(
    fixture: &PrefixActorFixture,
) -> TestResult<impl FnOnce(&Shard) -> TestResult + use<>> {
    let runs = prefix_run_guard(&fixture.shard);
    let authorities = prefix_authority_guard(&fixture.shard);
    let resources = prefix_resource_guard(&fixture.shard);
    let before = fixture.journal.snapshot()?;
    let journal = fixture.journal.clone();
    Ok(move |after: &Shard| {
        runs(after);
        authorities(after);
        resources(after);
        assert_eq!(journal.snapshot()?, before);
        Ok(())
    })
}

fn assert_raw_rejection(fixture: &mut PrefixActorFixture, expected: RuntimeError) -> TestResult {
    let command = prefix_retry_command(fixture)?;
    let checkpoint_before = fixture.checkpoint.clone();
    let guard = prefix_actor_guard(fixture)?;
    assert_eq!(fixture.shard.handle_recover(command), Err(expected));
    guard(&fixture.shard)?;
    assert_eq!(fixture.checkpoint, checkpoint_before);
    Ok(())
}
