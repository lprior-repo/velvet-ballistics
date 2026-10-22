# vb-ct9lz: Executor catalog uses shared artifact issuance helper

**Type:** Rust — proportional type/behavior contract
**Lanes:** `crates/vb_storage/src/executor/packed.rs`
**Depends on:** `vb-qz2yn` (artifact acceptance counter helper) — must land first
**Contract status:** source candidate (not yet accepted)

## Change

Replace the explicit `put_compiled_ir` + `persist_strict` pattern in `ExecutorCatalog::submit_artifact` with a call to the shared `issue_next_artifact_sequence` helper from dependency qz. Preserve post-commit readback verification.

## Actual source anchors

- ExecutorError (actual): `crates/vb_storage/src/executor/error.rs` — variants include `Artifact(#[from] crate::JournalError)`, `Codec(#[from] postcard::Error)`, `Corrupt`, `PackMismatch`, `Storage`, `Io`, and others. NOT Journal/Postcard/NotAccepted/DigestMismatch/NotFound/InvalidContract/PackMissing/UnexpectedAction.
- Actual `packed.rs` submit flow (OLD): prepare → `put_compiled_ir` → unconditional `persist_strict` → readback → verify → compare. The unconditional `persist_strict` is INTENDED, not wrong.
- Shared helper: `pub(crate) fn issue_next_artifact_sequence(journal, artifact, policy)` in `journal/artifact_acceptance.rs` — handles counter RMW, seq stamping, encoding, policy durability, single batch commit.
- Actual counter: global u64 big-endian stored at key `last_issued` in `artifact_acceptance` keyspace.
- Actual types: `EventSeq(u64)` public; internal `IssuedArtifactSequence` wraps NonZeroU64; `accepted_at_seq` is an `EventSeq` field; `.get()` returns u64.
- Actual conformance tests: existing fixtures in `crates/workspace_tests/tests/vb_7r7jv_executor_conformance.rs` (already in packet 06-*.txt).

## New submit flow

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
    // OLD: journal.put_compiled_ir(...); journal.persist_strict()?;
    // NEW:
    let stamped = crate::journal::artifact_acceptance::issue_next_artifact_sequence(
        journal, artifact, RuntimePolicy::Strict,
    )?;
    let stored = journal.compiled_ir(stamped.digest)?.ok_or(ExecutorError::Corrupt)?;
    let restored = postcard::from_bytes(&stored.ir)?;
    self.verify_artifact(&restored, policy)?;
    if stamped != restored {
        return Err(ExecutorError::Corrupt);
    }
    Ok(stamped)
}
```

## Error semantics

- **New issuance errors** (counter read/overflow, encode, precommit persist, batch commit) flow through `issue_next_artifact_sequence` → `JournalError` → wrapped as `ExecutorError::Artifact(...)`. No new error enum required.
- **Malformed counter** (not 8 bytes) → `JournalError::MalformedKeyspaceRow` (exact prefix 0, expected_len 8) → `Artifact(...)`.
- **Counter overflow** (u64::MAX) → `JournalError::SequenceOverflow` → `Artifact(...)`.
- **Counter/encode validation and injected precommit persist/batch hooks** fail before publication: counter and compiled record unchanged. Arbitrary batch I/O errors can occur after commit.
- Postcommit readback None or equality mismatch remains `Corrupt`; postcard decode remains `Codec`; journal reads and catalog verification retain their actual existing conversions.
- Issuance artifact encoding now maps through `Artifact(JournalError::PostcardEncodeFailed)` rather than the old explicit artifact encode `Codec` path; no claim every possible failure variant stays identical.
- **IO errors during commit** → may already have committed; no universal rollback guarantee.

## Behavioral guarantees

1. Artifacts now receive non-zero `accepted_at_seq` (first issuance is seq 1).
2. Counter persists across journal close/reopen.
3. Every catalog submission uses `RuntimePolicy::Strict` for helper storage durability, including artifacts prepared for Relaxed/Journaled admission. Original admission policy is retained in evidence and used by catalog verification. Successful packed submission preserves unconditional SyncAll.
4. Pure `prepare_artifact` unchanged: returns seq=0, unpersisted artifact.
5. All existing public APIs unchanged: signatures, types, variants.

## Test scope

Proposed: 2-3 small unit files under executor with `#[cfg(test)]` (fixture module, policy tests, issuance tests), each ≤300 lines/fn ≤25 lines, using existing crate-private hooks. Existing workspace conformance tests in packet 06-*.txt already exist; no need to claim non-existent tests. Test counter faults, seq issuance, mixed policies, close/reopen, pure prepare.