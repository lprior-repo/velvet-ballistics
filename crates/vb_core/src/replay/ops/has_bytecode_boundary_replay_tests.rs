#[test]
#[cfg(feature = "test-util")]
fn has_bytecode_boundary_replay_zero_values_error_at_pc_one() -> Result<(), CoreError> {
    let (mut store, mut run, key) = has_bytecode_fixture()?;
    assert_replay_error_at_pc_one(replay_has(
        malformed_has_program(MalformedHasDepth::Zero),
        key,
        &mut run,
        &mut store,
    ));
    Ok(())
}

#[test]
#[cfg(feature = "test-util")]
fn has_bytecode_boundary_replay_one_value_error_at_pc_one() -> Result<(), CoreError> {
    let (mut store, mut run, key) = has_bytecode_fixture()?;
    assert_replay_error_at_pc_one(replay_has(
        malformed_has_program(MalformedHasDepth::One),
        key,
        &mut run,
        &mut store,
    ));
    Ok(())
}

#[test]
#[cfg(feature = "test-util")]
fn has_bytecode_boundary_replay_three_values_fail_finalization_at_pc_one() -> Result<(), CoreError>
{
    let (mut store, mut run, key) = has_bytecode_fixture()?;
    assert_replay_error_at_pc_one(replay_has(
        malformed_has_program(MalformedHasDepth::Three),
        key,
        &mut run,
        &mut store,
    ));
    Ok(())
}

#[test]
fn has_bytecode_boundary_replay_validated_control_takes_true_branch() -> Result<(), CoreError> {
    let (mut store, mut run, key) = has_bytecode_fixture()?;
    let plan = checked_has_plan(key).map_err(|_| CoreError::InvalidCompiledWorkflow {
        reason: "valid Has positive-control plan rejected",
    })?;
    let outcome = replay_choose_with_plan(&plan, &mut run, &mut store);
    let Ok(crate::replay::step::ReplayAction::Continue(target)) = outcome else {
        return Err(CoreError::InvalidCompiledWorkflow {
            reason: "valid Has replay control did not continue",
        });
    };
    assert_eq!(target, StepIdx::new(2));
    assert_eq!(run.pc(), StepIdx::new(2));
    Ok(())
}
