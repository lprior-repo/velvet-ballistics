# vb-gy80b Migration Path Guard Fix - Final Verification

## Problem
The `use crate::expr_proofs as _;` approach in `proof_module_paths::expr_proofs_module_accessible` failed with `-D warnings` due to unused import warning. Marker-type candidate rejected by supervisor.

## Solution
Moved the path guard to a compile-pass rustdoc doctest in the crate documentation at `crates/vb_compile/src/lib.rs`. The doctest imports `vb_compile::expr_proofs`, which is a compile-time check. If the path doesn't resolve, the doctest fails.

## Files Changed
1. `crates/vb_compile/src/lib.rs` - Added doctest to crate documentation
2. `crates/vb_compile/src/migration_compat.rs` - Removed old `proof_module_paths` unit test module

## Verification Results

### Positive Doctest (correct import)
```
running 1 test
test crates/vb_compile/src/lib.rs - (line 11) ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
EXIT: 0
```

### Negative Doctest (broken import)
```
error[E0432]: unresolved import `vb_compile::expr_proofs_broken`
  --> crates/vb_compile/src/lib.rs:14:5
   |
14 | use vb_compile::expr_proofs_broken;
   |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ no `expr_proofs_broken` in the root
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out
EXIT: 101
```

### Full vb_compile --lib Test Suite
```
running 496 tests
test result: ok. 496 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
EXIT: 0
```
Note: One fewer test than before (497 -> 496) because the guard moved from unit test to doctest.

### Canonical Moon Check
```
Tasks: 5 completed (3 cached)
 Time: 6s 325ms
EXIT: 0
```
Some remote cache warnings but not related to code changes.

## Design Decisions
- Doctest approach: Compile-time path verification without runtime overhead
- No production APIs added: The `use` statement in a doc snippet doesn't introduce new exports
- Removed vacuous unit test: The original `use ... as _` test failed under `-D warnings`
- Honest test count: 496 tests pass (one fewer than before due to guard relocation)

## Remaining Risks
- None identified. The path guard is now more robust (compiles as actual code) and passes all verification steps.
