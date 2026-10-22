# Test Review: vb-m6w04

**Verdict: APPROVED**

**Frozen SHA:** 531f72e6fa6dcc20562938097e110ac4c57faaee
**Reviewer:** LOCAL RTX3090 Qwen3.8-27B via OMP (independent, did not author source)
**Date:** 2026-10-21

## Test Changes Analyzed

### Removed Cases (2)
Both `compile_source_returns_exact_error_variants_for_contract_taxonomy` and `public_compile_apis_return_exact_error_variants_for_contract_taxonomy` had their `top_level_inputs` case removed. This is correct per the master specification which lists `inputs` as a valid optional top-level field.

### Modified Assertions (1)
The `assert_exact_together` helper now correctly expects the second SetConst node to reference constant index 1 (not 0). This matches the test source which has two distinct values ("1" and "2").

### Removed Enum Variant (1)
`ExpectedCompileError::UnsupportedTopLevelDeclaration` was removed from the test enum since no test case requires it anymore. This is clean — the actual `CompileError::UnsupportedTopLevelDeclaration` variant still exists in production code for other fields (vars, secrets, examples).

## Test Quality Assessment

**No test weakening occurred.** Both changes fix stale oracles that contradicted the documented contract:

1. **Constant index oracle:** The compiler correctly resolves distinct literal values to distinct constant table entries. The test was wrong to expect both branches to share index 0.

2. **inputs taxonomy oracle:** The master specification explicitly permits top-level `inputs` declarations. The test was wrong to expect it to be rejected.

## Coverage Verification

- **together primitive:** Still fully tested — the shape test (TogetherStart, TogetherBranch, TogetherJoin nodes) remains unchanged; only the constant index expectation was corrected.
- **Error taxonomy:** Still tested for empty steps, unsupported top-level result, step control fields, duplicate IDs, output name issues, field shape errors, and slot range errors.
- **Digest collision framing:** All 9 tests pass, confirming the formatting changes to part_05_digest.rs did not affect collision detection or canonicalization behavior.

## Evidence

- 47/47 primitive lowering tests pass
- 496/496 lib tests pass
- 9/9 digest collision framing tests pass
- Both touched files pass rustfmt check

## Conclusion

The test changes are accurate corrections that align the test suite with the documented contract. No coverage was lost; no tests were weakened. Approved.