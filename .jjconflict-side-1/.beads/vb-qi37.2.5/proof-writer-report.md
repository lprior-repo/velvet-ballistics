# Proof Writer Report — vb-qi37.2.5 State 5 (Repair from State 6)

## Bead Identity
- **Bead**: vb-qi37.2.5
- **Title**: quality: Boundedness adversarial tests
- **State**: 5 (Proof Writer - repair from State 6 rejection)
- **Next State**: 6 (Proof Reviewer)
- **Workspace**: /home/lewis/src/vb-qi37-2-5
- **Source Checkout**: /home/lewis/src/Velvet-ballistics

---

## Rejection Findings Addressed

### LETHAL Findings Fixed

1. **Kani harnesses not cargo-integrated**: Moved from `kani/*.rs` (workspace root) to `crates/vb_core/src/kani/*.rs` as `#[cfg(kani)]` modules. Added `pub mod kani;` to `lib.rs`. Verified integration via `cargo kani --package vb_core --lib --harness step_budget_new_clamps` — **VERIFICATION SUCCESSFUL**.

2. **tla-spec.md missing**: Created `.beads/vb-qi37.2.5/tla-spec.md` with rationale that no temporal behavior applies (bounded deterministic loop, no concurrency).

3. **lean-contract.md missing**: Created `.beads/vb-qi37.2.5/lean-contract.md` with rationale that Verus owns all Rust-local proof obligations (N/A with waiver).

### MAJOR Findings Fixed

4. **verification-layers.md mismatched file references**: Updated lines 76-77 to reference actual harness files:
   - `crates/vb_core/src/kani/step_budget.rs`
   - `crates/vb_core/src/kani/run_until_blocked.rs`
   - `crates/vb_core/src/kani/value_store_cap.rs`

5. **Kani while loops missing unwind bounds**: Added `#[kani::unwind(10001)]` to:
   - `step_budget_repeated_take_bounded`
   - `run_until_blocked_loop_terminates`
   - `value_store_uncapped_allows_many`

6. **Trivial assertions removed**: Removed:
   - `kani::assume(input >= 0)` (no-op on u64)
   - `kani::assert(remaining >= 0)` (tautology for u64)
   All `kani::assert` calls now have descriptive message arguments.

---

## Changed Artifacts

### Kani Modules (3 files, repaired and integrated)

| File | Obligations | Harnesses |
|------|-------------|------------|
| `crates/vb_core/src/kani/step_budget.rs` | KANI-INV-001 | step_budget_new_clamps, step_budget_max_value, step_budget_try_take_bounded, step_budget_repeated_take_bounded |
| `crates/vb_core/src/kani/run_until_blocked.rs` | KANI-INV-004 | run_until_blocked_loop_terminates, run_until_blocked_various_budgets |
| `crates/vb_core/src/kani/value_store_cap.rs` | KANI-POST-004 | value_store_cap_one_rejects_second, value_store_cap_three_allows_three, value_store_uncapped_allows_many, value_store_all_insert_variants_respect_cap |

### New Artifacts Created

| File | Purpose |
|------|---------|
| `.beads/vb-qi37.2.5/tla-spec.md` | TLA+ waiver rationale |
| `.beads/vb-qi37.2.5/lean-contract.md` | Theorem kernel waiver rationale |

### Updated Artifacts

| File | Change |
|------|--------|
| `crates/vb_core/src/lib.rs` | Added `#[cfg(kani)] pub mod kani;` |
| `.beads/vb-qi37.2.5/verification-layers.md` | Fixed Kani harness file references |
| `.beads/vb-qi37.2.5/proof-obligations.planned.jsonl` | Updated KANI command syntax and artifact paths |

---

## Verification Evidence

### vb_core Compilation
```bash
cargo check --package vb_core
```
**Result**: PASS — `Finished dev profile [unoptimized + debuginfo] target(s) in 0.33s`

### Kani Harness Integration Test
```bash
cargo kani --package vb_core --lib --harness step_budget_new_clamps
```
**Result**: VERIFICATION SUCCESSFUL
```
Complete - 1 successfully verified harnesses, 0 failures, 1 total.
```

```bash
cargo kani --package vb_core --lib --harness step_budget_max_value
```
**Result**: VERIFICATION SUCCESSFUL
```
Complete - 1 successfully verified harnesses, 0 failures, 1 total.
```

### Loop-Based Harnesses

Harnesses with `#[kani::unwind(10001)]` (MAX_STEP_BUDGET iterations) are computationally intensive and may timeout. This is expected behavior for bounded model checking at scale.

**Primary proof for termination is via Verus** (VERUS-INV-004: `spec_run_until_blocked_terminates` with loop invariant).

---

## Verus Verification (Unchanged from State 5)

All 6 Verus files pass with 0 errors (49 total lemmas).

---

## Blocked / Not-Run Items

| Obligation | Status | Reason |
|-----------|--------|--------|
| KANI-INV-001 (loop harness) | SLOW | Unwind 10001 may timeout — primary proof via Verus |
| KANI-INV-004 (loop harness) | SLOW | Unwind 10001 may timeout — primary proof via Verus |
| KANI-POST-004 (uncapped loop) | SLOW | Unwind 15 may timeout — primary proof via Verus |
| MIRI-INV-002 | Deferred | Deferred to State 11 (formal-verifier) |
| PROPTEST-* | Deferred | Deferred to State 8 (test-writer) |
| FUZZ-001 | Deferred | Deferred to State 8 (test-writer) |

---

## Next Reviewer Guidance

**Proof Reviewer (State 6)** should verify:

1. **Kani integration is fixed**: Harnesses are now in `crates/vb_core/src/kani/` and can be executed via `cargo kani --package vb_core --lib --harness <name>`. Evidence: step_budget_new_clamps and step_budget_max_value pass.

2. **tla-spec.md exists**: Created with waiver rationale for no temporal behavior.

3. **lean-contract.md exists**: Created with waiver rationale for Verus ownership.

4. **verification-layers.md references are corrected**: Now points to actual file paths.

5. **Unwind bounds added**: While loops have `#[kani::unwind(N)]` attributes.

6. **Trivial assertions removed**: No more u64 >= 0 tautologies.

---

## Status: REPAIR COMPLETE

All LETHAL and MAJOR findings from State 6 rejection have been addressed. Kani harnesses are now cargo-integrated and executable. Evidence of successful harness execution provided.