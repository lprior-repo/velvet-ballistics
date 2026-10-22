//! Issuance tests for executor catalog packed admission.
//! Verifies seq issuance, counter persistence, mixed policies, close/reopen,
//! pure prepare, and packed vs legacy behavior.

use super::issuance_test_support::*;
use crate::executor::ExecutorCatalog;
use crate::{AcceptedArtifact, FjallJournal, admission};
use vb_core::RuntimePolicy;

fn contract_slice(contract: &ActionContract) -> &[ActionContract] {
    std::slice::from_ref(contract)
}

fn pack_slice(pack: &ActionPack) -> &[ActionPack] {
    std::slice::from_ref(pack)
}

/// Pure prepare_artifact returns seq 0 and does not mutate the counter or store.
#[test]
fn pure_prepare_returns_seq_zero_counter_absent() -> Result<(), Box<dyn std::error::Error>> {
    let catalog = ExecutorCatalog::run_registered_conformance()?;
    let tj = TestJournal::open()?;
    let workflow = action_workflow()?;
    let contract = action_contract()?;
    let scope = register_scope();
    let pack = catalog.register_pack(&contract, scope, RecoveryClass::Blocked)?;

    let artifact = catalog.prepare_artifact(
        &workflow,
        RuntimePolicy::Relaxed,
        contract_slice(&contract),
        pack_slice(&pack),
    )?;

    assert_eq!(artifact.accepted_at_seq.get(), 0);
    let counter = read_counter(&tj.journal)?;
    assert_eq!(counter, None);
    let stored = tj.journal.compiled_ir(artifact.digest)?;
    assert_eq!(stored, None);
    Ok(())
}

/// Submitting a packed artifact with Relaxed policy succeeds and returns seq 1.
#[test]
fn packed_relaxed_submission_persists_and_verifies() -> Result<(), Box<dyn std::error::Error>> {
    let catalog = ExecutorCatalog::run_registered_conformance()?;
    let tj = TestJournal::open()?;
    let workflow = action_workflow()?;
    let contract = action_contract()?;
    let scope = register_scope();
    let pack = catalog.register_pack(&contract, scope, RecoveryClass::Blocked)?;

    let artifact = catalog.submit_artifact(
        &tj.journal,
        &workflow,
        RuntimePolicy::Relaxed,
        contract_slice(&contract),
        pack_slice(&pack),
    )?;

    assert_eq!(artifact.accepted_at_seq.get(), 1);
    let counter = read_counter(&tj.journal)?;
    assert_eq!(counter, Some(1));
    verify_artifact_readback(&tj.journal, artifact.digest, &artifact)?;
    Ok(())
}

/// Submitting a packed artifact with Journaled policy succeeds and returns seq 1.
#[test]
fn packed_journaled_submission_persists_and_verifies() -> Result<(), Box<dyn std::error::Error>> {
    let catalog = ExecutorCatalog::run_registered_conformance()?;
    let tj = TestJournal::open()?;
    let workflow = action_workflow()?;
    let contract = action_contract()?;
    let scope = register_scope();
    let pack = catalog.register_pack(&contract, scope, RecoveryClass::Blocked)?;

    let artifact = catalog.submit_artifact(
        &tj.journal,
        &workflow,
        RuntimePolicy::Journaled,
        contract_slice(&contract),
        pack_slice(&pack),
    )?;

    assert_eq!(artifact.accepted_at_seq.get(), 1);
    let counter = read_counter(&tj.journal)?;
    assert_eq!(counter, Some(1));
    verify_artifact_readback(&tj.journal, artifact.digest, &artifact)?;
    Ok(())
}

/// Submitting a packed artifact with Strict policy succeeds and returns seq 1.
#[test]
fn packed_strict_submission_persists_and_verifies() -> Result<(), Box<dyn std::error::Error>> {
    let catalog = ExecutorCatalog::run_registered_conformance()?;
    let tj = TestJournal::open()?;
    let workflow = action_workflow()?;
    let contract = action_contract()?;
    let scope = register_scope();
    let pack = catalog.register_pack(&contract, scope, RecoveryClass::Blocked)?;

    let artifact = catalog.submit_artifact(
        &tj.journal,
        &workflow,
        RuntimePolicy::Strict,
        contract_slice(&contract),
        pack_slice(&pack),
    )?;

    assert_eq!(artifact.accepted_at_seq.get(), 1);
    let counter = read_counter(&tj.journal)?;
    assert_eq!(counter, Some(1));
    verify_artifact_readback(&tj.journal, artifact.digest, &artifact)?;
    Ok(())
}

/// Multiple artifacts with mixed policies get unique seqs and digests.
#[test]
fn mixed_policy_submissions_get_unique_seqs_and_digests() -> Result<(), Box<dyn std::error::Error>>
{
    let catalog = ExecutorCatalog::run_registered_conformance()?;
    let tj = TestJournal::open()?;
    let workflow = action_workflow()?;
    let contract = action_contract()?;
    let scope = register_scope();
    let pack = catalog.register_pack(&contract, scope, RecoveryClass::Blocked)?;

    let art1 = catalog.submit_artifact(
        &tj.journal,
        &workflow,
        RuntimePolicy::Relaxed,
        contract_slice(&contract),
        pack_slice(&pack),
    )?;
    assert_eq!(art1.accepted_at_seq.get(), 1);
    verify_artifact_readback(&tj.journal, art1.digest, &art1)?;

    let art2 = catalog.submit_artifact(
        &tj.journal,
        &workflow,
        RuntimePolicy::Journaled,
        contract_slice(&contract),
        pack_slice(&pack),
    )?;
    assert_eq!(art2.accepted_at_seq.get(), 2);
    verify_artifact_readback(&tj.journal, art2.digest, &art2)?;

    let art3 = catalog.submit_artifact(
        &tj.journal,
        &workflow,
        RuntimePolicy::Strict,
        contract_slice(&contract),
        pack_slice(&pack),
    )?;
    assert_eq!(art3.accepted_at_seq.get(), 3);
    verify_artifact_readback(&tj.journal, art3.digest, &art3)?;

    assert_ne!(art1.digest, art2.digest);
    assert_ne!(art2.digest, art3.digest);
    assert_ne!(art1.digest, art3.digest);

    let counter = read_counter(&tj.journal)?;
    assert_eq!(counter, Some(3));
    Ok(())
}

/// Counter persists across journal close and reopen.
#[test]
fn counter_persists_across_close_and_reopen() -> Result<(), Box<dyn std::error::Error>> {
    let catalog = ExecutorCatalog::run_registered_conformance()?;
    let tj1 = TestJournal::open()?;
    let workflow = action_workflow()?;
    let contract = action_contract()?;
    let scope = register_scope();
    let pack = catalog.register_pack(&contract, scope, RecoveryClass::Blocked)?;

    let art1 = catalog.submit_artifact(
        &tj1.journal,
        &workflow,
        RuntimePolicy::Relaxed,
        contract_slice(&contract),
        pack_slice(&pack),
    )?;
    assert_eq!(art1.accepted_at_seq.get(), 1);
    let path = tj1.path().to_path_buf();
    drop(tj1);

    let journal2 = FjallJournal::open(&path, None)?;
    let art2 = catalog.submit_artifact(
        &journal2,
        &workflow,
        RuntimePolicy::Journaled,
        contract_slice(&contract),
        pack_slice(&pack),
    )?;
    assert_eq!(art2.accepted_at_seq.get(), 2);
    let art3 = catalog.submit_artifact(
        &journal2,
        &workflow,
        RuntimePolicy::Strict,
        contract_slice(&contract),
        pack_slice(&pack),
    )?;
    assert_eq!(art3.accepted_at_seq.get(), 3);
    drop(journal2);

    let journal3 = FjallJournal::open(&path, None)?;
    verify_artifact_readback(&journal3, art1.digest, &art1)?;
    verify_artifact_readback(&journal3, art2.digest, &art2)?;
    verify_artifact_readback(&journal3, art3.digest, &art3)?;
    let counter = read_counter(&journal3)?;
    assert_eq!(counter, Some(3));
    drop(journal3);
    Ok(())
}

/// Legacy and packed submissions share the same counter.
#[test]
fn legacy_and_packed_submissions_share_counter() -> Result<(), Box<dyn std::error::Error>> {
    let catalog = ExecutorCatalog::run_registered_conformance()?;
    let tj = TestJournal::open()?;
    let workflow = action_workflow()?;
    let contract = action_contract()?;
    let scope = register_scope();
    let pack = catalog.register_pack(&contract, scope, RecoveryClass::Blocked)?;

    let legacy = admission::submit_artifact_with_contracts(
        &tj.journal,
        &workflow,
        RuntimePolicy::Relaxed,
        contract_slice(&contract),
    )?;
    assert_eq!(legacy.accepted_at_seq.get(), 1);

    let packed = catalog.submit_artifact(
        &tj.journal,
        &workflow,
        RuntimePolicy::Relaxed,
        contract_slice(&contract),
        pack_slice(&pack),
    )?;
    assert_eq!(packed.accepted_at_seq.get(), 2);

    verify_artifact_readback(&tj.journal, legacy.digest, &legacy)?;
    verify_artifact_readback(&tj.journal, packed.digest, &packed)?;

    let counter = read_counter(&tj.journal)?;
    assert_eq!(counter, Some(2));
    Ok(())
}
