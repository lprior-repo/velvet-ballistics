//! Raw journal snapshot test support for F1 duplicate and F2 reopen full-state checks.

use crate::FjallJournal;

type Rows = Vec<(Vec<u8>, Vec<u8>)>;
type Snapshot = [Rows; 8];

fn iterate_keyspace(
    rows: impl Iterator<Item = fjall::Guard>,
) -> Result<Rows, Box<dyn std::error::Error>> {
    let mut result: Rows = Vec::new();
    for (i, guard) in rows.take(17).enumerate() {
        if i >= 16 {
            return Err("keyspace exceeds 16-row bound".into());
        }
        let (key, value) = guard.into_inner().map_err(|e| format!("into_inner: {e}"))?;
        result.push((key.to_vec(), value.to_vec()));
    }
    Ok(result)
}

pub(super) fn snapshot_journal(
    journal: &FjallJournal,
) -> Result<Snapshot, Box<dyn std::error::Error>> {
    let artifact_acceptance = iterate_keyspace(journal.artifact_acceptance.iter())?;
    assert!(
        artifact_acceptance.len() <= 16,
        "artifact_acceptance exceeds bound"
    );

    let workflow_source = iterate_keyspace(journal.workflow_source.iter())?;
    assert!(workflow_source.len() <= 16, "workflow_source exceeds bound");

    let compiled_ir = iterate_keyspace(journal.compiled_ir.iter())?;
    assert!(compiled_ir.len() <= 16, "compiled_ir exceeds bound");

    let run_header = iterate_keyspace(journal.run_header.iter())?;
    assert!(run_header.len() <= 16, "run_header exceeds bound");

    let run_inputs = iterate_keyspace(journal.run_inputs.iter())?;
    assert!(run_inputs.len() <= 16, "run_inputs exceeds bound");

    let events = iterate_keyspace(journal.events.iter())?;
    assert!(events.len() <= 16, "events exceeds bound");

    let index_status = iterate_keyspace(journal.index_status.iter())?;
    assert!(index_status.len() <= 16, "index_status exceeds bound");

    let index_workflow = iterate_keyspace(journal.index_workflow.iter())?;
    assert!(index_workflow.len() <= 16, "index_workflow exceeds bound");

    Ok([
        artifact_acceptance,
        workflow_source,
        compiled_ir,
        run_header,
        run_inputs,
        events,
        index_status,
        index_workflow,
    ])
}
