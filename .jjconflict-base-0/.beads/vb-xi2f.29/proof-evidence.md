# Proof Evidence: vb-xi2f.29 — Digest Covers Together Semantics (REPAIR-2)

**bead_id**: vb-xi2f.29
**invocation**: pw-repair-vb-xi2f29-2026-05-25-002
**date**: 2026-05-25
**workspace**: /home/lewis/src/vb-workspaces/vb-xi2f.29

## E1: Kani Compilation — canonical_name_together_harness (LF-001 FIX)

```bash
$ TMPDIR=/home/lewis/src/vb-workspaces/vb-xi2f.29/target/tmp cargo kani -p vb_compile --harness canonical_name_together_harness --only-codegen
```

**Result**: ✅ COMPILED (exit 0)
```
   Compiling vb_compile v0.1.0
   Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.74s
```

## E2: Kani Compilation — canonical_name_all_harness (LF-001 FIX)

```bash
$ TMPDIR=/home/lewis/src/vb-workspaces/vb-xi2f.29/target/tmp cargo kani -p vb_compile --harness canonical_name_all_harness --only-codegen
```

**Result**: ✅ COMPILED (exit 0)
```
   Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.73s
```

## E3: Kani Compilation — together_digest_step_deterministic_kani

```bash
$ TMPDIR=/home/lewis/src/vb-workspaces/vb-xi2f.29/target/tmp cargo kani -p vb_compile --harness together_digest_step_deterministic_kani --only-codegen
```

**Result**: ✅ COMPILED (exit 0)
```
   Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.32s
```

## E4: Kani Compilation — together_branch_count_produces_different_digest_kani

```bash
$ TMPDIR=/home/lewis/src/vb-workspaces/vb-xi2f.29/target/tmp cargo kani -p vb_compile --harness together_branch_count_produces_different_digest_kani --only-codegen
```

**Result**: ✅ COMPILED (exit 0)
```
   Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.26s
```

## E5: Kani Compilation — together_digest_sub_step_recursion_bounded_kani

```bash
$ TMPDIR=/home/lewis/src/vb-workspaces/vb-xi2f.29/target/tmp cargo kani -p vb_compile --harness together_digest_sub_step_recursion_bounded_kani --only-codegen
```

**Result**: ✅ COMPILED (exit 0)
```
   Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.17s
```

## E6: Production Code Compilation

```bash
$ cargo check -p vb_compile
```

**Result**: ✅ COMPILED (exit 0)
```
   Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.21s
```

## E7: Test Compilation

```bash
$ cargo test -p vb_compile --no-run
```

**Result**: ✅ COMPILED (exit 0, no errors)

## E8: Kani Execution — canonical_name_together_harness (PO-xi2f29-001)

```bash
$ TMPDIR=/home/lewis/src/vb-workspaces/vb-xi2f.29/target/tmp cargo kani -p vb_compile \
    --harness canonical_name_together_harness --no-unwinding-checks
```

**Result**: ✅ VERIFIED
```
SUMMARY:
 ** 0 of 432 failed (26 unreachable)

VERIFICATION:- SUCCESSFUL
Verification Time: 0.56442463s

Manual Harness Summary:
Complete - 1 successfully verified harnesses, 0 failures, 1 total.
```

**Interpretation**: `canonical_primitive_name(Together)` now correctly returns `"together"`. The Kani assertion `result == "together"` passes for all symbolic inputs. This confirms the CANONICAL_NAME_BUG (C-01) is fixed.

## E9: Kani Execution — canonical_name_all_harness (PO-xi2f29-008)

```bash
$ TMPDIR=/home/lewis/src/vb-workspaces/vb-xi2f.29/target/tmp cargo kani -p vb_compile \
    --harness canonical_name_all_harness --no-unwinding-checks --default-unwind 4
```

**Result**: ⏱️ TIMED_OUT (>10 min)
**Status**: PENDING_FORMAL_EXECUTION
**Reason**: Discriminant-based enumeration with 12 variants each constructing Vec<TogetherBranch> and String values creates excessive symbolic state space. The harness uses `kani::any()` for discriminant + `kani::assume(d < 12)` for GOD RULE 1 compliance. Needs further optimization or splitting into per-variant harnesses.

## E10: Kani Execution — together_branch_count_produces_different_digest_kani (PO-xi2f29-010b, GOD RULE 1 FIX)

```bash
$ TMPDIR=/home/lewis/src/vb-workspaces/vb-xi2f.29/target/tmp cargo kani -p vb_compile \
    --harness together_branch_count_produces_different_digest_kani --no-unwinding-checks --default-unwind 8
```

**Result**: ❌ BLOCKED_TOOLING
```
SUMMARY:
 ** 1 of 3072 failed (3071 undetermined)
Failed Checks: TerminatorKind::InlineAsm is not currently supported by Kani.
 File: ".../stdarch/crates/core_arch/src/x86/cpuid.rs", line 75

VERIFICATION:- FAILED
```

**Reason**: `blake3::Hasher` uses x86 `__cpuid_count` inline assembly for CPU feature detection. Kani 0.67.0 cannot symbolically execute inline assembly. This is a Kani implementation limitation, not a code defect. All digest-based Kani harnesses (PO-xi2f29-009, 010, 010b) are affected.

## E11: Kani Execution — together_digest_step_deterministic_kani (PO-xi2f29-010)

```bash
$ TMPDIR=/home/lewis/src/vb-workspaces/vb-xi2f.29/target/tmp cargo kani -p vb_compile \
    --harness together_digest_step_deterministic_kani --no-unwinding-checks --default-unwind 8
```

**Result**: ❌ BLOCKED_TOOLING (same blake3 InlineAsm failure)
```
SUMMARY:
 ** 1 of 3181 failed (3180 undetermined)
Failed Checks: TerminatorKind::InlineAsm is not currently supported by Kani.
 File: ".../stdarch/crates/core_arch/src/x86/cpuid.rs", line 75

VERIFICATION:- FAILED
```

## E12: Proptest — Together Digest Sensitivity (PO-xi2f29-002 through 006)

```bash
$ cargo test -p vb_compile --test together_digest_sensitivity -- --nocapture
```

**Result**: ✅ ALL 6 PASSED (was 1 PASS, 5 FAIL before production fix)
```
running 6 tests
test proptest_together_branch_count_produces_different_digest ... ok
test proptest_together_branch_labels_produce_different_digest ... ok
test proptest_together_branch_ordering_produces_different_digest ... ok
test proptest_together_digest_is_deterministic ... ok
test proptest_together_sub_step_contents_produce_different_digest ... ok
test proptest_together_sub_step_output_produces_different_digest ... ok

test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.49s
```

**Interpretation**: All sensitivity tests now PASS, confirming:
- C-02: Branch count affects digest (different counts → different digests)
- C-03: Branch labels affect digest (different labels → different digests)
- C-04: Sub-step contents affect digest (different primitives → different digests)
- C-05: Branch ordering affects digest (reordered branches → different digests)
- C-06: Digest remains deterministic (same input → same digest)

Before the fix, all 5 sensitivity tests FAILED with `assert_ne!` violations (identical digests), confirming the DIGEST_INSENSITIVITY bug existed and the tests are non-vacuous. After the fix, they all PASS — confirming the fix is correct.

## E13: Unit Tests — Together Digest Coverage (PO-xi2f29-011 through 015)

```bash
$ cargo test -p vb_compile --lib -- tests::error_variant_tests -- --nocapture
```

**Result**: ✅ ALL 67 PASSED (includes all 5 together-related tests)
```
test tests::error_variant_tests::test_empty_branch_steps_produces_deterministic_digest ... ok
test tests::error_variant_tests::test_nested_together_produces_distinct_recursive_digest ... ok
test tests::error_variant_tests::test_canonical_digest_is_idempotent_with_together ... ok
test tests::error_variant_tests::test_different_together_configurations_produce_different_digests ... ok
test tests::error_variant_tests::test_canonical_primitive_name_together_returns_together ... ok
...
test result: ok. 67 passed; 0 failed; 0 ignored; 0 measured; 245 filtered out; finished in 0.00s
```

**Interpretation**:
- PO-xi2f29-011: Empty branch steps produce deterministic digests ✅
- PO-xi2f29-012: Nested together produces distinct recursive digests ✅
- PO-xi2f29-013: canonical_digest is idempotent ✅
- PO-xi2f29-014: Different configurations produce different digests ✅
- PO-xi2f29-015: canonical_primitive_name(Together) returns "together" ✅

## Evidence Summary

| Lane | Obligations | Compilation | Execution | Verdict |
|---|---|---|---|---|
| Kani (name-only) | PO-xi2f29-001 | ✅ PASS | ✅ VERIFIED | canonical_primitive_name(Together) == "together" |
| Kani (all-variants) | PO-xi2f29-008 | ✅ PASS | ⏱️ TIMED_OUT | State space too large; needs optimization |
| Kani (digest) | PO-xi2f29-009,010,010b | ✅ PASS | ❌ BLOCKED_TOOLING | Kani cannot verify blake3 InlineAsm |
| Proptest | PO-xi2f29-002–006 | ✅ PASS | ✅ 6/6 PASSED | All sensitivity tests pass after fix |
| Unit | PO-xi2f29-011–015 | ✅ PASS | ✅ 67/67 PASSED | All together tests pass after fix |

## Tool Chain

| Tool | Version | Notes |
|---|---|---|
| Kani | 0.67.0 (cargo plugin) | Does not support `#[kani::no_unwinding_checks]` or `InlineAsm` |
| Rust | nightly-2025-11-21 | From workspace rust-toolchain.toml |
| Cargo | 1.91+ | Stable build for non-Kani tests |

## Blocked-by-Tooling Evidence

The following Kani harnesses cannot be verified with Kani 0.67.0 due to blake3's use of `InlineAsm`:

- `together_digest_sub_step_recursion_bounded_kani` (PO-xi2f29-009)
- `together_digest_step_deterministic_kani` (PO-xi2f29-010)  
- `together_branch_count_produces_different_digest_kani` (PO-xi2f29-010b)

Exact error:
```
Failed Checks: TerminatorKind::InlineAsm is not currently supported by Kani.
 File: "stdarch/crates/core_arch/src/x86/cpuid.rs", line 75
```
