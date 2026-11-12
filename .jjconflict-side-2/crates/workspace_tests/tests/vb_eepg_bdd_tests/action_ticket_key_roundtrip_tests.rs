use super::*;

#[test]
fn index_action_key_preserves_all_fields() -> Result<(), JournalError> {
    // Given: ActionId, RunId, and StepIdx with arbitrary values
    let action_id = ActionId::new(0x1234);
    let run_id = RunId::new(0xDEAD_BEEF_CAFE_BABE);
    let step_idx = StepIdx::new(0x5678);

    // When: index_action_key is called
    let key = keys::index_action_key(action_id, run_id, step_idx)?;

    // Then: key = [0x32][action_u16_be][run_u64_be][step_u16_be]
    assert_eq!(
        key[0],
        vb_storage::constants::PREFIX_INDEX_ACTION,
        "prefix must be 0x32"
    );
    assert_eq!(
        &key[1..3],
        action_id.get().to_be_bytes(),
        "action_id bytes must be preserved"
    );
    assert_eq!(
        &key[3..11],
        run_id.get().to_be_bytes(),
        "run_id bytes must be preserved"
    );
    assert_eq!(
        &key[11..13],
        step_idx.get().to_be_bytes(),
        "step_idx bytes must be preserved"
    );

    // And: each field decodes to its original value
    let recovered_action = ActionId::new(u16::from_be_bytes(key[1..3].try_into().unwrap()));
    let recovered_run_id = RunId::new(u64::from_be_bytes(key[3..11].try_into().unwrap()));
    let recovered_step = StepIdx::new(u16::from_be_bytes(key[11..13].try_into().unwrap()));
    assert_eq!(recovered_action, action_id);
    assert_eq!(recovered_run_id, run_id);
    assert_eq!(recovered_step, step_idx);
    Ok(())
}

#[test]
fn index_action_key_max_values() -> Result<(), JournalError> {
    // Given: ActionId::new(u16::MAX), RunId::new(u64::MAX), StepIdx::new(u16::MAX)
    let action_id = ActionId::new(u16::MAX);
    let run_id = RunId::new(u64::MAX);
    let step_idx = StepIdx::new(u16::MAX);

    // When: index_workflow_key is called (typo in spec, should be index_action_key)
    let key = keys::index_action_key(action_id, run_id, step_idx)?;

    // Then: all field bytes equal their type::MAX in big-endian
    assert_eq!(
        &key[1..3],
        u16::MAX.to_be_bytes(),
        "max action_id must encode as 0xFFFF"
    );
    assert_eq!(
        &key[3..11],
        u64::MAX.to_be_bytes(),
        "max run_id must encode as all 0xFF"
    );
    assert_eq!(
        &key[11..13],
        u16::MAX.to_be_bytes(),
        "max step_idx must encode as 0xFFFF"
    );
    Ok(())
}
