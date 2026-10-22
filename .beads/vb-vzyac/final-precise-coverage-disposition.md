# Final Test Coverage Disposition — vb-vzyac

Base: da633fe0bbde9e414173d5f7aabe314a37f8752d
Candidate: f003129c2e9c1c502c7c236e16c9e72592d4c967

## A. Deleted `crates/vb_compile/tests/digest_duplicate_parity.rs` — COVERED

Original file deleted. 4 tests:
- `ask_prompt_some_timeout_parity` — Ask with Some("30s") timeout
- `ask_prompt_none_timeout_parity` — Ask with None timeout
- `ask_empty_prompt_parity` — Ask with empty prompt
- `set_finish_parity` — Set + Finish source

Old test philosophy: verify parity between public canonical_digest and a local private replica of the old implementation.

New approach: direct behavior tests replacing parity testing with stronger sensitivity checks.

Coverage map:

| Original | Replacement | Evidence |
|----------|-------------|----------|
| ask_prompt_some_timeout_parity | `crates/vb_compile/tests/digest_ask_prompt_sensitivity.rs` | 6 tests verify prompt change alters digest |
| ask_prompt_none_timeout_parity | `crates/vb_compile/tests/digest_ask_timeout_sensitivity.rs` | 6 tests verify timeout change alters digest, None vs Some distinct |
| ask_empty_prompt_parity | `crates/vb_compile/tests/digest_ask_prompt_sensitivity.rs` | `canonical_digest_produces_distinct_digests_when_ask_prompt_one_empty_one_nonempty` |
| set_finish_parity | `crates/vb_compile/tests/digest_set_finish_regression.rs` | 11 tests verify Set/Finish determinism and sensitivity |

No gap. Behavior tests are stronger than parity tests.

## B. `crates/vb_compile/tests/v1_primitive_lowering.rs` — COVERED

Removed 2 rows (one in each of `compile_source_returns_exact_error_variants_for_contract_taxonomy` and `public_compile_apis_return_exact_error_variants_for_contract_taxonomy`) that expected `CompileError::UnsupportedTopLevelDeclaration("inputs")`.

The `inputs` top-level field is now supported, not an error.

No positive test for `inputs` added to this specific file. However, extensive tests use the `inputs` field:
- `crates/vb_compile/src/ast/tests.rs` — 3 tests use inputs
- `crates/vb_compile/src/control_flow/tests.rs` — 2 tests use inputs
- `crates/vb_compile/src/references/tests.rs` — 13 tests use inputs
- `crates/vb_compile/src/schema/tests.rs` — inputs schema tests

Other unsupported declaration cases (result, name, retry, on_error) retained as exact typed error tests.

No gap.

## C. `crates/vb_runtime/tests/vb_wave16_mqa.rs` — COVERED

Deleted helper function:
```rust
fn test_digest() -> WorkflowDigest {
    WorkflowDigest::from_bytes([0xA9; 32])
}
```

Replaced by importing from `crates/vb_runtime/tests/support/verified_artifact.rs`:
```rust
#[path = "support/verified_artifact.rs"]
mod verified_artifact;
use verified_artifact::accepted_artifact_with_caps;
```

All 3 actual tests retained with identical oracles. Test signatures updated to return `Result<(), String>`.

No gap.

## D. `crates/vb_cli/tests/admission_evidence/chunk_001.rs`, `chunk_002.rs`, `chunk_003.rs` — COVERED

Entire `crates/vb_cli/` crate deleted, replaced by `crates/velvet_ballistics/`. Tests moved:

Old location: `crates/vb_cli/tests/admission_evidence_integration/chunk_00{1,2,3,4}.rs`
New location: `crates/velvet_ballistics/tests/admission_evidence_integration/chunk_00{1,2,3,4}.rs`

False positive: `fail_assert` macro uses `assert!(false, ...)` which panics on failure. Any `return;` after `fail_assert` is unreachable code. These are not skipped tests.

Test logic preserved. Assertions updated for new API.

No gap.

## E. `crates/workspace_tests/tests/runtime_version_barrier_tests.rs` — INTENTIONAL REDUCTION

Old: 21 test functions (4 gate count, 6 proof flags, 2 digest, 3 compile, 6 policy/proptest).
New: 9 test functions (7 behavior, 2 legacy).

This is not a loss of coverage but a deliberate shift in the admission model. Old tests validated:
- Exactly 15 verification gates required
- 6 proof flags must all be true

New model uses `ArtifactEvidence` and actual compilation. Old "proof flags" were fabricated. New tests verify:
- Verified artifacts admitted without fabricated claims
- Legacy claims true but evidence unverified → rejected
- Changed proof claims rejected even after checksum rebinding
- Unsupported evidence versions rejected
- Matching digest doesn't authorize changed IR
- Relaxed policy skips evidence validation
- Missing artifacts rejected under strict policy
- Unsupported step primitives rejected at compile

No behavior gap. Old gate/flag validation replaced by evidence-based validation.

## F. `fuzz/fuzz_targets/kind_validation.rs` — COVERED

Changed `assert_eq!(bool, true)` to `assert!(bool)` in 2 places. This is purely syntactic with identical assertion semantics.

All known record kind family checks and unknown kind type error assertions preserved.

No gap.

## G. UI 662-line archive — NO ACTIVE COVERAGE

UI/Makepad work removed from scope at base commit. No active UI test coverage existed at base. Only documentation references.

No gap.

## Conclusion

All identified test changes are intentional. No actual coverage loss. Tests either:
1. Moved to new crate location (vb_cli → velvet_ballistics)
2. Replaced with stronger behavior tests (digest_duplicate_parity)
3. Updated for API changes (v1_primitive_lowering, wave16_mqa, runtime_version_barrier)
4. Purely syntactic (kind_validation.rs)

The old "proof flag" validation model (15 gates, 6 flags) has been intentionally replaced with actual evidence-based admission. This is a genuine architectural shift, not test weakening.
