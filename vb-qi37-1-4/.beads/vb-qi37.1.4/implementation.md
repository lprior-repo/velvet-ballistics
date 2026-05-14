# vb-qi37.1.4 Implementation

## State: 10 (holzman-rust)

## Implemented Behavior

### `verify_digests` — Current Implementation

**File**: `crates/vb_storage/src/recovery/recover.rs:54-72`

```rust
pub fn verify_digests(
    journal: &FjallJournal,
    run: RunId,
    workflow_digest: WorkflowDigest,
    ir_digest: WorkflowDigest,
    found_ir_digest: WorkflowDigest,
    level: DigestCheck,
) -> RecoveryResult<()>
```

**Current digest checks**:

| `DigestCheck` variant | Workflow source | Compiled IR | Action ABI | Policy |
|---------------------|-----------------|-------------|------------|--------|
| `WorkflowSourceOnly` | ✓ checked | — | — | — |
| `WorkflowAndIr` | ✓ checked | ✓ checked | — | — |
| `Full` | ✓ checked | ✓ checked | — | — |

**Current error variants returned**:
- `RecoveryError::WorkflowSourceDigestMismatch` — when stored workflow digest differs
- `RecoveryError::CompiledIrDigestMismatch` — when stored IR digest differs
- `RecoveryError::NoRecoveryData` — when no `RunAccepted` event found

**Missing GAP**: `DigestCheck::Full` is documented as "Verify all digests including action ABI and policy" but action ABI and policy digests are NOT currently verified.

---

## GAP: Extended `verify_digests` Signature

### Required Changes

1. **Extend function signature** with two new parameters:
   ```rust
   pub fn verify_digests(
       journal: &FjallJournal,
       run: RunId,
       workflow_digest: WorkflowDigest,
       ir_digest: WorkflowDigest,
       found_ir_digest: WorkflowDigest,
       action_abi_digests: &[(ActionId, WorkflowDigest)],
       policy_digests: &[(StepIdx, WorkflowDigest)],
       level: DigestCheck,
   ) -> RecoveryResult<()>
   ```

2. **Implement action ABI verification** at `DigestCheck::Full`:
   - For each `(action_id, expected_digest)` in `action_abi_digests`
   - Look up the stored action ABI digest from journal events
   - If mismatch found, return `RecoveryError::ActionAbiMismatch { action_id }`

3. **Implement policy digest verification** at `DigestCheck::Full`:
   - For each `(step_idx, expected_digest)` in `policy_digests`
   - Look up the stored policy digest from journal events
   - If mismatch found, return `RecoveryError::PolicyDigestMismatch { step: step_idx }`

### Error Variants (Already Exist)

```rust
// crates/vb_storage/src/recovery/types.rs:40-49
#[error("action ABI digest mismatch for action {action_id:?}")]
ActionAbiMismatch { action_id: ActionId },

#[error("policy digest mismatch for step {step:?}")]
PolicyDigestMismatch { step: StepIdx },
```

### Test Update Required

The 4 negative tests in `crates/vb_storage/src/recovery/tests.rs:1298-1467` use the current 6-arg signature. After extending the signature, these tests must be updated to pass empty slices for the new parameters and renamed as positive tests that verify the new behavior.

---

## Evidence

- **Clippy**: `cargo clippy -p vb_storage` → No issues found
- **Fmt**: `cargo fmt --check` → (no output = passed)
- **Tests**: `cargo test -p vb_storage --lib` → 927 passed
- **No forbidden patterns** in `crates/vb_storage/src/recovery/` or `crates/vb_runtime/src/recovery.rs`

---

## Invariant Coverage

| Invariant | Status |
|-----------|--------|
| INV-RC-006 (Full verifies action ABI digest) | **GAP** — requires extended signature |
| INV-RC-008 (verify_digests returns ActionAbiMismatch) | **GAP** — requires extended signature |
| INV-RC-009 (verify_digests returns PolicyDigestMismatch) | **GAP** — requires extended signature |
