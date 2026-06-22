# Test Review — Slice 3: vb_compile, vb_cli, vb_validate, vb_proof_kernels

**Scope:** 619 Rust files across 4 crates (82 + 54 + 23 + 22 reported; actual count includes all
submodule test files, integration tests in `tests/`, proptest artifacts, Kani harnesses, and
Flux spec files in `src/`).

**Date:** 2026-06-21
**Reviewer:** test-reviewer agent (slice 3 of 4)

## STATUS: REJECTED

The slice contains pervasive banned patterns. The single most damaging pattern is the
`if let Ok(Command::Variant { .. }) = parsed { /* real assertion */ } else { assert!(parsed.is_ok(), ...) }`
shape used in every CLI args test file and BDD scenario: deleting the
production `Command::Variant` arm or returning a different `Ok(Command::OtherVariant)` would
not be caught by any test in `vb_cli/src/args/tests/` or by the `cli_vb_m214_bdd_scenarios.rs`
suite. The `budget_analyzer.rs` and `red_queen_budget.rs` "field reachability" tests use 16-27
`let _ = budget.field;` statements that pass even if every budget field is zeroed. The
`taint/tests/secret_finish_tests.rs` suite uses `matches!(result, Ok(_))` 10+ times so the
Section 47 contract (Secret in Finish must compile to `Ok(CompiledWorkflow)`) is not enforced.
TDD-red gap-exposing tests in `mod_compile_lowering/together_*_tests.rs` accept either `Ok` or
`Err`, so deleting the entire Together lowering implementation would still make the tests pass.
`proptest_save_canonical_name.rs` tests a local copy of `canonical_primitive_name`, not the
production function. The slice has **7 CRITICAL**, **12 HIGH**, **9 MEDIUM**, **10 LOW**, and
**8 OBSERVATION** findings; cannot be approved.

---

## Findings (CRITICAL first)

| ID | Sev | File:Line | Defect | Mutation thought experiment | Recommended fix |
|----|-----|-----------|--------|------------------------------|------------------|
| C-01 | CRITICAL | `crates/vb_cli/src/args/tests/workflow.rs:13,29,45,73,89,104,131,149,168,187,206,264,280,302,328,344,359,370,385,400,416,430,446` (23 instances) | `if let Ok(Command::Validate{..}) = parsed { real asserts } else { assert!(parsed.is_ok(), "expected Ok, got {parsed:?}") }`. The fallback `is_ok()` accepts any `Ok(Command::OtherVariant)`. | Delete `Command::Validate` arm in `args/mod.rs` and route to `Command::Run` instead. All 23 tests pass because the if-let-else falls into `assert!(parsed.is_ok())` and `Ok(Command::Run)` is also `Ok`. | Replace else branch with `panic!("expected Command::Validate, got {parsed:?}")` or assert on the exact variant. |
| C-02 | CRITICAL | `crates/vb_cli/src/args/tests/status.rs:13,36,76,123,167,210,254,297,343` (9 instances) | Same `if let Ok(Command::SystemStatus{..}) / else assert!(parsed.is_ok())` pattern. | Same as C-01: any `Ok(Command::X)` instead of `Ok(Command::SystemStatus{..})` passes. | Same as C-01. |
| C-03 | CRITICAL | `crates/vb_cli/src/args/tests/run.rs:11 instances` | Same pattern for `Command::Run`. | Same. | Same. |
| C-04 | CRITICAL | `crates/vb_cli/src/args/tests/cancel.rs:8 instances` | Same pattern for `Command::Cancel`. | Same. | Same. |
| C-05 | CRITICAL | `crates/vb_cli/src/args/tests/action.rs:7 instances` | Same pattern for `Command::Action`. | Same. | Same. |
| C-06 | CRITICAL | `crates/vb_cli/src/args/tests/parse_run.rs`, `parse_workflow.rs`, `parse_other.rs`, `parse_misc.rs`, `parse_misc2.rs` (10+ total) | Same `if let Ok / else assert!(parsed.is_ok())` pattern across all parser-args test modules. | Delete the matching parser arm — test passes. | Replace with concrete `assert!(matches!(parsed, Ok(Command::ExpectedVariant{..})))` plus field-level equality. |
| C-07 | CRITICAL | `crates/vb_compile/src/budget_analyzer.rs:126-137,206-217` (25 instances of `let _ = budget.field;`) plus `crates/vb_compile/tests/red_queen_budget.rs:450-465` (16 instances) | `analyzer_exposes_all_twelve_master_section_64_fields` and `red_queen_budget_has_all_documented_fields` only test that the 12 (or 16) `WholeWorkflowBudget` field names are syntactically reachable. Every `let _ = budget.max_X;` discards the value. | Set every field to 0 in `vb_core::budget::WholeWorkflowBudget` (or delete the field assignments). Both tests pass because the value is never checked. The test claim "12 master §64 fields must all be reachable" is a syntactic existence check, not a behavior check. | Replace with `assert!(budget.max_steps_executable >= 1, ...)` for each field with a concrete value contract (e.g. linear_workflow(N) must produce max_total_steps = N+1). |
| C-08 | CRITICAL | `crates/vb_compile/src/taint/tests/secret_finish_tests.rs:42,69,94,120,144,167,190,229,400,422,486,578,598` (13 instances) | `matches!(result, Ok(_))` in `compile_accepts_secret_finish_result` and friends. The Section 47 contract says "secret in Finish must compile to `Ok(CompiledWorkflow)` with the secret data preserved" — but the test only asserts `Ok(_)`, so the production code could return `Ok(empty_workflow)` (a stripped CompileError version) and pass. Also `proptest_compile_accepts_clean_finish` and `proptest_compile_accepts_literal_finish` accept any `Ok(_)`. | Modify `compile_workflow` to strip the secret data from Finish results and return `Ok(workflow_without_secret)`. All 13 assertions pass. | Assert on the workflow content: `assert!(matches!(result, Ok(wf) if wf.finish_result_contains_secret()))`. |
| C-09 | CRITICAL | `crates/vb_compile/src/mod_compile_lowering/together_lowering_tests.rs:208-231,256-265,291-...` and `together_e2e_tests.rs:166-197,243-265,289-347,608-623` and `together_integration_tests.rs:360-...` (5+ files, 15+ tests) | TDD-red gap-exposing pattern `if let Ok(()) = result { /* detailed asserts */ } // TDD: Accept either Ok or Err (implementation may not exist yet)`. The test passes whether Together lowering exists or not. | Delete the entire `emit_single_body_set` Together branch. All "TDD: acceptable" tests still pass. | Either (a) remove the TDD-red comments and convert the `if let Ok(())` into a hard assertion, or (b) delete the test entirely until the implementation is real. |
| C-10 | CRITICAL | `crates/vb_compile/tests/proptest_save_canonical_name.rs:30-46,80-105` | Test calls a locally-defined `fn canonical_name(primitive: &StepPrimitive) -> &'static str` that **duplicates** the production `canonical_primitive_name` match arms at `mod_compile_lowering/part_05.rs:98-114`. Test verifies the local copy against itself, not the production function. The file's own comment admits "This is a trusted-base reproduction." | Production `canonical_primitive_name` could revert `Save{..}` to `"save"` (the pre-fix bug). The test would still pass because it never calls production. | Expose `canonical_primitive_name` as `pub` (or `pub(crate)` with a test-only re-export) and call the production function. |
| C-11 | CRITICAL | `crates/vb_compile/src/tests/do_choose_digest_unit_tests.rs:179,202,206,223,227,243,247,269,300,304,326,330,353,357,379,383,404,408` (18 instances) | `let _ = digest_step_primitive(&mut hasher, &step);` discards the `Result<()>`. Then `hasher.finalize()` is checked. | If `digest_step_primitive` returns `Err` AND short-circuits without writing to the hasher, the test sees a zero-hash and a `assert_ne!(digest, other_digest)` could spuriously pass if the comparison is also zero. (Latent risk; some tests use `assert_eq!` for determinism which is more clearly broken.) | Assert on the result: `digest_step_primitive(&mut hasher, &step).expect("digest must succeed for valid primitive");` |
| C-12 | CRITICAL | `crates/vb_compile/tests/digest_ask_explicit_arm.rs:144,150,157,167,173,179,185,191,221,227,233` (11 instances) | `let _ = canonical_digest(&source).expect("valid test input");` discards the digest value. Test name is `digest_step_primitive_does_not_panic_for_*` — it only checks non-panic, not digest shape. | If `canonical_digest` returns `Ok(zero_digest)` for every variant, every "does_not_panic" test passes. The contract is "digest must be non-trivial" but the test doesn't check it. | Capture the digest and assert `assert_ne!(digest, ZERO_DIGEST)` plus `assert_eq!(digest1, digest2)` for determinism. |
| H-01 | HIGH | `crates/vb_cli/tests/cli_vb_m214_bdd_scenarios.rs:373,640,654,709,721,739,788,797,...` (~25 instances of `assert!(output.status.code().is_some())` or `assert!(code == Some(0) || code == Some(2))`) | Tests assert "exit code is some value" without checking the specific contract. E.g. `cli_bench_run_valid_workflow_produces_output` accepts any exit code; `exit_code_two_on_verification_failure` accepts `0` or `2`. | Change `verify` to exit 0 on all inputs. Tests pass because the acceptable range includes 0. | Pick a single expected exit code per scenario. |
| H-02 | HIGH | `crates/vb_compile/src/proptest_together_errors.rs:262-264,283-291` | `prop_assert!(matches!(result, Ok(()) | Err(_)), "zero-branch together must return a Result without panic");` — vacuous; accepts everything. | If `emit_single_body_set` always returns `Ok(())` for any body, the test passes. The proptest provides no false-positive protection. | Assert on a specific error variant for the zero-branch case. |
| H-03 | HIGH | `crates/vb_compile/tests/proptest/proptest_choose_depth.rs:62-66` | `prop_assert!(matches!(inner, Ok(_) | Err(_)), "varied_choose_yaml compiles or errors gracefully (never panics), got {:?}", inner);` — vacuous. | Always-Ok or always-Err compiles pass. | Assert on the success branch (`wf.node_count() >= 2`) when Ok, or a specific error variant. |
| H-04 | HIGH | `crates/vb_compile/tests/vb_xi2f_compile_source_proptest.rs:176-180` | `prop_assert!(result.is_ok(), "YamlCompiler::compile on valid YAML must return Ok, got {:?}", result);` — banned `is_ok()`. | YamlCompiler could return `Ok(empty)` instead of `Ok(workflow_with_nodes)`. | Add `assert!(result.unwrap().node_count() >= 2)`. |
| H-05 | HIGH | `crates/vb_compile/tests/vb_xi2f_nested_do_lowering.rs:488` (and similar) | `result.is_err()` as the only assertion in a "rejects bad input" test. Doesn't check which error variant. | Change the error variant to `CompileError::Other`. Test passes because it only checks `is_err()`. | Use `assert!(matches!(result, Err(CompileError::Expected {..})))`. |
| H-06 | HIGH | `crates/vb_cli/src/agent_context/tests/unit.rs:551,982,1694` (and 200+ more lines) | `panic!("gate '{}' must be an object", gate_name)` and `panic!("expected Inspect command, got {parsed:?}")` patterns. Bare `panic!()` in tests is banned by the rubric, even though `unwrap_or_else(\|\| panic!())` is used for fixture construction. (Project's own `AGENTS.md` says no panic, but allows test clippy to be loose.) | n/a — these are fixture/assertion panics, not should_panic tests. LOW if owned as test policy. | Convert to `expect` or `assert!` with a message. (Project decision: keep allowed-by-test-policy, but file as observation.) |
| H-07 | HIGH | `crates/vb_cli/src/args/tests/journal.rs:157,176,194,227,245,263,298,319,358,440,469,487,505,555,579` (15+ `panic!("expected X command, got {parsed:?}")`) | Same as H-06 but in journal tests. | Same. | Same. |
| H-08 | HIGH | `crates/vb_validate/tests/red_phase_validation.rs:163,221,332` | `assert!(validate(&parts).is_ok(), "expected Ok for valid accessor symbols")` — banned. | Gate 12 could return `Ok(WrongVariant)` and the test would pass. | Use `assert!(matches!(result, Ok(ValidationOutcome::Accept)))`. |
| H-09 | HIGH | `crates/vb_cli/src/main_tests.rs:62,425,515,529,533,718,722,728,737,747,758,761,881` (13 instances) | `assert!(journal.is_ok(), "journal should reopen")` — banned. Most are fixture/IO construction checks. | Journal reopen could fail with a different error and the test would pass (matches `is_ok()`). | Use `.expect("journal must reopen: {err:?}")` instead. |
| H-10 | HIGH | `crates/vb_cli/src/args/tests/parse_misc2.rs:2 instances` and similar | Same `if let Ok / else assert!(parsed.is_ok())` pattern in additional CLI args test files. | Same as C-01. | Same. |
| H-11 | HIGH | `crates/vb_cli/tests/vb_qi37_14_1_run_step.rs:1316-1323` | `let has_output = json.get("output_slot").is_some() \|\| (json.get("deltas").is_some() && json.get("deltas").unwrap().get("slot_deltas").is_some());` plus a TODO comment indicating the test is deferred. | Regression where the step returns neither `output_slot` nor `slot_deltas` would fail — but anything else passes, so the contract is effectively "produce some output field." | Pick one of the two contract shapes and assert exactly. Remove the TODO. |
| H-12 | HIGH | `crates/vb_cli/tests/lifecycle_integration.rs:1731` | `assert!(result.is_err(), "cancel from Pending must not succeed (no self-loop)")` — banned `is_err()` without specifying the variant. | Cancel from Pending could return `Err(StorageError)` instead of `Err(InvalidState)`. Test passes. | `assert!(matches!(result, Err(lifecycle::LifecycleError::InvalidState{..})))`. |
| M-01 | MEDIUM | `crates/vb_compile/src/mod_compile_lowering/together_e2e_tests.rs:378` | `let _ = result;` discards compile result entirely. | Test passes regardless of compile outcome. | Either assert or remove. |
| M-02 | MEDIUM | `crates/vb_compile/src/tests/property_validation_tests.rs:13,23,47` | TDD-red tests: `Ok(_) => panic!("GAP EXPOSED: ...")` else `Err(e) => println!("PASS")` and `Err(_) => {}` (no specific variant). Plus `println!()` is a banned output pattern. | The "PASS" branch only checks `is_err()`; the test would pass for any error. | Replace with concrete `assert!(matches!(result, Err(CompileError::EmptyTogether{..})))`. Remove `println!`. |
| M-03 | MEDIUM | `crates/vb_compile/src/tests/integration_reduce_tests.rs:70` | `println!("GAP EXPOSED: reduce.rs does not detect missing accumulator update")` in a #[test] body. Banned: `println!` in tests. | n/a — output noise. | Use `eprintln!` only if needed, or remove. |
| M-04 | MEDIUM | `crates/vb_compile/src/tests/integration_reduce_tests.rs:36-37` | `try_from_parts(parts).ok().unwrap_or_else(\|\| panic!("workflow must compile"))` — converts Err to panic. This is fixture-construction-style panic, not banned, but masks the underlying error. | n/a. | Use `try_from_parts(parts).expect("workflow must compile")`. |
| M-05 | MEDIUM | `crates/vb_compile/tests/proptest/proptest_choose_otherwise.rs:50-55,61-66` | `prop_assert!(result.is_ok(), "choose with otherwise must compile Ok, got {:?}", result);` — banned `is_ok()`. | Change choose parsing to always return `Ok(empty_workflow)`. Test passes. | Assert on the workflow's `node_count()` and the `ChooseSlot::otherwise` field. |
| M-06 | MEDIUM | `crates/vb_compile/tests/proptest/proptest_choose_fallthrough.rs:48-52` | `assert!(result.is_ok(), "choose yaml must compile Ok, got {:?}", result);` — banned. | Same as M-05. | Same. |
| M-07 | MEDIUM | `crates/vb_compile/src/mod_compile_lowering/tests.rs:1509-1513,2912-2916` | `assert!(matches!(output_slot, Some(_)), ...)` and `assert!(matches!(node.next, Some(_)), ...)` — banned `Some(_)`. | `output_slot = Some(invalid_slot)` (e.g. out of range) would pass; `node.next = Some(0)` (incorrect loop target) would pass. | Match exact value or assert on discriminant. |
| M-08 | MEDIUM | `crates/vb_compile/src/mod_compile_lowering/together_integration_tests.rs:355-380` | "TDD: will succeed after implementation" — matches `if let Ok(workflow) = result { ... }` with no else clause. | Same as C-09. | Same as C-09. |
| M-09 | MEDIUM | `crates/vb_compile/src/kani/kani_validation_error_code.rs:14-79` | Hardcoded list of 64 symbolic code names; the test does not consult the actual `CODE_REGISTRY` (uses local `REGISTERED_CODES` constant). | If the registry removes a name, this test still passes because it consults its own copy. (Bounded by the file's stated `Bound: 64 variants`.) | Iterate over `CODE_REGISTRY` to build the constant or assert against the registry in the harness. |
| L-01 | LOW | `crates/vb_cli/tests/cli_integration.rs:1246,1411,1554,1571,1604,1610,1722` | `assert!(text.is_err(), "...")` patterns that are mostly valid for "rejects bad input" — but they don't check the specific error variant. | The error taxonomy could change and the test still passes. | Document as low priority; pick the specific variant. |
| L-02 | LOW | `crates/vb_cli/src/io.rs:7 instances` | Banned assertions on test helper functions (not behavior). | n/a — test infrastructure. | Consider replacing with `Result` types. |
| L-03 | LOW | `crates/vb_cli/src/naming_scan/classify.rs:7 instances` | Same as L-02 (test fixtures, not behavior). | n/a. | Same. |
| L-04 | LOW | `crates/vb_cli/tests/deliver_sink_integration.rs:877-878` | `static PROBE: OnceLock<Mutex<...>>` is hidden shared mutable state, BUT it is a fingerprint-keyed cache that documents the rebuild-invalidation property. Acceptable. | n/a — well-documented test cache. | Add doc comment if not present. |
| L-05 | LOW | `crates/vb_validate/src/diag_render/fallback.rs:14` | `static FALLBACK: OnceLock<SymbolicCode>` in production code, NOT a test. | n/a. | Out of scope. |
| L-06 | LOW | `crates/vb_compile/src/enums/side_effect_tests.rs:165,173,181,189,197,205,213` (7 instances) | `let _ = SideEffect::Variant;` as a variant-existence smoke test. The variants are also enumerated in the `variants` array which is asserted. So this is a redundant pattern that could be removed but doesn't fail to catch the bug. | n/a. | Refactor to single `let _ = VARIANTS;` once. |
| L-07 | LOW | `crates/vb_cli/src/agent_context/tests/kani_harnesses.rs` and similar `kani_harnesses.rs`/`kani_*.rs` | Kani harnesses are verifier-driven, not behavior tests. They appear in this file count but are correctly `#[cfg(kani)]` gated. | n/a. | Verify that the `kani-list.sh` and `kani-check-*.sh` scripts actually invoke them; otherwise they are dead artifacts. |
| L-08 | LOW | `crates/vb_proof_kernels/src/profile_contract/contract_lemmas.rs:12` and `contract_witnesses.rs:6` | Banned patterns in Verus proof files. These are proof annotations, not behavior tests. | n/a. | Out of scope for behavior-test review. |
| L-09 | LOW | `crates/vb_compile/src/flux_choose.rs` (and other `*_flux.rs` files) | Flux spec files use `extern_spec` and `sig` annotations. Not behavior tests; the banned-pattern grep picks up `let _ =` in non-test files. | n/a. | Out of scope. |
| L-10 | LOW | `crates/vb_compile/proptest-regressions/*.txt` and `crates/vb_validate/proptest-regressions/*.txt` | Proptest's auto-generated regression cache. The files are 8 small `.txt` files (3D and 8F). They are NEVER executed — they are inputs to `proptest!`'s `seed_from_replay_path`. They count as test fixtures, not test code. | n/a. | Out of scope. |
| O-01 | OBSERVATION | `crates/vb_compile/src/budget_analyzer.rs:64` | `WholeWorkflowBudget::compute(...).ok().unwrap_or_else(unbounded_default)` — silently discards the inner error and substitutes a zero budget. This is a production-code issue surfaced by the test review. | If the second `compute` returns `Err`, the test would still see `Ok(budget_with_zeros)` and the `let _ =` field-reachability tests pass. | The analyzer should propagate the error or assert non-fallibility; not return `ok().unwrap_or_else(...)`. |
| O-02 | OBSERVATION | `crates/vb_compile/src/tests/do_choose_digest_unit_tests.rs:1-99` | The file has a 99-line `#![allow(...)]` block suppressing all clippy lints, including `clippy::let_underscore_must_use`. This blanket allow hides the silent suppression pattern in the 18 `let _ =` lines. | n/a. | Reduce the allow list to only the lints actually needed. |
| O-03 | OBSERVATION | `crates/vb_compile/src/mod_compile_lowering/tests.rs:1-110` | Same 110-line blanket `#![allow(...)]` block. | n/a. | Same as O-02. |
| O-04 | OBSERVATION | `crates/vb_cli/src/args/tests/workflow.rs:1-...` and other CLI args tests | The repeated `if let Ok(Command::X{..}) = parsed { ... } else { assert!(parsed.is_ok(), "...") }` shape is **mass-copied** across 6+ files. A code review that fixes one must fix all 6+. | n/a. | Refactor into a `match parsed { Ok(Command::X{..}) => ..., Ok(Command::Y{..}) => panic!("got Y, expected X"), Err(e) => panic!("got Err: {e:?}") }` shape. |
| O-05 | OBSERVATION | `crates/vb_compile/src/expression_bytecode_tests.rs:1-100+` | 100+ line `#![allow(...)]` suppression list (2008 total lines in the file). The blanket allow turns off `clippy::unnecessary_unwrap`, `clippy::unwrap_used`, etc. This is a maintenance hazard, not a test defect. | n/a. | Refactor: move to per-`#[test]` `#[allow(...)]` or use a `cfg(test)` helper. |
| O-06 | OBSERVATION | `crates/vb_compile/src/kani/vb_compile_constant.rs:99-101` | Stale code: `let empty_count_val = match empty_count { Ok(v) => v, Err(_) => { kani::assume(false); loop {}} }, "slot_count on empty builder should be Ok");` — line 102 has a stray `, "slot_count on empty builder should be Ok");` after the closing brace, suggesting copy-paste duplication. (Lines 154 and 171 have the same pattern.) | n/a — Kani-only, syntax error if kani doesn't accept it. | Refactor. |
| O-07 | OBSERVATION | `crates/vb_compile/tests/proptest_digest_foreach.rs:309` | A single commented-out `// #[test]` for `proptest_foreach_cross_path_digest_equivalence` (path A vs path B). Documented as deferred. Not a defect. | n/a. | Track via bead. |
| O-08 | OBSERVATION | `crates/vb_compile/src/mod_compile_lowering/together_lowering_tests.rs:122` | Module docstring `//! These tests are TDD-red until State 11 implementation adds:` — entire file is TDD-red documentation. 21 banned patterns in this file. | n/a. | Either remove the file until State 11 lands, or convert all to hard assertions. |

---

## Pattern Census

### `assert!(...is_ok()) / assert!(...is_err()) / matches!(..., Some(_) | Ok(_) | Err(_))` and bare `unwrap()`

| Crate | Total matches | Top files |
|-------|---------------|-----------|
| `vb_cli/src` | 110 | `args/tests/workflow.rs` (24), `main_tests.rs` (13), `args/tests/run.rs` (11), `args/tests/status.rs` (9), `args/tests/cancel.rs` (8), `args/tests/action.rs` (7), `naming_scan/classify.rs` (7), `io.rs` (7), `app_impl_tests.rs` (13) |
| `vb_cli/tests` | 63 | `cli_vb_m214_bdd_scenarios.rs` (~25 in helpers), `cli_trace_integration.rs` (15), `cli_integration.rs` (8), `lifecycle_integration.rs` (5), `ir_artifact_admission.rs` (5) |
| `vb_compile/src` | 6 | `flux_choose.rs` (5 — these are spec files, not tests), `tests/accumulator_overflow_tests.rs` (5) |
| `vb_compile/src/mod_compile_lowering` | 3 | `together_lowering_tests.rs` (21 raw), `tests.rs` (2) |
| `vb_compile/tests` | 15 | `digest_ask_explicit_arm.rs` (11 unwraps in `let _ =` form) |
| `vb_proof_kernels/src` | 18 | `profile_contract/contract_lemmas.rs` (12 — Verus proofs), `profile_contract/contract_witnesses.rs` (6) |
| `vb_validate/src` | 5 | `gate_08_verus_proof.rs` (2 — Verus) |
| TOTAL | ~225 | (concentrated in `vb_cli/`) |

### `let _ = ...` (silent suppression, excluding flux/verus files)

| Crate | Total matches | Top files |
|-------|---------------|-----------|
| `vb_compile/src` | 37 | `budget_analyzer.rs` (27), `tests/do_choose_digest_unit_tests.rs` (18 — counted in banned total above), `expression/...` (5) |
| `vb_compile/src/kani` | 25 | `kani_validation_error_code.rs` (25 — kani::assert wrappers, legitimate) |
| `vb_compile/tests` | 28 | `red_queen_budget.rs` (16), `digest_ask_explicit_arm.rs` (11) |
| `vb_compile/src/mod_compile_lowering` | 24 | `together_e2e_tests.rs` (6), `together_integration_tests.rs` (5), `tests.rs` (3) |
| `vb_cli/src` | 12 | `naming_scan/classify.rs` (3), `io.rs` (3), `agent_context/...` (6) |
| `vb_cli/tests` | 17 | `cli_integration.rs` (12), `lifecycle_integration.rs` (3), `cli_trace_integration.rs` (1) |
| `vb_validate/src` | 3 | `type_taint/type_taint_tests.rs` (3) |
| TOTAL | ~146 | (concentrated in `vb_compile/`) |

### `#[ignore]` / `#[should_panic]` / `sleep(` / `todo!()` / `unimplemented!()`

| Crate | Total | Notes |
|-------|-------|-------|
| `vb_cli/src` | 1 | `doctor.rs` — likely a #[cfg(test)] flag, not a behavior test |
| `vb_cli/tests` | 1 | `cli_integration.rs` — need to check if it's a #[should_panic] or #[ignore] |
| TOTAL | 2 | (low) |

### `lazy_static` / `OnceLock` / `static mut` / `thread_local!`

| Crate | Total | Notes |
|-------|-------|-------|
| `vb_cli/tests` | 3 | `deliver_sink_integration.rs:877-878` — `OnceLock` cache with fingerprint invalidation (acceptable, see L-04) |
| `vb_validate/src` | 2 | `diag_render/fallback.rs:14` — production `OnceLock`, not a test |
| `vb_cli/src` | 2 | `deliver_sink/deliver_test_support.rs` (1), `deliver_sink/deliver_debug_test_support.rs` (1) — test support, not behavior |
| TOTAL | 7 | (all acceptable, well-documented or test-support-only) |

### `panic!` and `println!` in test code

| Crate | Total matches |
|-------|---------------|
| `vb_cli/src` (incl. test submodules) | 44 (panic!) + `println!` mostly in `args/tests/journal.rs` (15+) and `agent_context/tests/unit.rs` (3+) |
| `vb_compile/src` | 12 (mostly in `tests/` submodules) |
| `vb_validate/src` | 2 |
| `vb_cli/tests` | 21 (`println!` for diagnostics) |
| `vb_compile/tests` | 3 (`println!` for GAP EXPOSED) |

**Total panic!/println! in test code: ~80+**

---

## Mutation Gaps (top 5 most dangerous bugs the slice would NOT catch)

1. **`compute_whole_workflow_budget` returns all-zeros budget.** The 27 `let _ = budget.field;` and 16
   `let _ = budget.field;` in `budget_analyzer.rs` and `red_queen_budget.rs` would all pass.
   `analyzer_handles_single_node_workflow` does check `max_steps_executable >= 1` for the simple
   linear case, so a complete zero would be caught — but partial corruption (e.g., `max_total_steps`
   zeroed while `max_steps_executable` is correct) would NOT. **File:Line:** production
   `crates/vb_compile/src/budget_analyzer.rs:35-52` and `vb_core::budget::WholeWorkflowBudget`.

2. **Wrong `Command::*` variant returned by `parse_args`.** Delete `Command::Validate` from
   `vb_cli/src/args/mod.rs` and rewire to `Command::Run`; all 23 `if let Ok(Command::Validate{..})
   / else assert!(parsed.is_ok())` tests in `args/tests/workflow.rs` pass because the fallback
   `is_ok()` accepts any `Ok(Command::X)`. **File:Line:** `vb_cli/src/args/mod.rs` and `args.rs`
   in the same directory.

3. **`emit_single_body_set` Together branch deleted.** The TDD-red `if let Ok(()) = result { /* ... */ }`
   pattern in `mod_compile_lowering/together_lowering_tests.rs:208-231, 256-265, 291-...` and the
   `let _ = result;` in `together_e2e_tests.rs:378` and the `matches!(result, Ok(_) | Err(_))` in
   `proptest_together_errors.rs:262-264` and `proptest_choose_depth.rs:62-66` would all pass if
   `emit_single_body_set` was simplified to `fn emit_single_body_set(...) -> Result<(), Err> {
   Err(CompileError::UnsupportedStepPrimitive) }`. **File:Line:** production
   `crates/vb_compile/src/mod_compile_lowering/part_04/body_dispatch.rs`.

4. **Section 47 contract violation: `compile_workflow` strips secret data from Finish results.**
   The 13 `matches!(result, Ok(_))` assertions in `taint/tests/secret_finish_tests.rs` accept any
   `Ok(workflow)`. Modify `compile_workflow` to remove `$secrets.*` from the Finish result, return
   `Ok(workflow)` with the secret redacted, and all 13 `compile_accepts_secret_*` tests pass. The
   secret-taint data flow would be silently broken. **File:Line:** production
   `crates/vb_compile/src/taint/engine.rs` and `crate::CompileError` mapping.

5. **`canonical_primitive_name(Save{..})` reverts to "save".** The proptest
   `proptest_save_canonical_name.rs` tests a local copy of the function, not the production
   function in `mod_compile_lowering/part_05.rs:98-114`. The pre-fix bug ("save" vs "set") would
   return and the proptest would still pass. **File:Line:** production
   `crates/vb_compile/src/mod_compile_lowering/part_05.rs:98-114`.

---

## Top 5 Fixes (impact-per-effort)

### Fix 1 — Replace the `if let Ok / else assert!(parsed.is_ok())` pattern in CLI args tests
**Impact:** 23+9+11+8+7+10 = **68 test sites** in `vb_cli/src/args/tests/*.rs` strengthened.
**Effort:** 30 min. Mechanical refactor. Test count is small per file.

```rust
// BEFORE (vb_cli/src/args/tests/workflow.rs:7-15):
#[test]
fn parse_validate_requires_workflow() {
    let parsed = parse_args(&args(&["velvet-ballistics", "validate", "workflow.yaml"]));
    if let Ok(Command::Validate { workflow, output }) = parsed {
        assert_eq!(workflow, PathBuf::from("workflow.yaml"));
        assert_eq!(output, OutputFormat::Text);
    } else {
        assert!(parsed.is_ok(), "expected Ok, got {parsed:?}");
    }
}

// AFTER:
#[test]
fn parse_validate_requires_workflow() {
    let parsed = parse_args(&args(&["velvet-ballistics", "validate", "workflow.yaml"]));
    match parsed {
        Ok(Command::Validate { workflow, output }) => {
            assert_eq!(workflow, PathBuf::from("workflow.yaml"));
            assert_eq!(output, OutputFormat::Text);
        }
        other => panic!("expected Command::Validate, got {other:?}"),
    }
}
```

### Fix 2 — Replace `let _ = budget.field;` with concrete value assertions in `budget_analyzer.rs` and `red_queen_budget.rs`
**Impact:** 25+16 = **41 field-reachability smoke tests** become real behavior tests.
**Effort:** 1 hour. Map each budget field to a linear_workflow(N) or fanout_workflow(N) fixture that produces a known value.

```rust
// BEFORE (vb_compile/src/budget_analyzer.rs:198-218):
let _ = budget.max_steps_executable; // #1
let _ = budget.max_action_tickets; // #2
// ... 10 more

// AFTER:
let workflow = linear_workflow(20); // 20 set + 1 finish = 21 steps
let budget = compute_whole_workflow_budget(&workflow).expect("bounded");
assert_eq!(budget.max_total_steps, 21, "20 sets + 1 finish = 21");
assert_eq!(budget.max_steps_executable, 21);
assert!(budget.max_action_tickets >= 0, "field must be reachable");
assert_eq!(budget.max_for_each_iterations, 0, "no for_each in linear");
```

### Fix 3 — Replace `matches!(result, Ok(_))` in `taint/tests/secret_finish_tests.rs` with workflow-content assertions
**Impact:** 13+ tests covering Section 47 (Secret in Finish) become real contract tests.
**Effort:** 2 hours. Requires inspecting `CompiledWorkflow` to find the secret-data accessor.

```rust
// BEFORE (taint/tests/secret_finish_tests.rs:41-47):
let result = compile_workflow(source);
assert!(matches!(result, Ok(_)), "Section 47: secret in Finish must compile, got {:?}", result);

// AFTER:
let result = compile_workflow(source);
let workflow = result.expect("Section 47: secret in Finish must compile");
assert!(workflow.finish_contains_secret_data(),
        "Finish result must preserve secret data per Section 47");
```

### Fix 4 — Convert TDD-red `if let Ok(()) = result { ... }` in `mod_compile_lowering/together_*_tests.rs` to hard assertions
**Impact:** 15+ Together-lowering tests become real contract tests.
**Effort:** 4 hours. Requires deciding whether to remove the tests or convert to hard asserts.

```rust
// BEFORE (mod_compile_lowering/together_lowering_tests.rs:200-231):
if let Ok(()) = result {
    // ... detailed assertions
}
// TDD: Accept either Ok or Err (implementation may not exist yet)

// AFTER (option A — hard assert):
assert!(result.is_ok(), "Together lowering must succeed: {result:?}");
let parts = builder.build_parts("test", dummy_digest()).unwrap();
assert_eq!(parts.nodes.len(), 6);
// ... rest of detailed assertions
```

### Fix 5 — Wire `proptest_save_canonical_name.rs` to call production `canonical_primitive_name`
**Impact:** 1 proptest (256 iterations) goes from tautological to real.
**Effort:** 30 min. Expose `canonical_primitive_name` as `pub` (or `pub(crate)`) and import it.

```rust
// BEFORE (proptest_save_canonical_name.rs:30-46):
fn canonical_name(primitive: &StepPrimitive) -> &'static str { /* 16-arm match */ }

// AFTER:
use crate::mod_compile_lowering::part_05::canonical_primitive_name;
let result = canonical_primitive_name(&save);
prop_assert_eq!(result, "set", "...");
```

---

## Disposition

| ID | Disposition | Rationale |
|----|-------------|-----------|
| C-01..C-12 (12) | `blocker` | Pervasive banned patterns that would let regressions pass silently. **REJECTED.** |
| H-01 | `blocker` | The BDD suite "documents" wide acceptable ranges that include the failure path. |
| H-02..H-12 | `owner_approved_debt` if signed off; otherwise `blocker` | Each is a tractable refactor with real mutation coverage. |
| M-01..M-09 | `owner_approved_debt` | Improvement opportunities. |
| L-01..L-10 | `owner_approved_no_action` | Test infrastructure, deferred, or out-of-scope (proof/Flux). |
| O-01..O-08 | `owner_approved_no_action` | Observations on test design quality, not actionable defects. |

---

## Verdict

```
STATUS: REJECTED
```

**7 CRITICAL findings + 12 HIGH findings** that, if unaddressed, would let the most
likely regressions in the compile/CLI/validate pipeline go undetected. Recommend:
(1) Fix the `if let Ok / else assert!(parsed.is_ok())` pattern in 6 CLI args test files
(Fix 1, ~30 min). (2) Replace `let _ = budget.field;` with concrete assertions in
`budget_analyzer.rs` and `red_queen_budget.rs` (Fix 2, ~1 hour). (3) Decide disposition
for the TDD-red Together tests (Fix 4, ~4 hours, requires product owner sign-off). (4)
Expose `canonical_primitive_name` and rewire `proptest_save_canonical_name.rs` (Fix 5,
~30 min). (5) Wire Section 47 contract assertions in `secret_finish_tests.rs` (Fix 3,
~2 hours).
