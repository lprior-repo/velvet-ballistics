# Proof Strategy: Digest Covers Together Semantics

**Bead**: vb-xi2f.29
**Phase**: P1 - Digest together coverage
**Planner invocation**: p4-plan-vb-xi2f.29-001
**Date**: 2026-05-24

## Executive Summary

This bead fixes two defects in `canonical_digest()`:

1. **CANONICAL_NAME_BUG** (already fixed in source): `canonical_primitive_name(Together)` returned `"parallel"` instead of `"together"`. Line 105 of `part_05.rs` now reads `=> "together"`. The existing Kani harness `canonical_name_together_harness` in `kani_canonical_name.rs` expects `"together"` and serves as the regression gate.

2. **DIGEST_INSENSITIVITY** (requires implementation): `digest_step_primitive()` lacks a `Together` arm. Together steps fall through to the `other` wildcard arm which only hashes `canonical_primitive_name(other)`. This means branch count, branch labels, sub-step contents, and branch ordering are NOT reflected in the digest. Two workflows with different together configurations produce identical digests.

### Strategy Overview

- **Primary risk**: Structural blindness in digest — together semantics invisible to hash
- **Surface area**: ~30 lines of new code for `digest_sub_step` + `Together` arm in `digest_step_primitive`
- **Bound**: Recursion bounded by `MAX_LANGUAGE_NESTING_DEPTH = 8`
- **Verification approach**: Kani (name fix gate + bounded recursion safety) + Proptest (structural sensitivity + determinism) + Unit (edge cases)
- **Non-goals in this bead**: Other nested-step primitives (for_each, collect, aggregate, repeat), Aggregate canonical name fix, dead-code cleanup

## Risk Classification

| Risk Tag | Severity | Behavior-Affecting | Primary Lane |
|---|---|---|---|
| CANONICAL_NAME_BUG | MEDIUM | Yes (already fixed) | Kani |
| DIGEST_INSENSITIVITY | HIGH | Yes | Proptest, Kani |
| NESTED_STEP_BLINDNESS | HIGH | Yes | Proptest, Kani |
| RECURSION (unbounded) | MEDIUM | Yes | Kani |
| REGRESSION | MEDIUM | No | Proptest, Unit |
| DEAD_CODE (compile/mod.rs) | LOW | No | Not applicable |

## Verifier Lane Selection

| Verifier | Decision | Rationale |
|---|---|---|
| Kani | **REQUIRED** | Bounded verification: canonical_primitive_name correctness, digest_step_primitive Together arm determinism, recursion depth <= MAX_LANGUAGE_NESTING_DEPTH |
| Proptest | **REQUIRED** | Structural sensitivity properties: branch count, labels, sub-step contents, ordering. Works on end-to-end `compile_source` path. |
| Unit | **REQUIRED** | Edge cases: empty branches, nested together, deterministic idempotency |
| TLA+ | not_applicable | Single-step deterministic hash computation; no temporal/state-machine behavior |
| Verus | not_applicable | Simple hash construction with bounded recursion; Kani provides stronger coverage at lower cost |
| Flux | not_applicable | No numeric refinement predicates in scope; properties are structural inclusion, not refinement-type |
| Loom | not_applicable | No concurrency in digest computation; pure single-threaded function |
| Miri | not_applicable | No unsafe, FFI, raw pointers in digest path |
| cargo-fuzz | not_applicable | Digest inputs are already-validated AST structures; not parsing untrusted bytes |

## Obligation Map

| Obligation ID | Contract Clause | Proof Seed | Verifier | Target |
|---|---|---|---|---|
| PO-xi2f29-001 | C-01 | PS-xi2f29-001 | kani | canonical_name_together_harness regression |
| PO-xi2f29-002 | C-02 | PS-xi2f29-002 | proptest | branch count sensitivity |
| PO-xi2f29-003 | C-03 | PS-xi2f29-003 | proptest | branch label sensitivity |
| PO-xi2f29-004 | C-04 | PS-xi2f29-004 | proptest | sub-step content sensitivity |
| PO-xi2f29-005 | C-05 | PS-xi2f29-005 | proptest | branch ordering sensitivity |
| PO-xi2f29-006 | C-06 | PS-xi2f29-006 | proptest | digest determinism |
| PO-xi2f29-007 | C-07 | PS-xi2f29-007 | proptest | non-together regression |
| PO-xi2f29-008 | C-01 | PS-xi2f29-010 | kani | exhaustive name match |
| PO-xi2f29-009 | C-04 | PS-xi2f29-009 | kani | recursion bounded at MAX_LANGUAGE_NESTING_DEPTH |
| PO-xi2f29-010 | C-02,C-04 | PS-xi2f29-002,004 | kani | together arm digest determinism |
| PO-xi2f29-011 | C-06 | PS-xi2f29-011 | unit | empty branches deterministic |
| PO-xi2f29-012 | C-04 | PS-xi2f29-012 | unit | nested together recursion |
| PO-xi2f29-013 | C-06 | PS-xi2f29-006 | unit | digest idempotency |
| PO-xi2f29-014 | C-02,C-03,C-04 | PS-xi2f29-002,003,004 | unit | together structural coverage |
| PO-xi2f29-015 | C-01 | PS-xi2f29-001 | unit | canonical_primitive_name returns "together" |

## Trusted Base

- `blake3::Hasher` is trusted as a correct cryptographic hash
- `vb_yaml::ast::StepPrimitive` and `vb_yaml::ast::TogetherBranch` type definitions are correct
- `MAX_LANGUAGE_NESTING_DEPTH = 8` bounds recursion depth
- The dead code in `compile/mod.rs` is excluded from scope

## Non-Vacuity Measures

- Proptest strategies generate actual together workflows through the full compile pipeline
- Kani harnesses use `kani::any()` for symbolic enumeration (existing harness `canonical_name_all_harness` already follows this)
- Proptest `assert_ne!` on distinct together configurations prevents vacuous pass
