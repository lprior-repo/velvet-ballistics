**Bead**: vb-3l40a
**State**: 5
**Reviewer**: black-hat-reviewer (local5090)
**Source checkout**: /home/lewis/src/isoloated/velvet-ballistics-vb-3l40a-review-5090
**Attempt**: 2 (post-fix)
**Review snapshot**: 92186542 vs baseline da633fe0

**STATUS: REJECTED**

---

## PHASE 1: Contract & Bead Parity

| Requirement | Status | Evidence |
|-------------|--------|----------|
| Restore vb_runtime compilation | ✅ | `cargo check -p vb_runtime` passes, 0 errors |
| Define RecoverRunCommand with all fields | ✅ | `shard/types.rs` has struct with run, frame, artifact_digest, workflow_digest, next_seq, collect_states, boundary |
| ShardCommand::Recover expanded | ✅ | Carries RecoverRunCommand |
| validate_submit_admission_with_contracts added | ⚠️ | Method exists but is WEAK — validates duplicate IDs and empty names only, no contract semantics |
| action_abi_digest methods implemented | ⚠️ | Methods exist but `action_abi_digests_remove` is a no-op placeholder |
| Input slot slot_value_kind exhaustive | ✅ | Catch-all arm added |
| Error diagnostics complete | ✅ | Match arms added |
| Recovery preserves action contracts | ❌ | `handle_recover` sets `action_contracts: Box::new([])` — empty. Artifact contains them but they're discarded |

---

## PHASE 2: Farley Engineering Rigor

| Function | Lines | Limit | Status |
|----------|-------|-------|--------|
| handle_recover | ~90 | 25 | ❌ OVER |
| handle_submit_with_inputs_contracts_and_header_mode | ~60 | 25 | ❌ OVER |
| validate_submit_admission_with_contracts | 35 | 25 | ❌ OVER |
| seed_input_slots | ~20 | 25 | ✅ |
| action_abi_digest_for_run_action | 20 | 25 | ✅ |

**Note**: `handle_recover` and `handle_submit_with_inputs_contracts_and_header_mode` are far too large. `validate_submit_admission_with_contracts` is over 25 lines and could be split.

---

## PHASE 3: Holzman Rust (The Big 6)

| Rule | Status |
|------|--------|
| Zero `unsafe` | ✅ |
| Zero `.unwrap()`/`.expect()` in production | ✅ |
| Zero `panic!`/`todo!`/`dbg!` | ✅ |
| Checked arithmetic | ✅ |
| No boolean parameters | ✅ |
| No unchecked indexing | ✅ |

---

## PHASE 4: Ruthless Simplicity & DDD

| Check | Status |
|-------|--------|
| No Option-based state machines | ✅ |
| CUPID compliant | ✅ |
| No clever abstractions | ✅ |
| Dead code: slot_value_kind catch-all | ⚠️ All variants covered, catch-all is dead code |
| Dead code: action_abi_digests_remove | ⚠️ Empty placeholder with no effect |

---

## PHASE 5: The Bitter Truth

The code looks functional but has significant design shortcuts. The recovery path discards action contracts entirely, which means recovered runs cannot execute any actions — this is a correctness bug that will cause runtime failures. The admission validation method is named as if it validates contracts but only checks for duplicate IDs and empty names. This smells like a junior developer trying to make the tests pass without understanding the recovery semantics.

---

## Findings (Ordered by Severity)

| Finding | Severity | File:Line | Status |
|---------|----------|-----------|--------|
| Recovery discards action contracts | CRITICAL | shard/lifecycle/chunk_001.rs:444 | open |
| validate_submit_admission_with_contracts is weak | HIGH | shard/lifecycle/chunk_001.rs:306 | open |
| action_abi_digests_remove is no-op | MEDIUM | shard/impl_parts/chunk_001.rs:347 | open |
| slot_value_kind has dead catch-all | LOW | shard/helpers.rs:69 | open |
| handle_recover is 90 lines | MEDIUM | shard/lifecycle/chunk_001.rs:383 | open |

### [FINDING-001]: Recovery discards action contracts — recovered runs cannot execute actions

**Location**: `crates/vb_runtime/src/shard/lifecycle/chunk_001.rs:444`

**Problem**: `handle_recover` constructs `RunState` with `action_contracts: Box::new([])` — an empty array. The artifact loaded during recovery contains the actual action contracts (`artifact.action_contracts`), but they're discarded. When a recovered run later tries to execute an action, `action_abi_digest_for_run_action` looks up the contract from `state.action_contracts` and fails with `action_contract_not_found` (`UnsupportedOperation`).

**Evidence**:
```rust
// handle_recover at chunk_001.rs:444
let state = crate::shard::types::RunState {
    // ...
    action_contracts: Box::new([]),  // EMPTY — artifact has them but we throw them away
};
```

Meanwhile the artifact comment says: "The runtime needs these contracts after crash recovery to validate action completion size, taint, idempotency, and output-slot authority."

**Impact**: ANY recovered run that needs to execute an action will fail. This is not a stub issue — it's a correctness bug that will cause production failures on restart/recovery.

**Required Fix**: Use `artifact.action_contracts` (converted appropriately) instead of `Box::new([])`.

---

### [FINDING-002]: validate_submit_admission_with_contracts is weak — only duplicate ID check

**Location**: `crates/vb_runtime/src/shard/lifecycle/chunk_001.rs:306`

**Problem**: The method name implies contract validation, but it only:
1. Calls `build_admission` (which validates capabilities against artifact)
2. Checks for duplicate action IDs
3. Checks for empty action names

It does NOT validate:
- Contract capabilities vs. granted capabilities
- Contract counts vs. workflow requirements
- Contract digests vs. artifact digests
- Contract compatibility with the workflow

**Evidence**:
```rust
pub(crate) fn validate_submit_admission_with_contracts(
    &self,
    run: RunId,
    digest: vb_core::ids::WorkflowDigest,
    caps: CapabilitySet,
    action_contracts: &[vb_core::action::ActionContract],
) -> RuntimeResult<()> {
    // Build admission to validate capabilities
    self.build_admission(run, digest, caps)?;

    // Only checks duplicates and empty names — no real contract validation
    let mut seen_ids = std::collections::HashSet::new();
    for contract in action_contracts {
        if !seen_ids.insert(contract.id) {
            return Err(RuntimeError::ContractValidationError { ... });
        }
    }
    // ...
}
```

**Impact**: The method is a weak gate. It provides a false sense of security that contracts are validated, when in reality only basic sanity checks are performed. Real contract validation happens later during execution, not at admission.

**Required Fix**: Either rename to `validate_submit_admission` (since it doesn't actually validate contracts) or implement real contract validation (capability matching, count checks, digest checks).

---

### [FINDING-003]: action_abi_digests_remove is an empty placeholder

**Location**: `crates/vb_runtime/src/shard/impl_parts/chunk_001.rs:347`

**Problem**: `action_abi_digests_remove` is called by the terminal transition cleanup path, but it's an empty method. The implementation report justifies this by saying "no separate digest cache exists," but if there's no cache, the method is meaningless. This is a placeholder that silently does nothing.

**Evidence**:
```rust
pub(crate) fn action_abi_digests_remove(&mut self, run: RunId) -> RuntimeResult<()> {
    Ok(())
}
```

**Impact**: If future code adds a digest cache, this method will silently fail to clean up. It's a maintenance hazard and code smell.

**Required Fix**: Either implement actual cleanup logic or remove the method and update all callers. If the method is truly a no-op, document why it exists and should be called.

---

### [FINDING-004]: slot_value_kind has unnecessary dead catch-all

**Location**: `crates/vb_runtime/src/shard/helpers.rs:69`

**Problem**: All `SlotValue` variants are already matched explicitly. The `&_ => InputSlotKind::Blob` catch-all is dead code that will never match.

**Evidence**:
```rust
fn slot_value_kind(value: &SlotValue) -> vb_core::InputSlotKind {
    match value {
        SlotValue::Null => vb_core::InputSlotKind::Null,
        SlotValue::Bool(_) => vb_core::InputSlotKind::Bool,
        // ... all variants covered ...
        SlotValue::Blob(_) => vb_core::InputSlotKind::Blob,
        &_ => vb_core::InputSlotKind::Blob,  // Dead code
    }
}
```

**Required Fix**: Remove the dead catch-all arm. If `SlotValue` is `#[non_exhaustive]` and future variants are expected, add a comment explaining why.

---

### [FINDING-005]: handle_recover is 90 lines — too large

**Location**: `crates/vb_runtime/src/shard/lifecycle/chunk_001.rs:383`

**Problem**: The recovery handler is far too large and complex. It should be split into smaller focused functions: validate recovery inputs, restore run state, handle boundary, drive from recovered state.

**Required Fix**: Split `handle_recover` into helper functions: `restore_recovered_run_state`, `handle_recovery_boundary`, etc.

---

## Quality Gates

| Gate | Result | Evidence |
|------|--------|----------|
| `cargo check -p vb_runtime` | ✅ | 0 errors, 1 pre-existing warning |
| `cargo test -p vb_runtime --lib` | ✅ | 1875 passed, 0 failed |
| `cargo clippy -p vb_runtime --lib` | ⚠️ | Would need to run to verify |
| `cargo fmt --check -p vb_runtime` | ⚠️ | Would need to run to verify |

**Note**: I verified compilation and tests. Clippy and fmt would need separate runs to confirm.

---

## Verdict

**STATUS: REJECTED**

### Summary
The compilation issues are resolved and all 1875 tests pass, but the implementation has a critical correctness bug: recovery discards action contracts, causing recovered runs to fail when executing any actions. Additionally, the contract validation method is weak and provides only a false sense of admission security. These are not stubs or field drops — they're real semantic gaps that will cause production failures. The code compiles and passes tests, but the tests don't exercise the full recovery+action-execution path, so they don't catch this.

---

## Required Repair Actions

1. **[CRITICAL]** Fix `handle_recover` to use `artifact.action_contracts` instead of `Box::new([])` — this is the main blocker for acceptance
2. **[HIGH]** Either rename `validate_submit_admission_with_contracts` to match its actual behavior (duplicate ID check only) or implement real contract validation
3. **[MEDIUM]** Either implement `action_abi_digests_remove` or remove it and update callers
4. **[LOW]** Remove the dead `&_ => Blob` catch-all arm in `slot_value_kind`
5. **[MEDIUM]** Split `handle_recover` into smaller functions (< 25 lines each)
6. **[TEST]** Add a recovery test that exercises action execution after recovery to catch this class of bug
