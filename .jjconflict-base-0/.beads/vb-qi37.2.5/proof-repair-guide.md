# Proof Repair Guide — vb-qi37.2.5

## Status: REJECTED

This guide provides exact fixes required to pass proof-review for bead vb-qi37.2.5.

---

## Critical Fix Required Before Re-review

### Kani Harnesses Must Be Cargo-Integrated

**Problem**: The 3 new Kani harnesses (step_budget_kani.rs, run_until_blocked_kani.rs, value_store_cap_kani.rs) are standalone files in the `kani/` workspace root directory. They are NOT part of any cargo package and cannot be compiled or executed.

**Proof**: `cargo kani --package vb_core --harness step_budget_kani` returns "no harnesses matched the harness filter".

**Required Fix** (choose one approach):

#### Option A: Move harnesses into vb_core tests directory (RECOMMENDED)

1. Create `crates/vb_core/tests/vb_qi37_2_5_kani_harnesses.rs`
2. Copy contents from `kani/step_budget_kani.rs` into a `#[cfg(test)]` module
3. Copy contents from `kani/run_until_blocked_kani.rs` into a `#[cfg(test)]` module
4. Copy contents from `kani/value_store_cap_kani.rs` into a `#[cfg(test)]` module
5. Remove the standalone `kani/*.rs` files (or keep if they serve other purposes)
6. Update proof-obligations.jsonl commands to use the new harness names:
   - `cargo kani --package vb_core --harness vb_qi37_2_5_kani_harnesses::step_budget_kani`
   - Or use `--harness step_budget` if using default module naming

#### Option B: Create a separate kani-harnesses cargo package

1. Create `kani-harnesses/Cargo.toml` with:
   ```toml
   [package]
   name = "kani-harnesses"
   version = "0.1.0"
   edition = "2024"

   [dependencies]
   vb_core = { path = "../crates/vb_core" }
   ```
2. Create `kani-harnesses/src/lib.rs` with harness modules
3. Add to workspace members in root Cargo.toml
4. Update proof-obligations.jsonl commands to:
   `cargo kani --package kani-harnesses --harness step_budget_kani`

---

## High Priority Fixes

### Fix 1: Update verification-layers.md Reference Mismatch

**Problem**: verification-layers.md references `kani/gate_11_loop.rs` and `kani/gate_12_14_15.rs` but proof-writer created different files.

**Required Fix**: After integrating Kani harnesses (above), update `.beads/vb-qi37.2.5/verification-layers.md` lines 76-77 to reference the actual harness file location, or move the harness code into gate_11_loop.rs / gate_12_14_15.rs.

### Fix 2: Add Explicit Kani Unwind Bounds

**Problem**: While loops with 10,000 iterations may exceed Kani's default unwind bound.

**Required Fix**: Update proof-obligations.jsonl with explicit unwind bounds:

```jsonl
{"id":"KANI-INV-001","command":"cargo kani --package vb_core --harness <harness_name> --default-unwind 10000",...}
{"id":"KANI-INV-004","command":"cargo kani --package vb_core --harness <harness_name> --default-unwind 10000",...}
```

### Fix 3: Remove Trivial Assertions

**Problem**: `kani::assume(input >= 0)` on u64 does nothing; `kani::assert(remaining >= 0)` on u64 is always true.

**Required Fix**: In `kani/step_budget_kani.rs`:
- Remove line 24: `kani::assume(input >= 0);`
- Remove line 31: `kani::assert(remaining >= 0);` (u64 is always >= 0)

---

## Lower Priority — Consider After Critical Fixes

### Enhancement: Strengthen run_until_blocked Harness

**Problem**: The run_until_blocked harness only verifies budget counter decreases, not actual workflow execution.

**Consider**: Either (a) accept that Verus INV-004 loop invariant is the primary termination proof, OR (b) add a separate harness that calls the actual `run_until_blocked()` function with a minimal mock workflow.

---

## Verification After Fixes

After making these fixes, proof-reviewer (State 6) expects:

1. `cargo kani --package vb_core --harness <name>` finds and runs the harnesses without errors
2. Kani reports "N harnesses verified, 0 failures" (or equivalent)
3. verification-layers.md references match actual file locations
4. proof-obligations.jsonl contains correct harness names and explicit unwind bounds

---

## Owner

Proof-writer (State 5 re-entry)
