#![forbid(unsafe_code)]

use std::{num::NonZeroUsize, path::Path, sync::Arc};
use vb_core::{RunId, RuntimePolicy};
use vb_runtime::{
    journal::{SharedRuntimeJournal, StorageRuntimeJournal},
    runtime::Runtime,
    shard::ShardConfig,
};
use vb_storage::FjallJournal;

#[path = "runtime_strict_resume_crash_prefix/checks.rs"]
mod checks;
#[path = "runtime_strict_resume_crash_prefix/corruption.rs"]
mod corruption;
#[path = "runtime_strict_resume_crash_prefix/cut.rs"]
mod cut;
#[path = "runtime_strict_resume_crash_prefix/fixture.rs"]
mod fixture;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
const RUN: RunId = RunId::new(47241);

fn runtime(journal: SharedRuntimeJournal) -> TestResult<Runtime> {
    Ok(Runtime::new(
        NonZeroUsize::MIN,
        ShardConfig {
            policy: RuntimePolicy::Strict,
            step_budget_per_tick: 16,
            ..ShardConfig::default()
        },
        journal,
    )?)
}

fn rows(path: &Path) -> TestResult<Vec<(String, Vec<u8>, Vec<u8>)>> {
    let database = fjall::Database::builder(path).open()?;
    let mut rows = Vec::new();
    for name in FjallJournal::declared_keyspaces() {
        let keyspace = database.keyspace(name, fjall::KeyspaceCreateOptions::default)?;
        for entry in keyspace.iter() {
            let (key, value) = entry.into_inner()?;
            rows.push((name.to_owned(), key.to_vec(), value.to_vec()));
        }
    }
    Ok(rows)
}

#[test]
fn durable_completion_and_interrupted_resume_recover_without_reissuing_action() -> TestResult {
    crash_prefix(true)
}

#[test]
fn durable_completion_before_continuation_recovers_without_reissuing_action() -> TestResult {
    crash_prefix(false)
}

fn crash_prefix(interrupted: bool) -> TestResult {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("crash");
    let (journal, live, issued) = fixture::crashed(&path, interrupted)?;
    let prefix = journal.events_for_run_full(RUN)?;
    checks::sequence_and_effects(&prefix, usize::from(interrupted))?;
    let receipt = journal.run_inputs(RUN)?;
    drop(live);
    drop(journal);
    let before = rows(&path)?;
    checks::empty_action_index(&before);
    let journal = Arc::new(FjallJournal::open(&path, None)?);
    let mut recovered = runtime(StorageRuntimeJournal::shared_strict(journal.clone()))?;
    let product = recovered.recover_product(RUN)?;
    assert!(
        matches!(
            product,
            vb_runtime::recovery::RuntimeRecoveryProduct::Resumable(_)
        ),
        "{product:?}"
    );
    recovered.recover_and_resume(RUN)?;
    fixture::pending_ask(&mut recovered, issued.ask)?;
    let reference = fixture::uninterrupted(&directory.path().join("reference"))?;
    checks::live_boundary(&recovered, &reference, issued)?;
    let events = journal.events_for_run_full(RUN)?;
    assert_eq!(events.get(..prefix.len()), Some(prefix.as_slice()));
    checks::sequence_and_effects(&events, usize::from(interrupted))?;
    assert_eq!(journal.run_inputs(RUN)?, receipt);
    drop(reference);
    drop(recovered);
    drop(journal);
    let at_ask = rows(&path)?;
    checks::empty_action_index(&at_ask);
    assert_ne!(at_ask, before);
    checks::old_ticket_refused_cold(&path, issued, &at_ask, &events, &receipt)?;
    Ok(())
}
