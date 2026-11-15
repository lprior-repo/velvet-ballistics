use super::{
    support::{Case, TestResult},
    wire,
};
use vb_runtime::RuntimeError;
use vb_storage::{JournalError, OwnedInputError, RecordKind};

#[test]
fn strict_invalid_owned_source_prefix_has_no_runtime_or_durable_publication() -> TestResult {
    let case = Case::new(true)?;
    let mut forged = wire::fixture_wire(&case)?;
    assert_eq!(forged.body.version, 2);
    *forged
        .body
        .checkpoint
        .symbols
        .first_mut()
        .ok_or("prefix symbol missing")? = "foreign-prefix".into();
    wire::redigest(&mut forged)?;
    let bytes = wire::encode(&forged, RecordKind::RunOwnedInputs)?;
    assert!(matches!(
        wire::decode(&case, &bytes),
        Err(JournalError::OwnedInputs(
            OwnedInputError::NonCanonicalRoots
        ))
    ));
    let key = vb_storage::keys::run_header_key(case.run)?;
    {
        let database = fjall::Database::builder(case.directory.path()).open()?;
        let inputs = database.keyspace(
            vb_storage::constants::KEYSPACE_RUN_INPUTS,
            fjall::KeyspaceCreateOptions::default,
        )?;
        inputs.insert(key, bytes.as_slice())?;
        database.persist(fjall::PersistMode::SyncAll)?;
    }
    let journal = case.open()?;
    let before = journal.events_for_run_full(case.run)?;
    let runtime = case.runtime(journal.clone())?;
    let boundary = runtime.pending_boundary_snapshot(8);
    let counters = runtime.counters_snapshot();
    assert!(
        matches!(runtime.activate_owned_submission(case.run,case.source.digest),Err(RuntimeError::StorageJournalAppend {source}) if matches!(*source,JournalError::OwnedInputs(OwnedInputError::NonCanonicalRoots)))
    );
    assert_eq!(runtime.pending_boundary_snapshot(8), boundary);
    assert_eq!(runtime.counters_snapshot(), counters);
    assert_eq!(
        runtime.inspect_run_state(case.run).map(|_| ()),
        Err(RuntimeError::RunNotFound)
    );
    assert_eq!(journal.events_for_run_full(case.run)?, before);
    drop(runtime);
    drop(journal);
    let database = fjall::Database::builder(case.directory.path()).open()?;
    let inputs = database.keyspace(
        vb_storage::constants::KEYSPACE_RUN_INPUTS,
        fjall::KeyspaceCreateOptions::default,
    )?;
    assert_eq!(
        inputs
            .get(key)?
            .ok_or("forged record disappeared")?
            .as_ref(),
        bytes.as_slice()
    );
    Ok(())
}
