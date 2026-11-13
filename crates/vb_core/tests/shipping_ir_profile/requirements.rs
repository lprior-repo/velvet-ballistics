use super::{CompiledWorkflow, WorkflowError, WorkflowIrProfile, fixtures};

#[test]
fn legacy_has_no_concurrent_requirements() -> Result<(), WorkflowError> {
    let workflow = CompiledWorkflow::try_from_parts(fixtures::parts(0))?;
    assert_eq!(workflow.concurrent_requirements(), None);
    Ok(())
}

#[test]
fn checked_profile_caches_root_only_requirements() -> Result<(), WorkflowError> {
    let original = fixtures::parts(0);
    let workflow = CompiledWorkflow::try_from_concurrent_parts(original.clone())?;
    let requirements = workflow
        .concurrent_requirements()
        .ok_or(WorkflowError::EmptyNodes)?;
    assert_eq!(requirements.frames(), 1);
    assert_eq!(requirements.groups(), 0);
    assert_eq!(workflow.ir_profile(), WorkflowIrProfile::ConcurrentV3);
    assert_eq!(workflow.to_parts(), original);
    assert_eq!(
        workflow.clone().concurrent_requirements(),
        Some(requirements)
    );
    Ok(())
}

#[cfg(feature = "test-util")]
#[test]
fn unchecked_has_no_concurrent_requirements() {
    let workflow = CompiledWorkflow::from_parts_unchecked(fixtures::parts(0));
    assert_eq!(workflow.concurrent_requirements(), None);
}
