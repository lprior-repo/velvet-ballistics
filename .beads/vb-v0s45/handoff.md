# vb-v0s45: Compiler Primitive Integration Repairs — Handoff

## Status: Ready for Independent Review

## Summary

Fixed five failing integration tests in `vb_compile/tests/v1_primitive_lowering.rs` caused by two stale test oracle issues:

1. **Together primitive constant indices** (3 tests): The test expected both SetConst nodes in the together primitive to use constant table index 0, but they have different values ("1" and "2") and must use different indices. Fixed to expect index 0 for the first and index 1 for the second.

2. **Top-level `inputs` taxonomy** (2 tests): The test expected `inputs:` to be an unsupported top-level declaration, but the master language specification (velvet-ballistics-MASTER.md §8) lists it as a valid optional top-level field. Removed the `top_level_inputs` case from both taxonomy tests.

## Changes Made

- `crates/vb_compile/tests/v1_primitive_lowering.rs`:
  - Updated `assert_exact_together` to expect const index 1 for the second SetConst node (was 0)
  - Removed `top_level_inputs` test case from `compile_source_returns_exact_error_variants_for_contract_taxonomy`
  - Removed `top_level_inputs` test case from `public_compile_apis_return_exact_error_variants_for_contract_taxonomy`
  - Removed unused `UnsupportedTopLevelDeclaration` enum variant and match arm
  - Applied rustfmt

- `crates/vb_compile/src/mod_compile_lowering/part_05_digest.rs`:
  - Applied rustfmt (pre-existing formatting issue, not related to the five failures)

## Evidence

- All 47 tests in `v1_primitive_lowering` pass (evidence in `.beads/vb-v0s45/evidence/`)
- All 496 `vb_compile` library tests pass
- Source clippy passes with canonical production flags
- Source and test formatting pass

## What Was NOT Changed

- No `vb_compile` source logic changes. The compiler behavior was correct; the test oracle was stale.
- No `vb_runtime` changes (out of scope, owned by vb-hzdts).

## Pending

- Independent code review
- Independent test review
- Root will merge with LOCAL5090's vb-hzdts work

## JJ State

- Fix commit: `xzwwlrno f2babab2` (vb-v0s45: fix five primitive lowering integration failures)
- Parent: `rlwywxsr/0 11e4146f` (codex/vb-wlnl4-interrupted-merge-snapshot)
- Working copy has an empty commit that should be abandoned before publish
