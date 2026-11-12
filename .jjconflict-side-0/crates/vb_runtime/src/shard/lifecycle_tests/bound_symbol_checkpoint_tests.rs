#[test]
fn bound_symbol_runtime_raw_bad_pin_precedes_wrong_prefix() -> TestResult {
    let mut fixture = prefix_actor_fixture()?;
    corrupt_historical_pin(&mut fixture)?;
    corrupt_prefix(&mut fixture)?;
    assert_raw_rejection(
        &mut fixture,
        RuntimeError::RecoveryCannotResume {
            reason: "admission_policy_pin_mismatch".into(),
        },
    )
}

#[test]
fn bound_symbol_runtime_raw_valid_pin_reaches_prefix_mismatch() -> TestResult {
    let mut fixture = prefix_actor_fixture()?;
    corrupt_prefix(&mut fixture)?;
    assert_raw_rejection(
        &mut fixture,
        prefix_retry_error("checkpoint compiler symbol prefix mismatch at index 0"),
    )
}

#[test]
fn bound_symbol_runtime_raw_workflow_precedes_bad_pin_and_prefix() -> TestResult {
    let mut fixture = prefix_actor_fixture()?;
    corrupt_historical_pin(&mut fixture)?;
    corrupt_prefix(&mut fixture)?;
    fixture.checkpoint.workflow_digest = WorkflowDigest::from_bytes([99; 32]);
    assert_raw_rejection(
        &mut fixture,
        prefix_retry_error("checkpoint workflow digest mismatch"),
    )
}

#[test]
fn bound_symbol_runtime_raw_nested_precedes_bad_pin_and_prefix() -> TestResult {
    let mut fixture = prefix_actor_fixture()?;
    corrupt_historical_pin(&mut fixture)?;
    corrupt_prefix(&mut fixture)?;
    fixture.checkpoint.value_store.list_taints.clear();
    assert_eq!(
        fixture.checkpoint.validate_structure_against(
            &fixture.workflow,
            fixture.checkpoint.artifact_digest,
            &fixture.checkpoint.action_contracts
        ),
        Err(RunCheckpointV1Error::InvalidNestedState)
    );
    assert_raw_rejection(
        &mut fixture,
        prefix_retry_error("nested checkpoint validation failed"),
    )
}

#[test]
fn bound_symbol_runtime_raw_valid_checkpoint_keeps_duplicate_tail_ids() -> TestResult {
    let mut fixture = prefix_actor_fixture()?;
    let command = prefix_retry_command(&fixture)?;
    assert_eq!(fixture.shard.handle_recover(command), Ok(()));
    assert_full_store(&fixture)
}

fn assert_full_store(fixture: &PrefixActorFixture) -> TestResult {
    let events = fixture.journal.snapshot()?;
    let state = fixture
        .shard
        .run_state_get(RunId::new(71))
        .ok_or_else(|| format!("recovered state missing; events: {events:?}"))?;
    assert_eq!(
        state.store.checkpoint_v1(),
        populated_store()?.checkpoint_v1()
    );
    assert_eq!(state.store.symbol(vb_core::SymbolId::new(2)), Ok("key"));
    assert_eq!(
        state.frame.read_slot(vb_core::SlotIdx::ZERO).copied(),
        Ok(vb_core::SlotValue::List(vb_core::ListId::new(0)))
    );
    Ok(())
}

#[test]
fn bound_symbol_runtime_nonretry_full_store_keeps_duplicate_tail_ids() -> TestResult {
    let mut fixture = prefix_actor_fixture()?;
    let command = prefix_raw_command(&fixture)?
        .with_restored_store(Some(fixture.checkpoint.value_store.clone()));
    assert_eq!(fixture.shard.handle_recover(command), Ok(()));
    assert_full_store(&fixture)
}

#[test]
fn bound_symbol_runtime_nonretry_prefix_rejects_before_publication() -> TestResult {
    let mut fixture = prefix_actor_fixture()?;
    corrupt_prefix(&mut fixture)?;
    let command = prefix_raw_command(&fixture)?
        .with_restored_store(Some(fixture.checkpoint.value_store.clone()));
    let checkpoint_before = fixture.checkpoint.clone();
    let guard = prefix_actor_guard(&fixture)?;
    assert_eq!(fixture.shard.handle_recover(command), Err(RuntimeError::Recovery {
        error: "restored value store failed validation: checkpoint compiler symbol prefix mismatch at index 0".into(),
    }));
    guard(&fixture.shard)?;
    assert_eq!(fixture.checkpoint, checkpoint_before);
    Ok(())
}
