use super::issuance_test_support::{PackedFixture, TestResult, minimal_workflow, read_counter};
use crate::{FjallJournal, RunHeaderRecord, RunInputsRecord, RunSubmission, WorkflowSourceRecord};
use vb_core::{RunId, RuntimePolicy, WorkflowDigest, WorkflowId};

fn source() -> WorkflowSourceRecord {
    let source = b"atomic issuance source".to_vec();
    WorkflowSourceRecord {
        digest: WorkflowDigest::from_bytes(blake3::hash(&source).into()),
        source,
    }
}

fn header(run: RunId, digest: WorkflowDigest) -> RunHeaderRecord {
    RunHeaderRecord {
        run,
        workflow_id: WorkflowId::new(0),
        compiled_digest: digest,
        status: 0,
        accepted_at_ms: 123,
    }
}

fn scalar_submission(
    policy: RuntimePolicy,
    run: RunId,
) -> Result<RunSubmission, Box<dyn std::error::Error>> {
    let workflow = minimal_workflow()?;
    let artifact = crate::admission::prepare_compiled_artifact(&workflow, policy, &[])?;
    assert_eq!(artifact.accepted_at_seq.get(), 0);
    let source = source();
    let inputs = RunInputsRecord::new(
        run,
        artifact.digest,
        source.digest,
        &workflow,
        Box::default(),
    )?;
    Ok(RunSubmission::new(
        source,
        artifact,
        inputs,
        header(run, workflow.digest()),
    )?)
}

#[test]
fn atomic_submission_mixed_default_catalog_cli_order_survives_reopen() -> TestResult {
    let fixture = PackedFixture::new()?;
    for policy in [
        RuntimePolicy::Relaxed,
        RuntimePolicy::Journaled,
        RuntimePolicy::Strict,
    ] {
        let directory = tempfile::tempdir()?;
        let workflow = minimal_workflow()?;
        let journal = FjallJournal::open(directory.path(), None)?;
        assert_eq!(
            crate::admission::submit_artifact(&journal, &workflow, policy)?
                .accepted_at_seq
                .get(),
            1
        );
        assert_eq!(fixture.submit(&journal, policy)?.accepted_at_seq.get(), 2);
        journal.commit_run_submission(&scalar_submission(policy, RunId::new(201))?, policy)?;
        let retained = journal.run_inputs(RunId::new(201))?.ok_or("inputs")?;
        let ir = journal
            .compiled_ir(retained.artifact_digest())?
            .ok_or("artifact")?;
        let artifact: crate::AcceptedArtifact = postcard::from_bytes(&ir.ir)?;
        assert_eq!(artifact.accepted_at_seq.get(), 3);
        assert_eq!(read_counter(&journal)?, Some(3));
        drop(journal);
        let journal = FjallJournal::open(directory.path(), None)?;
        assert_eq!(read_counter(&journal)?, Some(3));
        assert_eq!(fixture.submit(&journal, policy)?.accepted_at_seq.get(), 4);
        journal.commit_run_submission(&scalar_submission(policy, RunId::new(202))?, policy)?;
        let retained = journal
            .run_inputs(RunId::new(202))?
            .ok_or("reopened inputs")?;
        let ir = journal
            .compiled_ir(retained.artifact_digest())?
            .ok_or("reopened artifact")?;
        let artifact: crate::AcceptedArtifact = postcard::from_bytes(&ir.ir)?;
        assert_eq!(artifact.accepted_at_seq.get(), 5);
        assert_eq!(read_counter(&journal)?, Some(5));
    }
    Ok(())
}

#[test]
fn atomic_submission_owned_catalog_rebinds_complete_input_digest() -> TestResult {
    let fixture = PackedFixture::new()?;
    for policy in [
        RuntimePolicy::Relaxed,
        RuntimePolicy::Journaled,
        RuntimePolicy::Strict,
    ] {
        let directory = tempfile::tempdir()?;
        let journal = FjallJournal::open(directory.path(), None)?;
        let artifact = fixture.prepare(policy)?;
        assert_eq!(artifact.accepted_at_seq.get(), 0);
        let source = source();
        let run = RunId::new(203);
        let checkpoint = vb_core::value_store::ValueStore::with_max_slots(
            fixture.workflow.resource_contract().max_slots,
        )
        .checkpoint_v1();
        let bundle = vb_core::root_bundle::OwnedRootBundle::new(
            Box::default(),
            checkpoint,
            &fixture.workflow,
        )?;
        let inputs = crate::OwnedRunInputsRecordV1::new(
            run,
            artifact.digest,
            source.digest,
            &bundle,
            &fixture.workflow,
        )?;
        let old_input_digest = inputs.input_digest();
        let source_digest = source.digest;
        let submission = RunSubmission::owned_with_executor_catalog(
            source,
            artifact,
            inputs,
            header(run, fixture.workflow.digest()),
            crate::ExecutorCatalogSubmissionContext::new(
                &fixture.catalog,
                vb_core::CapabilitySet::from_grants(fixture.contract.required_capabilities.clone()),
            ),
        )?;
        journal.commit_run_submission_with_executor_catalog(
            &submission,
            policy,
            &fixture.catalog,
        )?;
        assert_eq!(read_counter(&journal)?, Some(1));
        let artifacts = journal.list_artifacts()?;
        let issued_digest = *artifacts.first().ok_or("issued artifact")?;
        let ir = journal
            .compiled_ir(issued_digest)?
            .ok_or("issued artifact row")?;
        let artifact: crate::AcceptedArtifact = postcard::from_bytes(&ir.ir)?;
        assert_eq!(artifact.accepted_at_seq.get(), 1);
        assert_eq!(artifact.digest, artifact.verification.digest);
        fixture.catalog.verify_artifact(&artifact, policy)?;
        let retained = journal
            .owned_run_inputs(run, artifact.digest, source_digest, &fixture.workflow)?
            .ok_or("owned inputs")?;
        assert_eq!(retained.artifact_digest(), artifact.digest);
        assert_ne!(retained.input_digest(), old_input_digest);
        assert_eq!(retained.restore(&fixture.workflow)?.roots(), bundle.roots());
        if policy != RuntimePolicy::Relaxed {
            let events = journal.events_for_run(run)?;
            retained.validate_admission(events.get(1).ok_or("admission")?)?;
        }
    }
    Ok(())
}

#[test]
fn atomic_submission_bounded_concurrent_issuance_is_unique() -> TestResult {
    let directory = tempfile::tempdir()?;
    let journal = std::sync::Arc::new(FjallJournal::open(directory.path(), None)?);
    let handles: Vec<_> = (0..4)
        .map(|offset| {
            let journal = journal.clone();
            std::thread::spawn(move || -> Result<(), String> {
                let submission = scalar_submission(RuntimePolicy::Strict, RunId::new(300 + offset))
                    .map_err(|e| e.to_string())?;
                journal
                    .commit_run_submission(&submission, RuntimePolicy::Strict)
                    .map_err(|e| e.to_string())
            })
        })
        .collect();
    for handle in handles {
        handle.join().map_err(|_| "submission thread panicked")??;
    }
    let mut sequences = Vec::new();
    for offset in 0..4 {
        let retained = journal
            .run_inputs(RunId::new(300 + offset))?
            .ok_or("inputs")?;
        let record = journal
            .compiled_ir(retained.artifact_digest())?
            .ok_or("artifact")?;
        let artifact: crate::AcceptedArtifact = postcard::from_bytes(&record.ir)?;
        sequences.push(artifact.accepted_at_seq.get());
    }
    sequences.sort_unstable();
    assert_eq!(sequences, [1, 2, 3, 4]);
    assert_eq!(read_counter(&journal)?, Some(4));
    Ok(())
}
