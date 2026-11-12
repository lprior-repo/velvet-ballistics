bead_id: vb-2bzz
bead_title: storage: Expose action ABI and policy digest recovery mismatch checks
phase: 3
updated_at: 2026-05-17T02:00:00Z
attempt: 1-of-7

## Requirements (EARS)

1. **EARS-1**: When recovery validates action replay against an expected action ABI source, the storage recovery API shall return `RecoveryError::ActionAbiMismatch { action_id }` for exact mismatches.
2. **EARS-2**: When recovery validates policy identity for a recovered step/run, the storage recovery API shall return `RecoveryError::PolicyDigestMismatch { step }` for exact mismatches.
3. **EARS-3**: If the current journal record surface cannot carry or look up these expected digests, then the API shall expose an explicit verifier input rather than silently returning Ok.

## Assumptions

- `JournalEvent::ActionScheduled` carries `action: ActionId` but no ABI digest field. This is by design — ABI digests are external to the journal.
- `JournalEvent::RunAdmission` carries `policy: RuntimePolicy` but not a policy digest. Policy digests are external.
- The caller (runtime layer) holds the authoritative ABI and policy digest sources.
- `WorkflowDigest` is the type used for both action ABI digests and policy digests (32-byte Blake3).

## Invariants

- **INV-1**: `ActionAbiMismatch` is returned ONLY when a real action ABI mismatch input exists (not from missing data).
- **INV-2**: `PolicyDigestMismatch` is returned ONLY when a real policy digest mismatch input exists (not from missing data).
- **INV-3**: Matching ABI and policy digests must allow recovery to proceed (no false positives).
- **INV-4**: The error must carry the exact `action_id` or `step` that mismatched.

## Type/Domain Model

### New Public Functions

```rust
/// Checks action ABI digests against expected values.
/// Returns ActionAbiMismatch { action_id } on first mismatch.
/// Returns Ok(()) when all expected ABIs match or no expectations provided.
pub fn check_action_abi_digests(
    journal: &FjallJournal,
    run: RunId,
    expected_abis: &[(ActionId, WorkflowDigest)],
) -> RecoveryResult<()>;

/// Checks policy digests against expected values.
/// Returns PolicyDigestMismatch { step } on first mismatch.
/// Returns Ok(()) when all expected policy digests match or no expectations provided.
pub fn check_policy_digests(
    journal: &FjallJournal,
    run: RunId,
    expected_policy_digests: &[(StepIdx, WorkflowDigest)],
) -> RecoveryResult<()>;
```

### Extended verify_digests

The existing `verify_digests` function will be extended to accept optional ABI and policy digest inputs:

```rust
pub fn verify_digests(
    journal: &FjallJournal,
    run: RunId,
    workflow_digest: WorkflowDigest,
    ir_digest: WorkflowDigest,
    found_ir_digest: WorkflowDigest,
    level: DigestCheck,
    expected_abis: &[(ActionId, WorkflowDigest)],
    expected_policy_digests: &[(StepIdx, WorkflowDigest)],
) -> RecoveryResult<()>;
```

When `level == DigestCheck::Full`, the function will call `check_action_abi_digests` and `check_policy_digests`.

## Verification Layers

| Clause | Verification |
|---|---|
| EARS-1 | Unit test: action_abi_mismatch_returns_typed_error |
| EARS-2 | Unit test: policy_digest_mismatch_returns_typed_error |
| EARS-3 | API design: explicit input parameters, no guessing |
| INV-1 | Unit test: matching ABIs return Ok |
| INV-2 | Unit test: matching policy digests return Ok |
| INV-3 | Unit test: no false positives on empty input |
| INV-4 | Unit test: error carries exact action_id/step |

## Proof Obligations

No formal proof required — this is an API surface change with typed error returns. Unit tests provide sufficient evidence.

## Traceability Matrix

```jsonl
{"requirement":"EARS-1","contract_clause":"ActionAbiMismatch on exact mismatch","test":"action_abi_mismatch_returns_typed_error","implementation":"check_action_abi_digests"}
{"requirement":"EARS-2","contract_clause":"PolicyDigestMismatch on exact mismatch","test":"policy_digest_mismatch_returns_typed_error","implementation":"check_policy_digests"}
{"requirement":"EARS-3","contract_clause":"Explicit verifier input","test":"verify_digests_with_full_level","implementation":"verify_digests extended signature"}
{"requirement":"INV-1","contract_clause":"No false positive on missing data","test":"action_abi_match_returns_ok","implementation":"check_action_abi_digests empty input"}
{"requirement":"INV-2","contract_clause":"No false positive on missing data","test":"policy_digest_match_returns_ok","implementation":"check_policy_digests empty input"}
```
