use super::*;

pub(super) fn raw_supported() -> FixtureResult {
    [RuntimePolicy::Strict, RuntimePolicy::Journaled]
        .into_iter()
        .try_for_each(raw_for_policy)
}

fn raw_for_policy(policy: RuntimePolicy) -> FixtureResult {
    let artifact = legacy(policy)?;
    assert_eq!(
        artifact.artifact_version,
        vb_storage::compat::CURRENT_IR_VERSION
    );
    assert_eq!(artifact.artifact_version, SchemaVersion::new(2, 0, 0));
    let workflow = vb_storage::admission::verify_accepted_artifact(&artifact)?;
    assert_eq!(workflow.symbol_payloads(), None);
    assert_eq!(artifact.ir, postcard::to_allocvec(&workflow.to_parts())?);
    let decision = exact_decision(
        SchemaVersion::new(2, 0, 0),
        CompatibilityClass::Identical,
        100,
    )?;
    assert_eq!(check_artifact_ir_version(&artifact)?, decision);
    let store = FixedAcceptedStore {
        artifact: artifact.clone(),
    };
    admitted(&store, &artifact, policy, decision);
    assert_eq!(store.artifact, artifact);
    Ok(())
}

#[test]
fn version_guard_known2_corrupt_escape_fails_inner_evidence() -> FixtureResult {
    [RuntimePolicy::Strict, RuntimePolicy::Journaled]
        .into_iter()
        .try_for_each(corrupt_for_policy)
}

fn corrupt_for_policy(policy: RuntimePolicy) -> FixtureResult {
    let mut artifact = bound(policy, SchemaVersion::new(2, 0, 0), &["key"])?;
    *artifact.ir.get_mut(3).ok_or("missing escape header")? = b'C';
    artifact = rebind(artifact)?;
    rejected(
        artifact.clone(),
        policy,
        AdmissionError::ArtifactInvalidProofFlag { flag: "evidence" },
    );
    rejected_persisted(&artifact)
}

fn rejected_persisted(artifact: &vb_storage::AcceptedArtifact) -> FixtureResult {
    let (_directory, journal) = persisted(artifact)?;
    let before = journal.compiled_ir(artifact.digest)?;
    let events = journal.events_for_run_full(RunId::new(1))?;
    let store = StorageArtifactStore::new(journal.clone());
    super::rejected_checked_store(&store, artifact);
    assert_eq!(journal.compiled_ir(artifact.digest)?, before);
    assert_eq!(journal.events_for_run_full(RunId::new(1))?, events);
    assert_eq!(journal.run_header(RunId::new(1))?, None);
    Ok(())
}
