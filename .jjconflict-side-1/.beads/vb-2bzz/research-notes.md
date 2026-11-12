bead_id: vb-2bzz
bead_title: storage: Expose action ABI and policy digest recovery mismatch checks
phase: 2
updated_at: 2026-05-17T02:00:00Z
attempt: 1-of-7

## Codebase Map

### Touched Crates
- `vb_storage` — recovery API, error types, journal events

### Touched Files

| File | Role |
|---|---|
| `crates/vb_storage/src/recovery/types.rs` | RecoveryError enum (ActionAbiMismatch, PolicyDigestMismatch already defined) |
| `crates/vb_storage/src/recovery/recover.rs` | verify_digests() — has GAP-3 deferred comment |
| `crates/vb_storage/src/recovery/replay/core.rs` | recover_full_journal() — no ABI/policy inputs |
| `crates/vb_storage/src/recovery/mod.rs` | Re-exports |
| `crates/vb_storage/src/events.rs` | JournalEvent enum (ActionScheduled has no ABI field) |
| `crates/vb_storage/tests/recovery_bdd_tests.rs` | Ignored GAP-3 tests |

### Public APIs to Change

1. **`verify_digests`** in `recover.rs`:
   - Current signature: `(journal, run, workflow_digest, ir_digest, found_ir_digest, level)`
   - Need to add: `expected_abis: &[(ActionId, WorkflowDigest)]` and `expected_policy_digests: &[(StepIdx, WorkflowDigest)]`
   - Or: add a new function `check_action_abi_digests` and `check_policy_digests`

2. **`recover_full_journal`** in `replay/core.rs`:
   - Current signature: `(journal, run, tracker)`
   - Need to add verifier input parameters OR create a separate validation function

### Design Decision

The bead requirements say: "Do not make recover_full_journal guess mismatch from missing data. Add explicit durable fields or a dedicated verification input/lookup function."

The cleanest approach following the existing pattern:
- Add `check_action_abi_digests(journal, run, expected_abis)` → returns `ActionAbiMismatch` on first mismatch
- Add `check_policy_digests(journal, run, expected_policy_digests)` → returns `PolicyDigestMismatch` on first mismatch
- These are called by the caller BEFORE or AFTER `recover_full_journal`, keeping the replay function focused on event replay
- Alternatively, extend `verify_digests` to accept these inputs

Given the existing `verify_digests` already handles workflow and IR digests, extending it is the most consistent approach. However, `verify_digests` takes `DigestCheck` level which controls which checks run. We need to add `DigestCheck::Full` to actually check ABI and policy.

### Implementation Plan

1. Add `expected_abis: &[(ActionId, WorkflowDigest)]` and `expected_policy_digests: &[(StepIdx, WorkflowDigest)]` parameters to `verify_digests`
2. When `level == DigestCheck::Full`, iterate expected_abis and check each action_id against the journal's ActionScheduled events
3. When `level == DigestCheck::Full`, iterate expected_policy_digests and check each step against the journal's RunAdmission events (which carry policy)
4. Return `RecoveryError::ActionAbiMismatch { action_id }` on first ABI mismatch
5. Return `RecoveryError::PolicyDigestMismatch { step }` on first policy mismatch
6. Unignore and fix the two GAP-3 tests to use the new API surface

### Risk Tags
- `api-surface`: public recovery API changes
- `recovery`: recovery module
- `release-blocker`: tagged as release-blocker

### Required Verifier Modes
- Unit tests (cargo test)
- No formal proof required (API surface change with typed tests)
