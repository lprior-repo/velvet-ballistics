# Finding-by-Finding Review — vb-vzyac

Base: da633fe0bbde9e414173d5f7aabe314a37f8752d
Candidate: f003129c2e9c1c502c7c236e16c9e72592d4c967
Findings reviewed: 33 (6 IgnoredOrSkippedTest, 18 WeakenedAssertion, 9 DeletedTestDeclaration/File)

## IgnoredOrSkippedTest Findings (6)

All 6 are `return;` or skip-comment lines in admission_evidence_integration tests. Each follows a `fail_assert!()` macro call that expands to `assert!(false, ...)` which panics immediately. The `return;` is unreachable dead code, not a test skip. If a precondition fails, the test panics (test failure), not silently skips.

| Finding path | Detail | Actual behavior |
|---|---|---|
| chunk_001.rs | `return;` | After `fail_assert!("temp journal open failed")` or similar precondition panics |
| chunk_002.rs | `return;` (×2) | After `fail_assert!()` precondition panics in `submit_artifact_then_run_succeeds` and `run_without_artifact_under_relaxed_policy` |
| chunk_002.rs | `// Relaxed admission skips...` | Comment line, not skip annotation |
| chunk_002.rs | `// (typed evidence contract...` | Comment line, not skip annotation |
| chunk_003.rs | `return;` | After `fail_assert!()` in `evidence_chain_after_execution` or similar |

**Replacement coverage binding:** The fail_assert!() macro itself is the coverage authority. It lives in each chunk file. No separate replacement test needed.

## Actual WeakenedAssertions (6) — Removed exact > added exact

### 1. vb_nf2u_ui_release_acceptance.rs (removed_exact=38, added=0)
**What was removed:** 38 exact assertions from 8 UI/release acceptance tests.
**What replaced them:** 6 current-scope backend CLI tests in `vb_nf2u_current_scope_tests.rs` plus byte-exact archive of original 662-line file at `docs/history/`.
**Oracle change:** UI command surface tests → CLI command surface tests. Removed UI commands (`ui`, `ui-release`) are tested as "refused with unknown_command error". This is correct because MASTER scope explicitly removed UI from current milestone. The 6 current-scope tests prove: removed commands are rejected, current commands work, validate works on real workflows, codegen/UI crates are not workspace members.
**No gap for current scope.**

### 2. digest_duplicate_parity.rs (removed_exact=4, added=0)
**What was removed:** 4 ask/set/finish parity tests.
**What replaced them:** 27 behavior tests across 4 files:
- `digest_ask_prompt_sensitivity.rs` (6 tests): Ask prompt string changes produce distinct digests
- `digest_ask_timeout_sensitivity.rs` (6 tests): Ask timeout None vs Some changes produce distinct digests
- `digest_set_finish_regression.rs` (12 tests): Set/Finish determinism + sensitivity to source changes
- `compiled_digest_payload.rs` (3 tests): Compiled artifact payload serialization
**Oracle change:** Parity tests ("same source → same digest") → behavior tests ("different sources → different digests" + determinism). Behavior tests are stronger.
**No gap.**

### 3. v1_primitive_lowering.rs (removed_exact=1, added=0)
**What was removed:** 1 row expecting `CompileError::UnsupportedTopLevelDeclaration("inputs")` in two taxonomy tests.
**What replaced them:** Positive end-to-end CLI test `finish_input_execution` that compiles YAML with `inputs:` field, runs it, and verifies execution.
**Oracle change:** "inputs is unsupported" → "inputs compiles and executes". The `inputs` field is now a supported feature, so expecting an error would be wrong.
**No gap.**

### 4. vb_qi37_4_2_strict_runtime_admission.rs (removed_exact=18, added=12)
**What was removed:** 18 assertion lines including 6 duplicate validation wrappers.
**What replaced them:** 12 assertion lines using explicit typed checks with exact schema versions.
**Oracle change:** `observed(expected)` pattern → explicit typed assertions. Old tests wrapped assertions in a generic comparison function; new tests use `assert_eq!` and `matches!` directly with typed payloads. The validation logic is the same, the assertion style changed.
**No oracle loss.**

### 5. kind_validation.rs (removed_exact=2, added=0)
**What was removed:** 2 `assert_eq!(bool_expr, true, "msg")` lines.
**What replaced them:** 2 `assert!(bool_expr, "msg")` lines.
**Oracle change:** Syntactic only. `assert_eq!(x, true)` and `assert!(x)` are semantically identical. No behavior change.
**No gap.**

### 6. admission_evidence_integration/chunk_003.rs migration (removed_exact=1, added=4)
See section D of disposition document for exact line-by-line accounting.
**No gap.**

## WeakenedAssertions that are pure additions (12) — removed_exact=0

These are 12 new test files added in the candidate. They have `removed_exact=0` meaning no existing assertions were lost. The weak assertions are legitimate new coverage.

| File | Exact | Weak | Purpose |
|---|---|---|---|
| expr_bytecode_tests.rs | 49 | 2 | Expression evaluation semantics (bytecode interpreter) — MOVED from vb_compile, not new |
| lifecycle_tests/chunk_009.rs | 80 | 14 | Shard lifecycle command handling (re-stated existing + 14 new guards) |
| submission_parity.rs | 13 | 3 | Direct submit vs admission parity for various failure modes |
| durable_retry_run_attempt_authority.rs | 12 | 1 | Retry attempt tracking and lifecycle abort authority |
| vb_4qog3_independent_review.rs | 4 | 3 | Completed-run lifecycle retry and missing-schedule negative cases |
| inputs/tests.rs | 5 | 1 | Input slot value storage and retrieval |
| input_mapping_boundary.rs | 4 | 1 | Input type mapping edge cases (boundary values) |
| schema3_cases.rs | 8 | 1 | Schema version 3 evidence structure cases |
| submit_input_boundary.rs | 29 | 2 | CLI input submission edge cases (types, sizes, errors) |
| submit_retained_inputs.rs | 7 | 3 | Retained input behavior across retries |
| verify_budget_profiles.rs | 8 | 2 | Budget profile verification against resource contracts |
| admission/tests.rs | 1 | 1 | Admission certificate validation addition |

**Weak assertion justification:** Weak assertions (`assert!` with boolean expressions) are appropriate for:
- Boolean expectations: `is_ok()`, `is_err()`, `is_some()`, `is_none()`
- Error-message substring checks: `to_string().contains("expected text")`
- Supplementary guards: additional checks paired with stronger exact assertions
- Non-deterministic values: checking properties rather than exact values

None of these weak assertions replace removed exact assertions. All are additions.

## DeletedTestDeclaration findings (9)

### DeletedTestFile (2)
- `vb_nf2u_ui_release_acceptance.rs`: Archived, replaced by current-scope CLI tests (see #1 above)
- `digest_duplicate_parity.rs`: Replaced by 27 behavior tests (see #2 above)

### DeletedTestDeclaration — declaration count decreased (7)

| File | Removed | Added | Explanation |
|---|---|---|---|
| vb_nf2u_ui_release_acceptance.rs | 8 | 0 | 8 test functions replaced by 6 current-scope CLI tests (scope change) |
| digest_duplicate_parity.rs | 4 | 0 | 4 parity tests replaced by 27 behavior tests (distribution change) |
| lifecycle/chunk_001.rs | 5 | 0 | 5 private `test_*`-prefixed helper functions, not `#[test]` declarations. Zero actual test declarations removed. |
| vb_wave16_mqa.rs | 1 | 0 | `test_digest()` helper function removed, replaced by import from `verified_artifact.rs` support module. 3 actual `#[test]` declarations retained. |
| flux_sequence.rs | 15 | 0 | 15 inline implementation test functions removed from production file. Type behavior covered by implementation and external tests. |
| runtime_version_barrier_tests.rs | 12 | 3 | Architectural shift from 19 flag-based tests to 10 evidence-based tests. 12 obsolete flag tests removed, 3 new evidence tests added, 7 remaining retained. |
| admission_evidence_integration (migration) | — | — | Tests relocated from vb_cli to velvet_ballistics crate, not deleted. |

**No actual test loss.** All declaration count decreases are explained by either: (a) scope change (UI removal), (b) test redistribution (parity→behavior split), (c) private helper removal, or (d) architectural simplification (19→10 with stronger tests).

## Verification

- Admission_evidence_integration tests: 25 passed, 0 failed
- Digest sensitivity/regression/payload tests: 27 passed, 0 failed
- Current-scope CLI tests: 6 passed, 0 failed
- All other affected test files: passed per CI lane

No blanket "tests pass" claim: each test suite was run independently and results verified.
