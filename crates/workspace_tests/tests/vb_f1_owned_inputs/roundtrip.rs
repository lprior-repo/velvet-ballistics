#[path = "event_assertions.rs"]
mod event_assertions;
#[path = "graph_assertions.rs"]
mod graph_assertions;

use super::support::{Fixture, TestResult, fixture, record, submission};
use vb_core::{CompiledWorkflow, RuntimePolicy, Taint};
use vb_storage::{
    AcceptedArtifact, FjallJournal, OwnedRunInputsRecordV1, RetainedRunInputs, WorkflowSourceRecord,
};

struct Reopened {
    artifact: AcceptedArtifact,
    workflow: CompiledWorkflow,
    source: WorkflowSourceRecord,
    loaded: OwnedRunInputsRecordV1,
}

fn load_reopened_state(journal: &FjallJournal, f: &Fixture) -> TestResult<Reopened> {
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
    Ok(Reopened {
        artifact,
        workflow,
        source,
        loaded,
    })
}

fn assert_reopened_state(state: &Reopened, expected: &OwnedRunInputsRecordV1) -> TestResult {
    let Reopened {
        workflow, loaded, ..
    } = state;
    assert_eq!(loaded.input_digest(), expected.input_digest());
    assert_eq!(loaded.roots(), expected.roots());
    assert_eq!(loaded.checkpoint(), expected.checkpoint());
    graph_assertions::assert_owned_graph(loaded, workflow)?;
    Ok(())
}

fn assert_retained_state(journal: &FjallJournal, f: &Fixture, state: &Reopened) -> TestResult {
    let Reopened {
        artifact,
        workflow,
        source,
        ..
    } = state;
    assert!(journal.run_header(f.run)?.is_some());
    assert!(matches!(
        journal.retained_run_inputs(f.run, artifact.digest, source.digest, &workflow)?,
        Some(RetainedRunInputs::Owned(_))
    ));
    assert_eq!(workflow.to_parts(), f.workflow.to_parts());
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
    let state = load_reopened_state(&journal, &f)?;
    assert_reopened_state(&state, &expected)?;
    let events = journal.events_for_run(f.run)?;
    assert_eq!(events.len(), 3);
    event_assertions::assert_submission_events(&events, &f, &state.loaded, &expected)?;
    assert_retained_state(&journal, &f, &state)?;
    Ok(())
}
