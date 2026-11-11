fn assert_stale_cancelled_timer_rejected(
    shard: &mut Shard,
    journal: &std::sync::Arc<crate::journal::VolatileRuntimeJournal>,
    survivor: RunId,
    token: crate::shard::timer_wheel::TimerEntry,
) -> Result<(), String> {
    let before = authority_snapshot(shard, survivor, journal)?;
    let sequence = shard.journal_sequences.get(&survivor).copied();
    shard
        .enqueue(ShardCommand::TimerFired {
            run: token.run,
            generation: token.generation,
            deadline: token.deadline,
            kind: token.kind,
        })
        .map_err(|error| format!("{error:?}"))?;
    assert_eq!(shard.tick(), Err(RuntimeError::InvalidTimerFire));
    assert_authority_unchanged(shard, survivor, journal, &before)?;
    assert_eq!(shard.journal_sequences.get(&survivor).copied(), sequence);
    Ok(())
}
