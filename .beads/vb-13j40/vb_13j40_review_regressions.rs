#![forbid(unsafe_code)]
mod artifact_evidence_support;
use artifact_evidence_support::*;
use vb_core::action::{ActionRegistryError, Idempotency, RetrySafety, SideEffect};
use vb_core::{ActionId, CapabilitySet, CompiledNodeKind, RunId, RuntimePolicy};
use vb_runtime::journal::StorageRuntimeJournal;
use vb_runtime::runtime::Runtime;
use vb_runtime::shard::{Shard, ShardCommand, ShardConfig};
use vb_runtime::RuntimeError;
use vb_storage::admission::prepare_compiled_artifact;
use std::{num::NonZeroUsize, sync::Arc};
type TestResult = Result<(), Box<dyn std::error::Error>>;

fn config() -> ShardConfig {
    ShardConfig { policy: RuntimePolicy::Strict, ..ShardConfig::default() }
}
fn runtime(journal: Arc<vb_storage::FjallJournal>) -> Result<Runtime, Box<dyn std::error::Error>> {
    Ok(Runtime::new(NonZeroUsize::new(1).ok_or("zero shards")?, config(), StorageRuntimeJournal::shared_strict(journal)))
}

#[test]
fn sparse_and_duplicate_registries_cannot_mint_checked_evidence() -> TestResult {
    let dense_workflow = action_workflow()?;
    let mut sparse_parts = dense_workflow.to_parts();
    for node in &mut sparse_parts.nodes {
        if let CompiledNodeKind::Do { action, .. } = &mut node.kind { *action = ActionId::new(7); }
    }
    let sparse_workflow = canonicalize(sparse_parts)?;
    let mut sparse = contract()?; sparse.id = ActionId::new(7);
    for (workflow, contracts, expected) in [
        (sparse_workflow, vec![sparse], ActionRegistryError::IndexMismatch { index: 0, expected: ActionId::new(0), actual: ActionId::new(7) }),
        (dense_workflow.clone(), vec![contract()?, contract()?], ActionRegistryError::IndexMismatch { index: 1, expected: ActionId::new(1), actual: ActionId::new(0) }),
    ] {
        match vb_compile::verify_compiled_artifact(&workflow, &contracts) {
            Err(vb_compile::ArtifactVerificationError::ActionRegistry(actual)) => assert_eq!(actual, expected),
            other => return Err(format!("wrong compiler result: {other:?}").into()),
        }
        match prepare_compiled_artifact(&workflow, RuntimePolicy::Journaled, &contracts) {
            Err(vb_storage::JournalError::ArtifactActionRegistry(actual)) => assert_eq!(actual, expected),
            other => return Err(format!("wrong storage result: {other:?}").into()),
        }
    }
    vb_compile::verify_compiled_artifact(&dense_workflow, &[contract()?])?;
    prepare_compiled_artifact(&dense_workflow, RuntimePolicy::Journaled, &[contract()?])?;
    Ok(())
}

#[test]
fn replacement_contracts_reject_before_enqueue_and_before_processing_state() -> TestResult {
    let workflow = action_workflow()?;
    let mut original = contract()?;
    original.side_effect = SideEffect::None;
    original.idempotency = Idempotency::DeterministicPure;
    original.retry_safety = RetrySafety::Safe;
    let directory = tempfile::tempdir()?;
    let journal = Arc::new(vb_storage::FjallJournal::open(directory.path(), None)?);
    vb_storage::submit_artifact_with_contracts(&journal, &workflow, RuntimePolicy::Strict, &[original.clone()])?;
    let mut replacement = original.clone();
    replacement.side_effect = SideEffect::Writes;
    replacement.idempotency = Idempotency::AtLeastOnceExternal;
    replacement.retry_safety = RetrySafety::Unsafe;
    assert!(prepare_compiled_artifact(&workflow, RuntimePolicy::Strict, &[replacement.clone()]).is_err());
    let mut runtime = runtime(journal.clone())?;
    let expected = Err(RuntimeError::AdmissionArtifactInvalid { digest: workflow.digest() });
    assert_eq!(runtime.submit_direct_with_grants_and_contracts(RunId::new(100), workflow.clone(), CapabilitySet::empty(), Box::new([replacement.clone()])), expected);
    assert_eq!(runtime.submit_direct_with_inputs_grants_and_contracts(RunId::new(101), workflow.clone(), Box::new([]), CapabilitySet::empty(), Box::new([replacement.clone()])), expected);
    assert_eq!(runtime.pending_boundary_snapshot(10).command_queue_depth(), 0);
    assert!(journal.events_for_run(RunId::new(100))?.is_empty());
    let mut shard = Shard::new_with_journal(config(), StorageRuntimeJournal::shared_strict(journal.clone()));
    shard.enqueue(ShardCommand::SubmitWithContracts { run: RunId::new(102), workflow: workflow.clone(), caps: CapabilitySet::empty(), action_contracts: Box::new([replacement]) })?;
    assert_eq!(shard.tick(), Err(RuntimeError::AdmissionArtifactInvalid { digest: workflow.digest() }));
    assert_eq!(shard.active_run_count(), 0);
    assert!(journal.events_for_run(RunId::new(102))?.is_empty());
    assert_eq!(runtime.submit_direct_with_grants_and_contracts(RunId::new(103), workflow, CapabilitySet::empty(), Box::new([original])), Ok(()));
    runtime.tick_all()?;
    assert_eq!(runtime.counters_snapshot().runs_submitted, 1);
    Ok(())
}

#[test]
fn substituted_ir_rejects_before_enqueue_and_before_processing_state() -> TestResult {
    let workflow = workflow()?;
    let directory = tempfile::tempdir()?;
    let journal = Arc::new(vb_storage::FjallJournal::open(directory.path(), None)?);
    vb_storage::submit_artifact(&journal, &workflow, RuntimePolicy::Strict)?;
    let mut parts = workflow.to_parts();
    *parts.constants.first_mut().ok_or("missing constant")? = vb_core::ConstValue::I64(99);
    let altered = vb_core::CompiledWorkflow::try_from_parts(parts)?;
    let mut runtime = runtime(journal.clone())?;
    let expected = Err(RuntimeError::AdmissionArtifactInvalid { digest: workflow.digest() });
    assert_eq!(runtime.submit_direct(RunId::new(110), altered.clone()), expected);
    assert_eq!(runtime.submit_compiled_with_inputs(RunId::new(111), altered.clone(), Box::new([])), expected);
    assert_eq!(runtime.pending_boundary_snapshot(10).command_queue_depth(), 0);
    let mut shard = Shard::new_with_journal(config(), StorageRuntimeJournal::shared_strict(journal.clone()));
    shard.enqueue(ShardCommand::Submit { run: RunId::new(112), workflow: altered, caps: CapabilitySet::empty() })?;
    assert_eq!(shard.tick(), Err(RuntimeError::AdmissionArtifactInvalid { digest: workflow.digest() }));
    assert_eq!(shard.active_run_count(), 0);
    assert!(journal.events_for_run(RunId::new(112))?.is_empty());
    assert_eq!(runtime.submit_direct(RunId::new(113), workflow), Ok(()));
    runtime.tick_all()?;
    assert_eq!(runtime.counters_snapshot().runs_completed, 1);
    Ok(())
}
