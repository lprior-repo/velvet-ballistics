use super::*;
use vb_storage::compat::{CompatDecision, CompatibilityClass, SchemaVersion};

type FixtureResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[path = "version_guard_test_support.rs"]
mod support;
use support::*;

#[test]
fn version_guard_v2_current_exact_decision() -> FixtureResult {
    exact_decision(
        SchemaVersion::new(2, 0, 0),
        CompatibilityClass::Identical,
        100,
    )?;
    Ok(())
}

#[test]
fn version_guard_v2_minor_exact_decision() -> FixtureResult {
    exact_decision(
        SchemaVersion::new(2, 1, 0),
        CompatibilityClass::ForwardCompatible,
        102,
    )?;
    Ok(())
}

#[test]
fn version_guard_v2_patch_exact_decision() -> FixtureResult {
    exact_decision(
        SchemaVersion::new(2, 0, 1),
        CompatibilityClass::ForwardCompatible,
        102,
    )?;
    Ok(())
}

#[test]
fn version_guard_unknown3_exact_classification() -> FixtureResult {
    assert_eq!(
        check_artifact_ir_version(&future(RuntimePolicy::Strict)?),
        Err(future_error())
    );
    Ok(())
}

#[test]
fn version_guard_unknown3_valid_body_both_routes() -> FixtureResult {
    [RuntimePolicy::Strict, RuntimePolicy::Journaled]
        .into_iter()
        .try_for_each(|policy| {
            rejected(future(policy)?, policy, future_error());
            Ok(())
        })
}

#[test]
fn version_guard_unknown3_malformed_body_before_decode() -> FixtureResult {
    [RuntimePolicy::Strict, RuntimePolicy::Journaled]
        .into_iter()
        .try_for_each(|policy| {
            let mut artifact = future(policy)?;
            artifact.ir.clear();
            rejected(rebind(artifact)?, policy, future_error());
            Ok(())
        })
}

#[test]
fn version_guard_proof_identity_precedes_unknown3() -> FixtureResult {
    let mut artifact = future(RuntimePolicy::Strict)?;
    artifact.verification.digest = WorkflowDigest::from_bytes([9; 32]);
    let expected = AdmissionError::ArtifactDigestMismatch {
        requested: artifact.digest,
        found: WorkflowDigest::from_bytes([9; 32]),
    };
    rejected(artifact, RuntimePolicy::Strict, expected);
    Ok(())
}

#[test]
fn version_guard_outer_checksum_precedes_unknown3() -> FixtureResult {
    let mut artifact = future(RuntimePolicy::Strict)?;
    artifact.ir.clear();
    rejected(
        artifact,
        RuntimePolicy::Strict,
        AdmissionError::ArtifactInvalidProofFlag { flag: "evidence" },
    );
    Ok(())
}

#[test]
fn version_guard_known2_wrong_wire_fails_inner_evidence() -> FixtureResult {
    let mut artifact = legacy(RuntimePolicy::Strict)?;
    artifact.artifact_version = SchemaVersion::new(2, 0, 0);
    rejected(
        rebind(artifact)?,
        RuntimePolicy::Strict,
        AdmissionError::ArtifactInvalidProofFlag { flag: "evidence" },
    );
    Ok(())
}

#[test]
fn version_guard_known2_malformed_body_fails_inner_evidence() -> FixtureResult {
    let mut artifact = bound(RuntimePolicy::Strict, SchemaVersion::new(2, 0, 0), &["key"])?;
    artifact.ir = vec![0x01, 0xff, 0x56, 0x42];
    rejected(
        rebind(artifact)?,
        RuntimePolicy::Strict,
        AdmissionError::ArtifactInvalidProofFlag { flag: "evidence" },
    );
    Ok(())
}

#[test]
fn version_guard_valid2_public_routes_preserve_held_versions() -> FixtureResult {
    [
        (
            SchemaVersion::new(2, 0, 0),
            CompatibilityClass::Identical,
            100,
        ),
        (
            SchemaVersion::new(2, 1, 0),
            CompatibilityClass::ForwardCompatible,
            102,
        ),
        (
            SchemaVersion::new(2, 0, 1),
            CompatibilityClass::ForwardCompatible,
            102,
        ),
    ]
    .into_iter()
    .try_for_each(|(held, class, code)| public_version(held, class, code))
}

#[test]
fn version_guard_valid2_some_empty_stays_bound() -> FixtureResult {
    let artifact = bound(RuntimePolicy::Strict, SchemaVersion::new(2, 0, 0), &[])?;
    let restored = vb_storage::admission::verify_accepted_artifact(&artifact)?;
    assert_eq!(
        restored.symbol_payloads(),
        Some(Box::<[Box<str>]>::default().as_ref())
    );
    let decision = exact_decision(
        SchemaVersion::new(2, 0, 0),
        CompatibilityClass::Identical,
        100,
    )?;
    let store = FixedAcceptedStore {
        artifact: artifact.clone(),
    };
    admitted(&store, &artifact, RuntimePolicy::Strict, decision);
    assert_eq!(store.artifact, artifact);
    Ok(())
}

#[test]
fn version_guard_checked_store_unknown_reader_rejects_at_load_unchanged() -> FixtureResult {
    let artifact = future(RuntimePolicy::Strict)?;
    let (_directory, journal) = persisted(&artifact)?;
    let before = journal.compiled_ir(artifact.digest)?;
    let events = journal.events_for_run_full(RunId::new(1))?;
    let store = StorageArtifactStore::new(journal.clone());
    assert_eq!(
        store.load_accepted_artifact(artifact.digest),
        Err(ArtifactEnvelopeError::ArtifactEvidenceInvalid)
    );
    rejected_checked_store(&store, &artifact);
    assert_eq!(journal.compiled_ir(artifact.digest)?, before);
    assert_eq!(journal.events_for_run_full(RunId::new(1))?, events);
    Ok(())
}

fn rejected_checked_store(store: &StorageArtifactStore, artifact: &vb_storage::AcceptedArtifact) {
    let expected = Err(AdmissionError::ArtifactInvalidProofFlag { flag: "evidence" });
    assert_eq!(
        admit_run(
            store,
            RuntimePolicy::Strict,
            artifact.digest,
            RunId::new(1),
            CapabilitySet::empty()
        ),
        expected
    );
    assert_eq!(
        admit_artifact_run(
            store,
            RuntimePolicy::Strict,
            RunId::new(1),
            artifact.digest,
            CapabilitySet::empty()
        ),
        expected
    );
}

#[test]
fn version_guard_checked_store_valid_v2_is_admitted() -> FixtureResult {
    let held = SchemaVersion::new(2, 1, 0);
    let artifact = bound(RuntimePolicy::Strict, held, &["key"])?;
    let (_directory, journal) = persisted(&artifact)?;
    let before = journal.compiled_ir(artifact.digest)?;
    let store = StorageArtifactStore::new(journal.clone());
    assert_eq!(store.load_accepted_artifact(artifact.digest)?, artifact);
    let decision = exact_decision(held, CompatibilityClass::ForwardCompatible, 102)?;
    admitted(&store, &artifact, RuntimePolicy::Strict, decision);
    assert_eq!(journal.compiled_ir(artifact.digest)?, before);
    assert_eq!(journal.events_for_run_full(RunId::new(1))?, Vec::new());
    Ok(())
}

#[test]
fn version_guard_relaxed_retains_no_load_behavior() {
    let store = MissingAcceptedArtifactStore;
    let digest = WorkflowDigest::from_bytes([7; 32]);
    let expected = Ok(RunAdmission::new(
        digest,
        RunId::new(1),
        CapabilitySet::empty(),
        RuntimePolicy::Relaxed,
    ));
    assert_eq!(simple(&store, RuntimePolicy::Relaxed, digest), expected);
    assert_eq!(full(&store, RuntimePolicy::Relaxed, digest), expected);
}
