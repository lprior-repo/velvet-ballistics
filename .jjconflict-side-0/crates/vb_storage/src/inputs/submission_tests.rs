use super::*;
use crate::{JournalEvent, RunHeaderRecord, WorkflowSourceRecord};
use vb_core::RuntimePolicy;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[path = "submission_version_test_support.rs"]
mod version_support;

fn submission(
    policy: RuntimePolicy,
    value: i64,
) -> Result<RunSubmission, Box<dyn std::error::Error>> {
    submission_with_version(policy, value, crate::compat::CURRENT_IR_VERSION)
}

fn submission_with_version(
    policy: RuntimePolicy,
    value: i64,
    version: crate::compat::SchemaVersion,
) -> Result<RunSubmission, Box<dyn std::error::Error>> {
    let workflow = super::tests::workflow()?;
    let mut artifact = crate::admission::prepare_compiled_artifact(&workflow, policy, &[])?;
    artifact.artifact_version = version;
    crate::admission::bind_artifact_digest(&mut artifact)?;
    Ok(version_support::construct(&workflow, artifact, value)?)
}

fn assert_empty(journal: &FjallJournal) -> TestResult {
    for keyspace in [
        &journal.workflow_source,
        &journal.compiled_ir,
        &journal.run_header,
        &journal.run_inputs,
        &journal.events,
        &journal.index_status,
        &journal.index_workflow,
    ] {
        assert!(keyspace.is_empty()?);
    }
    Ok(())
}

#[test]
fn injected_atomic_commit_failure_exposes_no_record_or_index_and_retry_succeeds() -> TestResult {
    for policy in [
        RuntimePolicy::Relaxed,
        RuntimePolicy::Journaled,
        RuntimePolicy::Strict,
    ] {
        let directory = tempfile::tempdir()?;
        let journal = FjallJournal::open(directory.path(), None)?;
        let submission = submission(policy, 73)?;
        journal.fail_next_batch_commit_for_test();
        assert!(matches!(
            journal.commit_run_submission(&submission, policy),
            Err(JournalError::Fjall(_))
        ));
        assert_empty(&journal)?;
        journal.commit_run_submission(&submission, policy)?;
        let retained = journal.run_inputs(RunId::new(7))?.ok_or("missing inputs")?;
        assert_eq!(retained.inputs(), [(SlotIdx::new(0), SlotValue::I64(73))]);
        assert!(journal.compiled_ir(retained.artifact_digest())?.is_some());
        assert!(journal.workflow_source(retained.source_digest())?.is_some());
        assert!(journal.run_header(retained.run())?.is_some());
        let expected_event_count = if policy == RuntimePolicy::Relaxed {
            0
        } else {
            3
        };
        let events = journal.events_for_run(retained.run())?;
        assert_eq!(events.len(), expected_event_count);
        if policy != RuntimePolicy::Relaxed {
            let decision_event = events
                .get(2)
                .ok_or("missing compatibility decision event")?;
            assert!(matches!(
                decision_event,
                JournalEvent::CompatibilityDecisionEvent { decision, .. }
                    if decision.class == crate::compat::CompatibilityClass::Identical
                        && decision.boundary == crate::compat::CompatBoundary::IrVersion
            ));
        }
        assert!(
            journal.has_workflow_index_entry(crate::keys::index_workflow_key(
                vb_core::WorkflowId::new(0),
                retained.run()
            )?)?
        );
        assert!(
            journal.has_status_index_entry(crate::keys::index_status_key(
                crate::IndexStatusState::Submitted,
                123,
                retained.run()
            )?)?
        );
    }
    Ok(())
}

#[test]
fn future_major_ir_version_is_rejected_before_any_write() -> TestResult {
    [
        RuntimePolicy::Journaled,
        RuntimePolicy::Strict,
        RuntimePolicy::Relaxed,
    ]
    .into_iter()
    .try_for_each(version_support::future_rejection)
}

#[test]
fn supported_v2_rejects_legacy_wire_before_any_write() -> TestResult {
    [
        RuntimePolicy::Journaled,
        RuntimePolicy::Strict,
        RuntimePolicy::Relaxed,
    ]
    .into_iter()
    .try_for_each(version_support::wrong_wire_rejection)
}

#[test]
fn relaxed_supported_v1_submission_preserves_no_events() -> TestResult {
    let workflow = super::tests::workflow()?;
    assert_eq!(workflow.symbol_payloads(), None);
    version_support::relaxed_submission(&workflow, crate::compat::SchemaVersion::new(1, 0, 0))
}

#[test]
fn relaxed_supported_v2_submission_preserves_no_events() -> TestResult {
    let workflow = version_support::bound_workflow()?;
    assert_eq!(
        workflow.symbol_payloads(),
        Some(Box::<[Box<str>]>::default().as_ref())
    );
    version_support::relaxed_submission(&workflow, crate::compat::SchemaVersion::new(2, 0, 0))
}

#[test]
fn colliding_run_cannot_replace_retained_input_or_existing_admission() -> TestResult {
    for policy in [
        RuntimePolicy::Relaxed,
        RuntimePolicy::Journaled,
        RuntimePolicy::Strict,
    ] {
        let directory = tempfile::tempdir()?;
        let journal = FjallJournal::open(directory.path(), None)?;
        journal.commit_run_submission(&submission(policy, 19)?, policy)?;
        let before = journal.run_inputs(RunId::new(7))?;
        let events_before = journal.events_for_run(RunId::new(7))?;
        assert!(matches!(
            journal.commit_run_submission(&submission(policy, 99)?, policy),
            Err(JournalError::RunAlreadyExists)
        ));
        assert_eq!(journal.run_inputs(RunId::new(7))?, before);
        assert_eq!(journal.events_for_run(RunId::new(7))?, events_before);
    }
    Ok(())
}

#[test]
fn journaled_artifact_cannot_be_committed_as_strict_or_relaxed() -> TestResult {
    let directory = tempfile::tempdir()?;
    let journal = FjallJournal::open(directory.path(), None)?;
    let submission = submission(RuntimePolicy::Journaled, 1)?;
    assert!(matches!(
        journal.commit_run_submission(&submission, RuntimePolicy::Strict),
        Err(JournalError::ArtifactMalformed)
    ));
    assert_empty(&journal)?;
    assert!(matches!(
        journal.commit_run_submission(&submission, RuntimePolicy::Relaxed),
        Err(JournalError::ArtifactMalformed)
    ));
    assert_empty(&journal)?;
    Ok(())
}
