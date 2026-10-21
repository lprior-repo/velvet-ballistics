#![forbid(unsafe_code)]

mod artifact_evidence_support;
use artifact_evidence_support::*;
use vb_core::action::{Idempotency, RetrySafety, SideEffect};
use vb_core::capability::CapabilitySet;
use vb_core::{ActionId, CompiledNodeKind, RunId, RuntimePolicy};
use vb_runtime::runtime::Runtime;
use vb_runtime::shard::ShardConfig;
use vb_storage::admission::{prepare_compiled_artifact, verify_accepted_artifact};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn checked_artifact_accepts_registry_rejected_by_runtime() -> TestResult {
    let workflow = action_workflow()?;
    for contracts in [vec![contract()?], vec![contract()?, contract()?]] {
        vb_compile::verify_compiled_artifact(&workflow, &contracts)?;
        let artifact = prepare_compiled_artifact(&workflow, RuntimePolicy::Journaled, &contracts)?;
        verify_accepted_artifact(&artifact)?;
        let runtime = Runtime::new_for_tests_and_benchmarks_only(
            std::num::NonZeroUsize::new(1).ok_or("zero shards")?,
            ShardConfig { policy: RuntimePolicy::Relaxed, ..ShardConfig::default() },
        );
        let result = runtime.submit_direct_with_grants_and_contracts(
            RunId::new(1), workflow.clone(), CapabilitySet::empty(), contracts.into_boxed_slice(),
        );
        println!("artifact and compiler accepted registry; runtime result: {result:?}");
        assert!(matches!(result, Err(vb_runtime::RuntimeError::ContractValidationError { .. })));
    }
    Ok(())
}

#[test]
fn strict_submit_accepts_contracts_different_from_verified_artifact() -> TestResult {
    let mut parts = action_workflow()?.to_parts();
    for node in &mut parts.nodes {
        if let CompiledNodeKind::Do { action, .. } = &mut node.kind {
            *action = ActionId::new(0);
        }
    }
    let workflow = canonicalize(parts)?;
    let mut verified_contract = contract()?;
    verified_contract.id = ActionId::new(0);
    verified_contract.side_effect = SideEffect::None;
    verified_contract.idempotency = Idempotency::DeterministicPure;
    verified_contract.retry_safety = RetrySafety::Safe;
    let dir = tempfile::tempdir()?;
    let journal = std::sync::Arc::new(vb_storage::FjallJournal::open(dir.path(), None)?);
    let artifact = vb_storage::submit_artifact_with_contracts(
        &journal, &workflow, RuntimePolicy::Strict, &[verified_contract.clone()],
    )?;
    verify_accepted_artifact(&artifact)?;
    let mut submitted_contract = verified_contract;
    submitted_contract.side_effect = SideEffect::Writes;
    submitted_contract.idempotency = Idempotency::AtLeastOnceExternal;
    submitted_contract.retry_safety = RetrySafety::Unsafe;
    assert!(prepare_compiled_artifact(&workflow, RuntimePolicy::Strict, &[submitted_contract.clone()]).is_err());
    let runtime = Runtime::new(
        std::num::NonZeroUsize::new(1).ok_or("zero shards")?,
        ShardConfig { policy: RuntimePolicy::Strict, ..ShardConfig::default() },
        vb_runtime::journal::StorageRuntimeJournal::shared_strict(journal),
    );
    let result = runtime.submit_direct_with_grants_and_contracts(
        RunId::new(2), workflow, CapabilitySet::empty(), Box::new([submitted_contract]),
    );
    println!("strict submit with replacement Writes/AtLeastOnceExternal/Unsafe contract: {result:?}");
    assert!(result.is_ok());
    Ok(())
}

#[test]
fn strict_submit_accepts_payload_different_from_verified_artifact() -> TestResult {
    let workflow = workflow()?;
    let dir = tempfile::tempdir()?;
    let journal = std::sync::Arc::new(vb_storage::FjallJournal::open(dir.path(), None)?);
    vb_storage::submit_artifact(&journal, &workflow, RuntimePolicy::Strict)?;
    let mut parts = workflow.to_parts();
    *parts.constants.first_mut().ok_or("missing constant")? = vb_core::ConstValue::I64(99);
    // Deliberately preserve the accepted workflow's claimed digest.
    let altered = vb_core::CompiledWorkflow::try_from_parts(parts)?;
    assert!(vb_compile::verify_compiled_artifact(&altered, &[]).is_err());
    let runtime = Runtime::new(
        std::num::NonZeroUsize::new(1).ok_or("zero shards")?,
        ShardConfig { policy: RuntimePolicy::Strict, ..ShardConfig::default() },
        vb_runtime::journal::StorageRuntimeJournal::shared_strict(journal),
    );
    let result = runtime.submit_direct(RunId::new(3), altered);
    println!("strict submit with different constant99 but accepted workflow's claimed digest: {result:?}");
    assert!(result.is_ok());
    Ok(())
}
