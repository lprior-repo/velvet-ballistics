use super::*;
use crate::inputs::submission_tests::{assert_empty, submission};
use vb_core::RunId;
use vb_core::RuntimePolicy;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn counter(journal: &FjallJournal) -> Result<Option<Vec<u8>>, JournalError> {
    Ok(journal
        .artifact_acceptance
        .get(b"last_issued")?
        .map(|v| v.to_vec()))
}

#[test]
fn atomic_submission_issues_sequence_and_rebinds_all_policies() -> TestResult {
    for policy in [
        RuntimePolicy::Relaxed,
        RuntimePolicy::Journaled,
        RuntimePolicy::Strict,
    ] {
        let directory = tempfile::tempdir()?;
        let journal = FjallJournal::open(directory.path(), None)?;
        let prepared = submission(policy, 73)?;
        assert_eq!(prepared.artifact.accepted_at_seq.get(), 0);
        journal.commit_run_submission(&prepared, policy)?;
        let retained = journal.run_inputs(RunId::new(7))?.ok_or("inputs")?;
        let record = journal
            .compiled_ir(retained.artifact_digest())?
            .ok_or("artifact")?;
        let artifact: crate::AcceptedArtifact = postcard::from_bytes(&record.ir)?;
        assert_eq!(artifact.accepted_at_seq.get(), 1);
        assert_eq!(counter(&journal)?, Some(1u64.to_be_bytes().to_vec()));
        assert_eq!(artifact.digest, artifact.verification.digest);
        assert_eq!(artifact.digest, retained.artifact_digest());
        assert_ne!(artifact.digest, prepared.artifact.digest);
        assert_eq!(
            journal
                .run_header(retained.run())?
                .ok_or("header")?
                .compiled_digest,
            prepared.header.compiled_digest
        );
        crate::admission::verify_accepted_artifact_for_policy(&artifact, policy)?;
        let events = journal.events_for_run(retained.run())?;
        if policy == RuntimePolicy::Relaxed {
            assert!(events.is_empty());
        } else {
            assert!(
                matches!(events.first(), Some(crate::JournalEvent::RunAccepted { seq, .. }) if seq.get() == 0)
            );
            let admission = events.get(1).ok_or("admission")?;
            assert_eq!(admission.seq().get(), 1);
            retained.validate_admission(admission)?;
        }
        assert_eq!(prepared.artifact.accepted_at_seq.get(), 0);
    }
    Ok(())
}

#[test]
fn atomic_submission_precommit_refusals_preserve_counter_and_records() -> TestResult {
    for policy in [
        RuntimePolicy::Relaxed,
        RuntimePolicy::Journaled,
        RuntimePolicy::Strict,
    ] {
        for bytes in [vec![0; 7], vec![0; 9], u64::MAX.to_be_bytes().to_vec()] {
            let directory = tempfile::tempdir()?;
            let journal = FjallJournal::open(directory.path(), None)?;
            journal.artifact_acceptance.insert(b"last_issued", &bytes)?;
            let prepared = submission(policy, 73)?;
            let Err(error) = journal.commit_run_submission(&prepared, policy) else {
                return Err("invalid counter accepted".into());
            };
            match bytes.len() {
                8 => assert!(matches!(error, JournalError::SequenceOverflow)),
                len => assert!(
                    matches!(error, JournalError::MalformedKeyspaceRow { prefix: 0, expected_len: 8, actual_len } if actual_len == len)
                ),
            }
            assert_eq!(counter(&journal)?, Some(bytes));
            assert_empty(&journal)?;
        }
        let directory = tempfile::tempdir()?;
        let journal = FjallJournal::open(directory.path(), None)?;
        let prepared = submission(policy, 73)?;
        journal.fail_next_batch_commit_for_test();
        assert!(matches!(
            journal.commit_run_submission(&prepared, policy),
            Err(JournalError::Fjall(_))
        ));
        assert_eq!(counter(&journal)?, None);
        assert_empty(&journal)?;
        journal.commit_run_submission(&prepared, policy)?;
        assert_eq!(counter(&journal)?, Some(1u64.to_be_bytes().to_vec()));
        assert!(matches!(
            journal.commit_run_submission(&prepared, policy),
            Err(JournalError::RunAlreadyExists)
        ));
        assert_eq!(counter(&journal)?, Some(1u64.to_be_bytes().to_vec()));
    }
    Ok(())
}
