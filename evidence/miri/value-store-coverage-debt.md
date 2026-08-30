# Miri Skipped Value-Store Property Coverage Debt

**Bead:** `vb-5m428` (Miri: document skipped value-store property coverage debt)
**Date:** 2026-08-30
**Crate:** `vb_core` (value-store module)
**Severity:** P2 -- coverage gap with compensating verification

## Summary

Miri's UB-detector gate skips a significant subset of value-store property tests
because the test fixtures exceed Miri's practical execution budget. This document
records every skipped test, the reason for skipping, and the compensating coverage
that ensures correctness despite the gap.

## Current Miri Gate Scope

The canonical Miri task (`.moon/tasks/all.yml:459-472`) exercises only:

```
cargo miri test --quiet -p vb_core --lib --all-features ids::tests::run_id_zero_constant
```

This single test in `vb_core::ids` is a narrow smoke check. The Miri lane in
`xtask/src/lanes.rs:43-50` would run `cargo miri test -p <crate>` for a broader
scan, but the moon task explicitly narrows to one test to keep the 3-minute timeout
enforceable.

## Skipped Value-Store Tests

### 1. `value_store_object_at_exact_max_fields_is_accepted`

| File | Line |
|------|------|
| `crates/vb_core/src/value_store/tests.rs` | 530 |
| `crates/vb_core/src/value_store/tests_and_verification.rs` | 567 |

```rust
#[cfg_attr(miri, ignore = "max-size object fixture is too slow under Miri")]
fn value_store_object_at_exact_max_fields_is_accepted() -> Result<(), String>
```

**What it tests:** Inserting an object with exactly `MAX_OBJECT_FIELDS_PER_VALUE`
fields succeeds and round-trips correctly.

**Why skipped:** The test constructs a `Vec` of `MAX_OBJECT_FIELDS_PER_VALUE`
`ObjectField` structs. Miri's interpreted execution makes this allocation-heavy
fixture exceed practical timeouts.

**Compensating coverage:**
- Standard `cargo test` exercises this path on every commit.
- The `fuzz_core_value_store` target (planned in `fuzz/FUTURE.md` item #39)
  would exercise object insertion with fuzzed field counts.

### 2. `value_store_exact_max_object_preserves_duplicate_first_wins_index`

| File | Line |
|------|------|
| `crates/vb_core/src/value_store/tests.rs` | 914 |
| `crates/vb_core/src/value_store/tests_and_verification.rs` | 958 |

```rust
#[cfg_attr(miri, ignore = "max-size object fixture is too slow under Miri")]
fn value_store_exact_max_object_preserves_duplicate_first_wins_index() -> Result<(), String>
```

**What it tests:** When inserting an object with duplicate keys at exact max
capacity, the first-wins deduplication semantics are preserved.

**Why skipped:** Same reason as #1 -- the max-size fixture is too slow under
Miri's interpreted model.

**Compensating coverage:**
- Standard `cargo test` covers this path.
- Verus proof `value_store_invariant.rs` (REJECTED -- see below) attempted
  but did not succeed in formally proving this invariant.

### 3. `property_value_store_cap` (proptest)

| File | Line |
|------|------|
| `crates/vb_core/src/value_store/tests.rs` | 1526 |
| `crates/vb_core/src/value_store/tests_and_verification.rs` | 1572 |

```rust
#[cfg_attr(miri, ignore)]
#[test]
fn property_value_store_cap(cap: u16, insert_count: u16)
```

**What it tests (POPPRE-002):** ValueStore inserts return
`CoreError::BudgetExceeded` when `total_arena_count >= max_arena_entries`.
This is a proptest with arbitrary `cap` and `insert_count` inputs, verifying
the cap enforcement contract.

**Why skipped:** No explicit reason given. Proptest generates hundreds of
shrunk test cases per run, and Miri's slow interpreted execution makes
property-based testing infeasible.

**Compensating coverage:**
- `cargo test -p vb_core --lib` runs this proptest under normal conditions.
- The Verus file `verification/verus/value_store_invariant.rs` attempts to
  prove an equivalent cap invariant but was **REJECTED** by proof-review
  (see `verification/verus/proof-review.md` line 343) for producing vacuous
  tautologies rather than binding to production semantics.

## Compensating Verification Matrix

| Property | Miri | cargo test | Proptest | Verus | Kani | Fuzz |
|----------|------|-----------|----------|-------|------|------|
| Max-fields object insert | SKIPPED | Yes | No | No | No | Planned |
| Duplicate-key first-wins | SKIPPED | Yes | No | No | No | Planned |
| Arena cap enforcement | SKIPPED | Yes | Yes | Rejected | No | Planned |
| Symbol/string insertion | Covered | Yes | No | No | No | No |
| Blob insert/round-trip | Covered | Yes | No | No | No | No |

**Key observation:** The Miri gate currently only covers `ids::tests::run_id_zero_constant`,
which exercises the `ids` module but not the value-store code path. All value-store
correctness for Miri is therefore **unverified by UB detection** -- covered only by
standard test runners.

## Verus Value-Store Invariant (Rejected)

File: `verification/verus/value_store_invariant.rs`

The proof-review at `verification/verus/proof-review.md:343` rejected this file
with status REJECTED. The core issues:

- `proof_uncapped_always_allows` (line 76): Proves `spec_value_store_cap(total+1, 0)`
  with `max_entries=0`, which evaluates to `true` -- a vacuous tautology.
- `proof_cap_one_rejects_second` (line 84): Proves `spec_value_store_cap(1,1)` and
  `!spec_value_store_cap(2,1)` by direct evaluation -- no production binding.
- `proof_total_never_exceeds_cap` (line 118): The universal quantifier reduces to
  `max_entries == 0 || t <= max_entries` after reveal, which is trivially given.

The proof file does not establish a non-vacuous invariant over production
`ValueStore` behavior. A rework would need to bind through
`extern_value_store_invariant.rs` with actual production body mirroring rather
 than spec-mode evaluation.

## Recommendations

1. **Document the gap explicitly.** This file serves that purpose. Future agents
   auditing Miri coverage debt should find it here.

2. **Consider Miri lane expansion** (P3). If value-store tests can be refactored
   to use smaller fixtures (e.g., `MAX_OBJECT_FIELDS_PER_VALUE / 2`) with a
   separate large-fixture test that is explicitly annotated, Miri could gain
   value-store coverage without the 3-minute timeout constraint.

3. **Complete fuzz target #39** (`fuzz_core_value_store`) from `fuzz/FUTURE.md`.
   A property-based fuzzer would exercise the same paths as the skipped tests
   but with arbitrary inputs, providing real UB-detection coverage.

4. **Rework the Verus invariant.** The `spec_value_store_cap` function is
   well-defined; the proofs need to be restructured to establish non-trivial
   invariants that depend on production `ValueStore` state, not just the
   spec function's algebraic properties.

## Evidence

- Miri gate command: `.moon/tasks/all.yml:464`
- Miri lane config: `xtask/src/lanes.rs:43-50`
- Miri gate implementation: `xtask/src/gates.rs:112-113`
- Skipped test refs: `crates/vb_core/src/value_store/tests.rs:530,914,1526`
- Duplicate in extended tests: `crates/vb_core/src/value_store/tests_and_verification.rs:567,958,1572`
- Verus rejection: `verification/verus/proof-review.md:343`
- Fuzz target plan: `fuzz/FUTURE.md:100`
- Master Miri scope: `velvet-ballistics-MASTER.md:1700`
