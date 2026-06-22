# Test Review — Slice 3: vb_compile, vb_cli, vb_validate, vb_proof_kernels (Round 2)

**Scope:** 632 Rust files across 4 crates (`vb_compile`, `vb_cli` aka `velvet-ballistics`,
`vb_validate`, `vb_proof_kernels`).

**Date:** 2026-06-21
**Reviewer:** test-reviewer agent (round 2 of 40)

## STATUS: REJECTED

Round 1 fixed 4 of 12 CRITICAL pattern classes and 8 of them REGRESSED. The most
damaging survivors are: `assert!(result.is_ok() / is_err())` smoke assertions in
`vb_cli/src/main_tests.rs` (13 instances, H-09), the wide-range
`code == Some(0) || code == Some(2)` exit-code guard in `cli_vb_m214_bdd_scenarios.rs:373`
(H-01), the TDD-red `Ok(_) => panic!("GAP EXPOSED...") / Err(_) => println!("PASS")`
pattern in `vb_compile/src/tests/property_validation_tests.rs:13-15,23-25,47-49` (M-02),
the `let _ = result;` survivors in `mod_compile_lowering/together_e2e_tests.rs:368,407,446,494`
(M-01), the `is_ok()` proptest gates in
`proptest_choose_{otherwise,fallthrough,depth,emission}.rs` and
`vb_xi2f_compile_source_proptest.rs:177` (M-05/M-06, H-04), the
`matches!(result, Ok(_) | Err(_))` vacuous proptests in
`proptest_together_errors.rs:262` (H-02) and `proptest_choose_depth.rs:63` (H-03),
the `let _ = budget.max_for_each_iterations;` residual in `budget_analyzer.rs:216`
(C-07 partial), and the wave-7-introduced 3 smoke assertions in
`vb_proof_kernels/src/envelope_header/tests.rs` (lines 5-7, 144-147, 150-153) where
`compute_header_crc_returns_zero` and `validate_header_crc_always_true` look like
deliberate stubs that would pass if the CRC function were broken. The slice has
**3 CRITICAL**, **7 HIGH**, **6 MEDIUM**, **6 LOW**, **4 OBSERVATION** findings.
Cannot be approved.

---

## Round 1 Fix Verification (12 Round-1 Sites)

| ID  | Round-1 Fix Target                                               | Status        | Evidence (current line)                                                                |
|-----|------------------------------------------------------------------|---------------|----------------------------------------------------------------------------------------|
| F1  | `vb_cli/src/args/tests/{workflow,status,run,cancel,action,parse_*}.rs` (68 sites) — `if let Ok / else assert!(parsed.is_ok())` | **STILL APPLIED** | `workflow.rs:7-17` uses `match { Ok(Command::Validate{..}) => { /* real */ } other => panic!("expected Command::Validate, got {other:?}") }`. Same shape in `status.rs:5-17`, `journal.rs:152-158` (uses `panic!` else-branch). |
| F2  | `vb_compile/src/budget_analyzer.rs` + `tests/red_queen_budget.rs` (41 sites) — `let _ = budget.field;` → concrete `assert_eq!` | **PARTIALLY APPLIED** | `budget_analyzer.rs:118-172` and `red_queen_budget.rs:444-499` use `assert_eq!(budget.max_steps_executable, 2)` etc. **Regression:** `budget_analyzer.rs:216` still has `let _ = budget.max_for_each_iterations;` and `:224` has `let _ = other;` inside the `analyzer_handles_bounded_for_each_workflow_without_panicking` match. |
| F3  | `vb_compile/src/taint/tests/secret_finish_tests.rs` (13 sites) — `matches!(result, Ok(_))` → `assert!(workflow.finish_contains_secret_data())` | **STILL APPLIED** | `secret_finish_tests.rs:41-47, 68-74, 93-99, ...` all use `let workflow = compile_workflow(source).expect(...); assert!(workflow.finish_contains_secret_data(), ...);`. |
| F4  | `vb_compile/src/mod_compile_lowering/together_*_tests.rs` (15+ sites) — TDD-red `if let Ok(())` → hard `.expect()` | **STILL APPLIED** | `together_lowering_tests.rs:208, 257, 330, 364` use `let () = result.expect("Together lowering must succeed per spec");`. **Stale docstring/comments remain at lines 122, 231, 344, 515, 519** documenting the round-1 deletion — those are now comments only, not tests. |
| F5  | `vb_compile/tests/proptest_save_canonical_name.rs` (256 iterations) — local copy → production `canonical_primitive_name` | **STILL APPLIED** | `proptest_save_canonical_name.rs:15` is `use vb_compile::mod_compile_lowering::canonical_primitive_name as canonical_name;` — direct production binding. |
| F6  | `vb_compile/src/tests/do_choose_digest_unit_tests.rs` (18 sites) — `let _ = digest_step_primitive(...)` → `.expect("digest must succeed for valid primitive")` | **STILL APPLIED** | `do_choose_digest_unit_tests.rs:179, 202, 206, 223, 227, 243, 247, 269, 300, 304, 326, 330, 353, 357, 379, 383, 404, 408` all use `.expect("digest must succeed for valid primitive")`. |
| F7  | `vb_compile/tests/digest_ask_explicit_arm.rs` (11 sites) — `let _ = canonical_digest(...)` → capture + `assert_ne!(digest.as_bytes(), [0u8; 32])` | **STILL APPLIED** | `digest_ask_explicit_arm.rs:144-145, 151-152, 159-160, 170-171, 177-178, 184-185, 191-192, 198-199, 229-230, 236-237, 243-244` all use captured `digest` variable plus `assert_ne!(digest.as_bytes(), [0u8; 32], "digest must be non-trivial (not all zeros)")`. |
| F8  | `vb_cli/tests/cli_vb_m214_bdd_scenarios.rs` (~25 sites) — wide-range exit code assertion | **REGRESSED** | `cli_vb_m214_bdd_scenarios.rs:373` still has `assert!(code == Some(0) || code == Some(2), "verify should exit 0 (passed) or 2 (verification failure), got: {code:?}")`. Same pattern at `:579`. |
| F9  | `vb_compile/src/proptest_together_errors.rs:262-264` — vacuous `matches!(result, Ok(()) \| Err(_))` | **REGRESSED** | `proptest_together_errors.rs:262` still has `prop_assert!(matches!(result, Ok(()) \| Err(_)), "zero-branch together must return a Result without panic")`. |
| F10 | `vb_compile/tests/proptest/proptest_choose_depth.rs:62-66` — vacuous `matches!(inner, Ok(_) \| Err(_))` | **REGRESSED** | `proptest_choose_depth.rs:62-66` still has `prop_assert!(matches!(inner, Ok(_) \| Err(_)), "varied_choose_yaml compiles or errors gracefully (never panics), got {:?}", inner)`. |
| F11 | `vb_compile/tests/vb_xi2f_compile_source_proptest.rs:176-180` — `prop_assert!(result.is_ok())` smoke | **REGRESSED** | `vb_xi2f_compile_source_proptest.rs:177` still has `prop_assert!(result.is_ok(), "YamlCompiler::compile on valid YAML must return Ok, got {:?}", result)`. |
| F12 | `vb_compile/tests/proptest/proptest_choose_otherwise.rs:50-55,61-66` and `proptest_choose_fallthrough.rs:48-52` — `is_ok()` smoke | **REGRESSED** | `proptest_choose_otherwise.rs:51, 62`, `proptest_choose_fallthrough.rs:49`, `proptest_choose_emission.rs:65` all still have `prop_assert!(result.is_ok(), ...)`. |

**Round-1 regression count: 8 (F8, F9, F10, F11, F12, plus H-09, H-12, M-01, M-02, H-04, H-05, H-08, H-11, M-05, M-06 listed below).**

The CLI-args `if let Ok(Command::X) / else panic!` patterns in `journal.rs` (19
sites, lines 157, 176, 194, 227, 245, 263, 298, 319, ...) appear identical to the
banned C-01..C-06 shape but are actually the **correct fix** — they panic on any
`Err(...)` or any `Ok(Command::OtherVariant)`, so they catch misrouted variants.
**Not a regression.** They are equivalent to the `match { Ok(X) => ..., other =>
panic!() }` shape in workflow.rs/status.rs and are acceptable.

---

## Findings (severity-ordered)

| ID | Sev | File:Line | Defect | Mutation thought experiment | Recommended fix | Disposition |
|----|-----|-----------|--------|------------------------------|------------------|--------------|
| R2-C-01 | CRITICAL | `vb_cli/src/main_tests.rs:62,425,515,529,533,718,722,728,747,758,761,881,988` (13 sites) | `assert!(journal.is_ok(), "journal should reopen")`, `assert!(encoded.is_ok(), "slot value should encode: {encoded:?}")`, `assert!(dir.is_ok(), "test directory should be available: {dir:?}")` — banned `is_ok()` in 13 places. All are fixture-construction checks (open journal, encode slot value, get temp dir). | If `Journal::open` returns `Ok(empty_journal)` (a journal struct missing required state) instead of failing, all 13 sites pass. Test fixture contract is undocumented. | Replace with `.expect("journal must open: {err:?}")` so the message carries the actual error, or assert `result.is_ok_and(\|j\| j.name() == expected_name)`. | `blocker` |
| R2-C-02 | CRITICAL | `vb_cli/src/app_impl_tests.rs:68,474,578,592,596,622,626,632,651,662,665,785,899` (13 sites) | Same `assert!(encoded.is_ok(), ...)`, `assert!(journal.is_ok(), ...)`, `assert!(dir.is_ok(), ...)`, `assert!(events.is_ok(), ...)`, `assert!(ir.is_ok(), ...)`, `assert!(resolved.is_ok(), ...)`, `assert!(frame.is_ok(), ...)`. Banned `is_ok()` in 13 places. | If `frame.is_ok()` is replaced with `Ok(dummy_frame)` instead of the real `RunFrame`, the test passes. The pattern offers no signal about which fields are valid. | Replace with `.expect(...)` carrying error context, or assert on the returned frame's fields (e.g. `assert_eq!(frame.run_id, RunId::new(42))`). | `blocker` |
| R2-C-03 | CRITICAL | `vb_cli/src/args/tests/parse_misc2.rs:503` | `assert!(result.is_ok());` — bare banned `is_ok()` with no message, no variant. | Any `Ok(Command::X)` passes. Parser could route every flag to `Command::Run` and the test passes. | Replace with `match parsed { Ok(Command::Expected{..}) => {...}, other => panic!("expected Command::Expected, got {other:?}") }`. | `blocker` |
| R2-H-01 | HIGH | `vb_compile/src/tests/property_validation_tests.rs:13-15, 23-25, 47-49` | TDD-red + `println!` pattern: `Ok(_) => panic!("GAP EXPOSED: ..."), Err(e) => println!("PASS (validation exists): ...")`. The proptest is masquerading as a unit test (`#[test] fn together_empty_branches` at line 9 — no `proptest!` block). The Ok arm panics (good) but the Err arm only checks `is_err()` (smoke). | Change `compile_workflow` to always return `Err(CompileErrors(vec![CompileError::Other("nope")]))`. Test passes because the Err arm only checks `is_err()`. Section 38 row "Together empty branches" is not enforced. | Replace with `assert!(matches!(result, Err(e) if e.0.iter().any(\|c\| matches!(c, CompileError::StepFieldShape{field,..} if field == "branches"))))`. Remove `println!`. | `blocker` |
| R2-H-02 | HIGH | `vb_compile/tests/proptest/proptest_choose_otherwise.rs:50-55,61-66`, `proptest_choose_fallthrough.rs:48-52`, `proptest_choose_depth.rs:52`, `proptest_choose_emission.rs:65` (5 sites across 4 files) | `prop_assert!(result.is_ok(), "...")` — banned `is_ok()` smoke. The "Otherwise present" proptest (line 45-55) asserts only `is_ok()` and nothing about whether the `otherwise` field is reachable. | Change `compile_workflow` to return `Ok(empty_workflow)` for every input. All 5 proptests pass. Section 38 row "Choose otherwise" is silently broken. | Assert on the workflow's `node_count() >= 2` and that the second branch is the `otherwise` slot via `for i in 0..nc { let node = workflow.node(StepIdx::new(i)); if let Some(ChooseSlot::Otherwise) = node.choose_slot { ... } }`. | `blocker` |
| R2-H-03 | HIGH | `vb_validate/tests/red_phase_validation.rs:163-166, 220-224, 331-334` (3 sites) | `assert!(validate(&parts).is_ok(), "...")`, `assert!(pipeline.validate(&parts).is_ok(), ...)`, `assert!(result.is_ok(), ...)`. Banned `is_ok()` with weak message. | Change `validate` to return `Ok(())` for every `WorkflowParts`. All 3 sites pass. The Gate 7/8/9/10/11/13/14/15 pipeline correctness is silently broken. | Replace with `assert!(matches!(validate(&parts), Ok(())))` and add `assert_eq!(validate(&parts), Ok(()))` plus `let _ = parts.accessors.len();` style field-reachability checks where the gate behaviour is contract-critical. | `blocker` |
| R2-H-04 | HIGH | `vb_compile/src/mod_compile_lowering/together_e2e_tests.rs:368, 407, 446, 494` (4 sites) | `let _ = result;` discards compile result entirely. Round-1 C-09 fix removed the TDD-red mask but did NOT add any assertion on `result`. The 4 tests `e2e_together_with_nested_branches`, `e2e_together_with_reduce_branch`, `e2e_together_with_repeat_branch`, `e2e_together_with_foreach_in_branches` (round-1 M-01) are still smoke-only. | Delete the entire `emit_single_body_set` Together branch. All 4 tests still pass. The Together multi-control-flow contract is unenforced. | Replace `let _ = result;` with `let workflow = compile_yaml(yaml).expect("together with X must compile"); assert!(workflow.node_count() >= 6, "...");`. | `blocker` |
| R2-H-05 | HIGH | `vb_proof_kernels/src/envelope_header/tests.rs:5-7, 144-147, 150-153` (3 sites — NEW in wave-6) | (a) Line 5-7: `assert!(header.validate_magic());` — smoke; doesn't check magic value. (b) Line 144-147: `test_compute_header_crc_returns_zero` — `assert_eq!(compute_header_crc(&header), 0)` looks like a stub assertion; a real CRC should not return 0 for a non-trivial header. (c) Line 150-153: `test_validate_header_crc_always_true` — `assert!(validate_header_crc(&header))` is tautological; could mask a stub implementation that always returns `true`. | Replace `compute_header_crc` with `fn compute_header_crc(_h: &EnvelopeHeader) -> u32 { 0 }`. Test passes. Replace `validate_header_crc` with `fn validate_header_crc(_h: &EnvelopeHeader) -> bool { true }`. Test passes. The CRC contract is unenforced. | (a) Replace with `assert!(header.validate_magic(), "MAGIC_VALUE mismatch")`. (b) Compute CRC over a header with a known field set (e.g. magic=0xCAFEBABE, version=1, payload_len=42) and assert a specific non-zero CRC value. (c) Make `validate_header_crc` round-trip: assert `compute_header_crc(h)` and `validate_header_crc(h)` agree on a modified header that should fail. | `blocker` |
| R2-H-06 | HIGH | `vb_cli/tests/cli_integration.rs:1246, 1411, 1554, 1571, 1604, 1610, 1722` (7 sites — round-1 L-01 not fixed) | `assert!(text.is_err(), "binary is not valid UTF-8")`, `assert!(result.is_err(), "bad version string should fail validation")`, etc. — banned `is_err()` without specifying the variant. | Change `validate` to return `Err(Other)` instead of `Err(BadVersionString)`. Test passes because only `is_err()` is checked. | Replace with `assert!(matches!(result, Err(vb_validate::ValidationError::InvalidVersion{version: "v999"})))`. | `blocker` |
| R2-H-07 | HIGH | `vb_cli/tests/vb_qi37_14_1_run_step.rs:1316-1323` — round-1 H-11 not fixed | `let has_output = json.get("output_slot").is_some() || (json.get("deltas").is_some() && json.get("deltas").unwrap().get("slot_deltas").is_some());` plus `// TODO: assert exact structure` comment block. The test accepts either of two contract shapes — the contract is "produce some output field" rather than "produce output_slot". | Remove both `output_slot` and `slot_deltas` from the JSON. Test fails because neither is present. Add a third shape (e.g. `output`) — test still passes. | Pick one contract shape and assert exactly: `assert!(json.get("output_slot").is_some(), "Finished signal must include output_slot per spec"); let output_slot = json.get("output_slot").unwrap(); assert!(output_slot.get("value").is_some()); assert!(output_slot.get("taint").is_some());`. | `owner_approved_debt` |
| R2-M-01 | MEDIUM | `vb_compile/src/budget_analyzer.rs:216, 224` | `let _ = budget.max_for_each_iterations;` and `let _ = other;` in `analyzer_handles_bounded_for_each_workflow_without_panicking`. The round-1 fix removed 25 `let _ = budget.field;` patterns but missed these 2. | If `WholeWorkflowBudget::compute` returns a budget where `max_for_each_iterations` is u64::MAX (overflow sentinel), the test passes because the value is discarded. | Replace with `assert!(budget.max_for_each_iterations >= 1, "for_each workflow must report bounded iteration count, got {}", budget.max_for_each_iterations);`. Replace `let _ = other;` with `panic!("unexpected compile error variant: {other:?}");`. | `owner_approved_debt` |
| R2-M-02 | MEDIUM | `vb_cli/src/io.rs:281, 287, 294, 301, 308, 315, 322` (7 sites — round-1 L-02 not fixed) | `assert!(result.is_ok());` — banned `is_ok()` smoke. The "write_X_succeeds" tests only check that the write doesn't fail; they don't assert that bytes were written, formatting was applied, or destination was correct. | Change `write_version_stdout` to write empty bytes. Test passes. | Replace with `assert!(result.is_ok_and(\|_\| !written_buffer.is_empty()));` and capture the bytes written. | `owner_approved_debt` |
| R2-M-03 | MEDIUM | `vb_compile/src/tests/integration_reduce_tests.rs:36-37, 70` | `try_from_parts(parts).ok().unwrap_or_else(\|\| panic!("workflow must compile"))` (line 36-37) and `println!("GAP EXPOSED: reduce.rs does not detect missing accumulator update")` (line 70). Banned `println!` in tests + fixture-construction panic. | n/a (test infrastructure noise). | Replace with `try_from_parts(parts).expect("workflow must compile")`. Remove `println!`. | `owner_approved_debt` |
| R2-M-04 | MEDIUM | `vb_compile/tests/vb_xi2f_nested_do_lowering.rs:480-494` — round-1 H-05 not fixed | `assert!(result.is_err(), "nested do with out-of-range input slot should fail")` (line 488) — banned `is_err()` smoke. Test then unwraps the error and continues with field-shape checks (line 491-494+), but the outer `is_err()` accepts any Err variant. | Change `compile_workflow` to return `Err(CompileErrors(vec![CompileError::Other("bad input")]))` instead of the typed `StepIndexOutOfRange`. Test still passes. | Replace with `assert!(matches!(result, Err(e) if e.0.iter().any(\|c\| matches!(c, CompileError::StepIndexOutOfRange{..} \|\| CompileError::SlotIndexOutOfRange{..}))))`. | `owner_approved_debt` |
| R2-M-05 | MEDIUM | `vb_cli/tests/cli_vb_m214_bdd_scenarios.rs:304-308, 333-349, 370-380, 420-477, 525-530, 575-595` (~25 sites — round-1 H-01 partially fixed) | The round-1 fix added `assert_eq!(output.status.code(), Some(2))` to most "failing" BDD scenarios, but several "verify may pass or fail" scenarios still use the wide-range `code == Some(0) || code == Some(2)` pattern (line 373), `code == Some(0) || code == Some(1)` (line 579), `output.status.success() || output.status.code() == Some(5)` (line 304), etc. | Change `verify` to exit 0 on every input. Line 373 passes (acceptable range includes 0). Line 579 passes. | Pick a single expected exit code per BDD scenario, or rename the test to `cli_verify_exit_code_is_typed` and assert on the enum, not the integer. | `owner_approved_debt` |
| R2-M-06 | MEDIUM | `vb_cli/tests/cli_integration.rs:3739, 3752, 3756, 3770, 3774, 3778, 3791, 3802, 3814, 3828, 3871, 5400` (12 sites — round-1 O-04 not fixed) | `let _ = &report.repair_hints;` (and similar for `&report.checks`, `&report.warnings`, `&report.phase`, `&report.errors`, `&report.events`, `&report.trace`, `&report.diffs`) inside match arms that return `true`. The match is "is this field present in the report" but the assertion is just `true` — there is no field-reachability check. The `let _ =` discards the value. | Remove a field from the report. Test still passes because the match arm returns `true` regardless. The verify/explain report field reachability is unenforced. | Replace with `assert!(!report.repair_hints.is_empty(), "repair_hints field must be populated");` or `assert!(report.checks.len() > 0, "...");` per field. | `owner_approved_debt` |
| R2-L-01 | LOW | `vb_compile/src/enums/side_effect_tests.rs:165, 173, 181, 189, 197, 205, 213` (7 sites — round-1 L-06 not fixed) | `let _ = SideEffect::Pure;` (and 6 similar) as variant-existence smoke. Round-1 noted this is redundant with the `VARIANTS` array enumeration; no behavioral risk. | n/a — only catches compiler-level deletion of the enum variants. | Either remove the redundant tests (preferred) or convert to `assert!(VARIANTS.contains(&SideEffect::Pure));`. | `owner_approved_no_action` |
| R2-L-02 | LOW | `vb_compile/src/enums/tests/retry_safety_tests.rs:165, 173, 181, 189` (4 sites) | Same `let _ = RetrySafety::Idempotent;` variant-existence pattern. | n/a. | Same as R2-L-01. | `owner_approved_no_action` |
| R2-L-03 | LOW | `vb_cli/src/naming_scan/allowlist.rs:85, 99, 115` (3 sites) | `let ex = exact_exception("src/old.rs", &cfg).unwrap();` — `.unwrap()` on test fixture construction. Not banned, just flagged for completeness. | n/a — fixture. | Acceptable. | `owner_approved_no_action` |
| R2-L-04 | LOW | `vb_cli/tests/deliver_sink_integration.rs:877-878` (round-1 L-04 acknowledged) | `static PROBE: OnceLock<Mutex<Option<(BinaryFingerprint, Result<(), String>)>>>` — hidden shared mutable state. Documented as fingerprint-keyed cache for rebuild-invalidation property. | n/a — well-documented. | Add doc comment if missing. | `owner_approved_no_action` |
| R2-L-05 | LOW | `vb_cli/src/agent_context/tests/unit.rs` (200+ `panic!()` instances — round-1 H-06 not fixed) | `panic!("gate '{}' must be an object", gate_name)`, `panic!("expected Inspect command, got {parsed:?}")` patterns. Project AGENTS.md allows test clippy to be loose. | n/a — fixture/assertion panics, not `#[should_panic]` tests. | Convert to `.expect()` if owner wants strict; otherwise accept as test-policy. | `owner_approved_no_action` |
| R2-L-06 | LOW | `vb_cli/src/args/tests/journal.rs:19 sites — round-1 H-07 was a false alarm` | `if let Ok(Command::Inspect{..}) = parsed { /* real */ } else { panic!("expected Inspect command, got {parsed:?}") }`. This is the CORRECT fix shape (equivalent to `match { Ok(X) => ..., other => panic!() }`), not a regression. | If `parse_args` returns `Ok(Command::OtherVariant)`, the else branch panics with "expected Inspect command, got OtherVariant". | Already correct. | `owner_approved_no_action` |
| R2-O-01 | OBSERVATION | `vb_compile/src/budget_analyzer.rs:60-71` (round-1 O-01 not fixed) | `WholeWorkflowBudget::compute(&parts.nodes, ...).ok().unwrap_or_else(unbounded_default)` — production code silently substitutes a zero budget if the inner compute fails. The `let _ = other;` test that survives (R2-M-01) cannot catch this. | If the second compute call returns `Err`, the error is silently swallowed; the `UnboundedWorkflow` error carries a zeroed budget. Caller cannot distinguish "budget was zeroed by analyzer" from "budget was actually zero". | Either propagate the error (return `Err(CompileError::Workflow(...))`) or assert non-fallibility with `expect("compute must succeed after BudgetPolicyExceeded")`. | `owner_approved_no_action` |
| R2-O-02 | OBSERVATION | `vb_compile/src/tests/do_choose_digest_unit_tests.rs:1-99`, `mod_compile_lowering/tests.rs:1-110`, `vb_compile/src/property_tests/bytecode_ast_parity.rs:1-125`, `vb_compile/src/mod_compile_lowering/together_lowering_tests.rs:1-50+`, `vb_validate/src/property_tests/proptest_*.rs:1-9` (8+ files) | `#![allow(clippy::..., ...)]` blanket suppression of clippy lints including `clippy::let_underscore_must_use`, `clippy::unnecessary_unwrap`, `clippy::unwrap_used`. Maintenance hazard; the round-1 fixes were possible despite the suppression because the suppression does NOT include `clippy::panic` or `clippy::expect_used`. | n/a — the suppression is intentional for proptest ergonomics. | Reduce the allow list to only the lints actually needed; suppress per-`#[test]` rather than per-file. | `owner_approved_no_action` |
| R2-O-03 | OBSERVATION | `vb_compile/src/kani/kani_validation_error_code.rs:14-79` (round-1 M-09 not fixed) | Hardcoded 64-entry `REGISTERED_CODES` list parallel to production `CODE_REGISTRY`. | If `CODE_REGISTRY` removes a code name, this kani harness still passes because it consults its own copy. Bounded by file's `Bound: 64 variants`. | Iterate over `CODE_REGISTRY` to build the constant at compile time. | `owner_approved_no_action` |
| R2-O-04 | OBSERVATION | `vb_compile/src/property_tests/bytecode_ast_parity.rs:740 lines, vb_validate/src/property_tests/*.rs:1100+ lines (NEW wave-7)` | Wave-7 added 4 large proptest files to vb_validate. They are high-quality (concrete assertions, no `is_ok()` smoke). However, the `be_never_panics` proptests (R2-O-04 lines 309 in bound_enforcement.rs, 234-235 in state_machine.rs) use `let _ = validate_resource_limits(&wf, &hard);` and `let _ = validate_types(&wf); let _ = validate_taint(&wf);`. These are technically `must_use` silent suppression, but the docstring clearly states the contract is "never panics" — the assertion is the implicit non-panic of the function call. | n/a — proptest-ergonomic, well-documented. | If owner wants strict, replace with `let _ = validate_types(&wf).map_err(|e| prop_assert!(false, "validator panicked: {e:?}"));`. | `owner_approved_no_action` |

---

## Pattern Census (round 2 counts)

### `assert!(...is_ok()) / assert!(...is_err()) / matches!(..., Some(_) | Ok(_) | Err(_))` and bare `unwrap()`

| Crate | Total matches | Top files |
|-------|---------------|-----------|
| `vb_cli/src` | 78 | `main_tests.rs` (13), `app_impl_tests.rs` (13), `io.rs` (7), `args/tests/parse_misc2.rs` (1) |
| `vb_cli/tests` | 110+ | `cli_vb_m214_bdd_scenarios.rs` (~25 wide-range + 30 `assert_eq!`), `cli_trace_integration.rs` (15 unwraps), `cli_integration.rs` (8 is_err), `lifecycle_integration.rs` (1 is_err), `ir_artifact_admission.rs` (5) |
| `vb_compile/src` | 6 | `flux_choose.rs` (5 — spec files, not tests), `tests/accumulator_overflow_tests.rs` (5), `tests/property_validation_tests.rs` (3), `tests/integration_reduce_tests.rs` (1) |
| `vb_compile/tests` | 14 | `digest_ask_explicit_arm.rs` (11 unwraps in `let _ =` form — ROUND 1 FIXED), `proptest_choose_otherwise.rs` (2), `proptest_choose_fallthrough.rs` (1), `proptest_choose_emission.rs` (1), `proptest_choose_depth.rs` (1), `vb_xi2f_compile_source_proptest.rs` (1), `vb_xi2f_nested_do_lowering.rs` (1) |
| `vb_compile/src/mod_compile_lowering` | 2 | `together_lowering_tests.rs` (1 — `let () = result.expect(...)` is OK) |
| `vb_proof_kernels/src` | 5 | `profile_contract/contract_lemmas.rs` (3 — Verus proofs), `profile_contract/contract_witnesses.rs` (2) |
| `vb_validate/src` | 3 | `gate_08_verus_proof.rs` (2 — Verus), `proptest_state_machine.rs` (1 — `let _ = result;` in never-panic proptest) |
| `vb_validate/tests` | 3 | `red_phase_validation.rs` (3 — round-1 NOT fixed) |
| **TOTAL** | **~220** | (concentrated in `vb_cli/` — `main_tests.rs` + `app_impl_tests.rs` together = 26 banned `is_ok()` sites) |

### `let _ = ...` (silent suppression, excluding flux/verus files)

| Crate | Total matches | Top files |
|-------|---------------|-----------|
| `vb_cli/src` | 60+ | `commands_verify/pipeline.rs` (6), `commands_workflow/tests.rs` (2), `deliver_sink/atomic_publish.rs` (2), `deliver_sink/{deliver_debug_test_support,deliver_test_support}.rs` (2), `matrix/source_command_enum.rs` (1) |
| `vb_cli/tests` | 30 | `cli_integration.rs` (12), `lifecycle_integration.rs` (3), `cli_verify_integration.rs` (1), `deliver_sink_integration.rs` (1) |
| `vb_compile/src` | 15 | `tests/do_choose_digest_unit_tests.rs` (18 — counted above), `budget_analyzer.rs` (2 — RESIDUAL after round 1 fix), `tests/integration_reduce_tests.rs` (1), `mod_compile_lowering/together_e2e_tests.rs` (4 — RESIDUAL) |
| `vb_compile/src/kani` | 25 | `kani_validation_error_code.rs` (25 — kani::assert wrappers, legitimate) |
| `vb_validate/src` | 3 | `property_tests/proptest_state_machine.rs` (2), `property_tests/proptest_constant_folding_validation.rs` (1), `type_taint/type_taint_tests.rs` (3) |
| **TOTAL** | **~133** | (concentrated in `vb_cli/tests/cli_integration.rs` field-reachability tests) |

### `#[ignore]` / `#[should_panic]` / `sleep(` / `todo!()` / `unimplemented!()`

| Crate | Total | Notes |
|-------|-------|-------|
| `vb_cli/src` | 1 | `doctor.rs:31` — `std::thread::sleep(...)` in retry loop on `ProcessLockHeld`. Production code, not test. **Resource-risk finding**: unbounded retry may exceed moon ci budget. |
| `vb_cli/tests` | 1 | `cli_integration.rs:5348` — `std::thread::sleep(std::time::Duration::from_millis(10))` in answer-IPC test busy-wait. Bounded by `std::time::Duration::from_secs(5)` deadline — acceptable. |
| `vb_compile/src/property_tests/bytecode_ast_parity.rs:664` | 1 | Stale `// motivated the #[ignore] is resolved. The test now runs unconditionally` comment. The test runs unconditionally. |
| **TOTAL** | 3 | (low — all acceptable) |

### `lazy_static` / `OnceLock` / `static mut` / `thread_local!`

| Crate | Total | Notes |
|-------|-------|-------|
| `vb_cli/tests` | 3 | `deliver_sink_integration.rs:877-878` — fingerprint-keyed cache, documented. `deliver_test_support.rs:64`, `deliver_debug_test_support.rs:35` — test support thread-locals. |
| `vb_validate/src` | 1 | `diag_render/fallback.rs:14` — production `OnceLock`. |
| **TOTAL** | 4 | (all acceptable, well-documented) |

### `panic!` and `println!` in test code

| Crate | Total matches |
|-------|---------------|
| `vb_cli/src` (incl. test submodules) | 44 (`panic!`) + 4 (`println!`) — `args/tests/journal.rs` has 19 `panic!` which is the correct CLI args fix shape, not banned. `agent_context/tests/unit.rs` has 200+ `panic!` calls (fixture construction). |
| `vb_compile/src` | 12 (`panic!`) + 3 (`println!`) — `tests/property_validation_tests.rs` has 3 `println!("PASS ...")` in the TDD-red pattern (R2-H-01). |
| `vb_cli/tests` | 21 (`println!` for diagnostics in BDD scenarios) |
| `vb_compile/tests` | 3 (`println!` for GAP EXPOSED, all in `property_validation_tests.rs` and `integration_reduce_tests.rs`) |
| **Total panic!/println! in test code** | ~80+ |

### Wave-7 NEW test files (added since round 1)

| File | Lines | Quality |
|------|-------|---------|
| `vb_validate/src/property_tests/proptest_bound_enforcement.rs` | 337 | Excellent — concrete `prop_assert_eq!`, specific error variant matching. |
| `vb_validate/src/property_tests/proptest_state_machine.rs` | 321 | Excellent — concrete `prop_assert_eq!(validate_types(&wf), Ok(()))`. |
| `vb_validate/src/property_tests/proptest_constant_folding_validation.rs` | 282 | Excellent — concrete `prop_assert_eq!`. |
| `vb_validate/src/property_tests/proptest_taint_safety.rs` | 287 | Excellent — covers §38 taint safety property with concrete `prop_assert_eq!`. |
| `vb_proof_kernels/src/envelope_header/tests.rs` | 175 | **3 defects** — `test_compute_header_crc_returns_zero` (line 144), `test_validate_header_crc_always_true` (line 150), and `test_valid_magic`/`test_invalid_magic` smoke (lines 4-14). |
| `vb_proof_kernels/src/resource_budget/combinator/tests.rs` | 176 | Excellent — concrete `assert_eq!` for every field. |
| `vb_proof_kernels/src/resource_budget/lemmas.rs` (Verus) | n/a | Verus proof, not behavior test. |
| `vb_compile/src/property_tests/bytecode_ast_parity.rs` | 740 | Excellent — `prop_assert_eq!` for AST/bytecode evaluator parity; 1024 cases; production-bound. |

---

## Mutation Gaps (top 5 most dangerous bugs the slice would NOT catch)

1. **`compute_whole_workflow_budget` returns all-zeros budget.** The round-1 fix to
   `budget_analyzer.rs` and `red_queen_budget.rs` removed 41 `let _ = budget.field;`
   statements and replaced them with concrete `assert_eq!` calls, BUT the residual
   `let _ = budget.max_for_each_iterations;` at `budget_analyzer.rs:216` and the
   `Ok(_) | Err(CompileError::UnboundedWorkflow { .. }) | Err(other) { let _ = other; }`
   match at line 213-227 would pass even if `max_for_each_iterations` were always 0
   or always u64::MAX. **File:Line:** production
   `crates/vb_compile/src/budget_analyzer.rs:35-52` and
   `vb_core::budget::WholeWorkflowBudget`.

2. **Section 47 contract violation: `compile_workflow` strips secret data from Finish.**
   The 13 `assert!(workflow.finish_contains_secret_data())` assertions in
   `taint/tests/secret_finish_tests.rs` correctly enforce the data-preservation
   contract post-round-1, BUT the `integration_reduce_tests.rs:70` `println!("GAP EXPOSED: ...")`
   pattern plus the `property_validation_tests.rs:13-15,23-25,47-49` TDD-red pattern
   mean that structural validation gaps (Together empty branches, Reduce empty
   body, duplicate branch labels) silently accept any input. **File:Line:** production
   `crates/vb_compile/src/mod_compile_lowering/part_04/body_dispatch.rs` and
   `crates/vb_compile/src/validation/mod.rs`.

3. **`emit_single_body_set` Together branch deleted.** The 4 `let _ = result;` in
   `mod_compile_lowering/together_e2e_tests.rs:368, 407, 446, 494` and the
   `prop_assert!(matches!(result, Ok(()) | Err(_)))` in
   `proptest_together_errors.rs:262-264` and `proptest_choose_depth.rs:62-66` would all
   pass if `emit_single_body_set` was simplified to `fn emit_single_body_set(...) ->
   Result<(), Err> { Err(CompileError::UnsupportedStepPrimitive) }`. **File:Line:**
   production `crates/vb_compile/src/mod_compile_lowering/part_04/body_dispatch.rs`.

4. **Wrong `Command::*` variant returned by `parse_args`.** Round 1 converted 68+
   sites from `if let Ok(Command::X{..}) / else assert!(parsed.is_ok())` to
   `match { Ok(X) => ..., other => panic!("expected X, got {other:?}") }`, but
   `args/tests/parse_misc2.rs:503` still has the raw `assert!(result.is_ok());`
   pattern. A parser regression that routes every flag to `Command::Run` would be
   caught by all 68 fixed sites but NOT by `parse_misc2.rs:503`. **File:Line:**
   production `crates/vb_cli/src/args/mod.rs`.

5. **`compute_header_crc` and `validate_header_crc` are stubs.** The wave-7-added
   `envelope_header/tests.rs:144-147` asserts `compute_header_crc(&header) == 0`
   and `:150-153` asserts `validate_header_crc(&header) == true`. A
   regression that replaces both with `|_| 0` and `|_| true` would pass the tests.
   The CRC contract is unenforced. **File:Line:** production
   `crates/vb_proof_kernels/src/envelope_header.rs`.

---

## Top 5 Fixes (impact-per-effort)

### Fix 1 — Replace `assert!(*.is_ok())` in `vb_cli/src/main_tests.rs` and `app_impl_tests.rs` with `.expect(...)` or field-level assertions (26 sites)
**Impact:** 26 banned `is_ok()` sites strengthened. **Effort:** 30 min. Mechanical refactor.

```rust
// BEFORE (vb_cli/src/main_tests.rs:529):
assert!(journal.is_ok(), "journal should reopen");
// AFTER:
let journal = journal.expect("journal must reopen");
assert_eq!(journal.name(), expected_name);
```

### Fix 2 — Replace `let _ = result;` in `together_e2e_tests.rs` with concrete workflow assertions (4 sites)
**Impact:** 4 Together E2E tests become real contract tests. **Effort:** 1 hour. Map each together shape to expected `node_count()` and `TogetherBranch` index.

```rust
// BEFORE (together_e2e_tests.rs:490):
let result = compile_yaml(yaml);
let _ = result;
// AFTER:
let workflow = compile_yaml(yaml).expect("together with foreach in branches must compile");
assert!(workflow.node_count() >= 10, "2 branches * (1 for_each_start + 1 for_each_next + 1 for_each_done) + 1 together_start + 1 together_join + 1 finish");
```

### Fix 3 — Convert `property_validation_tests.rs` TDD-red `Ok(_) => panic! / Err(_) => println!("PASS")` to specific error variant match (3 sites)
**Impact:** Section 38 "Together empty branches", "Reduce empty body", "Duplicate branch labels" rows are now enforced. **Effort:** 1 hour.

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

### Fix 4 — Wire `cli_vb_m214_bdd_scenarios.rs` exit-code assertions to specific codes per scenario (8-12 sites)
**Impact:** BDD scenarios can no longer pass for either "verify succeeded" or "verify failed" — pick one per scenario. **Effort:** 1 hour.

```rust
// BEFORE (cli_vb_m214_bdd_scenarios.rs:373):
assert!(code == Some(0) || code == Some(2), "...");
// AFTER (rename scenario to cli_verify_failed_workflow_returns_2):
assert_eq!(output.status.code(), Some(2), "verification failure must exit 2");
```

### Fix 5 — Strengthen `vb_proof_kernels/src/envelope_header/tests.rs` CRC assertions (3 sites)
**Impact:** CRC contract is now enforced. **Effort:** 1 hour. Choose known-input/expected-output pairs.

```rust
// BEFORE (envelope_header/tests.rs:144-147):
fn test_compute_header_crc_returns_zero() {
    let header = EnvelopeHeader::new();
    assert_eq!(compute_header_crc(&header), 0);
}
// AFTER:
fn test_compute_header_crc_is_deterministic_and_nonzero() {
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
| R2-C-01 | `blocker` | 13 banned `is_ok()` sites in `vb_cli/src/main_tests.rs` regression. |
| R2-C-02 | `blocker` | 13 banned `is_ok()` sites in `vb_cli/src/app_impl_tests.rs` regression. |
| R2-C-03 | `blocker` | Banned `is_ok()` at `vb_cli/src/args/tests/parse_misc2.rs:503` — parser regression would not be caught. |
| R2-H-01 | `blocker` | `println!` + TDD-red in `property_validation_tests.rs` — Section 38 rows silently unenforced. |
| R2-H-02 | `blocker` | 5 banned `is_ok()` proptests in 4 `proptest_choose_*.rs` files — Section 38 "Choose" rows silently unenforced. |
| R2-H-03 | `blocker` | 3 banned `is_ok()` in `red_phase_validation.rs` — Gate 7/8/9/10/11/13/14/15 pipeline silently unenforced. |
| R2-H-04 | `blocker` | 4 `let _ = result;` in `together_e2e_tests.rs` — Together multi-control-flow contract unenforced. |
| R2-H-05 | `blocker` | 3 wave-7 smoke/stub-looking tests in `envelope_header/tests.rs` — CRC contract unenforced. |
| R2-H-06 | `blocker` | 7 banned `is_err()` in `cli_integration.rs` — error variant taxonomy silently unenforced. |
| R2-H-07 | `owner_approved_debt` | `vb_qi37_14_1_run_step.rs:1316-1323` — known TODO, contract choice pending. |
| R2-M-01 | `owner_approved_debt` | 2 residual `let _ = ...` in `budget_analyzer.rs` after round-1 fix. |
| R2-M-02 | `owner_approved_debt` | 7 banned `is_ok()` in `vb_cli/src/io.rs` — test infrastructure, low blast radius. |
| R2-M-03 | `owner_approved_debt` | `println!` + fixture-construction panic in `integration_reduce_tests.rs`. |
| R2-M-04 | `owner_approved_debt` | 1 banned `is_err()` in `vb_xi2f_nested_do_lowering.rs:488`. |
| R2-M-05 | `owner_approved_debt` | ~10 wide-range exit-code assertions remaining in `cli_vb_m214_bdd_scenarios.rs` after round-1 partial fix. |
| R2-M-06 | `owner_approved_debt` | 12 `let _ = &report.X` field-reachability tests in `cli_integration.rs:3739-3871`. |
| R2-L-01..L-06 | `owner_approved_no_action` | Variant-existence, fixture-construction, fingerprint cache, doc comment hygiene — all low impact. |
| R2-O-01..O-04 | `owner_approved_no_action` | Observations on production-code fall-through, blanket `#![allow(...)]` suppression, hardcoded kani code list, and proptest ergonomics. |

---

## Verdict

```
STATUS: REJECTED
```

**3 CRITICAL findings + 7 HIGH findings** remain in the slice, of which 8 are
direct regressions of round-1 fixes (F8, F9, F10, F11, F12 + R2-H-04, R2-H-06,
R2-H-07, R2-M-04). Wave-7 added ~2,400 lines of high-quality property tests in
`vb_validate/src/property_tests/` but did NOT backfill the round-1 regressions in
`vb_cli/src/main_tests.rs`, `app_impl_tests.rs`, `parse_misc2.rs`,
`together_e2e_tests.rs`, `property_validation_tests.rs`, `red_phase_validation.rs`,
`cli_integration.rs`, `vb_xi2f_nested_do_lowering.rs`, `vb_qi37_14_1_run_step.rs`,
`proptest_choose_*.rs`, `proptest_together_errors.rs`, and
`vb_xi2f_compile_source_proptest.rs`. Wave-6 also introduced 3 new smoke/stub
assertions in `vb_proof_kernels/src/envelope_header/tests.rs`.

Recommend: (1) Fix the 3 CRITICAL `is_ok()` patterns in
`vb_cli/src/main_tests.rs`, `app_impl_tests.rs`, and `parse_misc2.rs` (Fix 1,
~30 min). (2) Replace 4 `let _ = result;` in `together_e2e_tests.rs` (Fix 2,
~1 hr). (3) Convert `property_validation_tests.rs` TDD-red + `println!` to
specific error variant match (Fix 3, ~1 hr). (4) Pick single exit codes per BDD
scenario in `cli_vb_m214_bdd_scenarios.rs` (Fix 4, ~1 hr). (5) Strengthen
`envelope_header/tests.rs` CRC assertions (Fix 5, ~1 hr).
