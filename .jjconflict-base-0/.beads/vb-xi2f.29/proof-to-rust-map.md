# Proof-to-Rust Map: vb-xi2f.29 — Digest Covers Together Semantics

**Bead**: vb-xi2f.29  
**Bridge invocation**: p2i-vb-xi2f29-2026-05-25-001  
**Source proof review**: ppr-vb-xi2f29-2026-05-25-002 (STATUS: APPROVED)  
**State**: 7 (proof-to-implementation)  
**Production code status**: FIX APPLIED (name at part_05.rs:105, Together arm at part_05.rs:158-167, digest_sub_step at part_05.rs:174-177)

## Overview

This bridge maps every approved proof claim from the proof-review.md (ppr-vb-xi2f29-2026-05-25-002) to concrete Rust source symbols, independent behavior tests, refinement harnesses, and exact evidence commands. The implementation is narrow: a one‑character fix at line 105 (`"parallel"`→`"together"`), a 10‑line Together arm addition (lines 158‑167), and a 4‑line recursive `digest_sub_step` function (lines 174‑177).

## Source Target Map

### Primary Source File

| Symbol | Location | Role |
|--------|----------|------|
| `canonical_primitive_name` | `crates/vb_compile/src/mod_compile_lowering/part_05.rs:98-114` | Maps `Together`→`"together"` (line 105) |
| `canonical_digest` | `crates/vb_compile/src/mod_compile_lowering/part_05.rs:116-138` | Top-level workflow digest computation |
| `digest_step_primitive` | `crates/vb_compile/src/mod_compile_lowering/part_05.rs:140-172` | Per-primitive digest dispatch; Together arm at 158‑167 |
| `digest_sub_step` | `crates/vb_compile/src/mod_compile_lowering/part_05.rs:174-177` | Recursive sub-step hashing (new function) |

### Kani Harness Files

| File | Harness(s) |
|------|------------|
| `crates/vb_compile/src/kani_canonical_name.rs` | `canonical_name_together_harness` (line 41), `canonical_name_aggregate_harness` (line 84), `canonical_name_all_harness` (line 137) |
| `crates/vb_compile/src/together_digest_kani.rs` | `together_digest_sub_step_recursion_bounded_kani` (line 54), `together_digest_step_deterministic_kani` (line ~180), `together_branch_count_produces_different_digest_kani` (line ~260) |

### Behavior Test Files

| File | Tests | Obligations covered |
|------|-------|---------------------|
| `crates/vb_compile/tests/together_digest_sensitivity.rs` | 6 proptest property functions (all mapped: 2→PO-002, 3→PO-003, 4a+4b→PO-004, 5→PO-005, 6→PO-006) | PO-002, 003, 004, 005, 006 |
| `crates/vb_compile/tests/v1_primitive_lowering.rs` | 15 existing proptest tests | PO-007 |
| `crates/vb_compile/src/tests/error_variant_tests.rs` | 5 together-specific unit tests | PO-011, 012, 013, 014, 015 |

## Obligation Bridge Matrix

| Proof Obligation | Proof Claim | Rust Target | Source Refs | Behavior Test Refs | Refinement Harness Refs | Status |
|---|---|---|---|---|---|---|
| PO-xi2f29-001 | canonical_primitive_name(Together) == "together" | `canonical_primitive_name` | `crates/vb_compile/src/mod_compile_lowering/part_05.rs:105` | `crates/vb_compile/src/tests/error_variant_tests.rs::test_canonical_primitive_name_together_returns_together` | `crates/vb_compile/src/kani_canonical_name.rs::canonical_name_together_harness` (line 41) | **materialized** |
| PO-xi2f29-002 | Branch count affects digest | `digest_step_primitive` Together arm | `crates/vb_compile/src/mod_compile_lowering/part_05.rs:158-167` | `crates/vb_compile/tests/together_digest_sensitivity.rs::proptest_together_branch_count_produces_different_digest` | `crates/vb_compile/src/together_digest_kani.rs::together_branch_count_produces_different_digest_kani` | **materialized** |
| PO-xi2f29-003 | Branch labels affect digest | `digest_step_primitive` Together arm | `crates/vb_compile/src/mod_compile_lowering/part_05.rs:161-162` | `crates/vb_compile/tests/together_digest_sensitivity.rs::proptest_together_branch_labels_produce_different_digest` | — | **materialized** |
| PO-xi2f29-004 | Sub-step contents affect digest | `digest_sub_step` fn | `crates/vb_compile/src/mod_compile_lowering/part_05.rs:174-177` | `crates/vb_compile/tests/together_digest_sensitivity.rs::proptest_together_sub_step_contents_produce_different_digest` + `::proptest_together_sub_step_output_produces_different_digest` | `crates/vb_compile/src/together_digest_kani.rs::together_digest_step_deterministic_kani` | **materialized** |
| PO-xi2f29-005 | Branch ordering affects digest | `digest_step_primitive` Together arm loop | `crates/vb_compile/src/mod_compile_lowering/part_05.rs:161` | `crates/vb_compile/tests/together_digest_sensitivity.rs::proptest_together_branch_ordering_produces_different_digest` | — | **materialized** |
| PO-xi2f29-006 | Digest determinism preserved | `canonical_digest` | `crates/vb_compile/src/mod_compile_lowering/part_05.rs:116-138` | `crates/vb_compile/tests/together_digest_sensitivity.rs::proptest_together_digest_is_deterministic` + `crates/vb_compile/tests/v1_primitive_lowering.rs::proptest_equal_primitive_sources_compile_to_equal_digest_and_ir` | — | **materialized** |
| PO-xi2f29-007 | Non-Together digests don't regress | `canonical_digest` + `digest_step_primitive` (all arms) | `crates/vb_compile/src/mod_compile_lowering/part_05.rs:98-172` | `crates/vb_compile/tests/v1_primitive_lowering.rs` (all 15 tests) | — | **materialized** |
| PO-xi2f29-008 | Exhaustive variant match | `canonical_primitive_name` | `crates/vb_compile/src/mod_compile_lowering/part_05.rs:98-114` | — (defense-in-depth only) | `crates/vb_compile/src/kani_canonical_name.rs::canonical_name_all_harness` (line 137) | **blocked** (TIMED_OUT) |
| PO-xi2f29-008b | Aggregate canonical name (kni) | `canonical_primitive_name` Aggregate arm | `crates/vb_compile/src/mod_compile_lowering/part_05.rs:107` | — (out of scope per C-01 scope limit; Aggregate name fix deferred) | `crates/vb_compile/src/kani_canonical_name.rs::canonical_name_aggregate_harness` (line 84) | **deferred** — Aggregate → "reduce" out of scope (TB-xi2f29-013). Harness exists but expects "reduce"; production returns "aggregate". Will FAIL if executed. |
| PO-xi2f29-009 | Recursion bounded | `digest_sub_step` fn | `crates/vb_compile/src/mod_compile_lowering/part_05.rs:174-177` | `crates/vb_compile/src/tests/error_variant_tests.rs::test_nested_together_produces_distinct_recursive_digest` | `crates/vb_compile/src/together_digest_kani.rs::together_digest_sub_step_recursion_bounded_kani` (line 54) | **blocked** (BLOCKED_TOOLING: blake3 InlineAsm) |
| PO-xi2f29-010 | Together arm deterministic | `digest_step_primitive` Together arm | `crates/vb_compile/src/mod_compile_lowering/part_05.rs:158-167` | `crates/vb_compile/tests/together_digest_sensitivity.rs::proptest_together_digest_is_deterministic` | `crates/vb_compile/src/together_digest_kani.rs::together_digest_step_deterministic_kani` | **blocked** (BLOCKED_TOOLING: blake3 InlineAsm) |
| PO-xi2f29-010b | Branch count produces different digest (Kani) | `digest_step_primitive` Together arm | `crates/vb_compile/src/mod_compile_lowering/part_05.rs:158-167` | `crates/vb_compile/tests/together_digest_sensitivity.rs::proptest_together_branch_count_produces_different_digest` | `crates/vb_compile/src/together_digest_kani.rs::together_branch_count_produces_different_digest_kani` | **blocked** (BLOCKED_TOOLING: blake3 InlineAsm) |
| PO-xi2f29-011 | Empty branches deterministic | `digest_sub_step` fn | `crates/vb_compile/src/mod_compile_lowering/part_05.rs:174-177` | `crates/vb_compile/src/tests/error_variant_tests.rs::test_empty_branch_steps_produces_deterministic_digest` | — | **materialized** |
| PO-xi2f29-012 | Nested together recursion | `digest_sub_step` fn | `crates/vb_compile/src/mod_compile_lowering/part_05.rs:174-177` | `crates/vb_compile/src/tests/error_variant_tests.rs::test_nested_together_produces_distinct_recursive_digest` | — | **materialized** |
| PO-xi2f29-013 | Digest idempotency | `canonical_digest` | `crates/vb_compile/src/mod_compile_lowering/part_05.rs:116-138` | `crates/vb_compile/src/tests/error_variant_tests.rs::test_canonical_digest_is_idempotent_with_together` | — | **materialized** |
| PO-xi2f29-014 | Together structural coverage | `digest_step_primitive` Together arm + `digest_sub_step` | `crates/vb_compile/src/mod_compile_lowering/part_05.rs:158-167,174-177` | `crates/vb_compile/src/tests/error_variant_tests.rs::test_different_together_configurations_produce_different_digests` | — | **materialized** |
| PO-xi2f29-015 | canonical_primitive_name Together unit | `canonical_primitive_name` | `crates/vb_compile/src/mod_compile_lowering/part_05.rs:105` | `crates/vb_compile/src/tests/error_variant_tests.rs::test_canonical_primitive_name_together_returns_together` (indirect: verifies digest non-zero + determinism) | `crates/vb_compile/src/kani_canonical_name.rs::canonical_name_together_harness` (line 41) | **merged** (≡ PO-001) — same Kani harness provides strong evidence for both; unit test verifies digest behavior, not direct name assertion |

## Evidence Commands

| Obligation | Command | Workdir | Verifier |
|---|---|---|---|
| PO-xi2f29-001 | `TMPDIR=target/tmp cargo kani -p vb_compile --harness canonical_name_together_harness --no-unwinding-checks` | workspace root | kani |
| PO-xi2f29-008 | `TMPDIR=target/tmp cargo kani -p vb_compile --harness canonical_name_all_harness --no-unwinding-checks --default-unwind 4` | workspace root | kani |
| PO-xi2f29-008b | `TMPDIR=target/tmp cargo kani -p vb_compile --harness canonical_name_aggregate_harness --no-unwinding-checks` | workspace root | kani |
| PO-xi2f29-009 | `TMPDIR=target/tmp cargo kani -p vb_compile --harness together_digest_sub_step_recursion_bounded_kani --no-unwinding-checks` | workspace root | kani |
| PO-xi2f29-010 | `TMPDIR=target/tmp cargo kani -p vb_compile --harness together_digest_step_deterministic_kani --no-unwinding-checks --default-unwind 8` | workspace root | kani |
| PO-xi2f29-010b | `TMPDIR=target/tmp cargo kani -p vb_compile --harness together_branch_count_produces_different_digest_kani --no-unwinding-checks --default-unwind 8` | workspace root | kani |
| PO-xi2f29-002–006 | `cargo test -p vb_compile --test together_digest_sensitivity -- --nocapture` | workspace root | proptest |
| PO-xi2f29-007 | `cargo test -p vb_compile --test v1_primitive_lowering -- --nocapture` | workspace root | proptest |
| PO-xi2f29-011–015 | `cargo test -p vb_compile --lib tests::error_variant_tests -- --nocapture` | workspace root | unit |

## Blocked Obligations

### BLOCKED_TOOLING: blake3 InlineAsm (PO-009, 010, 010b)

Kani 0.67.0 cannot symbolically execute `blake3::Hasher` code paths because blake3 uses x86 `__cpuid_count` inline assembly. Error: `TerminatorKind::InlineAsm is not currently supported by Kani` (stdarch `cpuid.rs:75`).

**Compensating evidence** (documented in TB-xi2f29-022):
- Same structural sensitivity properties verified by proptest (6/6 PASS) and unit tests (5/5 PASS)
- Proptest exercises the identical `digest_step_primitive → blake3::Hasher::update → blake3::Hasher::finalize` path
- Kani canonical name verification (PO-001) proves the name dependency

### TIMED_OUT: canonical_name_all_harness (PO-008)

12‑variant symbolic enumeration exceeds Kani solver capacity. Individual per‑variant harness (PO‑001) verifies Together specifically. Split into per‑variant harnesses deferred to follow‑up bead.

## Contract Clause Coverage Summary

| Clause | Requirement | Obligations | Bridge Status |
|---|---|---|---|
| C-01 | Name fix: Together → "together" | PO-001, PO-008, PO-008b, PO-015 | materialized (PO-001 Kani VERIFIED; PO-015 merged into PO-001; PO-008 blocked; PO-008b deferred — Aggregate out of scope) |
| C-02 | Branch count in digest | PO-002, PO-010b, PO-014 | materialized (proptest/unit PASS, Kani blocked) |
| C-03 | Branch labels in digest | PO-003, PO-014 | materialized (proptest/unit PASS) |
| C-04 | Sub-step contents in digest | PO-004, PO-009, PO-012, PO-014 | materialized (proptest/unit PASS, Kani blocked) |
| C-05 | Branch ordering in digest | PO-005 | materialized (proptest PASS) |
| C-06 | Determinism preservation | PO-006, PO-011, PO-013 | materialized (proptest/unit PASS) |
| C-07 | Non-Together regression | PO-007 | materialized (15/15 PASS) |
| C-08 | Kani proof update | PO-001 | materialized (VERIFIED) |

## Non-Lethal Findings Impact on Bridge

| Finding | Impact | Bridge Status |
|---|---|---|
| NLF-004 (blake3 InlineAsm) | PO-009/010/010b cannot be Kani‑verified | **blocked** — compensating evidence accepted per TB-xi2f29-022 |
| NLF-005 (bridge contradiction) | Resolved in REPAIR-2: proof-to-implementation-input.md lines 39/43 harmonized; line 39 now reflects fixed state | **resolved** (VERIFIED: source returns "together" at part_05.rs:105) |
| NLF-006 (all-variants timeout) | PO-008 defense-in-depth deferred | **blocked** — Together already verified by PO-001 |
| NLF-007 (PO-015 weak test) | PO-015 merged into PO-001 (REPAIR-2). Kani canonical_name_together_harness provides definitive C-01 evidence. Unit test verifies digest behavior, not direct name assertion. | **merged** — PO-001 Kani VERIFIED covers both obligations definitively |
| NLF-008 (ledger incomplete) | Does not affect bridge mapping | **advisory** |

## Open Closure Obligations for State 12

1. **PO-xi2f29-008**: Either optimize `canonical_name_all_harness` (per‑variant split) or formally accept the timeout with Together‑verified compensating evidence.
2. **PO-xi2f29-008b**: Either execute `canonical_name_aggregate_harness` after fixing part_05.rs:107 (`"aggregate"`→`"reduce"`) in a follow-up bead, or remove the harness if Aggregate name fix is permanently out of scope. Current state: harness will FAIL due to production code mismatch.
3. **PO-xi2f29-009/010/010b**: Either upgrade to Kani version supporting InlineAsm or formally accept the BLOCKED_TOOLING waiver with proptest+unit compensating evidence.
4. **NLF-007 (closed)**: PO-015 merged into PO-001 per REPAIR-2. The unit test `test_canonical_primitive_name_together_returns_together` verifies digest non-zero and determinism (indirect coverage); PO-001 Kani evidence proves the direct name assertion definitively.
