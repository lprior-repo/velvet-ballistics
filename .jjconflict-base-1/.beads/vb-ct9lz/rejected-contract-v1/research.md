# vb-ct9lz: Research notes

## Actual source code analysis

### Shared issuance helper (dependency qz)

Located at `crates/vb_storage/src/journal/artifact_acceptance.rs`. The helper is `pub(crate)` and already implemented:

```rust
pub(crate) fn issue_next_artifact_sequence(
    journal: &FjallJournal,
    artifact: AcceptedArtifact,
    policy: RuntimePolicy,
) -> Result<AcceptedArtifact, JournalError> {
    issue_artifact(journal, artifact, policy)
}
```

The internal `issue_artifact` function:
- Acquires `journal.write_lock`
- Reads the counter from `artifact_acceptance` keyspace at key `b"last_issued"`
- Increments (u64 big-endian bytes, must be exactly 8 bytes)
- Stamps `accepted_at_seq` on the artifact
- Rebonds the digest via `bind_artifact_digest`
- Encodes the artifact into a `CompiledIrRecord`
- Builds an `fjall::OwnedWriteBatch` with both counter and artifact writes
- Applies policy durability (SyncAll only for Strict)
- Commits the batch
- Returns the stamped artifact with correct seq and digest

### Current executor catalog implementation

Located at `crates/vb_storage/src/executor/packed.rs`. The current `submit_artifact`:

```rust
pub fn submit_artifact(
    &self,
    journal: &FjallJournal,
    workflow: &CompiledWorkflow,
    policy: RuntimePolicy,
    contracts: &[ActionContract],
    packs: &[ActionPack],
) -> Result<AcceptedArtifact, ExecutorError> {
    let artifact = self.prepare_artifact(workflow, policy, contracts, packs)?;
    journal.put_compiled_ir(&CompiledIrRecord {
        digest: artifact.digest,
        ir: postcard::to_allocvec(&artifact)?,
    })?;
    journal.persist_strict()?;  // <-- called unconditionally (wrong for non-Strict)
    let stored = journal
        .compiled_ir(artifact.digest)?
        .ok_or(ExecutorError::Corrupt)?;
    let restored = postcard::from_bytes(&stored.ir)?;
    self.verify_artifact(&restored, policy)?;
    if artifact != restored {
        return Err(ExecutorError::Corrupt);
    }
    Ok(artifact)
}
```

Issues with current implementation:
1. `persist_strict()` called unconditionally — wrong for Relaxed/Journaled policies
2. No counter/seq issuance — artifacts have no `accepted_at_seq`
3. Bypasses the shared issuance path entirely
4. Two separate writes (put + persist) instead of one atomic batch

### Reference: storage-level admission flow

Located at `crates/vb_storage/src/admission.rs`. This is the pattern the executor catalog must adopt:

```rust
pub fn submit_artifact_with_contracts(
    journal: &FjallJournal,
    workflow: &vb_core::CompiledWorkflow,
    policy: vb_core::RuntimePolicy,
    action_contracts: &[ActionContract],
) -> Result<AcceptedArtifact, JournalError> {
    let artifact = prepare_compiled_artifact(workflow, policy, action_contracts)?;
    let stamped = crate::journal::artifact_acceptance::issue_next_artifact_sequence(
        journal, artifact, policy,
    )?;
    verify_artifact_persisted(journal, stamped.digest)?;
    Ok(stamped)
}
```

Key observation: The storage-level flow delegates to `issue_next_artifact_sequence` and then does a simple readback check. The executor catalog flow is similar but has additional conformance verification (`self.verify_artifact`) and equality check (`stamped != restored`).

### Error type analysis

`crates/vb_storage/src/executor/error.rs` defines `ExecutorError` with:

```rust
#[error("journal error: {0}")]
Journal(#[from] JournalError),
```

This means any `JournalError` value automatically converts to `ExecutorError::Journal(...)` via `?` and the From trait. The helper returns `JournalError`, so the conversion is automatic and no new error variants are needed.

### Actual counter behavior from tests

From `crates/vb_storage/src/admission/acceptance_sequence_*_tests.rs`:

1. Counter starts at 0, first issuance is seq 1
2. Counter increments for each submission
3. Pure prepare is seq 0
4. Counter survives journal close/reopen
5. 7-byte and 9-byte counter rows cause `MalformedKeyspaceRow`
6. Counter at u64::MAX causes `SequenceOverflow`
7. Strict durability failure is precommit — counter unchanged
8. Batch commit failure is precommit — counter unchanged
9. Compiled-IR readback failure is postcommit — counter advanced

### Actual FjallJournal test hooks

The journal has private test hooks for fault injection:
- `fail_next_persist_for_test()` — causes next `StrictDurabilityFailed`
- `fail_next_batch_commit_for_test()` — causes next batch commit to fail with Fjall error

These hooks are already used in existing tests and can be reused for this bead's tests.

## Design decisions

### Why use the shared helper?

The helper (`issue_next_artifact_sequence`) already implements the entire RMW sequence correctly:
- Atomic counter increment and artifact persistence
- Correct policy durability (SyncAll only for Strict)
- Proper error handling and typed errors
- Digest rebinding after seq stamping
- Write-lock protection for concurrency

The executor catalog calling this helper avoids code duplication and ensures consistent behavior across the storage-level and executor-level submission paths.

### Why preserve existing readback verification?

The old implementation had extensive readback verification. This bead preserves that while delegating issuance to the helper. The readback verification catches storage-level corruption or Fjall bugs that the helper's own checks don't cover.

### Why no new error variants?

The existing `#[from] JournalError` impl already handles all helper error paths. Adding new error variants would be redundant and break existing callers.

### Why no boolean flags?

The `RuntimePolicy` enum is already a typed policy seam. Adding boolean flags like `use_shared_helper: bool` or `enable_counter: bool` would be a new boolean flag — the task explicitly prohibits this.

### Why one bounded new test module?

The existing test infrastructure (fixtures, minimal workflow builders) is in the workspace TEST crate and `crates/vb_storage/src/admission/tests.rs`. This bead needs new tests for the executor catalog path with counter/seq behavior. A single bounded test module under the executor crate keeps the scope minimal and focused.

## Open questions

1. **Will existing executor tests break?** Any tests that assert `accepted_at_seq = 0` or check for the absence of seq values on executor-submitted artifacts may need updating. This is a known risk.

2. **Is there an existing test for the old behavior?** I didn't find explicit tests for the old `put_compiled_ir` + `persist_strict` pattern. This suggests the executor catalog tests are minimal or nonexistent.

3. **Should I update the old-style executor tests?** Yes, if any exist. The contract says no changes to other methods, but tests for `submit_artifact` that check the old behavior are in scope.

## Evidence collected

- Read actual `crates/vb_storage/src/journal/artifact_acceptance.rs` (helper implementation)
- Read actual `crates/vb_storage/src/executor/packed.rs` (current executor catalog)
- Read actual `crates/vb_storage/src/admission.rs` (reference storage-level flow)
- Read actual `crates/vb_storage/src/executor/error.rs` (error types)
- Read actual `crates/vb_storage/src/types.rs` (EventSeq type)
- Read actual `crates/vb_storage/src/records.rs` (CompiledIrRecord type)
- Read actual `crates/vb_storage/src/admission.rs` (AcceptedArtifact type)
- Read actual test files:
  - `crates/vb_storage/src/admission/acceptance_sequence_tests.rs`
  - `crates/vb_storage/src/admission/acceptance_sequence_boundary_tests.rs`
  - `crates/vb_storage/src/admission/acceptance_sequence_failure_tests.rs`
  - `crates/vb_storage/src/vb_2bok_durability_gate_tests.rs`
- Searched for `accepted_at_seq` usage across the codebase
- Searched for `submit_artifact` and `prepare_artifact` definitions

No APIs or types were invented. All references are grounded in actual source code.
