# Formal Verification Report — vb-xi2f.38

**Bead**: vb-xi2f.38
**Title**: P1: digest covers collect semantics
**State**: 12 (formal-verifier execution)
**Date**: 2026-05-25
**Workspace**: /home/lewis/src/vb-xi2f.38-ws

## Executive Summary

Verification executed across TLA+ model-checking, proptest, and Kani/Verus lanes. TLA+ TLC PASSED. Proptest PASSED (290 tests). Kani BLOCKED_TOOLING (internal compiler error with kani-compiler 0.67.0). Verus BLOCKED_TOOLING (workspace command misconfigured; `cargo verus --workspace` not supported).

## Verification Evidence

### TLA+ Model Checking (PO-001, PO-008, PO-008b, PO-012, PO-017)

**Command**: `java -jar /home/lewis/.local/share/mise/http-tarballs/36e4d95a99aa33dde9ff7b288bf3092f3dfbb26e450fc9758ee765cdb250ce38/tla2tools.jar verification/tla/collect_body_model.tla -config verification/tla/collect_body_model.cfg`

**Result**: PASS

```
TLC2 Version 2.19 of 08 August 2024
Running breadth-first search Model-Checking with fp 48 and seed -3893495832562066208
Model checking completed. No error has been found.
20 states generated, 20 distinct states found, 0 states left on queue.
The depth of the complete state graph search is 5.
```

**Invariants Verified**:
- NodeCountInvariant
- OffsetInvariant  
- NodeKindInvariant
- NoOverflowInvariant
- TypeOK
- LoweringDeterminism

### Proptest (PO-003, PO-004, PO-005, PO-006, PO-007, PO-009, PO-010, PO-014, PO-018)

**Command**: `cargo test -p vb_compile`

**Result**: PASS

```
test result: ok. 290 passed (6 suites, 2.62s)
```

**Test Coverage**:
- `digest_collect_variable_field` - variable field contributes to digest
- `digest_collect_source_field` - source field contributes to digest
- `digest_collect_pages_field` - pages field contributes to digest
- `digest_collect_items_field` - items field contributes to digest
- `digest_collect_body_recursive` - body recursive hashing verified
- `collect_digest_equality_property` - equality implies digest equality
- `compute_compiled_digest_determinism` - repeated calls produce identical digests
- `artifact_digest_depends_on_source` - artifact digest is function of source
- `postcard_serialization_deterministic` - serialization is deterministic

### Kani Verification (PO-002, PO-013, PO-015, PO-016, PO-020)

**Command**: `cargo kani --workspace --no-default-features`

**Result**: BLOCKED_TOOLING

```
Kani Rust Verifier 0.67.0 (cargo plugin)
thread 'rustc' (818362) panicked at kani-compiler/src/codegen_cprover_gotoc/overrides/hooks.rs:158:51:
called `Option::unwrap()` on a `None` value
error: internal compiler error: Kani unexpectedly panicked at panicked at kani-compiler/src/codegen_cprover_gotoc/overrides/hooks.rs:158:51:
                                called `Option::unwrap()` on a `None` value.
```

**Analysis**: Kani 0.67.0 panics during compilation of `vb_compile` when processing `kani_foreach_parity::foreach_no_backward_edge`. This is a tooling bug, not a verification failure. The workspace must be fixed before Kani can execute.

### Verus Verification (PO-011)

**Command**: `cargo verus --workspace`

**Result**: BLOCKED_TOOLING

```
error: Unrecognized option: 'workspace'
```

**Analysis**: `cargo verus --workspace` is not a valid command. Verus requires a specific input file. The proof-obligation command is misconfigured.

## Obligation Status Summary

| Obligation | Verifier | Status | Evidence |
|------------|----------|--------|----------|
| PO-001 | tla-plus | PASS | TLC model check passed, 20 states checked |
| PO-002 | kani | BLOCKED_TOOLING | Kani 0.67.0 internal compiler error |
| PO-003 | proptest | PASS | 290 tests passed |
| PO-004 | proptest | PASS | 290 tests passed |
| PO-005 | proptest | PASS | 290 tests passed |
| PO-006 | proptest | PASS | 290 tests passed |
| PO-007 | proptest | PASS | 290 tests passed |
| PO-008 | tla-plus | PASS | TLC model check passed |
| PO-008b | tla-plus | PASS | TLC model check passed |
| PO-009 | proptest | PASS | 290 tests passed |
| PO-010 | proptest | PASS | 290 tests passed |
| PO-011 | verus | BLOCKED_TOOLING | cargo verus --workspace invalid |
| PO-012 | tla-plus | PASS | TLC model check passed |
| PO-012b | integration-test | NOT EXECUTED | Test referenced does not exist |
| PO-013 | kani | BLOCKED_TOOLING | Kani 0.67.0 internal compiler error |
| PO-014 | proptest | PASS | 290 tests passed |
| PO-015 | kani | BLOCKED_TOOLING | Kani 0.67.0 internal compiler error |
| PO-016 | kani | BLOCKED_TOOLING | Kani 0.67.0 internal compiler error |
| PO-017 | tla-plus | PASS | TLC model check passed |
| PO-018 | proptest | PASS | 290 tests passed |
| PO-020 | kani | BLOCKED_TOOLING | Kani 0.67.0 internal compiler error |

## Waivers Required

The following tooling blockers require formal waivers:

1. **Kani BLOCKED_TOOLING**: Kani 0.67.0 crashes with internal compiler error on vb_compile
2. **Verus BLOCKED_TOOLING**: `cargo verus --workspace` command is invalid

## Findings

- TLA+ and proptest lanes PASSED with strong evidence
- Kani/Verus tooling blocks formal verification of bounded proofs
- PO-012b integration test does not exist in referenced file
- Proof obligations requiring Kani/Verus cannot be executed until tooling is fixed

## Recommendation

Issue WAIVED classification for Kani/Verus obligations (BLOCKED_TOOLING) with formal waiver entries. Proptest and TLA+ obligations achieved PASS. Recommend fixing Kani/Verus tooling before re-executing State 12 gate.