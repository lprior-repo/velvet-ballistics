use super::support::{TestResult, fixture, input_slot, record, submission};
use vb_core::{InputSlotKind, ListId, RuntimePolicy, SlotValue, Taint, ValueStore};
use vb_storage::{AcceptedArtifact, FjallJournal, JournalEvent, RetainedRunInputs};

fn assert_owned_graph(
    loaded: &vb_storage::OwnedRunInputsRecordV1,
    workflow: &vb_core::CompiledWorkflow,
) -> TestResult {
    let bundle = loaded.restore(workflow)?;
    assert!(bundle.matches_workflow(&workflow));
    assert_eq!(bundle.input_slots(), workflow.input_slots());
    assert_eq!(*bundle.resource_contract(), workflow.resource_contract());
    let entries = input_slot(workflow, InputSlotKind::List)?;
    assert_eq!(
        bundle.root(entries),
        Some((SlotValue::List(ListId::new(2)), Taint::Secret))
    );
    let store = ValueStore::from_checkpoint_v1(bundle.checkpoint().clone())?;
    assert_eq!(
        store.list(ListId::new(0))?,
        &[SlotValue::I64(7), SlotValue::I64(1), SlotValue::I64(7)]
    );
    assert_eq!(
        store.list_item_with_taint(ListId::new(0), 1)?,
        (SlotValue::I64(1), Taint::DerivedFromSecret)
    );
    assert_eq!(
        store.list_item_with_taint(ListId::new(0), 2)?,
        (SlotValue::I64(7), Taint::Secret)
    );
    Ok(())
}

#[test]
fn strict_owned_submission_reopens_with_exact_arenas_roots_taints_and_receipt() -> TestResult {
    let f = fixture(7, Taint::DerivedFromSecret, Taint::Secret)?;
    let expected = record(&f)?;
    let directory = tempfile::tempdir()?;
    {
        let journal = FjallJournal::open(directory.path(), None)?;
        journal.commit_run_submission(&submission(&f)?, RuntimePolicy::Strict)?;
    }
    let journal = FjallJournal::open(directory.path(), None)?;
    let stored = journal
        .compiled_ir(f.artifact.digest)?
        .ok_or("artifact missing")?;
    let artifact: AcceptedArtifact = postcard::from_bytes(&stored.ir)?;
    let workflow = vb_storage::admission::verify_accepted_artifact(&artifact)?;
    let source = journal
        .workflow_source(f.source.digest)?
        .ok_or("source missing")?;
    assert_eq!(source, f.source);
    let loaded = journal
        .owned_run_inputs(f.run, artifact.digest, source.digest, &workflow)?
        .ok_or("owned record missing")?;
    assert_eq!(loaded.input_digest(), expected.input_digest());
    assert_eq!(loaded.roots(), expected.roots());
    assert_eq!(loaded.checkpoint(), expected.checkpoint());
    assert_owned_graph(&loaded, &workflow)?;
    let events = journal.events_for_run(f.run)?;
    assert_eq!(events.len(), 2);
    let admission = events
        .iter()
        .find(|e| matches!(e, JournalEvent::RunAdmission { .. }))
        .ok_or("admission missing")?;
    loaded.validate_admission(admission)?;
    assert!(journal.run_header(f.run)?.is_some());
    assert!(matches!(
        journal.retained_run_inputs(f.run, artifact.digest, source.digest, &workflow)?,
        Some(RetainedRunInputs::Owned(_))
    ));
    assert_eq!(workflow.to_parts(), f.workflow.to_parts());
    Ok(())
}
