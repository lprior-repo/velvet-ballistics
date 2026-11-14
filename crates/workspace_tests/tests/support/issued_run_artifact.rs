use vb_core::RunId;
use vb_storage::{AcceptedArtifact, FjallJournal, JournalEvent};

pub(crate) fn read(
    journal: &FjallJournal,
    run: RunId,
    prepared: &AcceptedArtifact,
) -> Result<AcceptedArtifact, Box<dyn std::error::Error>> {
    let admissions: Vec<_> = journal
        .events_for_run_full(run)?
        .into_iter()
        .filter_map(|event| match event {
            JournalEvent::RunAdmissionBudget {
                artifact_digest, ..
            } => Some(artifact_digest),
            _ => None,
        })
        .collect();
    assert_eq!(admissions.len(), 1);
    let digest = *admissions.first().ok_or("issued admission")?;
    let record = journal.compiled_ir(digest)?.ok_or("issued artifact")?;
    let issued: AcceptedArtifact = postcard::from_bytes(&record.ir)?;
    assert!(issued.accepted_at_seq.get() > 0);
    let mut expected = prepared.clone();
    expected.accepted_at_seq = issued.accepted_at_seq;
    expected.digest = vb_storage::admission::accepted_artifact_digest(&expected)?;
    expected.verification.digest = expected.digest;
    assert_eq!(issued, expected);
    assert_eq!(issued.digest, digest);
    Ok(issued)
}
