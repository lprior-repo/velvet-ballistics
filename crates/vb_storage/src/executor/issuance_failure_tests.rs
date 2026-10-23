//! Issuance failure tests for executor catalog packed admission.
//! Verifies fault handling, counter state, and error wrapping through
//! the executor catalog's packed submission path.

use super::issuance_test_support::*;
use crate::JournalError;
use crate::executor::ExecutorError;
use vb_core::RuntimePolicy;

/// Strict/Journaled/Relaxed fresh journal: precommit persist failure yields
/// EXACT ExecutorError::Artifact(JournalError::StrictDurabilityFailed).
/// Counter unchanged (None) and compiled_ir empty BEFORE retry; retry gets
/// seq 1, counter 1, fixture.check full readback.
#[test]
fn strict_persist_failure_is_precommit() -> TestResult {
    let fixture = PackedFixture::new()?;
    check_persist_precommit(&fixture, RuntimePolicy::Strict)?;
    check_persist_precommit(&fixture, RuntimePolicy::Journaled)?;
    check_persist_precommit(&fixture, RuntimePolicy::Relaxed)?;
    Ok(())
}

fn check_persist_precommit(fixture: &PackedFixture, policy: RuntimePolicy) -> TestResult {
    let fresh = open_test_journal()?;
    fresh.1.fail_next_persist_for_test();
    let result = fixture.submit(&fresh.1, policy);
    assert!(
        matches!(
            &result,
            Err(ExecutorError::Artifact(
                JournalError::StrictDurabilityFailed
            ))
        ),
        "expected StrictDurabilityFailed, got: {result:?}"
    );
    assert_eq!(read_counter(&fresh.1)?, None);
    assert!(fresh.1.compiled_ir.iter().next().is_none());
    let artifact = fixture.submit(&fresh.1, policy)?;
    assert_eq!(artifact.accepted_at_seq.get(), 1);
    assert_eq!(read_counter(&fresh.1)?, Some(1));
    fixture.check(&fresh.1, &artifact, policy)
}

/// Injected batch commit failure is precommit: EXACT
/// ExecutorError::Artifact(JournalError::Fjall(fjall::Error::Io(io))) with
/// io.kind() == Other. Counter and compiled IR unchanged before retry;
/// retry yields seq 1 and full readback.
#[test]
fn batch_commit_failure_is_precommit() -> TestResult {
    let fixture = PackedFixture::new()?;
    let (_dir, journal) = open_test_journal()?;

    journal.fail_next_batch_commit_for_test();
    let result = fixture.submit(&journal, RuntimePolicy::Strict);
    assert!(
        matches!(
            &result,
            Err(ExecutorError::Artifact(JournalError::Fjall(e)))
                if matches!(
                    e,
                    fjall::Error::Io(io) if io.kind() == std::io::ErrorKind::Other
                )
        ),
        "expected Fjall(Io(Other)) wrapped in Artifact, got: {result:?}"
    );
    assert_eq!(read_counter(&journal)?, None);
    assert!(journal.compiled_ir.iter().next().is_none());

    let artifact = fixture.submit(&journal, RuntimePolicy::Strict)?;
    assert_eq!(artifact.accepted_at_seq.get(), 1);
    fixture.check(&journal, &artifact, RuntimePolicy::Strict)
}

/// Injected compiled IR readback failure is POSTCOMMIT: counter advances to 1,
/// artifact record is persisted, but submission yields EXACT
/// ExecutorError::Corrupt (NOT ArtifactMalformed). Followup submission
/// succeeds at seq 2.
#[test]
fn compiled_ir_readback_failure_is_postcommit() -> TestResult {
    let fixture = PackedFixture::new()?;
    let (_dir, journal) = open_test_journal()?;

    let mut expected = fixture.prepare(RuntimePolicy::Strict)?;
    expected.accepted_at_seq = crate::EventSeq::new(1);
    expected.digest = crate::admission::accepted_artifact_digest(&expected)?;
    expected.verification.digest = expected.digest;

    journal.fail_next_compiled_ir_readback_for_test();
    let result = fixture.submit(&journal, RuntimePolicy::Strict);
    assert!(
        matches!(&result, Err(ExecutorError::Corrupt)),
        "expected Corrupt, got: {result:?}"
    );

    assert_eq!(read_counter(&journal)?, Some(1));
    fixture.check(&journal, &expected, RuntimePolicy::Strict)?;

    let artifact2 = fixture.submit(&journal, RuntimePolicy::Strict)?;
    assert_eq!(artifact2.accepted_at_seq.get(), 2);
    fixture.check(&journal, &artifact2, RuntimePolicy::Strict)
}

/// Packed submission with invalid empty packs fails with EXACT
/// ExecutorError::PackMismatch before any counter or compiled_ir change.
#[test]
fn empty_packs_yield_pack_mismatch() -> TestResult {
    let fixture = PackedFixture::new()?;
    let (_dir, journal) = open_test_journal()?;

    let result = fixture.catalog.submit_artifact(
        &journal,
        &fixture.workflow,
        RuntimePolicy::Relaxed,
        std::slice::from_ref(&fixture.contract),
        &[],
    );
    assert!(
        matches!(&result, Err(ExecutorError::PackMismatch)),
        "expected PackMismatch for empty packs, got: {result:?}"
    );

    assert_eq!(read_counter(&journal)?, None);
    assert!(journal.compiled_ir.iter().next().is_none());
    Ok(())
}
