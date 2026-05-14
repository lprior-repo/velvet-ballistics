# Black-Hat Review — vb-qi37.1.4

## Reviewer: Black-Hat (State 12)
## Date: 2026-05-14
## Bead: vb-qi37.1.4 — runtime/recovery: Fail closed on incomplete recovery

---

## VERDICT: APPROVED — All Defects Fixed

---

## PHASE 1: Contract & Bead Parity

### Contract Requirements Review

| Postcondition | Source | Implementation | Status |
|---|---|---|---|
| POST-001: Err when slot_taint=true | contract.md:46 | `recovery.rs:83` — `|| seed.unsupported.slot_taint` | ✓ CORRECT |
| POST-002: Err when pending_actions unsupported=true, regardless of is_empty | contract.md:47 | `recovery.rs:84` — `|| seed.unsupported.pending_actions` (FIXED) | ✓ CORRECT |
| POST-003: verify_digests Full verifies all digests | contract.md:48 | `recover.rs` — GAP-3 waiver on record | ✓ WAIVED |
| INV-GAP2-001 | contract.md:56 | `recovery.rs:84` | ✓ CORRECT |

### Fix Applied to test-plan.md

**DEFECT-1 (BLOCKING) was fixed**:
- Location: test-plan.md:73-80
- Before: `reject_returns_ok_when_pending_actions_unsupported_but_empty` expected `Ok(())`
- After: `reject_returns_err_when_pending_actions_unsupported_but_empty` expects `Err(RuntimeError::InvalidRecoveryHydration)`
- Note updated: POST-002 — unsupported.pending_actions triggers fail-closed regardless of pending_actions.is_empty()

**Fix verification**:
- Line 77 changed from `Then: returns Ok(())` to `Then: returns Err(RuntimeError::InvalidRecoveryHydration)`
- Scenario name changed from `reject_returns_ok_when_pending_actions_unsupported_but_empty` to `reject_returns_err_when_pending_actions_unsupported_but_empty`
- Note changed from documenting buggy behavior to documenting correct POST-002 behavior

### Verus spec parity (`recovery.rs:77-79`):
```rust
#[verus::spec]
fn reject_unsupported_live_frame_state_spec(seed: &RecoveryFrameSeed) -> bool {
    !seed.unsupported.slot_taint && !seed.unsupported.pending_actions && !seed.unsupported.slot_values
}
```
Spec = true ↔ all three unsupported flags are false ↔ function returns `Ok(())`. Implementation checks `any_true → Err`. LOGICALLY EQUIVALENT. ✓

---

## PHASE 2: Farley Engineering Rigor

### Function Metrics

| Function | Lines | Parameters | Status |
|---|---|---|---|
| `reject_unsupported_live_frame_state` | 9 | 1 | ✓ < 25 lines |
| `empty_recovered_frame` | 8 | 1 | ✓ < 25 lines |
| `apply_recovered_steps` | 4 | 2 | ✓ |
| `apply_recovered_slots` | 6 | 2 | ✓ |
| `apply_recovered_pc` | 7 | 1 | ✓ |
| `hydrate_run_frame` | 7 | 0 (method) | ✓ |

### Functional Core / Imperative Shell

`DurableFrameRecoveryBoundary::hydrate_run_frame` (line 63-70) is the imperative shell:
```rust
fn hydrate_run_frame(&self) -> RuntimeResult<RunFrame> {
    reject_unsupported_live_frame_state(&self.seed)?;  // validation
    let mut frame = empty_recovered_frame(&self.seed)?;
    apply_recovered_steps(&mut frame, &self.seed)?;
    apply_recovered_slots(&mut frame, &self.seed)?;
    apply_recovered_pc(&mut frame, &self.seed)?;
    Ok(frame)
}
```
Pure logic functions (`reject_unsupported_live_frame_state`, `empty_recovered_frame`) are separate from I/O. ✓

### Test Design

Tests assert behavior (WHAT), not implementation (HOW):
- `assert_eq!(frame.run_id(), run)` — behavior ✓
- `assert_eq!(frame.pc(), StepIdx::new(3))` — behavior ✓

---

## PHASE 3: Holzman Rust (The Big 6)

### Make Illegal States Unrepresentable

`UnsupportedRecoveryState` is a boolean flag struct. Each flag independently tracks unsupported state. ✓

### Parse, Don't Validate

`reject_unsupported_live_frame_state` is called at the hydration boundary (line 64) BEFORE any frame construction. ✓

### Types as Documentation

No boolean parameters. ✓

### No unwrap/expect/panic/todo in production

Checked `recovery.rs` — no forbidden patterns in modified functions. ✓

---

## PHASE 4: Ruthless Simplicity

### Panic Vector

Checked all modified functions:
- `reject_unsupported_live_frame_state`: uses `RuntimeResult` with typed error, no unwrap ✓
- `empty_recovered_frame`: uses `.map_err(|_| RuntimeError::InvalidRecoveryHydration)` — discards inner error but uses typed error variant ✓
- `apply_recovered_steps/slots/pc`: use `?` operator with typed errors ✓

### CUPID Properties

Composable: `RuntimeRecoveryBoundary` trait allows multiple implementations. ✓
Predictable: Pure boolean logic in `reject_unsupported_live_frame_state`. ✓
Idiomatic: Uses standard Rust error propagation. ✓

---

## PHASE 5: Bitter Truth

The fix at line 84 is correct and minimal. No clever workarounds. The logic is painfully obvious:
```rust
|| seed.unsupported.pending_actions  // if unsupported, fail closed
```

---

## SUMMARY OF FINDINGS

### DEFECT-1: FIXED ✓

**Before**: test-plan.md:73-80 expected `Ok(())` but POST-002 requires `Err`
**After**: test-plan.md:73-80 now expects `Err(RuntimeError::InvalidRecoveryHydration)` - CORRECT

All contract requirements satisfied. All phases pass.

---

## VERDICT

| Phase | Status |
|---|---|
| PHASE 1: Contract & Bead Parity | ✓ PASS |
| PHASE 2: Farley Engineering Rigor | ✓ PASS |
| PHASE 3: Holzman Rust | ✓ PASS |
| PHASE 4: Ruthless Simplicity | ✓ PASS |
| PHASE 5: Bitter Truth | ✓ PASS |

**APPROVED** — DEFECT-1 has been fixed. The test-plan.md now correctly reflects POST-002 behavior.

---

*black-hat-review: state 12 for vb-qi37.1.4 — APPROVED after DEFECT-1 fix*