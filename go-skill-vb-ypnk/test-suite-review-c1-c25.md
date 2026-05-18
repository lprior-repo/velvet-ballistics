# Test Suite Review: LETHALs C.1–C.25 (MODE 2: Suite Inquisition)

## VERDICT: **REJECTED**

---

## Tier 0 — Static Analysis

| Check | Result |
|-------|--------|
| Banned pattern scan | **FAIL** — see LETHAL-1 |
| Determinism/evidence scan | PASS |
| Mock interrogation | PASS |
| Integration test purity | PASS |
| Error variant completeness | PASS |
| Density audit | PASS (1795 tests / 294 pub fn = **6.1x**, target ≥5x) |

### LETHAL FINDINGS

**LETHAL-1: Empty property_tests.rs — No property-based test infrastructure**
- **File:** `crates/vb_runtime/src/engine/property_tests.rs`
- **Severity:** LETHAL
- **Finding:** File contains 1 byte (empty). This is the designated home for proptest/proptest-derived tests covering the Calc/Data layer. For LETHALs C.1–C.25 covering expression evaluation, slot value semantics, budget exhaustion, and taint propagation, there are **zero property-based tests** in this file.
- **Implication:** Every pure-function invariant (budget clamping, taint monotonicity, slot value encoding) relies solely on example-based tests. Mutation kill rate is bounded by author imagination, not exhaustively generated inputs.

**LETHAL-2: Thin-wrapper fuzz targets mask weak assertions in fuzz library**
- **Files:** `fuzz/fuzz_targets/generated_compare.rs`, `fuzz/fuzz_targets/compiled_ir.rs`, `fuzz/fuzz_targets/ipc_frame.rs`, `fuzz/fuzz_targets/expression.rs`
- **Severity:** LETHAL
- **Finding:** All four files are **thin wrappers** (30–31 lines each) that call `fuzz_lib::fuzz_*`. The actual implementations live in `fuzz/src/lib.rs`. The task's static analysis grep on these files finds nothing, but the underlying library has weak assertions:
  - `fuzz/src/lib.rs:54` — `assert!(result.is_ok())` in `fuzz_capability_name_schema`
  - `fuzz/src/lib.rs:214` — `match decode_frame_payload(...) { Ok(_) | Err(_) => {} }` (silent error drop)
  - `fuzz/src/lib.rs:232,240,249` — `let _ = vb_ipc::IpcFrameHeader::decode(...)` (suppressed errors)
  - `fuzz/src/lib.rs:1905` — `assert!(result.is_ok(), "try_take must not error")` — acceptable per context
- **Implication:** The "thin wrapper" pattern means the listed files give **false confidence**. Real coverage lives in lib.rs which contains banned assertion patterns.

**LETHAL-3: Missing test coverage for `ShardDirective::allows_admission` edge cases**
- **File:** `crates/vb_runtime/src/shard/directive.rs`
- **Severity:** LETHAL
- **Finding:** `allows_admission()` returns `true` only for `Continue`. The test suite tests all 4 variants but does **not** test the `has_migration_target()` behavior which always returns `false` (lines 73–75). The docstring claims "Only `Migrate` carries a target" but `Migrate` **does not exist** in the enum. This is a **contract/docstring mismatch** — the function always returns `false` regardless of variant.

---

## Tier 1 — Execution

| Check | Result |
|-------|--------|
| Test compile | PASS (builds successfully) |
| nextest | **1469 passed, 0 failed, 0 flaky** |
| Ordering probe | PASS (consistent at threads=1 and threads=8) |
| Insta | N/A (no insta dependency detected) |

---

## Tier 2 — Coverage

Not run. Tier 0 rejection stops the pipeline.

---

## Tier 3 — Mutation

Not run. Tier 0 rejection stops the pipeline.

---

## MAJOR FINDINGS (3)

1. **`has_migration_target()` always returns `false`** — The function at `directive.rs:73–75` unconditionally returns `false` because `ShardDirective` has no `Migrate` variant. The docstring at line 69–72 describes a `Migrate` variant that does not exist. This is dead code hiding a design inconsistency.

2. **Fuzz target `collect_page_pagination.rs` is a stub** — The binary at `fuzz/src/bin/collect_page_pagination.rs` (47 lines) calls `fuzz_lib::fuzz_collect_page_pagination` which does **not exist** in `fuzz/src/lib.rs`. The `collect_page` pagination obligation (C.25) has no implementation backing it.

3. **No test for `ShardDirective` behavior under invalid discriminants** — The enum is `u8`-derived but tests only exercise the 4 valid variants. No test verifies behavior if a discriminant is constructed via unsafe/transmutation (not applicable in safe Rust, but worth noting for verification completeness).

---

## MINOR FINDINGS (2/5 threshold)

1. `fuzz/src/lib.rs:1347–1383` — Multiple `.ok()` calls that discard `Result` in accessor traversal setup (`run_with_data.write_slot_with_taint(...).ok()`). These are in test setup, not assertions, but they hide potential setup failures silently.

2. `directive.rs:280–298` — `shard_directive_exhaustive_match` test is tautological: it iterates over all variants and assigns to `_description` but asserts nothing. The "test" proves only that the match is exhaustive — not that any behavior is correct.

3. `directive.rs:255–272` — `shard_directive_all_variants_serializable` uses a flawed assertion: it checks `debug_str.contains("Continue") || debug_str.contains("Suspend") || ...` which passes for ANY variant because the debug string will always contain one of these names. Should be an exact match per variant.

4. `fuzz/src/lib.rs:1518–1520` — Three `let _ = ...` in `fuzz_strict_artifact_decoder` suppress all decode results without assertion, making the function a no-op oracle.

5. `property_tests.rs` — Empty file. No placeholder, no `TODO`, no说明. Completely blank.

---

## MANDATE

Before resubmission, the following **must** be addressed:

1. **LETHAL-1:** Populate `crates/vb_runtime/src/engine/property_tests.rs` with property-based tests for:
   - `StepBudget::new` clamping (FUZZ-001 boundary)
   - Taint propagation monotonicity (Clean input → Clean output)
   - Slot value encoding/decoding roundtrip invariants
   - Expression evaluation Result-returning contract (never panics)

2. **LETHAL-2:** Either:
   - (a) Move the actual fuzz target bodies into the `fuzz/fuzz_targets/*.rs` files directly (eliminating the thin-wrapper pattern), OR
   - (b) Audit `fuzz/src/lib.rs` and fix all `assert!(result.is_ok())` / `assert!(result.is_err())` to use exact variant matching

3. **LETHAL-3:** Fix `has_migration_target()` docstring to match implementation, or implement the `Migrate` variant if the docstring is the source of truth.

4. **MAJOR-2:** Implement `fuzz_collect_page_pagination` in `fuzz/src/lib.rs` or remove the stub binary.

5. **MINOR-3:** Fix `shard_directive_all_variants_serializable` to check exact variant names per-variant instead of using a compound OR.

---

## SUMMARY

| Tier | Status |
|------|--------|
| Tier 0 Static | **REJECTED** (3 LETHAL) |
| Tier 1 Execution | PASSED (would pass) |
| Tier 2 Coverage | SKIPPED |
| Tier 3 Mutation | SKIPPED |

**The suite is not ready.** Three LETHAL findings require immediate resolution. The empty `property_tests.rs` combined with thin-wrapper fuzz targets hiding weak assertions in `lib.rs` means the LETHAL C.1–C.25 coverage contract is **not satisfied**. Submitter must resolve all LETHALs before re-review.
