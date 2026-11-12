fn validate_ticket_attempt_accepts_valid_ticket() -> Result<(), Box<dyn std::error::Error>> {
    let mut shard = Shard::new(small_config());
    let Some(wf) = suspended_workflow() else {
        return Err("missing workflow fixture".into());
    };
    let run = RunId::new(60);

    submit_with_contracts(&shard, run, wf);
    assert_eq!(shard.tick(), Ok(true));

    // Complete with real issued ticket (attempt=1, equal to current=1 — valid)
    let ticket = shard
        .pending_action_get(run)
        .ok_or("action should have issued ticket")?;
    let output = ActionOutputReady {
        output_slot: SlotIdx::ZERO,
        value: SlotValue::I64(100),
        taint: Taint::Clean,
        encoded_len: 3,
    };
    assert_eq!(
        shard.enqueue(ShardCommand::ActionCompleted { ticket, output }),
        Ok(())
    );
    assert_eq!(shard.tick(), Ok(true), "valid ticket should be accepted");

}