//! Concurrency test for the artifact acceptance sequence counter.
//!
//! Tests that 8 concurrent submissions to the same journal each receive a
//! unique sequence number (1-8), proving the write_lock serialization and
//! counter atomicity.

use std::sync::{Arc, Mutex};
use vb_core::RuntimePolicy;

use super::{minimal_workflow, temp_journal};
use crate::admission::*;

/// Helper to run a submission in a thread, returning the accepted artifact.
fn submit_thread(
    journal: Arc<Mutex<super::TestJournal>>,
    workflow: vb_core::CompiledWorkflow,
) -> std::thread::JoinHandle<Result<AcceptedArtifact, String>> {
    std::thread::spawn(move || {
        let guard = journal.lock().map_err(|e| format!("lock poisoned: {e}"))?;
        submit_artifact(&*guard, &workflow, RuntimePolicy::Relaxed)
            .map_err(|e| format!("submit_artifact failed: {e}"))
    })
}

/// 8 concurrent submissions must all succeed with unique sequences 1-8.
#[test]
fn eight_concurrent_submissions_get_unique_sequences_1_through_8() -> Result<(), String> {
    let journal = temp_journal().map_err(|e| format!("temp_journal: {e}"))?;
    let workflow = minimal_workflow()?;

    // Arc<Mutex<>> to share journal across threads. FjallJournal is not
    // Send+Sync itself; the internal write_lock provides serialization.
    let shared_journal = Arc::new(Mutex::new(journal));

    let mut handles = Vec::new();
    for _ in 0..8 {
        let wf = workflow.clone();
        let j = Arc::clone(&shared_journal);
        handles.push(submit_thread(j, wf));
    }

    // Join all threads, collecting results.
    let mut artifacts = Vec::new();
    for handle in handles {
        let result = handle
            .join()
            .map_err(|e| format!("thread join failed: {e:?}"))??;
        artifacts.push(result);
    }

    if artifacts.len() != 8 {
        return Err(format!("expected 8 artifacts, got {}", artifacts.len()));
    }

    // Extract sequences and digests.
    let mut seqs = Vec::new();
    for artifact in &artifacts {
        seqs.push(artifact.accepted_at_seq.get());
    }

    // Sequences must be unique and form the set {1, 2, 3, 4, 5, 6, 7, 8}.
    let mut sorted_seqs = seqs.clone();
    sorted_seqs.sort();
    let expected: Vec<u64> = (1..=8).collect();
    if sorted_seqs != expected {
        return Err(format!("sequences must be 1..=8, got {:?}", sorted_seqs));
    }

    // All digests must be unique.
    let mut digests = Vec::new();
    for artifact in &artifacts {
        digests.push(artifact.digest);
    }
    digests.sort();
    digests.dedup();
    if digests.len() != 8 {
        return Err(format!("expected 8 unique digests, got {}", digests.len()));
    }

    // Verify each artifact is readable and valid by calling
    // verify_accepted_artifact_for_policy on each.
    let journal_guard = shared_journal
        .lock()
        .map_err(|e| format!("lock poisoned: {e}"))?;
    for (i, artifact) in artifacts.iter().enumerate() {
        // Verify artifact can be read back from journal.
        let record = journal_guard
            .compiled_ir(artifact.digest)
            .map_err(|e| format!("compiled_ir read failed for artifact {i}: {e}"))?
            .ok_or_else(|| format!("artifact {i} not found in journal"))?;
        if record.digest != artifact.digest {
            return Err(format!("record digest mismatch for artifact {i}"));
        }

        // Verify artifact is valid for its policy.
        verify_accepted_artifact_for_policy(artifact, RuntimePolicy::Relaxed)
            .map_err(|e| format!("verify failed for artifact {i}: {e}"))?;

        // Verify digest matches.
        let computed = accepted_artifact_digest(artifact)
            .map_err(|e| format!("accepted_artifact_digest failed for {i}: {e}"))?;
        if computed != artifact.digest {
            return Err(format!("digest mismatch for artifact {i}"));
        }
        if computed != artifact.verification.digest {
            return Err(format!("verification digest mismatch for artifact {i}"));
        }
    }

    // Verify next submission gets seq 9.
    let workflow = minimal_workflow()?;
    let next = submit_artifact(&*journal_guard, &workflow, RuntimePolicy::Relaxed)
        .map_err(|e| format!("seq 9 submission failed: {e}"))?;
    if next.accepted_at_seq.get() != 9 {
        return Err(format!(
            "next artifact should have seq 9, got {}",
            next.accepted_at_seq.get()
        ));
    }

    Ok(())
}
