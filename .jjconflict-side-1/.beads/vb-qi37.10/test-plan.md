# Test Plan: vb-qi37.10 — generated final IR coverage and parity

## Startup / Authority

- Test-planner startup files read:
  - `/home/lewis/.claude/skills/test-planner/SKILL.md` lines 8-10: this role writes only `test-plan.md` and does not write implementation or test code.
  - `/home/lewis/.agents/skills/test-planner/SKILL.md` lines 8-10: same rule; this file wins on conflict. No conflict found.
  - `/home/lewis/.agents/skills/test-planner/references/testing-philosophy.md` lines 5-16, 82-86: test behavior through public APIs, acceptance tests are the spec, and every cared-about behavior needs automated evidence.
- Repository/bead authority read:
  - `.beads/vb-qi37.10/STATE.md`: State 7 must write this artifact; State 6 reviews are approved; formal lanes are deferred follow-up only.
  - `.beads/vb-qi37.10/contract.md`, `verification-layers.md`, proof ledgers, traceability matrix, proof reviews, and codebase map.
  - Existing harness patterns in `crates/vb_codegen/src/tests.rs`, `crates/vb_codegen/src/lib.rs`, and `crates/vb_codegen/tests/trybuild_tests.rs`.

## Summary

- Behaviors identified: 12 required acceptance behaviors.
- Trophy allocation: 2 static / 9 integration / 1 workspace gate / 0 e2e. This intentionally skews integration-heavy because generated-vs-runtime parity must execute emitted Rust against the IR/runtime oracle.
- Proptest invariants: 5 proposed for extracted pure helper/builders if implementation exposes them; not an acceptance substitute for executable parity.
- Fuzz targets: 1 conditional build target (`generated_compare`) only if fuzz files change; no fuzz pass is claimed as semantic proof.
- Kani/TLA+/Verus harnesses: 0 acceptance harnesses for this bead. Formal lanes are explicitly non-acceptance follow-up beads `vb-w20g`, `vb-h3fx`, and `vb-mnv0`.
- Mutation threshold: focused `cargo-mutants` on `vb_codegen` touched functions should kill >=90% of non-equivalent mutants; every listed mutation target must be killed by a named test.

## 1. Behavior Inventory

1. Support matrix rejects or accepts every final IR node and relevant expression before emission when generated mode is asked to compile a workflow.
2. Repeat generated execution matches runtime oracle on attempt state, routing, typed errors, pc, slots, taints, step states, and journal signature.
3. Reduce generated execution matches runtime oracle on accumulator initialization, item binding, iteration, finish materialization, typed errors, taints, and journal signature.
4. Together generated execution matches runtime oracle on fanout, branch result aggregation, join state, failure policy, typed errors, taints, and journal signature.
5. Collect generated execution matches runtime oracle on pagination state, duplicate/stale page handling, materialization, capacity bounds, typed errors, taints, and journal signature; if unsupported, validation must fail closed and bead closure must not claim completion without an approved scope change.
6. Expression helpers `Append`, `AppendIf`, `Merge`, `Sum`, `Count`, `Unique`, `Has`, `Exists`, `Length`, `Empty` match value/order/type/error/taint runtime or expression oracle semantics.
7. Accessor traversal matches runtime oracle for root, object field, list index, missing field, missing index, wrong type, and taint propagation.
8. Generated taint propagation preserves `Clean`, `DerivedFromSecret`, and `Secret` across expression/accessor/list/object/repeat/reduce/together/collect/finish paths.
9. Text helpers `Contains`, `StartsWith`, and `EndsWith` either have executable text/symbol parity evidence or are rejected with exact typed unsupported-feature diagnostics before source emission.
10. Generated source compiles, formats, passes generated-code lint/static contract, and contains no forbidden constructs or unchecked operations.
11. Trybuild compile-fail coverage is non-empty and fails if no compile-fail fixtures are present.
12. Journal signature parity compares normalized semantic event kinds/order/essential fields between generated and IR/runtime executions for bead-local workflows.

## 2. Trophy Allocation

| Group | Layer | Location | Rationale |
|---|---|---|---|
| Support matrix totality | Unit/integration boundary | `crates/vb_codegen/src/tests.rs` | Public validation API plus representative workflow builders; fail-closed behavior. |
| Node-family parity | Integration | `crates/vb_codegen/src/tests.rs` | Must execute generated Rust and compare to runtime oracle, not just source strings. |
| Expression/accessor parity | Integration | `crates/vb_codegen/src/tests.rs` | Generated helper stores interact with emitted executable source and core/runtime value semantics. |
| Taint parity | Integration | `crates/vb_codegen/src/tests.rs` | Observable run output must preserve taint, including final `Finish`. |
| Text helper decision | Unit/integration | `crates/vb_codegen/src/tests.rs` | Either runtime parity or exact pre-emission rejection. |
| Generated source contract | Static/integration | `crates/vb_codegen/src/tests.rs` | Compile/rustfmt/clippy/static scan over emitted source. |
| Trybuild non-empty compile-fail | Static compile-fail | `crates/vb_codegen/tests/trybuild_tests.rs`, `crates/vb_codegen/tests/compile-fail/*.rs` | Proves contract violations fail at compile time and closes empty-fixture loophole. |
| Journal signature parity | Integration | `crates/vb_codegen/src/tests.rs` | Semantic journal observation must match runtime/storage oracle signature. |
| Final gates | Workspace | moon task | Repository canonical `moon ci`. |

## 3. Required Failure-First Tests

All names below are intentionally failure-first. Test-writer should add the test skeleton/assertions first and confirm they fail for the current gap or loophole before implementation makes them pass. Tests must assert exact values or exact error variants/fields; `is_ok()`/`is_err()` only is forbidden.

### 3.1 Support matrix — obligation `SUPPORT-MATRIX-EXEC-001` / `PO-001`

Placement: `crates/vb_codegen/src/tests.rs`.

Test-writer adds:

- `generated_support_matrix_totality_rejects_no_final_ir_variant_silently`
  - Given representative validated workflows or minimal nodes for every `CompiledNodeKind` family.
  - When `validate_generated_subset` and `emit_rust_workflow` are called.
  - Then every supported family emits source and has a named parity test owner; every unsupported family returns `CodegenError::UnsupportedIr { feature }` with exact feature name before source emission.
  - Must include `TogetherStart`, `TogetherBranch`, `TogetherJoin`, `ReduceStart`, `ReduceNext`, `ReduceFinish`, `RepeatStart`, `RepeatAttempt`, `RepeatCheck`, `RepeatFinish`, `CollectStart`, `CollectPage`, `CollectNext`, `CollectFinish`.
- `generated_support_matrix_totality_rejects_unsupported_expr_helpers_before_emission`
  - Covers `Contains`, `StartsWith`, `EndsWith` exact features.
- `generated_support_matrix_totality_requires_parity_owner_for_every_supported_family`
  - If implementation marks a family supported, this test must require a named parity test group in this plan/evidence map; no silent support additions.

Implementation later makes pass by either implementing support plus parity tests or keeping exact fail-closed rejection. For bead closure, required node families `Repeat`, `Reduce`, `Together`, and `Collect` need implemented parity evidence unless an approved scope change says otherwise.

Negative/error paths:

- Unsupported node rejects before `emit_step_function` can emit `UnsupportedStep` stubs.
- Invalid accessor root/depth/field symbol still rejects exactly.
- Expression text helper rejection includes helper name and reason, not a generic string.

### 3.2 Repeat parity — obligation `NODE-REPEAT-001` / `PO-002`

Placement: `crates/vb_codegen/src/tests.rs`.

Test-writer adds:

- `repeat_generated_parity_matches_runtime_for_first_attempt`
- `repeat_generated_parity_matches_runtime_for_later_attempt`
- `repeat_generated_parity_routes_to_done_when_attempt_limit_exhausted`
- `repeat_generated_parity_preserves_attempt_slot_and_final_pc`
- `repeat_generated_parity_returns_typed_error_when_attempt_counter_overflows`
- `repeat_generated_parity_preserves_taint_on_repeat_finish`

Assertions:

- Generated executable output equals runtime oracle on terminal value/error, final pc, relevant slots, taints, step states, retry/attempt counters, and normalized journal signature.
- Error tests compare exact generated/runtime-compatible error variant and fields, not string containment only.

Implementation later makes pass by supporting `RepeatStart`, `RepeatAttempt`, `RepeatCheck`, `RepeatFinish` in active `crates/vb_codegen/src/lib.rs`, with checked arithmetic and no unchecked casts/indexing in emitted source.

### 3.3 Reduce parity — obligation `NODE-REDUCE-001` / `PO-003`

Placement: `crates/vb_codegen/src/tests.rs`.

Test-writer adds:

- `reduce_generated_parity_matches_runtime_for_empty_input`
- `reduce_generated_parity_matches_runtime_for_single_item`
- `reduce_generated_parity_matches_runtime_for_multiple_items`
- `reduce_generated_parity_preserves_accumulator_taint_join`
- `reduce_generated_parity_returns_type_error_for_non_list_input`
- `reduce_generated_parity_returns_capacity_error_when_materialization_exceeds_contract`

Assertions:

- Accumulator initialization, item-slot binding, tail iteration, output slot, final value, final taint, pc, step states, and journal signature equal runtime oracle.
- Empty input follows runtime oracle exactly; no generated special-case may invent a different terminal result.

Implementation later makes pass by implementing `ReduceStart`, `ReduceNext`, `ReduceFinish` or by leaving exact rejection and not closing bead as complete without approved scope change.

### 3.4 Together parity — obligation `NODE-TOGETHER-001` / `PO-004`

Placement: `crates/vb_codegen/src/tests.rs`.

Test-writer adds:

- `together_generated_parity_matches_runtime_for_all_successful_branches`
- `together_generated_parity_preserves_branch_result_order`
- `together_generated_parity_matches_runtime_when_branch_returns_typed_error`
- `together_generated_parity_preserves_join_taint_lattice`
- `together_generated_parity_returns_capacity_error_when_fanout_exceeds_contract`

Assertions:

- Deterministic generated execution may be sequential, but observable branch fanout/join semantics, aggregation order, error policy, slots, taints, step states, and journal signature must match runtime oracle.

Implementation later makes pass by implementing `TogetherStart`, `TogetherBranch`, `TogetherJoin` with bounded branch state; OS parallelism is not required.

### 3.5 Collect parity — obligation `NODE-COLLECT-001` / `PO-005`

Placement: `crates/vb_codegen/src/tests.rs`.

Test-writer adds:

- `collect_generated_parity_matches_runtime_for_single_page`
- `collect_generated_parity_matches_runtime_for_multiple_pages`
- `collect_generated_parity_rejects_duplicate_page_like_runtime`
- `collect_generated_parity_rejects_stale_page_like_runtime`
- `collect_generated_parity_materializes_items_with_runtime_order`
- `collect_generated_parity_preserves_page_lineage_and_taint`
- `collect_generated_parity_returns_capacity_error_when_max_collect_items_exceeded`
- `collect_generated_parity_fails_closed_with_blocker_when_unimplemented`

Assertions:

- Positive tests compare generated executable output with runtime `CollectStates` oracle for result, page state, slots, taints, step states, and normalized journal signature.
- If Collect remains unimplemented, only `collect_generated_parity_fails_closed_with_blocker_when_unimplemented` may pass; acceptance must record that `vb-qi37.10` is not closable without approved scope change because `POST-002` makes Collect required.

Implementation later makes pass by adding bounded generated side state for `CollectStart`, `CollectPage`, `CollectNext`, `CollectFinish` or by explicitly stopping bead closure.

### 3.6 Expression/accessor parity — obligation `EXPR-HELPERS-001` / `PO-006`

Placement: `crates/vb_codegen/src/tests.rs`.

Test-writer adds:

- `expression_generated_parity_matches_append_value_order_and_taint`
- `expression_generated_parity_matches_append_if_true_and_false`
- `expression_generated_parity_matches_merge_field_precedence_and_taint`
- `expression_generated_parity_matches_sum_for_empty_and_non_empty_lists`
- `expression_generated_parity_matches_count_for_lists_and_objects`
- `expression_generated_parity_matches_unique_stable_order`
- `expression_generated_parity_matches_has_exists_length_empty`
- `expression_generated_parity_returns_exact_type_error_for_wrong_helper_input`
- `accessor_generated_parity_matches_root_field_and_index_traversal`
- `accessor_generated_parity_returns_exact_error_for_missing_field_missing_index_and_wrong_type`
- `accessor_generated_parity_preserves_leaf_and_container_taint`

Assertions:

- Compare generated executable output to `vb_core`/`vb_expr`/runtime oracle for exact `SlotValue`, order, type error variant/fields, and taint.
- Include object/list handle values only through observable contents or agreed helper debug output, not unstable internal handle IDs unless the oracle also exposes them deterministically.

Implementation later makes pass by fixing generated helper code and bounded stores, not by weakening oracle semantics.

### 3.7 Taint parity — obligation `TAINT-001` / `PO-007`

Placement: `crates/vb_codegen/src/tests.rs`.

Test-writer adds:

- `generated_taint_parity_preserves_clean_values`
- `generated_taint_parity_preserves_derived_from_secret_values`
- `generated_taint_parity_preserves_secret_values`
- `generated_taint_parity_joins_list_object_and_branch_taints_like_runtime`
- `generated_taint_parity_preserves_finish_result_taint`
- `generated_taint_parity_does_not_invent_secret_for_clean_inputs`

Assertions:

- Use `GeneratedRunState::new_with_taints` style harness and runtime `write_slot_with_taint` oracle initialization.
- Every scenario compares exact taint enum value and terminal value.

Implementation later makes pass by carrying taint through all generated writes/helpers/node families, including new Repeat/Reduce/Together/Collect support.

### 3.8 Text helper support/rejection — obligation `EXPR-TEXT-001` / `PO-008`

Placement: `crates/vb_codegen/src/tests.rs`.

Test-writer adds:

- `text_helper_generated_support_or_rejection_contains_has_text_store_parity_or_exact_rejection`
- `text_helper_generated_support_or_rejection_starts_with_has_text_store_parity_or_exact_rejection`
- `text_helper_generated_support_or_rejection_ends_with_has_text_store_parity_or_exact_rejection`
- `text_helper_generated_support_or_rejection_does_not_emit_partial_text_semantics`

Assertions:

- If implemented, execute generated code against text/symbol oracle for true, false, empty text, missing symbol/text, and wrong type.
- If not implemented, `validate_generated_subset` returns exact `CodegenError::UnsupportedIr { feature: "text helper ... requires runtime symbol store" }` before emission, and the test asserts `emit_rust_workflow` returns the same rejection.
- No emitted source may contain text-helper stubs that compile and return a generic runtime error.

Implementation later chooses one path. Rejection is acceptable for this helper only with blocker/scope evidence; it is not a formal proof.

### 3.9 Generated source contract — obligation `COMPILE-001` / `PO-009`

Placement: `crates/vb_codegen/src/tests.rs`.

Test-writer adds:

- `generated_source_contract_compiles_representative_supported_workflows`
- `generated_source_contract_formats_with_rustfmt`
- `generated_source_contract_contains_no_forbidden_constructs`
- `generated_source_contract_contains_no_unchecked_indexing_slicing_casts_or_arithmetic`
- `generated_source_contract_contains_no_runtime_yaml_json_http_or_string_action_lookup`
- `generated_source_contract_checks_new_repeat_reduce_together_collect_workflows`

Assertions:

- Scan representative generated source for `unsafe`, `unwrap`, `expect`, `panic`, `todo`, `unimplemented`, `dbg`, unchecked `[` indexing/slicing, ` as ` casts, unchecked arithmetic patterns where repository rules prohibit them, and runtime YAML/JSON/HTTP/string action/reference lookup.
- Run compile/rustfmt/clippy or repository equivalent over emitted source.

Implementation later makes pass by generated code using checked helpers and by extending existing static scan beyond the current forbidden-source patterns if needed.

### 3.10 Non-empty trybuild compile-fail — obligation `TRYBUILD-001` / `PO-010`

Placement:

- Harness: `crates/vb_codegen/tests/trybuild_tests.rs`.
- Fixtures: `crates/vb_codegen/tests/compile-fail/*.rs`.

Test-writer adds:

- `trybuild_compile_fail_tests_fails_when_compile_fail_fixture_dir_is_empty`
- At least one real compile-fail fixture, suggested names:
  - `generated_contract_forbids_unchecked_index.rs`
  - `generated_contract_forbids_unwrap.rs`
  - `generated_contract_forbids_partial_unsupported_step.rs`

Assertions:

- Empty fixture directory is a failing test, not an `eprintln!` pass.
- Each fixture fails for a real generated-code contract violation and has checked `.stderr` output if trybuild requires it.

Implementation later makes pass by removing the empty-fixture pass loophole and adding actual compile-fail fixtures. Test-writer must not count pass fixtures as satisfying compile-fail coverage.

### 3.11 Journal signature parity — obligation `JOURNAL-001` / `PO-011`

Placement: `crates/vb_codegen/src/tests.rs`.

Test-writer adds:

- `journal_signature_generated_parity_matches_basic_slot_write_and_finish`
- `journal_signature_generated_parity_matches_repeat_reduce_together_collect_families`
- `journal_signature_generated_parity_matches_action_wait_ask_boundary_signatures_already_in_scope`
- `journal_signature_generated_parity_reports_exact_mismatch_dimension`

Assertions:

- Compare normalized semantic signature only: event kind, order, step id, slot id, value kind, taint, action/wait/ask/retry scheduling essentials, terminal event.
- Do not require byte-for-byte storage envelope equality; full recovery/hydration remains out of scope.
- On mismatch, exact `GeneratedJournalMismatch`/semantic mismatch dimension is reported.

Implementation later makes pass by recording equivalent generated lightweight journal observations and by using runtime/storage semantic signatures as oracle.

### 3.12 Final gates — obligation `GATE-001` / `PO-012`

Placement: evidence only after implementation; no test code location.

Required commands for formal-verifier/final states:

1. `cargo test -p vb_codegen generated_support_matrix_totality -- --nocapture`
2. `cargo test -p vb_codegen repeat_generated_parity -- --nocapture`
3. `cargo test -p vb_codegen reduce_generated_parity -- --nocapture`
4. `cargo test -p vb_codegen together_generated_parity -- --nocapture`
5. `cargo test -p vb_codegen collect_generated_parity -- --nocapture`
6. `cargo test -p vb_codegen expression_generated_parity -- --nocapture`
7. `cargo test -p vb_codegen generated_taint_parity -- --nocapture`
8. `cargo test -p vb_codegen text_helper_generated_support_or_rejection -- --nocapture`
9. `cargo test -p vb_codegen generated_source_contract -- --nocapture`
10. `cargo test -p vb_codegen journal_signature_generated_parity -- --nocapture`
11. `cargo test -p vb_codegen --test trybuild_tests`
12. Conditional if fuzz target changes: `cargo fuzz build generated_compare`
13. Final repository gate: `moon ci`

Any `moon ci` failure must be classified as bead-local failure or unrelated `DEFERRED_GLOBAL` debt with raw output. Bead-local failures block acceptance.

## 4. What Test-Writer Adds vs What Implementation Makes Pass

| Area | Test-writer must add | Implementation must later make pass |
|---|---|---|
| Support matrix | Exact totality/rejection tests and owner mapping in `src/tests.rs`. | Update `validate_generated_subset`, support matrix, and active `src/lib.rs`; do not touch only duplicate inactive paths. |
| Repeat/Reduce/Together/Collect | Executable generated-vs-runtime oracle tests named above. | Emit real generated support or fail closed and stop closure if required family remains unsupported. |
| Expression/accessor | Helper/accessor parity tests for value, order, type errors, missing paths, taint. | Correct generated helpers/stores and checked errors. |
| Taint | Taint lattice scenarios across new and existing paths. | Preserve exact taint on all writes, joins, and finish. |
| Text helpers | Three helper decision tests plus no-partial-emission test. | Implement text/symbol parity or exact pre-emission rejection with blocker evidence. |
| Generated source | Static/compile/format/lint scan tests. | Emit compliant source with no forbidden constructs. |
| Trybuild | Empty-dir failure plus real compile-fail fixtures. | Wire trybuild to fail empty and maintain `.stderr` expectations. |
| Journal | Semantic signature comparison tests. | Emit/normalize generated journal events matching runtime signature. |
| Final gates | No code; evidence commands only. | Run gates and provide raw evidence. |

## 5. Proptest Invariants

These are recommended when implementation extracts pure helpers; they do not replace required executable parity tests.

1. `generated_support_matrix_totality_prop`
   - Invariant: every generated-support decision is either supported or exact typed unsupported; never unknown/generic.
   - Strategy: safely generated representative node/expression families, not invalid arbitrary internals.
2. `generated_taint_join_matches_runtime_prop`
   - Invariant: generated taint join equals runtime taint join for all combinations of `Clean`, `DerivedFromSecret`, `Secret`.
   - Anti-invariant: weakening `Secret` or `DerivedFromSecret` must fail.
3. `generated_list_helpers_preserve_order_prop`
   - Invariant: append/append_if/unique/count/length preserve expected order/count semantics under bounded small lists.
   - Anti-invariant: duplicate removal that reorders first occurrence must fail.
4. `generated_store_capacity_prop`
   - Invariant: inserting up to capacity succeeds; inserting over capacity returns exact typed capacity error before allocation/unchecked indexing.
   - Anti-invariant: saturating/wrapping handle reuse must fail.
5. `journal_signature_normalization_prop`
   - Invariant: normalizer preserves event order and essential fields; unrelated envelope bytes do not affect signature.
   - Anti-invariant: dropped slot/taint/action field must fail.

## 6. Fuzz Targets

- Conditional target: `fuzz/src/bin/generated_compare.rs` / `cargo fuzz build generated_compare` if fuzz files change.
- Scope: build/smoke only for this bead unless implementation modifies fuzz behavior.
- Risk covered: validation rejection must not be treated as panic or as semantic parity proof.
- Required stance: fuzz is supplementary; it is not a formal proof and not a substitute for named parity tests.

## 7. Kani / TLA+ / Verus Harnesses

No Kani, TLA+, or Verus acceptance coverage is claimed for `vb-qi37.10`.

- `vb-w20g`: future bounded TLA+ generated-vs-IR temporal model.
- `vb-h3fx`: future production-bound Verus support/store/helper proofs.
- `vb-mnv0`: future production-bound Kani support/store bounds harnesses.

These are non-acceptance follow-up beads only. Test-writer must not add vacuous formal artifacts and must not mark this bead accepted based on formal deferrals.

## 8. Mutation Checkpoints

Minimum focused mutation kill target: >=90% for touched `vb_codegen` production/test-helper surfaces; all critical mutants below must be killed.

| Mutant | Must be killed by |
|---|---|
| Change unsupported `Repeat*`/`Reduce*`/`Together*`/`Collect*` to supported without emitter parity. | `generated_support_matrix_totality_requires_parity_owner_for_every_supported_family`; family parity tests. |
| Return generic unsupported feature for text helpers. | `text_helper_generated_support_or_rejection_*` exact rejection tests. |
| Emit `UnsupportedStep` stub for accepted final IR. | Support matrix no-partial-emission and generated source contract tests. |
| Drop taint on `Copy`, `EvalExpr`, BuildList/Object, join/reducer/finish. | `generated_taint_parity_*` tests. |
| Reverse Reduce/Together/Collect materialization order. | Reduce/Together/Collect order tests. |
| Ignore duplicate/stale Collect page. | Collect duplicate/stale tests. |
| Use saturating attempt increment where runtime errors/wrap handling differs. | Repeat overflow/attempt tests. |
| Remove generated journal slot-write or terminal event. | `journal_signature_generated_parity_*` tests. |
| Allow trybuild empty compile-fail directory to pass. | `trybuild_compile_fail_tests_fails_when_compile_fail_fixture_dir_is_empty`. |
| Remove forbidden source scan pattern for `unwrap`, `unsafe`, unchecked indexing/casts. | `generated_source_contract_contains_no_forbidden_constructs` and compile-fail fixtures. |

## 9. Combinatorial Coverage Matrix

| Scenario | Input class | Expected output | Layer |
|---|---|---|---|
| Support accepted family | Supported node/expression workflow | `emit_rust_workflow` returns source and named parity owner exists | Unit/integration |
| Support rejected family | Unsupported node/expression workflow | Exact `CodegenError::UnsupportedIr { feature }` before emission | Unit |
| Repeat first/later/exhausted | Valid repeat workflows | Generated and runtime oracle exact parity | Integration |
| Repeat overflow | Max/near-max attempt state | Exact typed overflow/capacity error parity | Integration |
| Reduce empty/single/many | Bounded list input | Exact accumulator/output/taint/journal parity | Integration |
| Reduce wrong type | Non-list input | Exact runtime-compatible type error | Integration |
| Together all success/failure | Branch workflows | Exact aggregation or typed failure policy parity | Integration |
| Together fanout over cap | Fanout > resource contract | Exact typed capacity/fanout error | Integration |
| Collect single/multiple/duplicate/stale | Page workflows | Exact page/materialization/error parity | Integration |
| Collect over cap | Items > max collect items | Exact typed capacity error | Integration |
| Expression helpers | Lists/objects/scalars | Exact value/order/type/taint parity | Integration |
| Accessors | root/field/index/missing/wrong type | Exact value or exact error parity | Integration |
| Taint lattice | Clean/Derived/Secret contributors | Exact taint enum parity, no weakening/invention | Integration |
| Text helpers implemented | true/false/empty/missing/wrong type text cases | Exact text/symbol oracle parity | Integration |
| Text helpers rejected | Contains/StartsWith/EndsWith workflows | Exact pre-emission unsupported error | Unit |
| Generated source | Representative supported workflows | compile/rustfmt/clippy/static scan pass | Static |
| Trybuild non-empty | Empty and non-empty fixture dirs | Empty fails; real compile-fail fixtures fail compilation | Static |
| Journal signature | Basic and node-family workflows | Normalized event signature parity | Integration |
| Final gate | Full workspace after implementation | `moon ci` pass or exact unrelated/global debt classification | Workspace |

## 10. Open Questions / Acceptance Traps

- If `Collect*` remains unsupported, the plan permits an exact fail-closed test but does not consider `vb-qi37.10` complete unless contract acceptance is revised by an approved scope decision.
- `Contains`, `StartsWith`, and `EndsWith` may remain fail-closed only with exact rejection and blocker/scope evidence; this is executable acceptance for rejection, not formal coverage.
- `compare_generated_to_ir` in `src/lib.rs` is currently a static guard per codebase map; tests must not treat it alone as semantic parity evidence.
- Do not write tests at repository root. Keep codegen tests under `crates/vb_codegen` and cross-crate tests under `crates/workspace_tests` only if needed.

## 11. State Transition

State 7 exit artifact is this file: `.beads/vb-qi37.10/test-plan.md`.

Exact next transition after this artifact is accepted: **go-skill State 8 — test-reviewer reviews `.beads/vb-qi37.10/test-plan.md` before any State 9 test writing or implementation work.**
