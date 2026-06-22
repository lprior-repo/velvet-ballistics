# Test Review — Slice 3: vb_compile, vb_cli, vb_validate, vb_proof_kernels (Round 3)

**Scope:** ~635 Rust files across 4 crates (`vb_compile`, `vb_cli` aka `velvet-ballistics`,
`vb_validate`, `vb_proof_kernels`).

**Date:** 2026-06-21
**Reviewer:** test-reviewer agent (round 3 of 40)

## STATUS: REJECTED

Round 3 surfaces one NEW CRITICAL dead-code regression that round 1 + round 2 reviewers
missed: **`crates/vb_compile/src/taint/mod.rs` is not declared in `lib.rs`** (only
`mod type_taint;` at line 37). The round-1 F3 fix at
`taint/tests/secret_finish_tests.rs` (13 sites replacing `matches!(result, Ok(_))` with
`assert!(workflow.finish_contains_secret_data())`) is applied to a file that is **never
compiled** because the parent `taint` module is missing from `lib.rs`. The 13 Section 47
contract tests for "secret in Finish must compile to Ok(CompiledWorkflow) with secret data
preserved" are dead artifacts. `cargo test -p vb_compile --list` confirms 0 tests under
`taint::tests::*` — only `type_taint::tests::*` appears in the binary, with 25 tests for
the *separate* `type_taint/tests.rs` module. Round 2 also missed this because they only
read the file's assertions without confirming the file was wired in.

In addition, **7 of 12 round-1/2 fixes are still REGRESSED**: F8 (wide-range exit codes),
F9 (proptest_together_errors vacuous match), F10 (proptest_choose_depth vacuous match),
F11 (vb_xi2f_compile_source_proptest is_ok smoke), F12 (proptest_choose_* is_ok smoke),
R2-H-04 (4 `let _ = result;` in together_e2e_tests), R2-H-05 (wave-7 envelope_header CRC
stubs), R2-H-07 (vb_qi37_14_1_run_step TODO), R2-H-12 (lifecycle_integration is_err), and
R2-M-04 (vb_xi2f_nested_do_lowering is_err). These regressions persist from round 1 and
round 2 unchanged.

Wave-9 added high-quality new test files (`gate_07_stack/tests.rs`, `gate_09_slots/tests.rs`,
`gate_10_node/tests.rs`, `gate_13_cycles/tests.rs`, `gates/tests.rs`,
`proptest_finish_digest.rs`) that use concrete `assert_eq!(validate(&parts), Ok(()))` and
specific variant matches — exemplary quality. The wave-9 expansion does NOT backfill
round-1/2 regressions. The slice has **1 NEW CRITICAL** + **0 HIGH** + **6 MEDIUM** +
**3 LOW** + **4 OBSERVATION** findings. Cannot be approved.

`cargo test -p vb_compile --tests` → **1053 passed, 2 ignored** (same as rounds 1–2).

---

## Round 1 + 2 Fix Verification (15 Sites)

| ID  | Round-1/2 Fix Target                                               | Status        | Evidence (current line)                                                                |
|-----|-------------------------------------------------------------------|---------------|----------------------------------------------------------------------------------------|
| F1  | `vb_cli/src/args/tests/{workflow,status,run,cancel,action,parse_*}.rs` — `if let Ok / else assert!(parsed.is_ok())` → `match { Ok(X) => ..., other => panic!("expected X, got {other:?}") }` | **STILL APPLIED** | `workflow.rs:7-18` `match { Ok(Command::Validate{..}) => { assert_eq!(...) } other => panic!(...) }`. Same shape in `status.rs:5-18`, `cancel.rs:14-26`, `action.rs:7-20`, `run.rs:7-22`. `journal.rs:152-158` uses `if let Ok(Command::Inspect{..}) / else panic!("expected Inspect command, got {parsed:?}")` (functionally equivalent). |
| F2  | `vb_cli/src/args/tests/parse_misc2.rs:503` — `assert!(result.is_ok())` → `.expect()` + content check | **STILL APPLIED** | `parse_misc2.rs:503` `.expect("positional_str on 'one two' at last index must succeed"); assert_eq!(val, "two")`. |
| F3a | `vb_cli/src/main_tests.rs` — 13 sites `assert!(journal/encoded/dir.is_ok())` → `.expect()` | **STILL APPLIED** | `main_tests.rs:62 .expect("slot value must encode")`, `:423 .expect("action 2 must resolve")`, `:508 .expect("test directory must be available")`, `:522 .expect("journal must reopen for valid dir")`, `:526 .expect("events for run must be readable")`, `:709 .expect("test directory must be available")`, `:713 .expect("journal must open")`, `:715 .expect("workflow parts must encode")`, `:732 .expect("resolver must load compiled IR")`, `:740 .expect("test directory must be available")`, `:742 .expect("journal must open")`, `:858 .expect("frame must build for valid step")`, `:956 .expect("test payload must encode for valid SlotValue vec")`. 13/13 sites. |
| F3b | `vb_cli/src/app_impl_tests.rs` — 13 sites `assert!(encoded/journal/dir.is_ok())` → `.expect()` | **STILL APPLIED** | `app_impl_tests.rs:68, 472, 571, 585, 589, 613, 617, 619, 636, 644, 646, 762, 864` — 13 sites all use `.expect("...")`. |
| F4  | `vb_compile/src/budget_analyzer.rs` — 41 `let _ = budget.field` → concrete `assert_eq!` | **STILL APPLIED** (with 2 residuals) | `budget_analyzer.rs:118-172, 240-268` use `assert_eq!(budget.max_steps_executable, 2)` etc. Residuals: `:190 let _ = other;` and `:233 let _ = other;` in `match result { Ok(_) | Err(UnboundedWorkflow) => {} Err(other) => { let _ = other; } }` (defensive only, contract is "must not panic"). |
| F5  | `vb_compile/src/taint/tests/secret_finish_tests.rs` — 13 sites `matches!(result, Ok(_))` → `assert!(workflow.finish_contains_secret_data())` | **APPLIED TO DEAD CODE** | `secret_finish_tests.rs:41-47, 68-74, 93-99, 119-122, 144-146, 167-168, 190-192, 229-231, 397-399, 419-421, 481-484, 574-576, 598-600` — all 13 sites use `let workflow = compile_workflow(source).expect(...); assert!(workflow.finish_contains_secret_data(), ...);` pattern. **HOWEVER**, the parent `taint` module is NOT declared in `lib.rs` (only `mod type_taint;` at line 37). `cargo test -p vb_compile --list | grep taint::tests` returns 0 tests; the 13 Section 47 tests are dead artifacts. **R3-C-01**. |
| F6  | `vb_compile/src/mod_compile_lowering/together_*_tests.rs` — TDD-red `if let Ok(())` → hard `.expect()` | **STILL APPLIED** | `together_lowering_tests.rs:208, 257, 292, 328, 361, 392, 541, 577, 614, 655` use `let () = result.expect("Together lowering must succeed per spec");`. `together_integration_tests.rs:272, 361` use `.expect("Together lowering must succeed per spec")`. `together_e2e_tests.rs:166, 236, 554` use `.expect("Together lowering must succeed per spec")`. |
| F7  | `vb_compile/tests/proptest_save_canonical_name.rs` — local `canonical_name()` → production `canonical_primitive_name` | **STILL APPLIED** | `proptest_save_canonical_name.rs:15` `use vb_compile::mod_compile_lowering::canonical_primitive_name as canonical_name;`. |
| F8  | `vb_compile/src/tests/do_choose_digest_unit_tests.rs` — 18 sites `let _ = digest_step_primitive(...)` → `.expect()` | **STILL APPLIED** | `do_choose_digest_unit_tests.rs:179-180, 203, 207, 224, 228, 244, 248, 270-271, 302-303, 307-308, 330-331, 335-336, 359-360, 364-365, 387-388, 392-393, 414, 418` — 18 sites all use `.expect("digest must succeed for valid primitive")`. |
| F9  | `vb_compile/tests/digest_ask_explicit_arm.rs` — 11 sites `let _ = canonical_digest(...)` → capture + `assert_ne!(digest, [0u8; 32])` | **STILL APPLIED** | `digest_ask_explicit_arm.rs:144-147, 155-158, 167-170, 182-185, 193-196, 204-207, 215-218, 226-229, 261-264, 272-275, 283-286` — 11 sites all use captured `digest` plus `assert_ne!(digest.as_bytes(), [0u8; 32], "digest must be non-trivial")`. |
| F10 | `vb_cli/tests/cli_vb_m214_bdd_scenarios.rs` — wide-range exit code `Some(0) || Some(2)` | **STILL REGRESSED** | `:304 output.status.success() || output.status.code() == Some(5)`, `:373 code == Some(0) || code == Some(2)`, `:531 code == Some(0) || code == Some(1) || code == Some(2) || code == Some(7)`, `:1226`, `:1266`, `:1309` — 6 wide-range assertions persist. |
| F11 | `vb_compile/src/proptest_together_errors.rs:262-264` — vacuous `matches!(result, Ok(()) \| Err(_))` | **STILL REGRESSED** | `:262 prop_assert!(matches!(result, Ok(()) \| Err(_)), "zero-branch together must return a Result without panic")`. The `proptest_together_error_zero_branches` test contract is "must not panic" — but the assertion after the function call is vacuous (any Ok or Err passes). |
| F12 | `vb_compile/tests/proptest/proptest_choose_depth.rs:62-66` — vacuous `matches!(inner, Ok(_) \| Err(_))` | **STILL REGRESSED** | `:63 prop_assert!(matches!(inner, Ok(_) \| Err(_)), "varied_choose_yaml compiles or errors gracefully (never panics), got {:?}", inner)`. Vacuous — combined with `prop_assert!(result.is_ok())` at line 52 (no panic), the inner is necessarily `Ok | Err`. |
| F13 | `vb_compile/tests/vb_xi2f_compile_source_proptest.rs:177` — `prop_assert!(result.is_ok())` smoke | **STILL REGRESSED** | `:177 prop_assert!(result.is_ok(), "YamlCompiler::compile on valid YAML must return Ok, got {:?}", result)`. Followed by `prop_assert!(workflow.node_count() > 0)` at line 72 — but the bare `is_ok()` at line 177 still accepts any `Ok(workflow)`. |
| F14 | `vb_compile/tests/proptest/proptest_choose_{otherwise,fallthrough,emission}.rs` — `prop_assert!(result.is_ok())` smoke | **STILL REGRESSED** | `proptest_choose_otherwise.rs:51, 62`, `proptest_choose_fallthrough.rs:49`, `proptest_choose_emission.rs:65` all still have `prop_assert!(result.is_ok(), "...")`. `otherwise_target_exists` and `fallthrough_default_branch_compiles` would pass if `compile_workflow` returned `Ok(empty_workflow)`. |
| F15 | `vb_compile/src/mod_compile_lowering/together_e2e_tests.rs:368,407,446,494` — `let _ = result;` smoke | **STILL REGRESSED** | `:366 let _ = result;`, `:405 let _ = result;`, `:444 let _ = result;`, `:492 let _ = result;`. All 4 tests documented as "Top-level compilation not in scope for body-position lowering (vb-xi2f.22)" — TDD-red "must not panic" only. |

**Round-3 regression count: 9 (F10, F11, F12, F13, F14, F15 + R2-H-04, R2-H-05, R2-H-07,
R2-H-12, R2-M-04)**.

**Round-3 NEW dead-code regression: F5 (taint module not wired)**.

---

## Findings (severity-ordered)

| ID | Sev | File:Line | Defect | Mutation thought experiment | Recommended fix | Disposition |
|----|-----|-----------|--------|------------------------------|------------------|--------------|
| R3-C-01 | CRITICAL | `crates/vb_compile/src/lib.rs:37` (missing `mod taint;`) + `taint/mod.rs:7-9` (declares `mod tests;`) + `taint/tests/secret_finish_tests.rs` (entire file, 602 lines) | The round-1 F3 fix converted 13 sites from `matches!(result, Ok(_))` to `assert!(workflow.finish_contains_secret_data())` per Section 47. The file is correct. **But the parent `taint` module is missing from `lib.rs`** — only `mod type_taint;` at line 37 (which loads `type_taint/tests.rs`, a *different* module with `validate_workflow_ast`-based tests). The `taint/mod.rs` file is not reachable. `cargo test -p vb_compile --tests --list | grep taint::tests::compile_accepts` returns 0 tests. The 13 Section 47 contract tests (`compile_accepts_secret_finish_result`, `compile_handles_untrusted_data_in_non_finish`, `compile_accepts_secret_object_finish_result`, etc.) are dead artifacts. The contract "secret in Finish must compile to Ok(CompiledWorkflow) with secret data preserved per Section 47" is enforced only by `type_taint::tests::compile_and_parse_ast_accept_secret_*` (which use `parse_ast_valid` returning `Result<(), String>`, not the production `compile_workflow` pipeline). | Delete the production `compile_workflow`'s secret-preservation logic. None of the dead-code `taint::tests::compile_*` tests run. Only `type_taint::tests::compile_and_parse_ast_accept_secret_*` runs (testing `validate_workflow_ast`, not `compile_workflow`). | Add `mod taint;` to `crates/vb_compile/src/lib.rs` (alongside `mod type_taint;` at line 37). Then run `cargo test -p vb_compile taint::tests -- --nocapture` to confirm 13+ tests pass. Verify `taint::tests::compile_accepts_secret_finish_result` returns `workflow.finish_contains_secret_data() == true` for `result: $secrets.token` sources. | `blocker` |
| R3-M-01 | MEDIUM | `vb_compile/src/proptest_finish_digest.rs:262` | `let _sid = "s".to_string();` — the variable is shadowed by the `sid = "s"` argument in the format! macro at line 266. The `let _sid` assignment is dead. The `proptest::prop_assume!(id != "s")` at line 261 ensures `id` is not "s", so the format! uses "s" as the step id and the test is consistent. But the dead `let _sid` is misleading code (suggests it should be used). | Compiler will warn but won't fail. Not a behavior bug. | Remove `let _sid = "s".to_string();` at line 262 — it's shadowed and unused. | `owner_approved_debt` |
| R3-M-02 | MEDIUM | `vb_compile/src/tests/error_variant_tests.rs:587-589, 837-841` | `let _compiler = YamlCompiler::default();` and `let _digest = WorkflowDigest::from_bytes([0u8; 32]);` — pure construction smoke tests. Test names: `yaml_compiler_default_constructs` and `workflow_digest_from_bytes_creates_digest`. Comments document "Just verify it can be created without panicking". | `YamlCompiler::default()` and `WorkflowDigest::from_bytes` are infallible constructors. The tests verify the type compiles and the function doesn't panic — the type system already enforces this. These tests cannot catch any behavioral regression. | Either remove these tests (preferred — redundant with the type system) or convert them to assertion-bearing tests: `let compiler = YamlCompiler::default(); assert!(compiler.can_compile_empty_source());` (would require new method). | `owner_approved_debt` |
| R3-M-03 | MEDIUM | `vb_compile/src/taint/tests/secret_finish_tests.rs:572-577, 596-601` (if R3-C-01 is fixed) | `prop_assert!(workflow.finish_contains_secret_data(), ...)` is used in BOTH `proptest_compile_accepts_clean_finish` (line 557) and `proptest_compile_accepts_literal_finish` (line 583). For CLEAN (non-secret) inputs and LITERAL integer values, `finish_contains_secret_data()` returns `slot.get() > 0`, which is **false** for slot 0 and `result: ${input_name}` where input_name is a text input. The assertion will fail for every proptest case once the module is wired in. | Activate the dead module (add `mod taint;` to lib.rs) and run the proptests — `proptest_compile_accepts_clean_finish` and `proptest_compile_accepts_literal_finish` will fail 100% of cases. | Replace with `prop_assert!(!workflow.finish_contains_secret_data(), "clean Finish must NOT contain secret data")` and `prop_assert_eq!(workflow.finish_result_slot(), Some(SlotIdx::new(0)), "literal Finish result: <int> must compile to slot 0 or None")`. The `finish_contains_secret_data()` is a weak proxy for "slot > 0" — needs replacement with a real secret-data accessor. | `blocker` (after R3-C-01 lands) |
| R3-M-04 | MEDIUM | `vb_validate/src/red_phase_proptest.rs:81-84, 165-167` | `prop_assert!(result.is_ok(), "validate_gate_08 should pass when symbol {symbol} < symbols_count {symbols_count}, got {result:?}")` (line 81-84) and `prop_assert!(validate_gate_08_accessor_path_segments(&parts).is_ok(), "empty accessors should always pass gate 8")` (line 165). Both are smoke `is_ok()` with no follow-up field-level check. | Modify `validate_gate_08_accessor_path_segments` to return `Ok(())` for every input. Both proptests pass. | Replace `prop_assert!(result.is_ok())` with `assert_eq!(result, Ok(()))` plus a follow-up invariant (e.g., "must not produce AccessorSymbolOutOfBounds for valid symbol"). For empty accessors, document that the contract is "Ok with empty error list" and add `assert!(result.is_ok_and(\|_\| true), "empty accessors must pass")`. | `owner_approved_debt` |
| R3-M-05 | MEDIUM | `vb_validate/src/property_tests/proptest_state_machine.rs:243-250` | `sm_taint_outcome_is_typed` proptest: `let _ = result;` discards `validate_taint(&wf)`. Docstring: "the proptest just exercises the code path. The type system enforces the variant set." | Change `validate_taint` to always return `Ok(())`. Test passes. The type system does enforce `Result<(), ValidationError>`, but the test doesn't verify any variant exists. | Replace `let _ = result;` with `match result { Ok(()) \| Err(ValidationError::X { .. }) => {} Err(other) => prop_assert!(false, "unexpected variant: {other:?}") }` — enumerate the acceptable variants to catch new variants silently appearing. | `owner_approved_debt` |
| R3-L-01 | LOW | `vb_cli/src/main_tests.rs:1080 lines` and `app_impl_tests.rs:1957 lines` — overall file sizes | Both files exceed 1000 lines and contain ~30 `#[test]` functions each. The `.expect()`-based test idioms are mechanically correct, but the file size makes it hard to maintain consistency. Project AGENTS.md says no architectural-drift file limit applies to test files, but the standard under-300-line rule is for production code. | Not a behavior defect — test discovery still works. | Optional refactor: split `main_tests.rs` into `main_tests/journal_tests.rs`, `main_tests/run_tests.rs`, `main_tests/cli_tests.rs`, etc. Acceptable as-is. | `owner_approved_no_action` |
| R3-L-02 | LOW | `vb_compile/src/property_tests/bytecode_ast_parity.rs:556, 712` | `prop_assert!(ast_outcome == bytecode_outcome).is_err()` (line 556) — actually it's `let err = ...; .is_err()` checking. Plus `if load_const(&constants, idx).is_err()` at line 712. Both are production-helper call sites (not test assertions). Acceptable. | n/a — production paths. | Out of scope. | `owner_approved_no_action` |
| R3-L-03 | LOW | `vb_compile/src/ast/parse/step.rs:219` | `let _ = sub_index;` inside production loop body — silences unused-variable warning. The loop iterates by `sub_index` but doesn't use it (the sub-index is propagated via `parse_step` indirectly via the `marks` arg, not the index). Production code, not a test. | n/a. | Out of scope. | `owner_approved_no_action` |
| R3-O-01 | OBSERVATION | `vb_compile/src/proptest_finish_digest.rs:1-111` | 100+ line `#![allow(...)]` blanket suppression of clippy lints including `clippy::let_underscore_must_use`, `clippy::unnecessary_unwrap`, `clippy::unwrap_used`, `clippy::expect_used`, `clippy::panic`, `clippy::todo`, `clippy::unimplemented`. This is a maintenance hazard — the suppression hides banned patterns that the round-1 fixes deliberately introduced. | n/a — file is a proptest where `.expect()` is acceptable. | Reduce the allow list to only the lints actually needed; suppress per-`#[test]` rather than per-file. | `owner_approved_no_action` |
| R3-O-02 | OBSERVATION | `vb_validate/src/kani_gate_08_structural.rs` and `kani_gate_08_support.rs` (NEW wave-7+) | Both files are `#[cfg(kani)]`-gated — only compile when running Kani. They use `kani::assert(result.is_ok(), ...)` which is the canonical Kani idiom, but `kani::assert(matches!(result, Ok(_) \| Err(_)))` at `vb_compile/src/kani/vb_compile_error_bounds.rs:44, 59` is tautological (any Result matches). | n/a — verifier harnesses, not behavior tests. | Out of scope. | `owner_approved_no_action` |
| R3-O-03 | OBSERVATION | `vb_cli/src/app_impl_tests.rs:50-55` | `assert!(matches!(parsed, Ok(Command::AiContext { .. }))); if let Ok(Command::AiContext { run_id, db, output }) = parsed { ... }` — pattern uses `matches!` for variant check THEN `if let` for field access. Acceptable: the `matches!` is the variant guard, the `if let` is the field access. Different from the banned C-01 pattern (which uses `if let Ok(Command::X) / else assert!(parsed.is_ok())`). | n/a — pattern is correct. | No action needed. | `owner_approved_no_action` |
| R3-O-04 | OBSERVATION | `vb_compile/src/property_tests/bytecode_ast_parity.rs:740 lines, vb_validate/src/property_tests/*.rs:1100+ lines` | Wave-7+ property test files are exceptionally high quality: `prop_assert_eq!(ast_outcome, bytecode_outcome)` with concrete expected values, no `is_ok()` smokes. Bytecode/AST parity proptest uses 1024 cases with full production binding. Wave-9 additions to `vb_validate/src/gates/tests.rs` (1338 lines), `gate_07_stack/tests.rs` (229 lines), `gate_09_slots/tests.rs`, etc., use the strongest pattern observed: `assert_eq!(validate_gate_XX(&parts), Ok(()))` and `assert!(matches!(..., Err(ValidationError::XX { .. })))`. | n/a — exemplary. | Track as exemplar for future test additions. | `owner_approved_no_action` |

---

## Pattern Census (round 3 counts)

### `assert!(...is_ok()) / assert!(...is_err()) / matches!(..., Some(_) | Ok(_) | Err(_))` and bare `unwrap()`

| Crate | Total matches (round 3) | Notes |
|-------|--------------------------|-------|
| `vb_cli/src` | ~80 | `main_tests.rs` (0 — round-1 fix), `app_impl_tests.rs` (0 — round-1 fix), `io.rs` (7 `is_ok()`), `args/tests/parse_misc2.rs` (0), `agent_context/tests/unit.rs` (200+ `panic!()` accepted as test policy) |
| `vb_cli/tests` | ~110 | `cli_vb_m214_bdd_scenarios.rs` (6 wide-range exit codes REGRESSED), `cli_integration.rs` (7 `is_err()` REGRESSED + 12 `let _ = &report.X`), `cli_trace_integration.rs` (15 unwraps), `lifecycle_integration.rs` (1 `is_err()` REGRESSED), `vb_qi37_14_1_run_step.rs` (1 TODO REGRESSED), `admission_evidence_integration/chunk_004.rs` (2 `is_err()` REGRESSED) |
| `vb_compile/src` | ~10 | `taint/tests/secret_finish_tests.rs` (DEAD — F5, 13 sites), `tests/error_variant_tests.rs` (2 smoke), `tests/property_validation_tests.rs` (3 TDD-red + println!), `tests/integration_reduce_tests.rs` (1 println!), `tests/do_choose_digest_unit_tests.rs` (0 — round-1 fix), `budget_analyzer.rs` (2 `let _ = other` residuals), `mod_compile_lowering/together_e2e_tests.rs` (4 `let _ = result` REGRESSED), `mod_compile_lowering/together_integration_tests.rs` (2 `let _ = result`), `proptest_choose_lowering.rs` (concrete assertions), `proptest_step_offset.rs`, `proptest_collect.rs`, `proptest_body_dispatcher.rs`, `proptest_error_parity.rs`, `proptest_together_errors.rs` (1 vacuous match REGRESSED), `property_tests/bytecode_ast_parity.rs` (excellent) |
| `vb_compile/tests` | ~10 | `red_queen_budget.rs` (4 `is_ok()`, 4 `is_err()`), `proptest_choose_*.rs` (5 `is_ok()` REGRESSED), `vb_xi2f_compile_source_proptest.rs` (1 `is_ok()` REGRESSED), `proptest_choose_depth.rs` (1 vacuous match REGRESSED), `v1_primitive_lowering.rs` (2 `is_err()`), `vb_a001_for_each_topology.rs` (1 each), `vb_xi2f_nested_do_lowering.rs` (1 `is_err()` REGRESSED) |
| `vb_proof_kernels/src` | ~5 | `envelope_header/tests.rs` (3 CRC stubs REGRESSED), `profile_contract/*` (Verus proofs) |
| `vb_validate/src` | ~10 | `gates/tests.rs` (0 — excellent), `gate_XX/tests.rs` (0 — excellent), `red_phase_proptest.rs` (2 `is_ok()` smoke), `property_tests/proptest_state_machine.rs` (1 `let _ = result`), `property_tests/proptest_bound_enforcement.rs` (1 `let _ = validate_*`), `property_tests/proptest_constant_folding_validation.rs` (1 `let _ = validate_taint`), `type_taint/type_taint_tests.rs` (3 `let _ = validate_*` never-panic) |
| `vb_validate/tests` | ~3 | `red_phase_validation.rs` (3 `is_ok()` smoke REGRESSED) |
| **TOTAL** | **~225** | (concentrated in `vb_cli/tests/` regressions + `vb_compile/src/` together + taint dead-code) |

### `let _ = ...` (silent suppression, excluding kani/flux/verus files)

| Crate | Total matches (round 3) | Top files |
|-------|--------------------------|-----------|
| `vb_compile/src` | 22 | `budget_analyzer.rs` (2 residuals, R3-M-04), `enums/side_effect_tests.rs` (7 variant-existence smoke — round-1 L-06), `enums/tests/retry_safety_tests.rs` (4 — round-2 L-02), `mod_compile_lowering/together_e2e_tests.rs` (4 R2-H-04), `mod_compile_lowering/together_integration_tests.rs` (4), `mod_compile_lowering/together_lowering_tests.rs` (3 — *never_panic contract*), `ast/parse/step.rs` (1 production) |
| `vb_compile/tests` | 2 | `vb_xi2f_nested_do_lowering.rs:361` (`let _ = action`), `idempotency_parity.rs:529` (comment) |
| `vb_compile/src/property_tests` | 5 | `bytecode_ast_parity.rs` (production-bound helpers) |
| `vb_cli/src` | ~60 | `commands_verify/pipeline.rs` (6 — production code), `commands_workflow/tests.rs` (2), `deliver_sink/atomic_publish.rs` (2 production), `deliver_sink/deliver_*_test_support.rs` (2 test support), `matrix/source_command_enum.rs` (1) |
| `vb_cli/tests` | 30 | `cli_integration.rs` (12 `let _ = &report.X` R2-M-06 + 1 `let _ = server_tx.send`), `cli_verify_integration.rs` (1), `lifecycle_integration.rs` (3 production-style), `deliver_sink_integration.rs` (1) |
| `vb_validate/src` | 7 | `type_taint/type_taint_tests.rs` (3 *never_panic*), `property_tests/proptest_bound_enforcement.rs` (1), `property_tests/proptest_state_machine.rs` (3), `property_tests/proptest_constant_folding_validation.rs` (1), `gate_tests.rs` (1) |
| `vb_proof_kernels/src` | 1 | `profile_contract/validation.rs` (production code) |
| **TOTAL** | **~127** | (down from 133 in round-2; the reduction is due to more accurate filtering of kani/flux) |

### `#[ignore]` / `#[should_panic]` / `sleep(` / `todo!()` / `unimplemented!()`

| Crate | Total | Notes |
|-------|-------|-------|
| `vb_cli/src` | 1 | `doctor.rs:31` — `std::thread::sleep(...)` in production retry loop on `ProcessLockHeld`. **Resource-risk**: unbounded retry may exceed moon ci budget. |
| `vb_cli/tests` | 1 | `cli_integration.rs:5348` — `std::thread::sleep(10ms)` in answer-IPC test busy-wait. Bounded by `Duration::from_secs(5)` deadline. |
| `vb_compile/tests/finish_digest_integration.rs:276` | 1 | `#[ignore = "BLOCKED: legacy canonical_digest is not accessible from integration test crate"]` — documented visibility blocker. |
| **TOTAL** | 3 | (all acceptable — production retry, bounded busy-wait, documented blocker) |

### `lazy_static` / `OnceLock` / `static mut` / `thread_local!`

| Crate | Total | Notes |
|-------|-------|-------|
| `vb_cli/tests` | 3 | `deliver_sink_integration.rs:877-878` (fingerprint-keyed cache, documented), `deliver_test_support.rs:64`, `deliver_debug_test_support.rs:35` (test support) |
| `vb_validate/src` | 1 | `diag_render/fallback.rs:14` (production) |
| **TOTAL** | 4 | (all acceptable, well-documented) |

### `panic!` and `println!` in test code

| Crate | Total matches |
|-------|---------------|
| `vb_cli/src` (incl. test submodules) | 44 `panic!` + 4 `println!` — `args/tests/journal.rs` has 19 `panic!` which is the correct CLI args fix shape, not banned. `agent_context/tests/unit.rs` has 200+ `panic!` calls (fixture construction). |
| `vb_compile/src` | 12 `panic!` + 3 `println!` — `tests/property_validation_tests.rs` has 3 `println!("PASS ...")` in TDD-red pattern (R2-H-01 REGRESSED). |
| `vb_cli/tests` | 21 `println!` for diagnostics in BDD scenarios. |
| `vb_compile/tests` | 3 `println!` for GAP EXPOSED. |

### Wave-9 NEW test files (added since round 2)

| File | Lines | Quality |
|------|-------|---------|
| `vb_validate/src/gates/tests.rs` | 1338 | **Excellent** — concrete `assert_eq!(validate_gate_XX(&parts), Ok(()))` + specific `Err(ValidationError::XX { .. })` matches. 28 `assert!(matches!(...))` calls. |
| `vb_validate/src/gate_07_stack/tests.rs` | 229 | **Excellent** — `assert_eq!(validate_gate_07_expression_stack_depth(&parts), Ok(()))` and `assert!(matches!(..., Err(ValidationError::ExpressionStackMismatch { .. })))`. |
| `vb_validate/src/gate_09_slots/tests.rs` | 12.7K | **Excellent** — same pattern. |
| `vb_validate/src/gate_10_node/tests.rs` | 11.1K | **Excellent** — same pattern. |
| `vb_validate/src/gate_13_cycles/tests.rs` | (small) | **Excellent** — same pattern. |
| `vb_validate/src/kani_gate_08_structural.rs` | 662 | Kani-only (`#[cfg(kani)]`), out of scope. |
| `vb_validate/src/kani_gate_08_support.rs` | (small) | Kani-only, out of scope. |
| `vb_compile/src/proptest_finish_digest.rs` | 351 | **Excellent** — production-bound via `compile_source()`, uses `prop_assert_eq!(c1.digest(), c2.digest())` and `prop_assert_ne!(da, db, ...)`. Slight R3-M-01 dead-code at line 262. |

---

## Mutation Gaps (top 5 most dangerous bugs the slice would NOT catch)

1. **`compile_workflow` strips secret data from Finish results.** R3-C-01: the 13 Section 47
   contract tests in `taint/tests/secret_finish_tests.rs` are dead code (parent `taint` module
   not wired into `lib.rs`). Only `type_taint::tests::compile_and_parse_ast_accept_secret_*`
   (which uses `parse_ast_valid` returning `Result<(), String>`, NOT the production
   `compile_workflow` pipeline) covers the contract. A regression that strips `$secrets.token`
   references during lowering would NOT be caught. **File:Line:** production
   `crates/vb_compile/src/mod_compile_lowering/part_04/body_dispatch.rs` and the
   `taint` module wiring in `crates/vb_compile/src/lib.rs:37`.

2. **Wave-6 `compute_header_crc` and `validate_header_crc` are stubs.** R2-H-05 REGRESSED:
   `envelope_header/tests.rs:144-147` asserts `compute_header_crc(&header) == 0` and
   `:150-153` asserts `validate_header_crc(&header) == true`. Confirmed in test binary:
   `envelope_header::tests::test_compute_header_crc_returns_zero` and
   `envelope_header::tests::test_validate_header_crc_always_true` are in
   `target/debug/deps/vb_proof_kernels-1d2e60d2283d5364 --list`. A regression that
   replaces both with `|_| 0` and `|_| true` would pass the tests. The CRC contract
   is unenforced. **File:Line:** production
   `crates/vb_proof_kernels/src/envelope_header.rs`.

3. **`emit_single_body_set` Together branch deleted.** R2-H-04 REGRESSED: 4 `let _ = result;`
   in `together_e2e_tests.rs:366, 405, 444, 492` plus the vacuous `matches!(result, Ok(()) | Err(_))`
   in `proptest_together_errors.rs:262` plus `matches!(inner, Ok(_) | Err(_))` in
   `proptest_choose_depth.rs:63`. Delete the Together branch and all 6 tests pass.
   **File:Line:** production `crates/vb_compile/src/mod_compile_lowering/part_04/body_dispatch.rs`.

4. **Property tests for clean/literal Finish silently fail when activated.** R3-M-03:
   `taint/tests/secret_finish_tests.rs:557-578` (`proptest_compile_accepts_clean_finish`)
   and `:582-601` (`proptest_compile_accepts_literal_finish`) use
   `assert!(workflow.finish_contains_secret_data())` — but `finish_contains_secret_data()`
   returns `slot > 0`, which is false for `result: 0` and `result: ${clean_input}`. If R3-C-01
   activates the dead module, both proptests fail 100% of cases. **File:Line:** production
   `crates/vb_compile/src/taint/tests/secret_finish_tests.rs:574, 598` (assertions must
   be inverted for non-secret tests).

5. **Section 47 contract violation: empty Finish result.** R3-C-01 + R3-M-03 combined: the
   `compile_handles_untrusted_data_in_non_finish` test asserts `finish_contains_secret_data()`
   on a workflow with `result: 0` — which is impossible. The implementation
   `finish_contains_secret_data = slot.get() > 0` is a weak proxy for "Finish preserves
   secret data" — it does not actually check that the slot contains secret data. The
   Section 47 contract needs a real `finish_contains_secret_data()` accessor that
   inspects the slot's taint bit or value type. **File:Line:** production
   `crates/vb_core/src/workflow/workflow.rs:166` (`finish_contains_secret_data` impl).

---

## Top 5 Fixes (impact-per-effort)

### Fix 1 — Wire `taint` module into `lib.rs` (R3-C-01) — 5 min
**Impact:** Activates 13+ dead Section 47 contract tests. Surfaces R3-M-03 (clean/literal Finish assertions must be inverted).

```rust
// crates/vb_compile/src/lib.rs, after line 37:
mod type_taint;
// ADD:
#[cfg(test)]
mod taint;
```

Then run `cargo test -p vb_compile taint::tests -- --nocapture` and fix R3-M-03 (invert `finish_contains_secret_data()` to `!finish_contains_secret_data()` for clean/literal Finish tests, or replace with `assert_eq!(workflow.finish_result_slot(), Some(SlotIdx::new(0)))` for `result: 0`).

### Fix 2 — Replace 4 `let _ = result;` in `together_e2e_tests.rs` with concrete workflow assertions (R2-H-04, 15 min)
**Impact:** 4 Together E2E tests become real contract tests. Map each together shape to expected `node_count()` and `TogetherBranch` index.

```rust
// BEFORE (together_e2e_tests.rs:490):
let result = compile_yaml(yaml);
let _ = result;
// AFTER:
let workflow = compile_yaml(yaml)
    .expect("together with foreach in branches must compile at this layer");
assert!(workflow.node_count() >= 6, "foreach: 1 start + 1 next + 1 done per branch * 2 branches + 1 together_start + 1 together_join + 1 finish");
```

### Fix 3 — Strengthen `proptest_choose_*.rs` to assert on workflow content (R2-H-02 / F14, 1 hour)
**Impact:** 5 banned `prop_assert!(result.is_ok())` proptests in 4 `proptest_choose_*.rs` files become real contract tests. Section 38 "Choose otherwise / fallthrough / depth / emission" rows become enforced.

```rust
// BEFORE (proptest_choose_otherwise.rs:50-55):
prop_assert!(result.is_ok(), "choose with otherwise must compile Ok, got {:?}", result);
// AFTER:
let wf = result.expect("choose with otherwise must compile");
// Find at least one ChooseSlot::Otherwise node in the workflow
let mut found_otherwise = false;
for i in 0..wf.node_count() {
    if let Some(ChooseSlot::Otherwise) = wf.node(StepIdx::new(i)).choose_slot {
        found_otherwise = true;
        break;
    }
}
prop_assert!(found_otherwise, "workflow with `otherwise:` must have at least one ChooseSlot::Otherwise node");
```

### Fix 4 — Convert `property_validation_tests.rs` TDD-red `Ok(_) => panic! / Err(_) => println!("PASS")` (R2-H-01, 1 hour)
**Impact:** Section 38 "Together empty branches", "Reduce empty body", "Duplicate branch labels" rows enforced.

```rust
// BEFORE (property_validation_tests.rs:11-16):
match result {
    Ok(_) => panic!("GAP EXPOSED: Together with 0 branches compiled successfully..."),
    Err(e) => println!("PASS (validation exists): Empty Together rejected: {:?}", e),
}
// AFTER:
let errors = result.expect_err("Together with 0 branches must fail at compile layer");
assert!(errors.0.iter().any(|e| matches!(e, CompileError::StepFieldShape { field, .. } if field == "branches")),
        "Together with 0 branches must produce StepFieldShape{{field: \"branches\"}}");
```

### Fix 5 — Strengthen `vb_proof_kernels/src/envelope_header/tests.rs` CRC assertions (R2-H-05, 1 hour)
**Impact:** CRC contract is now enforced. Pick known-input/expected-output pairs.

```rust
// BEFORE (envelope_header/tests.rs:144-147):
fn test_compute_header_crc_returns_zero() {
    let header = EnvelopeHeader::new();
    assert_eq!(compute_header_crc(&header), 0);
}
// AFTER:
fn test_compute_header_crc_is_deterministic_and_nonzero_for_known_input() {
    let mut header = EnvelopeHeader::new();
    header.magic = 0xCAFEBABE;
    header.version = 1;
    header.payload_len_u32 = 42;
    let crc1 = compute_header_crc(&header);
    let crc2 = compute_header_crc(&header);
    assert_eq!(crc1, crc2, "CRC must be deterministic");
    assert_ne!(crc1, 0, "CRC of non-empty header must be non-zero");
}
```

---

## Disposition

| ID | Disposition | Rationale |
|----|-------------|-----------|
| R3-C-01 | `blocker` | Dead-code Section 47 tests; activation fixes R3-M-03 simultaneously. |
| R3-M-01 | `owner_approved_debt` | Dead `let _sid` is misleading code, not a behavior defect. |
| R3-M-02 | `owner_approved_debt` | Pure construction smoke tests; redundant with type system. |
| R3-M-03 | `blocker` (after R3-C-01) | Inverted assertions will fail when taint module is activated. |
| R3-M-04 | `owner_approved_debt` | Smoke `is_ok()` in two proptests; bounded by file's explicit contract. |
| R3-M-05 | `owner_approved_debt` | `let _ = result;` in `sm_taint_outcome_is_typed`; well-documented. |
| R3-L-01..L-03 | `owner_approved_no_action` | File size, production-code patterns, well-documented. |
| R3-O-01..O-04 | `owner_approved_no_action` | Observations on test design quality, not actionable defects. |
| R2-H-04, R2-H-05, R2-H-07, R2-H-12 | `blocker` | Round-2 regressions still present. |
| R2-M-04 | `owner_approved_debt` | 1 banned `is_err()` in `vb_xi2f_nested_do_lowering.rs:488`. |
| R2-M-05, R2-M-06 | `owner_approved_debt` | BDD wide-range exit codes + report-field `let _ =` (round-2 partial fix). |
| F10, F11, F12, F13, F14, F15 | `blocker` (8 round-1 REGRESSED) | Persist from round 1 unchanged. |

---

## Verdict

```
STATUS: REJECTED
```

**1 NEW CRITICAL (R3-C-01) + 8 round-1/2 REGRESSIONS (F10-F15, R2-H-04, R2-H-05, R2-H-07,
R2-H-12, R2-M-04)** remain in the slice. Wave-9 added ~3,500 lines of high-quality
property tests in `vb_validate/src/gates/tests.rs` and gate_07/09/10/13/tests.rs, plus
the `proptest_finish_digest.rs` 351-line production-bound Finish digest proptest — all
exemplary. The wave-9 expansion did NOT backfill the round-1/2 regressions in
`vb_cli/src/main_tests.rs` (none), `app_impl_tests.rs` (none), `parse_misc2.rs` (none),
`together_e2e_tests.rs` (4 R2-H-04), `property_validation_tests.rs` (R2-H-01),
`red_phase_validation.rs` (R2-H-03), `cli_integration.rs` (R2-H-06), `vb_xi2f_nested_do_lowering.rs`
(R2-M-04), `vb_qi37_14_1_run_step.rs` (R2-H-07), `lifecycle_integration.rs` (R2-H-12),
`proptest_choose_*.rs` (F14), `proptest_together_errors.rs` (F11), `proptest_choose_depth.rs`
(F12), `vb_xi2f_compile_source_proptest.rs` (F13), `cli_vb_m214_bdd_scenarios.rs` (F10),
`envelope_header/tests.rs` (R2-H-05), nor activate the dead `taint/tests/secret_finish_tests.rs`
module (R3-C-01).

Recommend: (1) Add `#[cfg(test)] mod taint;` to `vb_compile/src/lib.rs` to activate the
Section 47 dead-code tests (Fix 1, ~5 min, surfaces R3-M-03). (2) Invert or replace the
`finish_contains_secret_data()` assertions in `proptest_compile_accepts_clean_finish`
and `proptest_compile_accepts_literal_finish` (~10 min, R3-M-03). (3) Replace 4 `let _ =
result;` in `together_e2e_tests.rs` (Fix 2, ~15 min). (4) Convert `property_validation_tests.rs`
TDD-red to specific error variant match (Fix 4, ~1 hr). (5) Strengthen 5 `prop_assert!(result.is_ok())`
in `proptest_choose_*.rs` to assert on workflow content (Fix 3, ~1 hr). (6) Strengthen 3 CRC
stub assertions in `envelope_header/tests.rs` (Fix 5, ~1 hr).