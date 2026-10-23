# Bead vb-vs630 Research: CLI Submission Artifact Sequence Stamp

## Domain Context

The velvet-ballistics durable execution engine maintains a global artifact acceptance sequence counter. This counter provides a total order across all accepted artifacts, enabling:
- Reproducible replay ordering
- Audit trails of acceptance sequence
- Deduplication and idempotency guarantees
- Consistent sequencing across multiple submission clients

The counter is stored in the `artifact_acceptance` keyspace under the key `b"last_issued"` as a big-endian u64 value.

## Current Implementation Analysis

### Two Submission Paths

The codebase has two distinct submission paths that publish artifacts:

1. **Default/API Path** (`commit_artifact` in `admission.rs`):
   - Used by direct API calls and the default submission flow
   - Acquires `write_lock`
   - Calls `issue_next_artifact_sequence` which internally:
     - Reads counter from keyspace
     - Increments counter
     - Stamps `accepted_at_seq` on artifact
     - Rebinds artifact digest via `bind_artifact_digest`
     - Encodes artifact
     - Builds `fjall::OwnedWriteBatch` with counter + artifact writes
     - Commits batch with policy durability

2. **CLI RunSubmission Path** (`commit_run_submission` in `submission.rs`):
   - Used by the `velvet-ballistics submit` command
   - Acquires `write_lock`
   - Validates the complete submission aggregate
   - Creates `JournalWriteBatch`
   - Stages: source, compiled IR, run inputs, run header, workflow index, status index, events
   - Commits batch with policy durability
   - Does NOT call `issue_next_artifact_sequence`
   - Leaves `accepted_at_seq` on the artifact unset

### Key Code Locations

| File | Function | Role |
|------|----------|------|
| `artifact_acceptance.rs` | `read_counter` | Read last issued sequence |
| `artifact_acceptance.rs` | `prepare_artifact` | Increment, stamp, rebind digest |
| `artifact_acceptance.rs` | `build_batch` | Build counter + artifact batch |
| `artifact_acceptance.rs` | `issue_artifact` | Full lock + issue flow (default path) |
| `admission.rs` | `commit_artifact` | Public default submission API |
| `submission.rs` | `commit_run_submission` | Public CLI submission API |
| `submission.rs` | `stage` | Stage all submission records |
| `submission.rs` | `stage_acceptance` | Stage acceptance events |

### Locking Analysis

Both paths acquire the same `journal.write_lock` (`std::sync::Mutex`). Calling `issue_next_artifact_sequence` from within `commit_run_submission` would deadlock because `issue_artifact` tries to acquire the lock again.

The bead explicitly states: "DO NOT call independently committing `issue_next_artifact_sequence` while outer lock held."

### Digest Cascade

When `accepted_at_seq` is stamped on the artifact, the artifact digest changes. This cascades to:
1. `AcceptedArtifact.digest` (direct)
2. `RunInputsRecord.artifact_digest` (referenced)
3. `RunAdmission` event `artifact_digest` (referenced)

The header's `compiled_digest` is the workflow IR digest, which does not change.

## Design Options

### Option A: Call `issue_next_artifact_sequence` from CLI Path

**Approach:** Have `commit_run_submission` call `issue_next_artifact_sequence` to stamp the artifact, then proceed with its batch.

**Tradeoffs:**
- Simple to implement
- Deadlocks: `issue_next_artifact_sequence` acquires `write_lock` itself
- Two separate transactions: one for counter+artifact, one for rest of submission
- If second transaction fails, counter was already incremented
- Violates bead requirement for single atomic transaction

**Verdict:** Not viable due to deadlock and transaction atomicity issues.

### Option B: Factor Out Lock-Free Prepare Helper

**Approach:** Extract the counter-read + increment + stamp + rebind logic from `issue_artifact` into a separate lock-free helper function. Both submission paths call this helper under their own lock.

**Tradeoffs:**
- Requires refactoring `artifact_acceptance.rs`
- Both paths use identical counter logic
- Single transaction for CLI path (counter write + all records in one batch)
- No deadlock risk
- Preserves existing behavior of default path

**Verdict:** Best approach. Matches bead requirements.

### Option C: Pre-increment Counter in CLI Path

**Approach:** Have the CLI caller increment the counter before calling `commit_run_submission`, passing the pre-incremented value.

**Tradeoffs:**
- Counter increment is not atomic with submission
- Race condition: two concurrent CLI submissions could get same counter value
- Caller must handle counter read/increment logic
- Violates encapsulation of counter management

**Verdict:** Not viable due to race conditions.

### Option D: Separate Counter Keyspace for CLI

**Approach:** Use a different counter for CLI submissions vs API submissions.

**Tradeoffs:**
- Breaks total ordering across all submissions
- Two counters to maintain
- Confusing semantics for replay and audit
- Violates bead requirement for "same global artifact acceptance counter"

**Verdict:** Not viable due to requirement for single global counter.

## Selected Design: Option B

Factor out the lock-free prepare helper from `issue_artifact`. The CLI submission path calls this helper under its existing `write_lock`, then includes the counter write in its submission batch.

### Implementation Steps (for implementation phase)

1. Create `pub(crate) fn prepare_artifact_with_sequence(journal, artifact)` in `artifact_acceptance.rs`
2. Modify `issue_artifact` to call the new helper (wrapper pattern)
3. Modify `commit_run_submission` to:
   - Call the helper under its existing lock
   - Rebind dependent digests (RunInputsRecord, RunAdmission event)
   - Stage counter write in submission batch
   - Re-encode stamped artifact
4. Add behavioral tests for:
   - Counter increment on CLI submission
   - Digest rebinding after stamping
   - Atomicity of combined transaction
   - Mixed default/CLI submission ordering

## Open Questions

### Counter Write in Submission Batch

The submission path uses `JournalWriteBatch` which has typed methods for standard record types. The counter write is a raw key-value insert to the `artifact_acceptance` keyspace. Options:
1. Add `put_artifact_sequence(u64)` method to `JournalWriteBatch`
2. Use raw Fjall operations from within submission path
3. Hybrid batch supporting both typed records and raw key-value writes

### RunInputsRecord Mutability

`RunInputsRecord` is designed as immutable with validation. Options for updating `artifact_digest`:
1. Add `set_artifact_digest` setter method
2. Reconstruct new record with updated digest
3. Make `artifact_digest` field `pub(crate)` and modify directly

### Artifact Re-encoding

After stamping, the artifact must be re-encoded. The submission path encodes the artifact in `stage()`. Options:
1. Encode stamped artifact in `stage()` instead of original
2. Provide stamped artifact to `RunSubmission::new()` and encode that
3. Delay encoding until after stamping in `commit()`

## Testing Strategy

### Behavioral Tests Required

1. **Counter Increment:** Submit via CLI, verify counter incremented, verify `accepted_at_seq` set
2. **Digest Rebinding:** Verify artifact digest, RunInputsRecord.artifact_digest, and RunAdmission.artifact_digest all match stamped artifact
3. **Atomicity:** Inject commit failure, verify no partial publication (no counter increment, no records)
4. **Mixed Submissions:** Alternate default and CLI submissions, verify monotonic counter
5. **Per-Run Event Seq:** Verify RunAccepted.seq=0, RunAdmission.seq=1 unchanged

### Test Fixtures

- Use existing test workflow fixtures from `submission_tests.rs`
- Create test journal instances for each test case
- Use `fail_next_batch_commit_for_test()` for atomicity tests

## Performance Considerations

- Counter read is a single keyspace get operation (fast)
- Counter write is included in the existing submission batch (no additional transaction overhead)
- No performance claim made for this bead

## Compatibility

- No new public API introduced
- Existing `commit_artifact` behavior unchanged
- Existing `commit_run_submission` signature unchanged
- Only internal implementation changes

## References

- `crates/vb_storage/src/journal/artifact_acceptance.rs` — counter implementation
- `crates/vb_storage/src/journal/admission.rs` — default submission path
- `crates/vb_storage/src/inputs/submission.rs` — CLI submission path
- `crates/velvet_ballistics/src/submit.rs` — CLI command implementation
- `crates/vb_storage/src/inputs/mod.rs` — RunInputsRecord definition