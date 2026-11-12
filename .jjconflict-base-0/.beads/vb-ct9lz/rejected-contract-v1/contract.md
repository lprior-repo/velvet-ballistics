# vb-ct9lz: Executor catalog uses shared artifact issuance helper

**Type:** Rust — proportional type/behavior contract
**Lanes:** `crates/vb_storage/src/executor/packed.rs` (executor catalog submission)
**Depends on:** `qz` (artifact acceptance counter helper) — must land first
**Contract status:** source candidate (not yet accepted)

## Actual source text anchors

This contract is grounded in the following actual source files from the velvet-ballistics repository. No invented APIs or behavior.

### 1. The shared issuance helper (dependency qz)

**File:** `crates/vb_storage/src/journal/artifact_acceptance.rs`
**Actual exported helper:** `pub(crate) fn issue_next_artifact_sequence(journal, artifact, policy)`

```rust
pub(crate) fn issue_next_artifact_sequence(
    journal: &FjallJournal,
    artifact: AcceptedArtifact,
    policy: RuntimePolicy,
) -> Result<AcceptedArtifact, JournalError> {
    issue_artifact(journal, artifact, policy)
}
```

This helper owns the RMW sequence: read counter, increment, stamp, rebind digest, encode record, stage both writes in one `fjall::OwnedWriteBatch`, commit with policy-appropriate durability. It holds the write_lock across the entire sequence.

### 2. Current executor catalog submission (to be changed)

**File:** `crates/vb_storage/src/executor/packed.rs`
**Actual method signature (unchanged by this bead):**

```rust
pub fn submit_artifact(
    &self,
    journal: &FjallJournal,
    workflow: &CompiledWorkflow,
    policy: RuntimePolicy,
    contracts: &[ActionContract],
    packs: &[ActionPack],
) -> Result<AcceptedArtifact, ExecutorError>
```

**Current implementation (OLD pattern to be replaced):**

```rust
let artifact = self.prepare_artifact(workflow, policy, contracts, packs)?;
journal.put_compiled_ir(&CompiledIrRecord {
    digest: artifact.digest,
    ir: postcard::to_allocvec(&artifact)?,
})?;
journal.persist_strict()?;
let stored = journal
    .compiled_ir(artifact.digest)?
    .ok_or(ExecutorError::Corrupt)?;
let restored = postcard::from_bytes(&stored.ir)?;
self.verify_artifact(&restored, policy)?;
if artifact != restored {
    return Err(ExecutorError::Corrupt);
}
Ok(artifact)
```

The OLD pattern uses `put_compiled_ir` + `persist_strict` (which is wrong for non-Strict policies — `persist_strict` is called unconditionally) and then manually does readback verification. It bypasses the counter/seq issuance entirely.

### 3. Current storage-level admission (reference behavior)

**File:** `crates/vb_storage/src/admission.rs`
**Actual method (unchanged by this bead):**

```rust
pub fn submit_artifact_with_contracts(
    journal: &FjallJournal,
    workflow: &vb_core::CompiledWorkflow,
    policy: vb_core::RuntimePolicy,
    action_contracts: &[ActionContract],
) -> Result<AcceptedArtifact, JournalError>
```

**Current implementation (reference pattern to follow):**

```rust
let artifact = prepare_compiled_artifact(workflow, policy, action_contracts)?;
let stamped = crate::journal::artifact_acceptance::issue_next_artifact_sequence(
    journal, artifact, policy,
)?;
verify_artifact_persisted(journal, stamped.digest)?;
Ok(stamped)
```

This is the pattern the executor catalog must adopt.

### 4. Actual error types

**File:** `crates/vb_storage/src/executor/error.rs`
**Actual ExecutorError variants (unchanged):**

```rust
pub enum ExecutorError {
    #[error("artifact not accepted by catalog")]
    NotAccepted,
    #[error("artifact digest verification failed")]
    DigestMismatch,
    #[error("artifact not found in journal")]
    NotFound,
    #[error("artifact corrupted")]
    Corrupt,
    #[error("invalid action contract")]
    InvalidContract,
    #[error("action pack missing for action {action:?}")]
    PackMissing { action: ActionId },
    #[error("unexpected action in artifact: {action:?}")]
    UnexpectedAction { action: ActionId },
    #[error("journal error: {0}")]
    Journal(#[from] JournalError),
    #[error("postcard error: {0}")]
    Postcard(#[from] postcard::Error),
}
```

The `#[from] JournalError` auto-impl means `JournalError` values convert directly to `ExecutorError::Journal(...)`. The actual conversions already exist — we must not invent new ones.

### 5. Actual conformance verification APIs (unchanged)

**File:** `crates/vb_storage/src/executor/packed.rs`
**Actual methods (unchanged by this bead):**

```rust
pub fn prepare_artifact(
    &self,
    workflow: &CompiledWorkflow,
    policy: RuntimePolicy,
    contracts: &[ActionContract],
    packs: &[ActionPack],
) -> Result<AcceptedArtifact, ExecutorError>

pub fn verify_artifact(
    &self,
    artifact: &AcceptedArtifact,
    policy: RuntimePolicy,
) -> Result<CompiledWorkflow, ExecutorError>
```

`prepare_artifact` stays pure (seq=0, unpersisted). `verify_artifact` is called on the restored artifact after readback to confirm it's still valid.

### 6. Actual types (unchanged)

**File:** `crates/vb_storage/src/types.rs`

```rust
pub struct EventSeq(u64);
```

**File:** `crates/vb_storage/src/records.rs`

```rust
pub struct CompiledIrRecord {
    pub digest: WorkflowDigest,
    pub ir: Vec<u8>,
}
```

**File:** `crates/vb_storage/src/admission.rs`

```rust
pub struct AcceptedArtifact {
    pub digest: WorkflowDigest,
    pub source_digest: WorkflowDigest,
    pub policy_digest: WorkflowDigest,
    pub ir: Vec<u8>,
    pub verification: VerificationProof,
    pub accepted_at_seq: EventSeq,
    pub required_capabilities: Box<[vb_core::capability::Capability]>,
    pub action_contracts: Box<[ActionContract]>,
}
```

`accepted_at_seq` is the field stamped by the issuance helper. `accepted_at_seq.get()` returns `u64` (the NonZeroU64 inner value).

## Behavioral contract

### 1. Single batch with policy-appropriate durability

**Old behavior:** Two separate writes (`put_compiled_ir` + `persist_strict`). `persist_strict` called unconditionally, wrong for non-Strict policies.

**New behavior (this bead):** One `fjall::OwnedWriteBatch` containing both the counter increment and the artifact. Policy controls only the SyncAll behavior:
- `RuntimePolicy::Strict` → `batch.durability(Some(fjall::PersistMode::SyncAll))`
- `RuntimePolicy::Relaxed`, `RuntimePolicy::Journaled` → no SyncAll

The helper (`issue_next_artifact_sequence`) already implements this correctly. The executor catalog must delegate to it.

### 2. Counter/seq issuance

**Old behavior:** No counter/seq. Artifacts had no `accepted_at_seq` stamp.

**New behavior (this bead):** Each submission gets the next `NonZeroU64` sequence number. Counter starts at 0, first issuance is seq 1. Counter is persisted in the `artifact_acceptance` keyspace at key `b"last_issued"` as a big-endian u64.

The returned `AcceptedArtifact` carries the `accepted_at_seq` field. The counter must survive journal close/reopen.

### 3. Pure prepare unchanged

`ExecutorCatalog::prepare_artifact` stays unchanged. It returns a pure, unpersisted artifact with `accepted_at_seq = EventSeq::new(0)`. No counter increment. No persistence.

### 4. Existing readback verification preserved

The old implementation had full readback verification. This bead preserves it:

```rust
let stamped = issue_next_artifact_sequence(journal, artifact, policy)?;
// The helper already committed. Now verify readback.
let stored = journal
    .compiled_ir(stamped.digest)?
    .ok_or(ExecutorError::Corrupt)?;
let restored = postcard::from_bytes(&stored.ir)?;
self.verify_artifact(&restored, policy)?;
if stamped != restored {
    return Err(ExecutorError::Corrupt);
}
Ok(stamped)
```

The `stamped` artifact from the helper is the canonical value returned to the caller. Its `accepted_at_seq` and digests are the real ones.

### 5. Failure semantics

**Precommit failures (counter unchanged, artifact not persisted):**
- Strict durability failure → `JournalError::StrictDurabilityFailed`
- Batch commit failure → `JournalError::Fjall(...)`
- Counter read/decode failure → `JournalError::MalformedKeyspaceRow` or `JournalError::Corrupt`
- Counter length not 8 bytes → `JournalError::MalformedKeyspaceRow`
- Counter overflow (at u64::MAX) → `JournalError::SequenceOverflow`

**Postcommit failures (counter advanced, artifact persisted):**
- Compiled-IR readback returns None → `ExecutorError::Corrupt`
- Postcard decode fails → `ExecutorError::Postcard(...)`
- Verify artifact fails → `ExecutorError` variant
- Stamped vs restored mismatch → `ExecutorError::Corrupt`

This matches the existing failure semantics in the storage-level admission flow.

### 6. ExecutorError conversions

`JournalError` values convert directly to `ExecutorError::Journal(...)` via the existing `#[from] JournalError` impl. No new error variants, no new conversion code.

The actual `JournalError` variants involved:
- `StrictDurabilityFailed` — strict policy persist failed
- `Fjall(e)` — underlying Fjall/rocksdb error
- `MalformedKeyspaceRow` — counter row not 8 bytes or invalid bytes
- `Corrupt` — general corruption
- `SequenceOverflow` — counter at u64::MAX
- `PostcardEncodeFailed(e)` / `PostcardDecodeFailed(e)` — serialization

## Test strategy

All tests use actual public/catalog wiring with mixed default/catalog1,2,3 policies. No fake boolean flags. No new public test helpers. No new modules.

### 1. Mixed policy submission (default + catalog1,2,3)

Submit workflows through the executor catalog under all three `RuntimePolicy` values and verify:
- Seq increments correctly (1, 2, 3...)
- All policies work (Relaxed, Journaled, Strict)
- Returned artifacts have correct `accepted_at_seq`
- Artifacts are persisted and readable

### 2. Same path close/reopen full packed artifact + counter

Submit, close journal, reopen, verify counter persisted (next submission gets seq 2). Read back full packed artifact (not just counter). Verify `accepted_at_seq` and digests survived reopen.

### 3. Pure prepare stays seq=0

```rust
let artifact = catalog.prepare_artifact(&workflow, policy, &[], &[])?;
assert_eq!(artifact.accepted_at_seq.get(), 0);
```

### 4. Stamped canonical proof binding and catalog verifier

Submit via catalog, verify the returned artifact passes `catalog.verify_artifact(&stamped, policy)` — the catalog's own verifier must accept its own issued artifact.

For all three admission policies (Relaxed, Journaled, Strict), demonstrate:
- The stamped artifact from the catalog has the correct `accepted_at_seq`
- The artifact passes catalog verification
- Strict policy demonstrates unconditional durability via injected `StrictDurabilityFailed` fault

### 5. Injected persist fault (strict) demonstrates unconditional durability

```rust
journal.fail_next_persist_for_test();
let result = catalog.submit_artifact(&journal, &workflow, RuntimePolicy::Strict, &[], &[]);
assert_eq!(result, Err(ExecutorError::Journal(JournalError::StrictDurabilityFailed)));
// Counter must be 0
// Compiled-IR keyspace must be empty
```

Then retry without fault — submission succeeds at seq 1.

### 6. Batch commit failure (precommit)

```rust
journal.fail_next_batch_commit_for_test();
let result = catalog.submit_artifact(&journal, &workflow, RuntimePolicy::Strict, &[], &[]);
assert_eq!(result, Err(ExecutorError::Journal(JournalError::Fjall(...))));
// Counter must be 0
// Compiled-IR keyspace must be empty
```

### 7. Postcommit readback failure

```rust
let stamped = catalog.submit_artifact(&journal, &workflow, RuntimePolicy::Relaxed, &[], &[])?;
assert_eq!(stamped.accepted_at_seq.get(), 1);
// Counter advanced, artifact persisted — readback failure is postcommit
```

### 8. Malformed counter 7/9/overflow

Directly corrupt the counter key in the `artifact_acceptance` keyspace:

```rust
journal.artifact_acceptance.insert(b"last_issued", &[0, 0, 0, 0, 0, 0, 0])?; // 7 bytes
let result = catalog.submit_artifact(&journal, &workflow, RuntimePolicy::Relaxed, &[], &[]);
assert_eq!(result, Err(ExecutorError::Journal(JournalError::MalformedKeyspaceRow)));
// Counter unchanged, no artifact persisted
```

9-byte test similarly. Overflow test: set counter to u64::MAX, next submission fails with `SequenceOverflow`.

### 9. Invalid packs before counter

Submit with invalid packs (wrong action, missing pack, etc.) — counter must not advance, artifact must not be persisted. The conformance verification in `prepare_artifact` catches these before issuance.

### 10. Readback None → Corrupt

Submit, then manually delete the compiled-IR record (or use a corrupted digest). Next readback returns None → `ExecutorError::Corrupt`.

```rust
// After successful submission, delete the record
journal.compiled_ir.delete(digest_key)?;
let stored = journal.compiled_ir(stamped.digest)?;
assert!(stored.is_none());
```

### 11. Registered conformance authority (not fake boolean flags)

Use actual registered action packs and verify the catalog's conformance checks pass. No fake boolean flags to "enable" or "disable" behavior.

## Implementation approach

### Single method change in `crates/vb_storage/src/executor/packed.rs`

Change `ExecutorCatalog::submit_artifact` to use the shared issuance helper:

```rust
pub fn submit_artifact(
    &self,
    journal: &FjallJournal,
    workflow: &CompiledWorkflow,
    policy: RuntimePolicy,
    contracts: &[ActionContract],
    packs: &[ActionPack],
) -> Result<AcceptedArtifact, ExecutorError> {
    // Step 1: Prepare (pure, seq=0, no persistence)
    let artifact = self.prepare_artifact(workflow, policy, contracts, packs)?;

    // Step 2: Issue next sequence via shared helper (atomic batch)
    let stamped = crate::journal::artifact_acceptance::issue_next_artifact_sequence(
        journal, artifact, policy,
    )?;

    // Step 3: Postcommit readback verification (existing, preserved)
    let stored = journal
        .compiled_ir(stamped.digest)?
        .ok_or(ExecutorError::Corrupt)?;
    let restored = postcard::from_bytes(&stored.ir)?;
    self.verify_artifact(&restored, policy)?;
    if stamped != restored {
        return Err(ExecutorError::Corrupt);
    }
    Ok(stamped)
}
```

The only actual code change is replacing the old `put_compiled_ir` + `persist_strict` + manual readback with the helper call + the same manual readback verification.

### No other changes

- `prepare_artifact` unchanged
- `verify_artifact` unchanged
- `check_packs` unchanged
- `pack_for` unchanged
- Error types unchanged
- No new modules, types, or public APIs
- No changes to other lanes (vb_core, vb_runtime, vb_cli, etc.)
- No boolean flags or wrapping types

## Test location

Minimal new test scope: existing workspace TEST has fixtures but private counter fault access needs `vb_storage` unit module tests.

**Suggested test location:** One bounded unit module under the executor with `#[cfg(test)]` and private hook use.

```
crates/vb_storage/src/executor/packed_tests.rs (new, <=300 lines)
```

Reuse actual existing conformance fixture builders. No new public test helpers — all private hooks.

## Risks and mitigations

| Risk | Severity | Mitigation |
|------|----------|------------|
| JournalError auto-conversion doesn't cover all variants | High | Test every error path; add `#[from]` impl if needed |
| Counter persistence across reopen fails | High | Test close/reopen explicitly |
| Strict durability not applied for non-Strict policies | High | Test injected `StrictDurabilityFailed` fault only for Strict |
| Readback returns wrong artifact (digest collision) | High | Verify digest equality in readback |
| Concurrency race on counter | High | Helper already holds write_lock |
| Old tests expect old behavior (no seq) | Medium | Update any old executor tests that assert no seq |
| New test module exceeds size limit | Low | Keep module <=300 lines |

## Acceptance criteria

1. `ExecutorCatalog::submit_artifact` uses `issue_next_artifact_sequence` helper
2. Artifacts get NonZeroU64 `accepted_at_seq` values
3. Counter survives journal close/reopen
4. All three policies work (Relaxed, Journaled, Strict)
5. Strict durability only applies SyncAll for Strict policy
6. Injected persist fault causes `StrictDurabilityFailed` for Strict only
7. Malformed counter (7/9/overflow) causes typed errors, counter unchanged
8. Readback None → `ExecutorError::Corrupt`
9. Readback equality verified via stamped artifact's actual digest
10. Pure prepare stays seq=0
11. Conformance verification catches invalid packs before counter advances
12. All existing public APIs unchanged (signatures, types, variants)
13. No new modules, types, boolean flags, or wrapping types
14. One bounded new test module (<=300 lines) under executor with private hooks
15. Test suite passes including this bead's new tests

## Out of scope

- No proof planning (Verus/Flux/Kani)
- No Go-skill
- No full v1 migration
- No power-loss tests
- No performance improvement claims
- No changes to other lanes (vb_core, vb_runtime, vb_cli, etc.)
- No changes to other methods on `ExecutorCatalog` (prepare, verify, check_packs, pack_for)
- No new public APIs
- No boolean flags to toggle behavior
- No wrapping types just to invoke the helper

## Notes

This is a proportional contract that changes exactly one method (`ExecutorCatalog::submit_artifact`) to use the same shared issuance helper that the storage-level admission flow already uses. The helper (`issue_next_artifact_sequence`) is already implemented in dependency qz. This bead wires the executor catalog to it.

The behavioral change is significant: executor-submitted artifacts now get sequence numbers and counter persistence. But the API change is zero: same method signature, same return type, same error types.
