# Black Hat Review — vb-qi37.1.5

**STATUS: APPROVED**

---

## Verification Gate Evidence

```
$ cargo test -p vb_storage --lib
cargo test: 924 passed (1 suite, 2.04s)
EXIT: 0

$ cargo clippy -p vb_storage
cargo clippy: No issues found
EXIT: 0

$ cargo fmt --check
EXIT: 0
```

---

## PHASE 1: Contract & Bead Parity

### Signatures ✓
`recover.rs` implements all three contract functions exactly:
- `check_workflow_source_digest` — exact signature match
- `check_compiled_ir_digest` — exact signature match
- `verify_digests` — exact signature match

### Preconditions ✓
- **PRE-001** (non-empty events): `check_workflow_source_digest` returns `Err(NoRecoveryData)` for empty list
- **PRE-002** (verify_digests params): enforced via type-level `WorkflowDigest` and `&FjallJournal`
- **PRE-003** (non-empty events for frame seed): `recover_runtime_frame_seed_from_events_inner` uses `ok_or(NoRecoveryData)` at line 205-207

### Postconditions ✓
- **POST-001**: `check_workflow_source_digest` returns `Ok(())` only on digest match; `Err(WorkflowSourceDigestMismatch {...})` on mismatch — IMPLEMENTATION VERIFIED at `recover.rs:28-35`
- **POST-002**: `check_compiled_ir_digest` returns `Ok(())` iff `expected == found` — pure byte equality, IMPLEMENTATION VERIFIED at `recover.rs:46-50`
- **POST-003**: `verify_digests` checks workflow before IR (priority order) and short-circuits on first error — IMPLEMENTATION VERIFIED at `recover.rs:62-70`
- **POST-004**: `reject_workflow_digest_mismatch` returns `Ok(())` on match or absent; `Err(WorkflowSourceDigestMismatch)` on mismatch — IMPLEMENTATION VERIFIED at `summary.rs:182-199`
- **POST-005**: Corruption injection tests have formal waivers (WAIVER-FJALL-CORRUPT-001/002/003, WAIVER-EVENTSEQ-ORDER-001) with Kani+unit test compensating evidence

### Deferred Clauses ✓ (Documented)
- Action ABI digest verification deferred: comment at `recover.rs:71-72`
- Policy Digest Mismatch detection deferred: `RecoveryError::PolicyDigestMismatch` defined but not instantiated
- Formal waivers approved in proof-obligations.jsonl

### Invariants ✓
- **INV-001**: `WorkflowDigest` is `[u8; 32]` — byte-exact equality, proven by Kani
- **INV-002**: `check_compiled_ir_digest` is pure (no side effects) — trivially satisfied
- **INV-003**: `RecoveryError` variants exhaustive — proven by Kani harness `kani_ir_digest_error_variant_exhaustive`
- **INV-004**: `UnsupportedRecoveryState::union` is monotonic — unit test `unsupported_recovery_state_union_is_monotonic` at `summary.rs:1213-1243` PASSES

---

## PHASE 2: Farley Engineering Rigor

### Function Length ✓
All production functions ≤ 25 lines:
- `check_workflow_source_digest`: 18 lines
- `check_compiled_ir_digest`: 9 lines
- `verify_digests`: 21 lines
- `recover_runtime_summary`: 10 lines
- `recover_runtime_frame_seed`: 9 lines
- `recover_run_admission`: 10 lines
- `recover_all_incomplete_runs`: 19 lines

### Function Parameters ✓
Max 7 parameters (`verify_digests`) — within 5-7 limit.

### I/O Separation ✓
`check_compiled_ir_digest` is pure — no I/O, no side effects. All other functions delegate to FjallJournal for I/O. No I/O hiding inside calculations.

### Test Design ✓
Unit tests assert behavior (WHAT) not implementation (HOW). Tests verify error variant exactness and digest match/mismatch outcomes.

---

## PHASE 3: Holzman Rust (The Big 6)

### Illegal States Unrepresentable ✓
- `RecoveryError`: sum type covering all failure modes
- `DigestCheck`: 3 explicit variants
- `UnsupportedRecoveryState`: additive flags with `union`
- `RecoveryHydration`: enum with `Summary`/`FrameSeed` variants
- `RecoveryTerminalState`: enum for `Cancelled`/`Finished`/`Failed`

### Parse, Don't Validate ✓
- `check_workflow_source_digest` parses journal events and extracts `RunAccepted.workflow` digest at the boundary — no validation without parsing
- `recover_runtime_frame_seed_from_events_with_workflow` calls `reject_workflow_digest_mismatch` before reconstruction

### Types as Documentation ✓
No boolean parameters in any public function signature.

### Workflows Explicit ✓
`reject_workflow_digest_mismatch` implements explicit state-to-state transition: `RunAccepted` present → check digest; absent → Ok.

### Newtypes ✓
`WorkflowDigest` wraps `[u8; 32]` — no unwrapped primitives in domain models.

---

## PHASE 4: Ruthless Simplicity & DDD

### No Option-based State Machines ✓
Error handling uses `RecoveryResult<T>` with exhaustive enum variants, not `Option<T>` as a state machine.

### CUPID ✓
- **Composable**: Functions chain (`verify_digests` → `check_workflow_source_digest` → `check_compiled_ir_digest`)
- **Predictable**: `check_compiled_ir_digest` is pure and deterministic
- **Idiomatic**: Standard Rust error handling with `thiserror`
- **Domain-based**: All types map to recovery domain vocabulary

### Panic Vector ✓ (CLEAN)
**Production code** (recover.rs, types.rs, summary.rs non-test):
- Zero `unwrap()`, `expect()`, `panic!()` calls
- One `ok_or(NoRecoveryData)` at `summary.rs:207` — appropriate early return
- Zero `unsafe` blocks (crates use `#![forbid(unsafe_code)]`)

**Test code**: `unwrap()`/`expect()` calls in `#[cfg(test)]` blocks are appropriate for test scaffolding.

### `let mut` Justification ✓
- `recover.rs:119`: `let mut recovered = Vec::new()` — necessary for accumulation loop
- `summary.rs:94`: `let mut summary` — necessary for event-by-event accumulation
- `summary.rs:537`: `let mut store` — necessary for `ValueStore::new()` which requires mutability
- All others in test blocks

---

## PHASE 5: The Bitter Truth

### Punish Cleverness ✓
- `verify_digests` is brutally linear: match level → call workflow check → call IR check → Ok
- `reject_workflow_digest_mismatch` uses `find_map` idiom — appropriate, not clever
- No hidden state machines or implicit flows

### YAGNI ✓
- No abstract traits with single implementers
- No generic handlers for speculative future use
- `RecoveryFrameSeedBuilder` is retained as documented compatibility adapter (lines 127-160), not YAGNI

### Sniff Test ✓
Code reads like a senior engineer wrote it: types are tight, errors are typed, comments explain deferred items. No junior-developer cleverness detected.

---

## Contract Parity Defects

**None.** All discrepancies are documented deferred items with formal waivers:
1. Action ABI digest verification — deferred (recover.rs:71-72), formal waiver approved
2. Policy Digest Mismatch detection — deferred, formal waiver approved
3. Fjall corruption injection tests — blocked by tooling, formal waivers approved
4. EventSeq ordering validation — not implemented, formal waiver approved

---

## Summary

| Gate | Status |
|------|--------|
| Tests (924 passed) | ✓ |
| Clippy (0 issues) | ✓ |
| Format (0 issues) | ✓ |
| Panic vector (production) | ✓ CLEAN |
| Unsafe (production) | ✓ FORBIDDEN |
| Contract parity | ✓ |
| Function length | ✓ |
| Type discipline | ✓ |
| DDD fidelity | ✓ |

**VERDICT: APPROVED**

The implementation correctly proves replay digest mismatch detection. All contract obligations are satisfied, deferred items have formal waivers, and the code is clean by all five review phases. No rewrites required.
