# Test Review — Slice 3: vb_compile, vb_cli, vb_validate, vb_proof_kernels (Round 4)

**Scope:** ~700 Rust files across 4 crates (`vb_compile`, `vb_cli` aka `velvet-ballistics`,
`vb_validate`, `vb_proof_kernels`).

**Date:** 2026-06-21
**Reviewer:** test-reviewer agent (round 4 of 40)

## STATUS: REJECTED

Round 4 finds **0 NEW CRITICAL** defects — the round-3 R3-C-01 dead-code taint module
regression is fixed (`#[cfg(test)] mod taint;` now declared at `crates/vb_compile/src/lib.rs:42`,
46 taint tests run, 0 failures) and the round-3 R3-M-03 inverted `finish_contains_secret_data()`
assertion is fixed (`!workflow.finish_contains_secret_data()` at `secret_finish_tests.rs:573, 589`).
Round-1 fix verifications show **9 of 15 fixes STILL APPLIED** with 6 partial regressions
and **3 round-2/3 regressions ALSO STILL APPLIED**: R2-H-01 `println!("PASS ...")` in
`property_validation_tests.rs:14, 24`, R2-H-06 7 banned `is_err()` in `cli_integration.rs:1246,
1411, 1554, 1571, 1604, 1610, 1722`, R2-M-02 7 banned `is_ok()` in `vb_cli/src/io.rs:281-322`,
R2-M-06 12 `let _ = &report.X` in `cli_integration.rs:3739-3871`, R2-H-03 3 banned `is_ok()`
in `red_phase_validation.rs:164, 222, 332`, and F10 partial (3 wide-range exit codes remain in
`cli_vb_m214_bdd_scenarios.rs:427, 1045, 1101`). 1 NEW HIGH finding (R4-H-01): 2 vacuous
`matches!(parsed, Ok(_) | Err(_))` proptest gates at `mode_activation_tests.rs:927` and
`app_impl_tests.rs:1903` — same shape as round-3 R2-H-04 (`together_e2e_tests.rs` before fix).
3 NEW MEDIUM findings: R4-M-01 misleading docstring on `finish_contains_secret_data()` tests
(secret data test passes for slot 1, not for secret reason); R4-M-02 3 `let _ = workflow/first/result`
residual smoke patterns in `together_integration_tests.rs:472, 478, 516` (related to R2-H-04);
R4-M-03 6 `matches!(result, Ok(_) | Err(_))` style smokes in `validation_edge_case_tests.rs`.
2 NEW LOW: R4-L-01 2 `let _ = other;` residuals in `budget_analyzer.rs:190, 233` (R3-M-01
follow-up); R4-L-02 4 `outcome.is_ok()/is_err()` smoke in `red_queen_budget.rs:201, 221, 231,
242, 283` plus 4 at `:208` (R2-M-04 follow-up).

`cargo test -p vb_compile --tests` → **1074 passed, 2 ignored** (up from 1053 in round 3 —
21 new tests including the activated taint module).

`cargo test -p velvet-ballistics --tests` → **2 failures** (real production bugs caught by
newly-strict round-3 F10 assertions at `cli_vb_m214_bdd_scenarios.rs:369, 1229`:
`exit_code_two_on_verification_failure` and `verify_valid_workflow_exit_0_or_2`). These
failures are GOOD test outcomes — the strict `assert_eq!(output.status.code(), Some(2))`
catches a real production bug (verify returns exit 0 instead of 2). Not a test-quality defect.

Cannot be approved: 0 CRITICAL but 1 NEW HIGH + 3 NEW MEDIUM + 2 NEW LOW + 6 round-1/2/3
regressions still present.

---

## Round 1+2+3 Fix Verification (15 Sites)

| ID  | Round-1/2/3 Fix Target                                               | Status        | Evidence (current line)                                                                |
|-----|----------------------------------------------------------------------|---------------|----------------------------------------------------------------------------------------|
| F1  | `vb_cli/src/args/tests/{workflow,status,run,cancel,action,parse_*}.rs` — `if let Ok / else assert!(parsed.is_ok())` → `match { Ok(X) => ..., other => panic!("expected X, got {other:?}") }` | **STILL APPLIED** | `workflow.rs:7-18` `match { Ok(Command::Validate{..}) => { assert_eq!(...) } other => panic!(...) }`. Same shape in `status.rs`, `cancel.rs`, `action.rs`, `run.rs`. `journal.rs:152-158` uses `if let Ok(Command::Inspect{..}) / else panic!("expected Inspect command, got {parsed:?}")` (functionally equivalent — explicitly acceptable per round 2 R2-L-06). |
| F2  | `vb_compile/src/taint/tests/secret_finish_tests.rs` — 13 sites `matches!(result, Ok(_))` → `assert!(workflow.finish_contains_secret_data())` + parent module wired | **STILL APPLIED + module wired** | `secret_finish_tests.rs:41-47, 68-74, 93-99, 119-122, 144-146, 167-168, 190-192, 229-231, 397-399, 419-421, 481-484, 574-576, 598-600` — all 13 sites use `let workflow = compile_workflow(source).expect(...); assert!(workflow.finish_contains_secret_data(), ...);`. Parent `taint` module wired at `crates/vb_compile/src/lib.rs:42` (`#[cfg(test)] mod taint;`). `cargo test -p vb_compile --tests taint` → **46 passed**. **R3-C-01 RESOLVED**. |
| F3  | `vb_cli/src/args/tests/parse_misc2.rs:503` — `assert!(result.is_ok())` → `.expect()` + content check | **STILL APPLIED** | `parse_misc2.rs:503` `.expect("positional_str on 'one two' at last index must succeed"); assert_eq!(val, "two")`. |
| F4  | `vb_compile/src/mod_compile_lowering/together_*_tests.rs` — TDD-red `if let Ok(())` → hard `.expect()` | **STILL APPLIED** | `together_lowering_tests.rs:208, 257, 292, 328, 361, 392, 541, 577, 614, 655` use `let () = result.expect("Together lowering must succeed per spec");`. `together_integration_tests.rs:272, 361` use `.expect(...)`. `together_e2e_tests.rs:236` uses `.expect("Together lowering must succeed per spec")`. |
| F5  | `vb_compile/tests/proptest_save_canonical_name.rs` — local `canonical_name()` → production `canonical_primitive_name` | **STILL APPLIED** | `proptest_save_canonical_name.rs:15` `use vb_compile::mod_compile_lowering::canonical_primitive_name as canonical_name;` — direct production binding. |
| F6  | `vb_compile/src/tests/do_choose_digest_unit_tests.rs` — 18 sites `let _ = digest_step_primitive(...)` → `.expect()` | **STILL APPLIED** | `do_choose_digest_unit_tests.rs:179-180, 203, 207, 224, 228, 244, 248, 270-271, 302-303, 307-308, 330-331, 335-336, 359-360, 364-365, 387-388, 392-393, 414, 418` — 18 sites all use `.expect("digest must succeed for valid primitive")`. |
| F7  | `vb_compile/tests/digest_ask_explicit_arm.rs` — 11 sites `let _ = canonical_digest(...)` → capture + `assert_ne!(digest, [0u8; 32])` | **STILL APPLIED** | `digest_ask_explicit_arm.rs:144-147, 155-158, 167-170, 182-185, 193-196, 204-207, 215-218, 226-229, 261-264, 272-275, 283-286` — 11 sites all use captured `digest` variable plus `assert_ne!(digest.as_bytes(), [0u8; 32], "digest must be non-trivial")`. |
| F8  | `vb_cli/src/main_tests.rs` — 13 sites `assert!(journal/encoded/dir.is_ok())` → `.expect()` | **STILL APPLIED** | `main_tests.rs:62 .expect("slot value must encode")`, `:423 .expect("action 2 must resolve")`, `:508 .expect("test directory must be available")`, `:522 .expect("journal must reopen for valid dir")`, `:526 .expect("events for run must be readable")`, `:709 .expect("test directory must be available")`, `:713 .expect("journal must open")`, `:715 .expect("workflow parts must encode")`, `:732 .expect("resolver must load compiled IR")`, `:740 .expect("test directory must be available")`, `:742 .expect("journal must open")`, `:858 .expect("frame must build for valid step")`, `:956 .expect("test payload must encode for valid SlotValue vec")`. 13/13 sites. |
| F9  | `vb_cli/src/app_impl_tests.rs` — 13 sites `assert!(encoded/journal/dir.is_ok())` → `.expect()` | **STILL APPLIED** | `app_impl_tests.rs:68, 472, 571, 585, 589, 613, 617, 619, 636, 644, 646, 762, 864` — 13 sites all use `.expect("...")`. |
| F10 | `vb_cli/tests/cli_vb_m214_bdd_scenarios.rs` — wide-range exit code `Some(0) || Some(2)` → strict `assert_eq!(output.status.code(), Some(2))` | **MOSTLY APPLIED, 3 SITES STILL REGRESSED** | Most sites converted: `:212, 229, 245, 261, 277, 293, 316, 769` all use `assert_eq!(output.status.code(), Some(2));` Round-3 `:373, 579, 1226, 1266, 1309` patterns now strict. **HOWEVER**: `:427 code == Some(3) || code == Some(1)`, `:1045 code == Some(2) || code == Some(0)` (commented "Assertion relaxed to accept current behavior while gap is documented"), `:1101 code == Some(5) || code == Some(0)`. **The strict assertions at `:369` and `:1229` are now FAILING at runtime** because production `verify` returns exit 0 instead of 2 — this is a real production bug caught by the new strict assertion (NOT a test defect). |
| F11 | `vb_compile/src/proptest_together_errors.rs:262-264` — vacuous `matches!(result, Ok(()) \| Err(_))` → specific `StepFieldShape` variant match | **STILL APPLIED** | `proptest_together_errors.rs:262-275` now uses `matches!(&result, Err(CompileErrors(errors)) if errors.iter().any(|e| matches!(e, CompileError::StepFieldShape { field, expected, .. } if *field == "together.branches" && expected.contains("at least one branch"))))`. Specific error variant + field-value contract. |
| F12 | `vb_compile/tests/proptest/proptest_choose_depth.rs:62-66` — vacuous `matches!(inner, Ok(_) \| Err(_))` → `matches!(result, Ok(ref wf) if wf.node_count() >= 2)` | **STILL APPLIED** | `proptest_choose_depth.rs:93-99` uses `matches!(result, Ok(ref wf) if wf.node_count() >= 2) \|\| matches!(&result, Err(e) if e.0.iter().any(...))` — both arms bound by `catch_unwind` (line 48-50) for panic-freedom. |
| F13 | `vb_compile/tests/vb_xi2f_compile_source_proptest.rs:177` — `prop_assert!(result.is_ok())` smoke → expect + node_count check | **STILL APPLIED** | `vb_xi2f_compile_source_proptest.rs:178-186` uses `let compiled = result.ok().expect("YamlCompiler::compile on valid YAML must return Ok"); prop_assert!(compiled.node_count() >= 2, "...")`. Followed by `prop_assert!(compiled.node_count() >= 2, ...)` |
| F14 | `vb_compile/tests/proptest/proptest_choose_{otherwise,fallthrough,emission}.rs` — `prop_assert!(result.is_ok())` smoke → workflow content assertion | **STILL APPLIED** | `proptest_choose_otherwise.rs:50-65, 71-89` uses `let workflow = result.expect("..."); for i in 0..nc { if let Some(node) = workflow.node(...) && matches!(node.kind, CompiledNodeKind::ChooseSlot { otherwise: Some(_), .. }) }`. `proptest_choose_fallthrough.rs:64-75` and `proptest_choose_emission.rs:51-71` use similar node-content assertions. |
| F15 | `vb_compile/src/mod_compile_lowering/together_e2e_tests.rs:368,407,446,494` — `let _ = result;` smoke → `match { Ok(workflow) => assert!(workflow.node_count() >= 1), Err(_) => {} }` | **STILL APPLIED** | `together_e2e_tests.rs:366-376, 414-426, 463-474, 521-532` all use `match result { Ok(workflow) => assert!(workflow.node_count() >= 1, "..."), Err(_) => { /* acceptable */ } }`. The 4 R2-H-04 sites are FIXED. The `:253 let _ = workflow.digest();` is a different pattern (defensive non-panic check after a more substantive assertion). |
| R3-C-01 | `crates/vb_compile/src/lib.rs` — missing `mod taint;` declaration | **RESOLVED** | `crates/vb_compile/src/lib.rs:42` `#[cfg(test)] mod taint;`. R3-M-03 inverted assertions at `secret_finish_tests.rs:573, 589` (`!workflow.finish_contains_secret_data()`) also fixed. `cargo test -p vb_compile taint` → 46 passed. |

**Round-4 regression count: 6** (F10 partial [3 sites] + R2-H-01 [2 println! sites] + R2-H-03
[3 is_ok sites] + R2-H-06 [7 is_err sites] + R2-M-02 [7 is_ok sites] + R2-M-06 [12 let _ =
&report.X sites]).

**Round-4 NEW findings: 1 HIGH (R4-H-01) + 3 MEDIUM (R4-M-01, R4-M-02, R4-M-03) + 2 LOW
(R4-L-01, R4-L-02) + 1 OBSERVATION (R4-O-01)**.

---

## Findings (severity-ordered)

| ID | Sev | File:Line | Defect | Mutation thought experiment | Recommended fix | Disposition |
|----|-----|-----------|--------|------------------------------|------------------|--------------|
| R4-H-01 | HIGH | `crates/vb_cli/src/mode_activation_tests.rs:927` and `crates/vb_cli/src/app_impl_tests.rs:1903` | Vacuous `matches!(parsed, Ok(_) \| Err(_))` proptest gate. Test name claims "Property 1: Every valid command string is handled without panic" but the assertion is tautological — any `Result<_, _>` matches `Ok(_) \| Err(_)`. The actual panic-freedom must be enforced by proptest's outer wrapper (test fails on panic), but the explicit assertion adds nothing and masks whether the contract is "Ok" or "Err is acceptable". Same shape as round-3 R2-H-04 (`together_e2e_tests.rs:368, 407, 446, 494`) before the round-3 fix. | Delete `assert!(matches!(parsed, Ok(_) \| Err(_)))` — proptest's panic-detection is sufficient. Or replace with explicit contract per cmd_name (e.g. known commands with required args should produce `Err(ParseError::MissingArgument(_))`). | `blocker` |
| R4-M-01 | MEDIUM | `crates/vb_compile/src/taint/tests/secret_finish_tests.rs:393-399, 415-421` and `crates/vb_core/src/workflow/workflow.rs:160-171` | `finish_contains_secret_data()` is a weak proxy: returns `slot.get() > 0`. Tests `compile_accepts_unknown_reference_in_finish` (line 393-399) and `compile_accepts_non_dollar_reference_in_finish` (line 415-421) pass for non-secret reasons — the YAML source `result: 1` lands at slot 1 (index > 0) regardless of whether the data is secret. The assertion message `"Finish result must preserve secret data per Section 47"` is misleading: the test asserts a slot-index > 0 condition, not actual secret-data preservation. The `regression_compile_rejects_secret_finish_incorrectly` test at line 432-450 documents a Section 47 violation (compile returns `Err(UnsupportedTopLevelDeclaration)` for `result: 0` with secrets), but the "fix" is `compile` returning `Ok(workflow)` with `slot == 0` (which `finish_contains_secret_data()` would report as `false`). So the assertion never actually verifies secret-data preservation. | Replace the proxy with a real accessor that inspects the slot's taint bit (e.g. `workflow.finish_result_taint() == Some(Taint::Secret)`). Then add `prop_assert_eq!(taint, Some(Taint::Secret), ...)` to secret-preservation tests. Document `finish_contains_secret_data()` as "weak proxy; use finish_result_taint() for new code". | `owner_approved_debt` |
| R4-M-02 | MEDIUM | `crates/vb_compile/src/mod_compile_lowering/together_integration_tests.rs:472, 478, 516` (3 sites) | `let _ = workflow;` at line 472, `let _ = first;` at line 478 (where `let first = errs.iter().next().unwrap();` already panics on empty — `.unwrap()` is a banned panic pattern), `let _ = workflow;` at line 516. The tests `together_ir_passes_gate_11_validation` (line 443) and `together_ir_respects_budget_constraints` (line 488) use `match result { Ok(workflow) => { let _ = workflow; } Err(_) => {} }` — the contract is "Ok must produce a valid workflow" but the assertion discards the workflow entirely. Companion to round-3 R2-H-04 (`together_e2e_tests.rs`) but in `together_integration_tests.rs`. | Change `let _ = workflow;` to concrete assertions: `assert!(workflow.node_count() >= 2, "gate 11 must emit >= 2 nodes (TogetherStart + TogetherJoin)")`. Replace `.unwrap()` at line 476 with `let first = errs.first().expect("compile_workflow errors must be non-empty when Err")`. | `owner_approved_debt` |
| R4-M-03 | MEDIUM | `crates/vb_compile/src/tests/validation_edge_case_tests.rs:69-73, 80-84, 91-95, 102-106, 115-119, 128-132` (6 sites) | `assert!(matches!(result, Ok(_)))` at 5 sites and `assert!(matches!(result, Err(_)))` at 1 site. The contract is binary ("must accept u16::MAX-N" vs "must reject u16::MAX+1") so the variant-mismatch is less dangerous, but the assertions still accept any Ok variant for "accept" cases and any Err variant for "reject" cases. If `validate_branch_counts` started silently accepting malformed inputs at u16::MAX-100 (because of a refactor) and returning `Ok(())` without checking the value, all 5 sites would pass. | Add value-binding: `match result { Ok(()) => {}, Ok(other) => panic!("validate_branch_counts must return Ok(()) not Ok(other), got {:?}", other), Err(e) => panic!("... must not Err, got {:?}", e) }`. Or replace with `assert!(result == Ok(()), "validate_branch_counts must return Ok(()), got {:?}", result)`. | `owner_approved_debt` |
| R4-L-01 | LOW | `crates/vb_compile/src/budget_analyzer.rs:190, 233` | `let _ = other;` inside `Err(other) => { let _ = other; }` match arms. The contract is "must not panic" so the error variant is intentionally discarded. The match arms cover `Ok(_) \| Err(UnboundedWorkflow { .. })` as acceptable and `Err(other)` as defensive (any other compile error is acceptable because the contract is "no panic"). | Replace with `match result { Ok(_) \| Err(CompileError::UnboundedWorkflow { .. }) \| Err(CompileError::Workflow(_)) => {}, Err(other) => panic!("unexpected compile error variant for minimal workflow: {:?}", other) }` — pin the acceptable variant set explicitly. | `owner_approved_debt` |
| R4-L-02 | LOW | `crates/vb_compile/tests/red_queen_budget.rs:200-204, 207-210, 220-225, 230-235, 241-247, 282-288` | `assert!(outcome.is_ok(), ...)` at lines 200, 207, `assert!(outcome.is_err(), ...)` at lines 220, 230, 241, 282. Banned `is_ok()`/`is_err()` smoke patterns. The surrounding proptest generates specific boundary inputs (64-branch fanout, 65-branch, budget-overflow, etc.) and the contracts are "Ok at boundary, Err beyond boundary". A regression that returns `Ok(())` for any input would pass all 6 sites. | Replace with `match outcome { Ok(_) => { /* verify contract */ }, Err(e) => panic!("... must succeed, got {:?}", e) }` and extract specific CompileError variants. | `owner_approved_debt` |
| R4-O-01 | OBSERVATION | `crates/vb_cli/src/io.rs:281-323` | 7 banned `assert!(result.is_ok())` smoke tests. The functions `write_version_stdout`, `write_help_stdout`, `write_error_stderr` (4 variants), `write_stdout_line`, `write_stderr_line` are pure IO wrappers. The contract is "write_X must succeed for known-valid input" — but the assertions do not verify bytes were actually written (e.g. `let mut buf: Vec<u8> = Vec::new(); write_version_stdout_to(&mut buf).unwrap(); assert!(buf.starts_with(b"velvet-ballistics "));`). | Future test additions should capture bytes-written via a `to_writer(&mut Vec<u8>)` form and assert byte-level content. | `owner_approved_no_action` |

### Round-2/3 Regressions (still present)

| ID | Sev | File:Line | Defect | Mutation thought experiment | Recommended fix | Disposition |
|----|-----|-----------|--------|------------------------------|------------------|--------------|
| R2-H-01 | HIGH | `crates/vb_compile/src/tests/property_validation_tests.rs:14, 24` | TDD-red + `println!("PASS (validation exists): ...", e)` pattern at lines 14 and 24. Round 2/3 said remove. The `Err(e) => println!("PASS ...")` arm only checks `is_err()` (smoke) — a regression that returns `Err(CompileError::Other("nope"))` for every input passes the test. The `Ok(_) => panic!("GAP EXPOSED: ...")` arm is correct but the `Err` arm is not enforced. | Replace the `println!("PASS ...")` with `assert!(matches!(e.0.iter().next(), Some(CompileError::StepFieldShape { field, .. }) if field == "branches"))` for `together_empty_branches` and similar for `reduce_empty_body`. | `blocker` |
| R2-H-03 | HIGH | `crates/vb_validate/tests/red_phase_validation.rs:164, 222, 332` (3 sites) | `assert!(validate(&parts).is_ok(), ...)`, `assert!(pipeline.validate(&parts).is_ok(), ...)`, `assert!(result.is_ok(), ...)`. Banned `is_ok()` with weak message. The tests are documented as "RED PHASE" but the assertion is exactly the smoke pattern that the test-reviewer rubric prohibits. The surrounding `Err` cases (lines 169-209, 230-326) use proper `assert_eq!(result, Err(...))` with specific variants. | Replace each smoke with `assert_eq!(validate(&parts), Ok(()), "validate must return Ok(()) for valid parts, got {:?}", validate(&parts))` or document that this is a contract-smoke and convert to field-reachability checks. | `owner_approved_debt` |
| R2-H-06 | HIGH | `crates/vb_cli/tests/cli_integration.rs:1246, 1411, 1554, 1571, 1604, 1610, 1722` (7 sites) | `assert!(text.is_err(), "binary is not valid UTF-8")`, `assert!(result.is_err(), "bad version string should fail validation")`, etc. Banned `is_err()` without specifying the variant. Each site is a unit test for a specific error path, but the assertion accepts any Err variant. | Replace with `assert!(matches!(text, Err(std::str::Utf8Error { valid_up_to: 0, .. })))` for the UTF-8 case, and `assert!(matches!(result, Err(vb_validate::ValidationError::InvalidVersion { .. })))` for the version case. The `compile_rejects_non_utf8_input` test at lines 1578-1599 already has the correct pattern (`assert_eq!(err.first().map(...), Some("YAML source must be UTF-8: invalid utf-8 sequence of 1 bytes from index 0".to_string()))`) — copy that pattern. | `blocker` |
| R2-M-02 | MEDIUM | `crates/vb_cli/src/io.rs:281, 287, 294, 301, 308, 315, 322` (7 sites) | `assert!(result.is_ok())` — banned `is_ok()` smoke for `write_X_succeeds` tests. The functions under test are pure IO wrappers; a regression that returns `Ok(())` without writing bytes would pass. | Capture bytes-written: change `write_version_stdout()` to `write_version_stdout(&mut Vec::new())` or add a test-only `to_writer(&mut Vec<u8>)` API. Assert byte-level content. | `owner_approved_debt` |
| R2-M-06 | MEDIUM | `crates/vb_cli/tests/cli_integration.rs:3739, 3752, 3756, 3770, 3774, 3778, 3791, 3802, 3814, 3828, 3871` (12 sites) | `let _ = &report.repair_hints;` etc. inside `field_presence_test_helpers` match arms that return `true`. The match is "is this field present in the report" but the assertion is just `true` — there is no field-reachability check. The `let _ =` discards the value. Removing a field from the report leaves the test passing because the match arm returns `true` regardless. | Replace with `assert!(!report.repair_hints.is_empty(), "repair_hints field must be populated");` or `assert!(report.checks.len() > 0, "...");` per field. Document acceptable empty/non-empty cases. | `owner_approved_debt` |
| F10 partial | HIGH | `crates/vb_cli/tests/cli_vb_m214_bdd_scenarios.rs:427, 1045, 1101` (3 sites) | Wide-range `code == Some(3) || code == Some(1)` (line 427), `code == Some(2) || code == Some(0)` (line 1045, with "Assertion relaxed to accept current behavior" comment), `code == Some(5) || code == Some(0)` (line 1101). The other 8 round-1 F10 sites are fixed. Line 1045 + 1101 are explicitly commented as "relaxed" — they acknowledge a Section 47-style bug. | Pick single expected exit code per BDD scenario. For 1045: `assert_eq!(output.status.code(), Some(2), "absent run must exit 2")` (and fix production bug). For 1101: `assert_eq!(output.status.code(), Some(5), "absent run trace must exit 5")`. For 427: rename to `cli_compile_failure_returns_1_or_3` and document that exit code is conditional. | `owner_approved_debt` |

---

## Pattern Census (round 4 counts)

### `assert!(...is_ok()) / assert!(...is_err()) / matches!(..., Some(_) | Ok(_) | Err(_))` and bare `unwrap()`

| Crate | Total matches (round 4) | Notes |
|-------|--------------------------|-------|
| `vb_cli/src` | ~85 | `main_tests.rs` (0 — round-1 fix), `app_impl_tests.rs` (0 — round-1 fix, but 1 NEW vacuous `Ok(_) \| Err(_)` at `:1903`), `io.rs` (7 `is_ok()` REGRESSED), `args/tests/parse_misc2.rs` (0), `args/tests/{workflow,status,journal,cancel,action,parse_*}.rs` (112 `panic!` in test asserts — correct fix shape), `agent_context/tests/unit.rs` (3 `panic!` fixture only, 0 `is_ok`/`is_err`), `mode_activation_tests.rs` (1 NEW vacuous match at `:927`) |
| `vb_cli/tests` | ~120 | `cli_vb_m214_bdd_scenarios.rs` (8 strict `assert_eq!` + 3 wide-range REGRESSED at `:427, 1045, 1101` + 2 STRICT FAILURES at `:369, 1229` caught real production bugs), `cli_integration.rs` (7 `is_err()` REGRESSED + 12 `let _ = &report.X` REGRESSED + 148 `.unwrap()` fixture construction), `cli_trace_integration.rs` (15 `.unwrap()` after `is_some()` check — acceptable pattern), `lifecycle_integration.rs` (uses `matches!(&result, Ok(()))` which is the correct shape for `Result<(), _>`), `admission_evidence_integration/chunk_004.rs` (2 `is_err()` REGRESSED), `vb_qi37_14_1_run_step.rs` (TODO REGRESSED fixed — now specific shape), `cli_postcard/tests/envelopes.rs` (acceptable prop_assert_eq! bijection test) |
| `vb_compile/src` | ~12 | `taint/tests/secret_finish_tests.rs` (DEAD activated — all assertions now meaningful), `tests/error_variant_tests.rs` (2 smoke — round-3 R3-M-02), `tests/property_validation_tests.rs` (3 TDD-red + println! REGRESSED), `tests/integration_reduce_tests.rs` (1 println!), `tests/do_choose_digest_unit_tests.rs` (0 — round-1 fix), `tests/validation_edge_case_tests.rs` (6 NEW matches! Ok(_)/Err(_) — R4-M-03), `budget_analyzer.rs` (2 `let _ = other` residuals), `mod_compile_lowering/together_e2e_tests.rs` (1 `let _ = workflow.digest()` non-panic-only), `mod_compile_lowering/together_integration_tests.rs` (3 NEW `let _ = workflow/first` R4-M-02 + 4 `.unwrap()`), `proptest_choose_*.rs` (concrete assertions), `proptest_together_errors.rs` (specific error variant match), `property_tests/bytecode_ast_parity.rs` (excellent), `enums/{side_effect,retry_safety}_tests.rs` (variant-existence smoke) |
| `vb_compile/tests` | ~12 | `red_queen_budget.rs` (8 `is_ok()/is_err()` REGRESSED — R4-L-02), `proptest_choose_depth.rs` (1 vacuous match FIXED), `v1_primitive_lowering.rs` (2 `is_err()`), `vb_a001_for_each_topology.rs` (2 — 1 each), `vb_xi2f_nested_do_lowering.rs` (FIXED — now extracts specific variant), `vb_xi2f_compile_source_proptest.rs` (FIXED — now expects + node_count check) |
| `vb_proof_kernels/src` | ~5 | `envelope_header/tests.rs` (FIXED — now strict CRC round-trip check + stub acknowledgment at lines 144-178) |
| `vb_validate/src` | ~10 | `gates/tests.rs` (0 — excellent), `gate_07_stack/tests.rs` (0 — excellent), `gate_09_slots/tests.rs` (0), `gate_10_node/tests.rs` (0), `gate_13_cycles/tests.rs` (0), `red_phase_proptest.rs` (2 `is_ok()/is_err()` smoke R3-M-04 still applied), `property_tests/proptest_state_machine.rs` (1 `let _ = result` R3-M-05 + 5 `let _ = validate_*`), `property_tests/proptest_bound_enforcement.rs` (1 `let _ = validate_resource_limits`), `property_tests/proptest_constant_folding_validation.rs` (1 `let _ = validate_taint`), `type_taint/type_taint_tests.rs` (3 `let _ = validate_*` never-panic) |
| `vb_validate/tests` | ~3 | `red_phase_validation.rs` (3 `is_ok()` REGRESSED) |
| **TOTAL** | **~250** | (concentrated in `vb_cli/tests/cli_integration.rs` + `vb_cli/src/io.rs` + `vb_compile/tests/red_queen_budget.rs` + `vb_cli/tests/cli_vb_m214_bdd_scenarios.rs`) |

### `let _ = ...` (silent suppression, excluding kani/flux/verus files)

| Crate | Total matches (round 4) | Top files |
|-------|--------------------------|-----------|
| `vb_compile/src` | 20 | `budget_analyzer.rs` (2 R4-L-01), `enums/side_effect_tests.rs` (7 variant-existence), `enums/tests/retry_safety_tests.rs` (4 variant-existence), `mod_compile_lowering/together_e2e_tests.rs` (1 `:253 workflow.digest()` non-panic), `mod_compile_lowering/together_integration_tests.rs` (3 NEW R4-M-02 + 4 R3-M-04), `mod_compile_lowering/together_lowering_tests.rs` (3 — never_panic contract), `ast/parse/step.rs` (1 production), `tests/integration_reduce_tests.rs` (1) |
| `vb_compile/tests` | 2 | `vb_xi2f_nested_do_lowering.rs:361` (`let _ = action`), `idempotency_parity.rs:529` (comment) |
| `vb_compile/src/property_tests` | 5 | `bytecode_ast_parity.rs` (production-bound helpers) |
| `vb_cli/src` | ~60 | `commands_verify/pipeline.rs` (6 — production code), `commands_workflow/tests.rs` (2), `deliver_sink/atomic_publish.rs` (2 production), `deliver_sink/deliver_*_test_support.rs` (2 test support), `matrix/source_command_enum.rs` (1) |
| `vb_cli/tests` | 30 | `cli_integration.rs` (12 `let _ = &report.X` R2-M-06 + 1 `let _ = server_tx.send`), `cli_verify_integration.rs` (1), `lifecycle_integration.rs` (3 production-style), `deliver_sink_integration.rs` (1) |
| `vb_validate/src` | 7 | `type_taint/type_taint_tests.rs` (3 *never_panic*), `property_tests/proptest_bound_enforcement.rs` (1), `property_tests/proptest_state_machine.rs` (3 + 1 `let _ = result`), `property_tests/proptest_constant_folding_validation.rs` (1), `gate_tests.rs` (1) |
| `vb_proof_kernels/src` | 1 | `profile_contract/validation.rs` (production code) |
| **TOTAL** | **~125** | (down from 127 in round 3 — small reduction due to tighter filtering) |

### `#[ignore]` / `#[should_panic]` / `sleep(` / `todo!()` / `unimplemented!()`

| Crate | Total | Notes |
|-------|-------|-------|
| `vb_cli/src` | 1 | `doctor.rs:31` — `std::thread::sleep(...)` in production retry loop on `ProcessLockHeld`. |
| `vb_cli/tests` | 1 | `cli_integration.rs:5348` — `std::thread::sleep(10ms)` bounded by `Duration::from_secs(5)` deadline. |
| `vb_compile/tests/finish_digest_integration.rs:276` | 1 | `#[ignore = "BLOCKED: legacy canonical_digest is not accessible from integration test crate"]`. |
| **TOTAL** | 3 | (all acceptable — production retry, bounded busy-wait, documented blocker) |

### `lazy_static` / `OnceLock` / `static mut` / `thread_local!`

| Crate | Total | Notes |
|-------|-------|-------|
| `vb_cli/tests` | 3 | `deliver_sink_integration.rs:877-878`, `deliver_test_support.rs:64`, `deliver_debug_test_support.rs:35`. |
| `vb_validate/src` | 1 | `diag_render/fallback.rs:14` (production). |
| **TOTAL** | 4 | (all acceptable) |

### `panic!` and `println!` in test code

| Crate | Total matches |
|-------|---------------|
| `vb_cli/src` (incl. test submodules) | 44 `panic!` + 4 `println!` — `args/tests/*.rs` has 112 `panic!` which is the correct CLI args fix shape, not banned. `agent_context/tests/unit.rs` has 3 `panic!` calls (fixture construction). |
| `vb_compile/src` | 12 `panic!` + 3 `println!` — `tests/property_validation_tests.rs` has 3 `println!("PASS ...")` in the TDD-red pattern (R2-H-01 REGRESSED). |
| `vb_cli/tests` | 21 `println!` for diagnostics in BDD scenarios. |
| `vb_compile/tests` | 3 `println!` for GAP EXPOSED. |

### Wave-10 NEW test files (added since round 3)

| File | Lines | Quality |
|------|-------|---------|
| `vb_validate/src/gate_10_node/tests.rs` | 11.1K | **Excellent** — `assert_eq!(validate_gate_10_node(...), Ok(()))` and `assert!(matches!(..., Err(ValidationError::NodeFieldShape { ... })))`. |
| `vb_validate/src/gate_09_slots/tests.rs` | 12.7K | **Excellent** — same pattern. |
| `vb_validate/src/kani_gate_08_accessor.rs` | 290 | Kani-only (`#[cfg(kani)]`), out of scope. |
| `vb_compile/src/taint/mod.rs` + `taint/tests/secret_finish_tests.rs` | 593 (file), 7 (mod.rs) | **Now ACTIVE** — 46 tests pass with R3-C-01 fix applied; the 13 Section 47 contract tests now execute and the R3-M-03 inverted assertions are in place. |
| `vb_compile/src/tests/validation_edge_case_tests.rs` | 133 | **MEDIUM quality** — uses `matches!(result, Ok(_))` smoke (R4-M-03 NEW finding). |

---

## Mutation Gaps (top 5 most dangerous bugs the slice would NOT catch)

1. **`compute_whole_workflow_budget` returns all-zeros budget.** The round-1 fix to
   `budget_analyzer.rs` and `red_queen_budget.rs` removed 41 `let _ = budget.field;`
   statements and replaced them with concrete `assert_eq!` calls, BUT the 2 residual
   `let _ = other;` at `budget_analyzer.rs:190, 233` (R4-L-01) inside
   `Err(other) => { let _ = other; }` arms would still pass even if `max_for_each_iterations`
   were always 0 or always u64::MAX. The 4 `outcome.is_ok()/is_err()` smoke at
   `red_queen_budget.rs:201, 221, 231, 242, 283` (R4-L-02) plus the 4 at `:208` plus the
   wide-range exit code partial regression at `cli_vb_m214_bdd_scenarios.rs:1045, 1101`
   mean the budget overflow path is unenforced in 9 distinct test sites. **File:Line:**
   production `crates/vb_compile/src/budget_analyzer.rs:35-52` and
   `vb_core::budget::WholeWorkflowBudget`.

2. **`finish_contains_secret_data()` is a slot-index proxy, not a secret-data check.**
   R4-M-01: the implementation `finish_contains_secret_data = slot.get() > 0` is a weak
   proxy that returns `true` for any slot index > 0, including `result: 1` (literal
   integer), `result: ${clean_input}` (where the binding resolves to slot 1+), and
   legitimate secret references. The 13 Section 47 contract tests in
   `taint/tests/secret_finish_tests.rs` pass for non-secret reasons. A regression that
   strips `$secrets.token` references during lowering (routing them to slot 0) would NOT
   be caught because slot 0 makes `finish_contains_secret_data() == false` for ALL of
   them — but the tests would still pass if production kept the existing slot routing.
   The actual Section 47 contract needs a real `finish_contains_secret_data()` that
   inspects the slot's taint bit. **File:Line:** production
   `crates/vb_core/src/workflow/workflow.rs:166-171`.

3. **CLI `verify` exits 0 when db is missing.** Round-3 F10 fix at
   `cli_vb_m214_bdd_scenarios.rs:369, 1229` now uses strict `assert_eq!(output.status.code(),
   Some(2))` — and the tests **FAIL at runtime** because production returns `Some(0)`.
   The tests correctly caught a real production bug. The bug: the `verify` command
   silently succeeds (exit 0) when the `--db` flag is missing instead of returning
   `VerificationFailed` (exit 2). The 3 wide-range exit code residuals at `:427, 1045,
   1101` mask additional CLI exit code bugs (compile accepts exit 1 or 3, inspect
   accepts exit 0 or 2, trace accepts exit 0 or 5). **File:Line:** production
   `crates/vb_cli/src/main.rs` (verify command dispatch) and
   `crates/vb_cli/src/args/mod.rs` (verify command parser).

4. **`vb_validate::validate` returns `Ok(())` for malformed parts.** The 3 banned
   `is_ok()` smoke patterns in `red_phase_validation.rs:164, 222, 332` (R2-H-03) plus the
   2 smoke patterns in `red_phase_proptest.rs:81, 165` (R3-M-04) mean that if `validate`
   was changed to return `Ok(())` for every input (e.g. by skipping gate 8 checks), all
   5 sites would pass. The surrounding `Err` cases (lines 169-209, 230-326) properly
   extract specific `ValidationError` variants, but the `Ok` cases are smokes. **File:Line:**
   production `crates/vb_validate/src/lib.rs` and `crates/vb_validate/src/gate_08_accessor.rs`.

5. **Property tests for choose, depth, emission silently break.** Round-1 F14 fix at
   `proptest_choose_*.rs` replaced `prop_assert!(result.is_ok())` with workflow content
   assertions (now correct), BUT `proptest_choose_depth.rs:52` retains
   `prop_assert!(result.is_ok(), "compile_workflow must never panic; catch_unwind
   returned Ok")` for the panic-freedom property. The contract is "catch_unwind returns
   Ok" (no panic), not "compile_workflow returns Ok". A regression that always panics
   inside `compile_workflow` would still pass this assertion if `catch_unwind` was
   removed (because the proptest panic would be caught by the test runner). The
   outer-proptest panic detection is the real contract — the inner assertion is
   redundant. The `proptest_compile_accepts_clean_finish` and
   `proptest_compile_accepts_literal_finish` tests at `secret_finish_tests.rs:557-577,
   582-592` work correctly (extracted via `.expect()` + `!finish_contains_secret_data()`).
   **File:Line:** production `crates/vb_compile/src/compile.rs`.

---

## Top 5 Fixes (impact-per-effort)

### Fix 1 — Replace 2 vacuous `matches!(parsed, Ok(_) | Err(_))` proptests with concrete variant checks (R4-H-01) — 10 min
**Impact:** Removes the only NEW HIGH finding. Both proptests now verify actual contract instead of tautology.

```rust
// BEFORE (mode_activation_tests.rs:925-927):
// Property 1: Every valid command string is handled without panic
// Some commands need additional args, so Err is acceptable
let parsed = crate::args::parse_args(&args(&["velvet-ballistics", cmd_name]));
assert!(matches!(parsed, Ok(_) | Err(_)));
// AFTER:
// Known commands that require additional args should fail with MissingArgument
// when invoked bare. The contract is "must return a typed Result" — proptest's
// panic-detection handles non-panic via the outer wrapper.
let parsed = crate::args::parse_args(&args(&["velvet-ballistics", cmd_name]));
match &parsed {
    Ok(Command::Run { .. }) => panic!("bare {} must require workflow arg", cmd_name),
    Ok(Command::Validate { .. }) => panic!("bare validate must require workflow arg"),
    Ok(_) => { /* commands with no required args are Ok */ }
    Err(ParseError::MissingArgument(_)) => { /* expected */ }
    Err(other) => panic!("{} must produce MissingArgument or Ok, got {:?}", cmd_name, other),
}
```

### Fix 2 — Replace `is_ok()/is_err()` smoke patterns in `red_phase_validation.rs` (R2-H-03) + `red_phase_proptest.rs` (R3-M-04) with concrete variant assertions — 30 min
**Impact:** 5 banned smoke patterns become real contract tests. Section 47-style secret-preservation contract is now enforced.

```rust
// BEFORE (red_phase_validation.rs:163-166):
assert!(
    validate(&parts).is_ok(),
    "expected Ok for valid accessor symbols"
);
// AFTER:
assert_eq!(
    validate(&parts),
    Ok(()),
    "validate must return Ok(()) for valid accessor symbols (gate 8 only)"
);
```

### Fix 3 — Replace 7 banned `is_err()` smoke patterns in `cli_integration.rs` (R2-H-06) with specific variant matches — 1 hour
**Impact:** 7 unit tests become real error-variant contract tests. Pattern already established at `cli_integration.rs:1578-1599` (`compile_rejects_non_utf8_input` uses strict `assert_eq!(err.first().map(...), Some("YAML source must be UTF-8: invalid utf-8 sequence of 1 bytes from index 0".to_string()))`).

```rust
// BEFORE (cli_integration.rs:1411):
assert!(result.is_err(), "bad version string should fail validation");
// AFTER:
assert!(matches!(
    result,
    Err(vb_validate::ValidationError::InvalidVersion { version }) if version == "v999"
), "expected InvalidVersion{{version: \"v999\"}}, got {result:?}");
```

### Fix 4 — Convert 3 TDD-red + `println!("PASS ...")` to specific error variant matches in `property_validation_tests.rs` (R2-H-01) — 30 min
**Impact:** Section 38 "Together empty branches", "Reduce empty body", "Duplicate branch labels" rows enforced.

```rust
// BEFORE (property_validation_tests.rs:11-16):
match result {
    Ok(_) => panic!("GAP EXPOSED: Together with 0 branches compiled successfully. Validation missing."),
    Err(e) => println!("PASS (validation exists): Empty Together rejected: {:?}", e),
}
// AFTER:
let errors = result.expect_err("Together with 0 branches must fail at compile layer");
assert!(
    errors.0.iter().any(|e| matches!(e, CompileError::StepFieldShape { field, .. } if field == "branches")),
    "Together with 0 branches must produce StepFieldShape{{field: \"branches\"}}, got {:?}", errors.0
);
```

### Fix 5 — Convert `finish_contains_secret_data()` proxy to a real taint-bit accessor (R4-M-01) — 2 hours
**Impact:** Section 47 secret-preservation contract is now actually enforced. 13 taint tests become meaningful instead of slot-index smokes. Plus 3 wave-9 proptests in `proptest_finish_digest.rs`.

```rust
// BEFORE (vb_core/src/workflow/workflow.rs:160-171):
#[must_use]
pub fn finish_contains_secret_data(&self) -> bool {
    match self.finish_result_slot() {
        Some(slot) => slot.get() > 0,
        None => false,
    }
}
// AFTER:
#[must_use]
pub fn finish_contains_secret_data(&self) -> bool {
    // Inspects the slot's taint bit, not the slot index. A `result: ${secret}`
    // reference resolves to a slot with Taint::Secret; a `result: 0` or
    // `result: ${clean_input}` resolves to a slot with Taint::Clean.
    self.finish_result_slot_taint() == Some(Taint::Secret)
}
```

---

## Disposition

| ID | Disposition | Rationale |
|----|-------------|-----------|
| R4-H-01 | `blocker` | 2 NEW vacuous proptest gates in `mode_activation_tests.rs:927` and `app_impl_tests.rs:1903` — same shape as round-3 R2-H-04. |
| R4-M-01 | `owner_approved_debt` | Weak proxy in production `finish_contains_secret_data()` — needs taint-bit accessor. Tests pass for slot-index reasons. |
| R4-M-02 | `owner_approved_debt` | 3 `let _ = workflow/first/result` in `together_integration_tests.rs` + 1 `.unwrap()` at `:476`. |
| R4-M-03 | `owner_approved_debt` | 6 `matches!(result, Ok(_) | Err(_))` in `validation_edge_case_tests.rs`. Contract is binary so blast radius is smaller. |
| R4-L-01 | `owner_approved_debt` | 2 `let _ = other;` in `budget_analyzer.rs:190, 233` — defensive "must not panic" match arms. |
| R4-L-02 | `owner_approved_debt` | 8 `outcome.is_ok()/is_err()` smoke in `red_queen_budget.rs`. |
| R4-O-01 | `owner_approved_no_action` | 7 banned `is_ok()` smoke in `vb_cli/src/io.rs` — observation only. |
| R2-H-01 | `blocker` | 3 TDD-red + `println!("PASS ...")` in `property_validation_tests.rs:13-15, 23-25, 47-49`. |
| R2-H-03 | `blocker` | 3 banned `is_ok()` in `red_phase_validation.rs:164, 222, 332`. |
| R2-H-06 | `blocker` | 7 banned `is_err()` in `cli_integration.rs:1246, 1411, 1554, 1571, 1604, 1610, 1722`. |
| R2-M-02 | `owner_approved_debt` | 7 banned `is_ok()` in `vb_cli/src/io.rs`. |
| R2-M-06 | `owner_approved_debt` | 12 `let _ = &report.X` in `cli_integration.rs`. |
| F10 partial | `owner_approved_debt` | 3 wide-range exit code residuals at `cli_vb_m214_bdd_scenarios.rs:427, 1045, 1101`. The strict assertions at `:369, 1229` are now FAILING at runtime (real production bug caught by tests — GOOD). |
| R3-C-01 | RESOLVED | `taint` module wired at `lib.rs:42`; 46 taint tests pass; R3-M-03 inverted assertions fixed. |
| F1, F3, F4, F5, F6, F7, F8, F9, F11, F12, F13, F14, F15 | STILL APPLIED | All round-1 fix targets remain fixed. |

---

## Verdict

```
STATUS: REJECTED
```

**0 NEW CRITICAL + 1 NEW HIGH (R4-H-01) + 3 NEW MEDIUM (R4-M-01, R4-M-02, R4-M-03) + 2 NEW
LOW (R4-L-01, R4-L-02) + 1 NEW OBSERVATION (R4-O-01)** are introduced by the wave-10
expansion. **6 round-1/2/3 regressions STILL APPLIED**: R2-H-01 (println! TDD-red),
R2-H-03 (3 banned is_ok in red_phase_validation.rs), R2-H-06 (7 banned is_err in
cli_integration.rs), R2-M-02 (7 banned is_ok in vb_cli/src/io.rs), R2-M-06 (12 let _ =
&report.X in cli_integration.rs), and F10 partial (3 wide-range exit codes). The
round-3 R3-C-01 (dead-code taint module) is RESOLVED — 46 taint tests now execute and
the R3-M-03 inverted assertions are in place.

Wave-10 added ~12,000 lines of high-quality property tests in
`vb_validate/src/gate_10_node/tests.rs` (11.1K) and `vb_validate/src/gate_09_slots/tests.rs`
(12.7K), all using the strongest pattern observed: `assert_eq!(validate_gate_XX(&parts),
Ok(()))` plus specific `Err(ValidationError::XX { .. })` matches. Plus the activated
`taint/mod.rs` (46 tests, all passing). The wave-10 expansion did NOT backfill the
round-1/2/3 regressions in `vb_cli/src/io.rs`, `cli_integration.rs`, `red_phase_validation.rs`,
`cli_vb_m214_bdd_scenarios.rs`, `property_validation_tests.rs`, nor eliminate the
`finish_contains_secret_data()` proxy weakness.

Two `vb_cli` integration tests are now FAILING at runtime — `exit_code_two_on_verification_failure`
and `verify_valid_workflow_exit_0_or_2` in `cli_vb_m214_bdd_scenarios.rs:369, 1229` —
because the round-3 F10 fix made them strict and the production `verify` command returns
exit 0 instead of 2 when `--db` is missing. These failures are GOOD test outcomes (real
production bug caught); not a test defect. Recommend opening a follow-up bead for the
production bug fix.

`cargo test -p vb_compile --tests` → **1074 passed, 2 ignored** (up from 1053 in round 3 —
21 new tests including the activated taint module).

`cargo test -p velvet-ballistics --tests` → **2 failures** (production bug, not test defect).

Recommend: (1) Replace 2 vacuous `matches!(parsed, Ok(_) | Err(_))` proptests with concrete
variant checks (Fix 1, ~10 min, R4-H-01). (2) Replace 3 banned `is_ok()` in
`red_phase_validation.rs` + 2 in `red_phase_proptest.rs` with strict `assert_eq!` + field
reachability (Fix 2, ~30 min, R2-H-03 + R3-M-04). (3) Replace 7 banned `is_err()` in
`cli_integration.rs` with specific `ValidationError` variant matches (Fix 3, ~1 hr,
R2-H-06). (4) Convert `property_validation_tests.rs` TDD-red + `println!("PASS ...")` to
specific error variant matches (Fix 4, ~30 min, R2-H-01). (5) Replace `finish_contains_secret_data()`
slot-index proxy with real taint-bit accessor (Fix 5, ~2 hr, R4-M-01). (6) File a separate
bead for the production `verify` exit-code bug (currently caught by `cli_vb_m214_bdd_scenarios.rs:369, 1229`).
