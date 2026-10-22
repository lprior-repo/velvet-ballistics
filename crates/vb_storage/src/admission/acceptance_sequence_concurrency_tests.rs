//! Concurrency test for the artifact acceptance sequence counter.
//!
//! Tests that 8 concurrent submissions to the same journal each receive a
//! unique sequence number (1-8), proving the write_lock serialization and
//! counter atomicity. Status: PENDING compiler and test execution by coordinator.

use std::collections::HashSet;
use vb_core::RuntimePolicy;

use super::{minimal_workflow, temp_journal};
use crate::admission::*;

/// 8 concurrent submissions must all succeed with unique sequences 1-8.
#[test]
fn eight_concurrent_submissions_get_unique_sequences_1_through_8() -> Result<(), String> {
    let journal = temp_journal().map_err(|e| format!("temp_journal: {e}"))?;
    let workflow = minimal_workflow()?;

    let journal_ref = &journal;
    let artifacts = std::thread::scope(|scope| {
        let mut handles = Vec::with_capacity(8);
        for _ in 0..8 {
            let wf = workflow.clone();
            let handle = scope.spawn(move || {
                submit_artifact(journal_ref, &wf, RuntimePolicy::Relaxed)
                    .map_err(|e| format!("submit_artifact failed: {e}"))
            });
            handles.push(handle);
        }

        let mut results = Vec::with_capacity(8);
        for handle in handles {
            let artifact = handle
                .join()
                .map_err(|e| format!("thread join failed: {e:?}"))??;
            results.push(artifact);
        }
        Ok(results)
    })?;

    if artifacts.len() != 8 {
        return Err(format!("expected 8 artifacts, got {}", artifacts.len()));
    }

    // Sequences must be unique and form the set {1, 2, 3, 4, 5, 6, 7, 8}.
    let mut seqs = Vec::with_capacity(8);
    for artifact in &artifacts {
        seqs.push(artifact.accepted_at_seq.get());
    }
    let mut sorted_seqs = seqs.clone();
    sorted_seqs.sort();
    let expected: Vec<u64> = (1..=8).collect();
    if sorted_seqs != expected {
        return Err(format!("sequences must be 1..=8, got {:?}", sorted_seqs));
    }

    // All digests must be unique (HashSet, not Ord sort).
    let mut digests = HashSet::new();
    for artifact in &artifacts {
        digests.insert(artifact.digest);
    }
    if digests.len() != 8 {
        return Err(format!("expected 8 unique digests, got {}", digests.len()));
    }

    // Full readback: decode each artifact and verify identity.
    for (i, artifact) in artifacts.iter().enumerate() {
        let record = journal
            .compiled_ir(artifact.digest)
            .map_err(|e| format!("compiled_ir read failed for artifact {i}: {e}"))?
            .ok_or_else(|| format!("artifact {i} not found in journal"))?;
        if record.digest != artifact.digest {
            return Err(format!("record digest mismatch for artifact {i}"));
        }
        let decoded: AcceptedArtifact = postcard::from_bytes(&record.ir)
            .map_err(|e| format!("postcard decode failed for artifact {i}: {e}"))?;
        if decoded != *artifact {
            return Err(format!("decoded artifact differs from original at {i}"));
        }
        if record.digest != artifact.digest {
            return Err(format!("record digest mismatch for artifact {i}"));
        }

        // Canonical verification assertions.
        verify_accepted_artifact_for_policy(artifact, RuntimePolicy::Relaxed)
            .map_err(|e| format!("verify failed for artifact {i}: {e}"))?;
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
    let next = submit_artifact(&journal, &workflow, RuntimePolicy::Relaxed)
        .map_err(|e| format!("seq 9 submission failed: {e}"))?;
    if next.accepted_at_seq.get() != 9 {
        return Err(format!(
            "next artifact should have seq 9, got {}",
            next.accepted_at_seq.get()
        ));
    }

    Ok(())
}
