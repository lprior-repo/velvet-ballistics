# State 8 Test Writer Report — vb-qi37.10

## Scope

- Worked only in isolated workspace: `/tmp/opencode/go-skill-vb-qi37-10`.
- Production implementation code was not modified.
- Red Queen was not invoked.

## Startup Evidence

- Read `/home/lewis/.claude/skills/test-writer/SKILL.md`: lines 21-30 require behavior-focused tests, exact assertions, and automated coverage for every cared-about behavior.
- Read `/home/lewis/.agents/skills/test-writer/SKILL.md`: same content wins on conflict; no conflict found. Lines 158-163 ban broad `is_ok()`/`is_err()` assertions and require sharp assertions.

## Files Changed

- `crates/vb_codegen/src/tests.rs`
- `crates/vb_codegen/tests/trybuild_tests.rs`
- `.beads/vb-qi37.10/test-writer-report.md`

## Tests Added

### Support matrix totality

- `generated_support_matrix_totality_rejects_no_final_ir_variant_silently`
  - Covers `TogetherStart`, `TogetherBranch`, `TogetherJoin`, `ReduceStart`, `ReduceNext`, `ReduceFinish`, `RepeatStart`, `RepeatAttempt`, `RepeatCheck`, `RepeatFinish`, `CollectStart`, `CollectPage`, `CollectNext`, and `CollectFinish`.
  - Asserts exact `CodegenError::UnsupportedIr { feature }` via `validate_generated_subset` and `emit_rust_workflow`.
- `generated_support_matrix_totality_rejects_unsupported_expr_helpers_before_emission`
  - Covers `Contains`, `StartsWith`, and `EndsWith` exact fail-closed text-helper diagnostics.
- `generated_support_matrix_totality_requires_parity_owner_for_every_supported_family`
  - Adds an executable owner map for currently supported families so new support cannot be silently added without named parity evidence.

### Text helper fail-closed behavior

- `text_helper_generated_support_or_rejection_contains_has_text_store_parity_or_exact_rejection`
- `text_helper_generated_support_or_rejection_starts_with_has_text_store_parity_or_exact_rejection`
- `text_helper_generated_support_or_rejection_ends_with_has_text_store_parity_or_exact_rejection`
- `text_helper_generated_support_or_rejection_does_not_emit_partial_text_semantics`

### Generated source contract scan

- `generated_source_contract_contains_no_forbidden_constructs`
- `generated_source_contract_contains_no_unchecked_indexing_slicing_casts_or_arithmetic`
- `generated_source_contract_contains_no_runtime_yaml_json_http_or_string_action_lookup`

### Trybuild non-empty compile-fail loophole

- `trybuild_compile_fail_tests_fails_when_compile_fail_fixture_dir_is_empty`
- Reworked the trybuild harness so an empty `tests/compile-fail` directory returns an exact error instead of `eprintln!` + pass.

### Node-family parity gap / fail-closed regression

- `repeat_generated_parity_fails_closed_with_non_closure_blocker_note`
  - Closest executable regression for Repeat family in State 8.
  - It asserts exact fail-closed rejection for `RepeatStart` from validation and emission.
  - Non-closure note: this does **not** satisfy required Repeat generated-vs-runtime parity; it records that the bead remains non-closable until Repeat support is implemented or scope is formally changed.

## Planned Tests Not Fully Written

- Full generated-vs-runtime parity for Repeat/Reduce/Together/Collect was not written because the current generated emitter rejects these families before source emission, and there is no generated execution harness for unsupported families. Closest executable regression added: exact fail-closed Repeat rejection.
- Journal-signature parity was not added in this State 8 subset because the highest-value requested subset focused on support matrix, text helper rejection, trybuild non-empty enforcement, generated source scanning, and one node-family gap test.

## Commands Run

1. `pwd -P && rtk cargo test -p vb_codegen generated_support_matrix_totality -- --nocapture`
   - Result: **FAIL** initially.
   - Evidence: `/home/lewis/.local/share/rtk/tee/1779030751_cargo_test.log`.
   - Failure exposed test-harness construction issue: `backward edge from StepIdx(0) to StepIdx(0)` for minimal unsupported-node representative.
2. `rtk cargo test -p vb_codegen text_helper_generated_support_or_rejection -- --nocapture`
   - Result: **PASS** — 4 passed, 349 filtered.
3. `rtk cargo test -p vb_codegen generated_source_contract -- --nocapture`
   - Result: **PASS** — 3 passed, 350 filtered.
4. `rtk cargo test -p vb_codegen repeat_generated_parity -- --nocapture`
   - Result: **PASS** — 1 passed, 352 filtered.
5. `rtk cargo test -p vb_codegen --test trybuild_tests`
   - Result: **PASS** — 3 passed.
6. `rtk cargo test -p vb_codegen generated_support_matrix_totality -- --nocapture`
   - Result: **FAIL** after first repair.
   - Evidence: `/home/lewis/.local/share/rtk/tee/1779030796_cargo_test.log`.
   - Failure exposed another test-harness construction issue: `node StepIdx(1) is not reachable from the entry step` for terminal unsupported-node representatives.
7. `rtk cargo test -p vb_codegen generated_support_matrix_totality -- --nocapture`
   - Result: **PASS** — 3 passed, 350 filtered.

## State 8 Outcome

- Failing-first tests were added and focused commands were run.
- The support-matrix failures were test-harness construction failures, then repaired within test code only.
- Current executable tests pass for the requested focused subset, but required node-family parity remains a real implementation gap.

## Exact Next State Transition

Proceed to **go-skill State 9 — test-reviewer reviews the new tests and this State 8 report before implementation work**.

---

# Repair Attempt 1 — go-skill State 8 for vb-qi37.10

## Scope Guard

- Worked only in isolated workspace: `/tmp/opencode/go-skill-vb-qi37-10`.
- Production implementation code was not modified.
- Red Queen was not invoked.
- Startup files re-read:
  - `/home/lewis/.claude/skills/test-writer/SKILL.md` lines 21-30 require behavior-focused tests and automated evidence for every cared-about behavior; lines 158-163 ban broad `is_ok()`/`is_err()` assertions.
  - `/home/lewis/.agents/skills/test-writer/SKILL.md` has the same content and wins on conflict.

## Files Changed in Repair Attempt 1

- `crates/vb_codegen/src/tests.rs`
- `.beads/vb-qi37.10/test-writer-report.md`
- Raw output logs under `.beads/vb-qi37.10/repair-attempt-1-outputs/`

## Tests Added / Strengthened

- Added failing-first executable parity tests so every previously zero-test required filter now selects at least one test:
  - `repeat_generated_parity_matches_runtime_for_first_attempt`
  - `reduce_generated_parity_matches_runtime_for_empty_input`
  - `together_generated_parity_matches_runtime_for_all_successful_branches`
  - `collect_generated_parity_matches_runtime_for_single_page`
  - `expression_generated_parity_matches_append_value_order_and_taint`
  - `expression_generated_parity_matches_merge_field_precedence_and_taint`
  - `generated_taint_parity_preserves_secret_values`
  - `journal_signature_generated_parity_matches_action_wait_ask_boundary_signatures_already_in_scope`
- Kept `repeat_generated_parity_fails_closed_with_non_closure_blocker_note` and added the required generated-vs-runtime Repeat parity expectation. It fails today because `emit_rust_workflow` rejects `RepeatStart`, which is the current product gap.
- Strengthened `generated_support_matrix_totality_requires_parity_owner_for_every_supported_family` so every owner is checked against `include_str!("tests.rs")` for an executable `fn <owner>` signature, not just a non-empty string.
- Added local helper `require_generated_runtime_parity_source` that requires generated source emission and `compare_generated_to_ir` parity before family-specific assertions. Current required-family failures are from unsupported generated emission, not test construction.

## Focused Command Results

All required filters selected at least one test in the final repair run. No required filter still ran 0 tests.

| # | Command | Result | Tests selected / status | Raw output |
|---|---|---|---|---|
| 1 | `rtk cargo test -p vb_codegen generated_support_matrix_totality -- --nocapture` | PASS | 3 passed, 358 filtered | `.beads/vb-qi37.10/repair-attempt-1-outputs/01_generated_support_matrix_totality.log` |
| 2 | `rtk cargo test -p vb_codegen repeat_generated_parity -- --nocapture` | FAIL expected | 1 passed, 1 failed, 355 filtered | `.beads/vb-qi37.10/repair-attempt-1-outputs/02_repeat_generated_parity.log` |
| 3 | `rtk cargo test -p vb_codegen reduce_generated_parity -- --nocapture` | FAIL expected | 1 failed, 356 filtered | `.beads/vb-qi37.10/repair-attempt-1-outputs/03_reduce_generated_parity.log` |
| 4 | `rtk cargo test -p vb_codegen together_generated_parity -- --nocapture` | FAIL expected | 1 failed, 356 filtered | `.beads/vb-qi37.10/repair-attempt-1-outputs/04_together_generated_parity.log` |
| 5 | `rtk cargo test -p vb_codegen collect_generated_parity -- --nocapture` | FAIL expected | 1 failed, 356 filtered | `.beads/vb-qi37.10/repair-attempt-1-outputs/05_collect_generated_parity.log` |
| 6 | `rtk cargo test -p vb_codegen expression_generated_parity -- --nocapture` | FAIL expected | 2 failed, 355 filtered | `.beads/vb-qi37.10/repair-attempt-1-outputs/06_expression_generated_parity.log` |
| 7 | `rtk cargo test -p vb_codegen generated_taint_parity -- --nocapture` | FAIL expected | 1 failed, 356 filtered | `.beads/vb-qi37.10/repair-attempt-1-outputs/07_generated_taint_parity.log` |
| 8 | `rtk cargo test -p vb_codegen text_helper_generated_support_or_rejection -- --nocapture` | PASS | 4 passed, 357 filtered | `.beads/vb-qi37.10/repair-attempt-1-outputs/08_text_helper_generated_support_or_rejection.log` |
| 9 | `rtk cargo test -p vb_codegen generated_source_contract -- --nocapture` | PASS | 3 passed, 358 filtered | `.beads/vb-qi37.10/repair-attempt-1-outputs/09_generated_source_contract.log` |
| 10 | `rtk cargo test -p vb_codegen journal_signature_generated_parity -- --nocapture` | FAIL expected | 1 failed, 356 filtered | `.beads/vb-qi37.10/repair-attempt-1-outputs/10_journal_signature_generated_parity.log` |
| 11 | `rtk cargo test -p vb_codegen --test trybuild_tests` | PASS | 3 passed | `.beads/vb-qi37.10/repair-attempt-1-outputs/11_trybuild_tests.log` |

## Failure Classification

- Expected failing-first parity failures are product gaps: generated mode rejects required `RepeatStart`, `ReduceStart`, `TogetherStart`, `CollectStart`, and accessor traversal before executable generated-vs-runtime parity can be proven.
- The `generated_taint_parity_*` and `journal_signature_generated_parity_*` failures route through Repeat because required taint and journal parity must cover final node families; generated Repeat remains unsupported.
- No failure is currently classified as a test-construction failure after owner-map repair.

## Exact Next State Transition

Proceed to **go-skill State 8 test-reviewer repair review** for repair attempt 1. Do not enter implementation until the repaired failing-first tests are accepted.

---

# Repair Attempt 2 — go-skill State 8 for vb-qi37.10

## Scope Guard

- Worked only in isolated workspace: `/tmp/opencode/go-skill-vb-qi37-10`.
- Production implementation code was not modified.
- Red Queen was not invoked.
- Startup files re-read:
  - `/home/lewis/.claude/skills/test-writer/SKILL.md` lines 21-30 require behavior-focused tests and exact automated evidence; lines 158-163 ban weak assertion shapes.
  - `/home/lewis/.agents/skills/test-writer/SKILL.md` has the same content and wins on conflict.

## Files Changed in Repair Attempt 2

- `crates/vb_codegen/src/tests.rs`
- `.beads/vb-qi37.10/test-writer-report.md`
- Raw output logs under `.beads/vb-qi37.10/repair-attempt-2-outputs/`

## Repairs Made

- Removed `unsupported_accessor_traversal_workflow()` from the two `expression_generated_parity_*` tests.
- Replaced repaired parity source-keyword assertions with executable generated-vs-IR comparisons:
  - Repeat/Reduce/Together/Collect now call a helper that uses the IR oracle when available, then calls `generated_drive_stdout`; current failures are exact generated-emission product gaps (`RepeatStart`, `ReduceStart`, `TogetherStart`, `CollectStart`).
  - Expression append/list and merge/object parity now use real generated execution via `generated_step_stdout` and compare exact final value/taint against `ir_drive_finished_output_with_init`.
  - Taint parity now uses generated execution and exact IR-derived value/taint for secret object-copy propagation.
  - Journal parity now uses generated execution and exact journal event stdout for the supported ActionScheduled boundary.
- Preserved owner-map function existence enforcement in `generated_support_matrix_totality_requires_parity_owner_for_every_supported_family`.

## Parity Source-Substring Audit

- Required repaired parity filters no longer assert `source.contains(...)` for semantic parity.
- Source scans remain in `generated_source_contract_*` tests and pre-existing source-shape tests outside this repair scope.

## Focused Command Results

All required filters selected at least one test in repair attempt 2. No required filter ran 0 tests.

| # | Command | Result | Tests selected / status | Raw output |
|---|---|---|---|---|
| 1 | `rtk cargo test -p vb_codegen generated_support_matrix_totality -- --nocapture` | PASS | 3 passed, 358 filtered | `.beads/vb-qi37.10/repair-attempt-2-outputs/01_generated_support_matrix_totality.log` |
| 2 | `rtk cargo test -p vb_codegen repeat_generated_parity -- --nocapture` | FAIL expected | 1 passed, 1 failed, 355 filtered; failing test errors with `unsupported generated Rust IR feature: RepeatStart` | `.beads/vb-qi37.10/repair-attempt-2-outputs/02_repeat_generated_parity.log` |
| 3 | `rtk cargo test -p vb_codegen reduce_generated_parity -- --nocapture` | FAIL expected | 1 failed, 356 filtered; error `unsupported generated Rust IR feature: ReduceStart` | `.beads/vb-qi37.10/repair-attempt-2-outputs/03_reduce_generated_parity.log` |
| 4 | `rtk cargo test -p vb_codegen together_generated_parity -- --nocapture` | FAIL expected | 1 failed, 356 filtered; error `unsupported generated Rust IR feature: TogetherStart` | `.beads/vb-qi37.10/repair-attempt-2-outputs/04_together_generated_parity.log` |
| 5 | `rtk cargo test -p vb_codegen collect_generated_parity -- --nocapture` | FAIL expected | 1 failed, 356 filtered; error `unsupported generated Rust IR feature: CollectStart` | `.beads/vb-qi37.10/repair-attempt-2-outputs/05_collect_generated_parity.log` |
| 6 | `rtk cargo test -p vb_codegen expression_generated_parity -- --nocapture` | PASS | 2 passed, 359 filtered | `.beads/vb-qi37.10/repair-attempt-2-outputs/06_expression_generated_parity.log` |
| 7 | `rtk cargo test -p vb_codegen generated_taint_parity -- --nocapture` | PASS | 1 passed, 360 filtered | `.beads/vb-qi37.10/repair-attempt-2-outputs/07_generated_taint_parity.log` |
| 8 | `rtk cargo test -p vb_codegen text_helper_generated_support_or_rejection -- --nocapture` | PASS | 4 passed, 357 filtered | `.beads/vb-qi37.10/repair-attempt-2-outputs/08_text_helper_generated_support_or_rejection.log` |
| 9 | `rtk cargo test -p vb_codegen generated_source_contract -- --nocapture` | PASS | 3 passed, 358 filtered | `.beads/vb-qi37.10/repair-attempt-2-outputs/09_generated_source_contract.log` |
| 10 | `rtk cargo test -p vb_codegen journal_signature_generated_parity -- --nocapture` | PASS | 1 passed, 360 filtered | `.beads/vb-qi37.10/repair-attempt-2-outputs/10_journal_signature_generated_parity.log` |
| 11 | `rtk cargo test -p vb_codegen --test trybuild_tests` | PASS | 3 passed | `.beads/vb-qi37.10/repair-attempt-2-outputs/11_trybuild_tests.log` |

## Failure Classification

- Repeat/Reduce/Together/Collect are expected failing-first product gaps: generated mode still rejects the required final IR families at emission.
- Expression/accessor fixture-construction failure is repaired: tests now execute actual BuildList/BuildObject/Copy behavior and compare exact generated stdout to IR-derived final value and taint.
- Taint and journal parity no longer rely on generated source substrings.

## Exact Next State Transition

Proceed to **go-skill State 9 — test-reviewer repair review for repair attempt 2**. Do not enter implementation until the repaired failing-first tests are accepted.
