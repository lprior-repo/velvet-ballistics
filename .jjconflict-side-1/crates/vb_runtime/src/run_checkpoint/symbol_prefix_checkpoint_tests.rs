use super::symbol_prefix_fixture::{
    TestResult, assert_nondefault_collect, complete_restoration_checkpoint, fixture_contract,
    legacy_workflow, populated_store, prefix_workflow, retry_checkpoint,
};
use super::*;

fn validate_checkpoint(
    checkpoint: &RunCheckpointV1,
    workflow: &CompiledWorkflow,
) -> TestResult<Result<(), RunCheckpointV1Error>> {
    let before = checkpoint.clone();
    let result = checkpoint.validate_against(
        workflow,
        WorkflowDigest::from_bytes([43; 32]),
        &[fixture_contract()?],
    );
    assert_eq!(checkpoint, &before);
    Ok(result)
}

#[test]
fn bound_symbol_runtime_v1_legacy_fixture_digest_cause() -> TestResult {
    let mut parts = legacy_workflow()?.to_parts();
    parts.name = "recovery-test".into();
    assert_eq!(parts.digest, WorkflowDigest::from_bytes([42; 32]));
    let bytes = postcard::to_allocvec(&parts)?;
    match vb_storage::decode_compiled_ir(&bytes, Some(vb_storage::compat::CURRENT_IR_VERSION)) {
        Err(vb_storage::JournalError::ArtifactChecksumMismatch) => {}
        other => {
            return Err(
                format!("expected exact legacy fixture checksum failure: {other:?}").into(),
            );
        }
    }
    Ok(())
}

#[test]
fn bound_symbol_runtime_full_checkpoint_missing_prefix_after_valid_nested_state() -> TestResult {
    let workflow = prefix_workflow(vec!["key".into()])?;
    let mut checkpoint = retry_checkpoint(&workflow)?;
    *checkpoint
        .frame
        .slots
        .get_mut(0)
        .ok_or("fixture slot missing")? = Some(vb_core::SlotValue::I64(7));
    checkpoint.value_store = ValueStore::with_max_slots(32).checkpoint_v1();
    assert_eq!(validate_structure(&checkpoint, &workflow)?, Ok(()));
    assert_eq!(
        validate_checkpoint(&checkpoint, &workflow)?,
        Err(RunCheckpointV1Error::SymbolPrefixMismatch { index: 0 })
    );
    Ok(())
}

fn validate_structure(
    checkpoint: &RunCheckpointV1,
    workflow: &CompiledWorkflow,
) -> TestResult<Result<(), RunCheckpointV1Error>> {
    Ok(checkpoint.validate_structure_against(
        workflow,
        WorkflowDigest::from_bytes([43; 32]),
        &[fixture_contract()?],
    ))
}

fn assert_restored_arenas(state: &RunState, checkpoint: &RunCheckpointV1) -> TestResult {
    assert_eq!(
        state.store.checkpoint_v1(),
        populated_store()?.checkpoint_v1()
    );
    assert_eq!(state.frame.checkpoint_v1(), checkpoint.frame);
    assert_eq!(
        state.collect_states.checkpoint_v1(),
        checkpoint.collect_states
    );
    assert_nondefault_collect(&state.collect_states.checkpoint_v1())?;
    Ok(())
}

fn assert_restored_metadata(
    state: &RunState,
    checkpoint: &RunCheckpointV1,
    workflow: &CompiledWorkflow,
) {
    assert_eq!(state.admission, checkpoint.admission);
    assert_eq!(state.action_attempts, checkpoint.action_attempts);
    assert_eq!(state.action_attempts.as_ref(), [2, 0]);
    assert_eq!(state.action_contracts, checkpoint.action_contracts);
    assert_eq!(state.run_started_millis, Some(101));
    assert_eq!(state.journal_bytes_written, 19);
    assert_eq!(state.run_attempt.get(), 2);
    assert_eq!(
        state.registered_execution,
        crate::shard::types::RegisteredExecutionMode::ReconcileOnly
    );
    assert_eq!(&state.workflow, workflow);
}

#[test]
fn bound_symbol_runtime_full_checkpoint_preserves_every_store_field() -> TestResult {
    let workflow = prefix_workflow(vec!["key".into()])?;
    let checkpoint = complete_restoration_checkpoint(&workflow)?;
    let before = checkpoint.clone();
    assert_eq!(validate_checkpoint(&checkpoint, &workflow)?, Ok(()));
    let (state, attempt, step, seq) = checkpoint.clone().restore(
        workflow.clone(),
        WorkflowDigest::from_bytes([43; 32]),
        &[fixture_contract()?],
    )?;
    assert_restored_arenas(&state, &checkpoint)?;
    assert_restored_metadata(&state, &checkpoint, &workflow);
    assert_eq!((attempt.get(), step, seq), (2, StepIdx::ZERO, 9));
    assert_eq!(checkpoint, before);
    Ok(())
}

#[test]
fn bound_symbol_runtime_full_checkpoint_rejects_wrong_prefix_without_mutation() -> TestResult {
    let workflow = prefix_workflow(vec!["key".into()])?;
    let mut checkpoint = retry_checkpoint(&workflow)?;
    *checkpoint
        .value_store
        .symbols
        .get_mut(0)
        .ok_or("fixture symbol missing")? = "wrong".into();
    assert_eq!(validate_structure(&checkpoint, &workflow)?, Ok(()));
    assert_eq!(
        validate_checkpoint(&checkpoint, &workflow)?,
        Err(RunCheckpointV1Error::SymbolPrefixMismatch { index: 0 })
    );
    Ok(())
}

#[test]
fn bound_symbol_runtime_full_checkpoint_retains_identity_and_nested_priority() -> TestResult {
    let workflow = prefix_workflow(vec!["key".into()])?;
    let mut checkpoint = retry_checkpoint(&workflow)?;
    *checkpoint
        .value_store
        .symbols
        .get_mut(0)
        .ok_or("fixture symbol missing")? = "wrong".into();
    checkpoint.workflow_digest = WorkflowDigest::from_bytes([99; 32]);
    assert_eq!(
        validate_checkpoint(&checkpoint, &workflow)?,
        Err(RunCheckpointV1Error::WorkflowIdentityMismatch)
    );
    checkpoint.workflow_digest = workflow.digest();
    checkpoint.value_store.list_taints.clear();
    assert_eq!(
        validate_checkpoint(&checkpoint, &workflow)?,
        Err(RunCheckpointV1Error::InvalidNestedState)
    );
    Ok(())
}
