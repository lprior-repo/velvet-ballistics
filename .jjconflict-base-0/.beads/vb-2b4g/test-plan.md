# Test Plan: vb-2b4g — Repeat/Reduce/Together/Collect generated-vs-runtime parity

## Startup / Authority

- Test-planner startup files read:
  - `/home/lewis/.claude/skills/test-planner/SKILL.md` lines 8-10: this role writes only `test-plan.md` and does not write implementation or test code.
  - `/home/lewis/.agents/skills/test-planner/SKILL.md` lines 8-10: same rule; this file wins on conflict. No conflict found.
  - `/home/lewis/.agents/skills/test-planner/references/testing-philosophy.md` lines 5-10, 12-16, 82-86, 106-116: test behavior through public APIs, use acceptance tests as executable specification, require automated evidence for every cared-about behavior, and reject weak `is_ok()`/`is_err()` tests.
- Bead inputs read:
  - `.beads/vb-2b4g/contract.md`: narrows inherited scope to real generated support and parity for `Repeat*`, `Reduce*`, `Together*`, and `Collect*` using `vb_runtime::engine::drive::drive_deterministic_full` as the oracle.
  - `.beads/vb-2b4g/traceability-matrix.jsonl`: maps each pre/postcondition/invariant to focused parity/static commands.
  - `.beads/vb-2b4g/codebase-map.md`: identifies runtime oracle modules, active generated-code surface, and known incomplete `TogetherJoin`/`CollectNext` risks.
  - `.beads/vb-qi37.10/test-plan.md`: inherited scenario expectations for generated final IR coverage and parity.

## Summary

- Behaviors identified: 17 focused acceptance behaviors.
- Trophy allocation: 1 unit/static-admission group / 14 integration parity groups / 2 static-command groups / 0 e2e. This intentionally skews integration-heavy because acceptance requires executing generated code and comparing it to the real runtime oracle.
- Proptest invariants: 5 recommended for extracted pure comparison/building helpers; they do not replace executable parity.
- Fuzz targets: 0 required for this bead unless implementation adds a new parsing/deserialization boundary.
- Kani/TLA+/Verus harnesses: 0 acceptance harnesses for this test-planning bead.
- Mutation threshold: focused `cargo-mutants` on touched `vb_codegen` generated-family surfaces must kill >=90% of non-equivalent mutants, and every critical mutant listed below must be killed by a named test.

## Non-Negotiable Oracle Rules

1. All `Repeat*`, `Reduce*`, `Together*`, and `Collect*` parity tests must invoke `vb_runtime::engine::drive::drive_deterministic_full` as the runtime oracle.
2. `vb_core::run_until_blocked` is forbidden as an oracle for these families because it can return `UnsupportedPrimitive { primitive: "not_yet_implemented" }`.
3. Any oracle result, generated result, helper comparison, or support check that treats `not_yet_implemented` as success is a failing test and a contract failure.
4. Parity means exact semantic parity across terminal result/error variant and fields, pc, slots, taints, step states, family counters/state, collect page state, and normalized journal signature.
5. Tests may use shared builders, but each scenario must assert exact values or exact typed errors. `is_ok()` or `is_err()` alone is forbidden.

## 1. Behavior Inventory

1. Parity harness initializes generated and runtime executions with identical workflow digest, constants, slots, inputs, limits, deterministic seeds, and action IDs.
2. Parity harness rejects `not_yet_implemented` from either side and reports the offending family/scenario.
3. Generated support admission for target families is decided in active `crates/vb_codegen/src/lib.rs` before emission; no inactive duplicate path can satisfy acceptance.
4. Repeat generated execution matches runtime on first attempt state, routing, pc, slots, taints, and journal signature.
5. Repeat generated execution matches runtime on later attempt state and attempt counter progression.
6. Repeat generated execution routes to exhaustion/done exactly like runtime when attempt limit is reached.
7. Repeat generated execution returns runtime-compatible typed counter/capacity errors and preserves taint on `RepeatFinish`.
8. Reduce generated execution matches runtime on empty input accumulator/output behavior.
9. Reduce generated execution matches runtime on single item and multi item accumulator/item binding/materialization order.
10. Reduce generated execution returns exact runtime-compatible type and capacity errors and preserves accumulator/output taint and journal signature.
11. Together generated execution matches runtime branch routing and deterministic branch result order.
12. Together generated execution matches runtime final append/join state, failure policy, taint join, capacity errors, and journal signature.
13. Collect generated execution matches runtime single-page and multi-page page state, materialization order, pc, slots, taints, and journal signature.
14. Collect generated execution rejects duplicate, stale, and out-of-order pages exactly like runtime with exact typed error fields.
15. Collect generated execution returns exact capacity errors and preserves page lineage/final taint.
16. Generated source for target-family workflows satisfies repository static contract: no `unsafe`, `unwrap`, `expect`, `panic`, `todo`, `unimplemented`, `dbg`, unchecked indexing/slicing/casts/arithmetic, runtime YAML/JSON/HTTP, string action/reference lookup, or emitted `not_yet_implemented` laundering.
17. Focused commands provide raw evidence for family parity, source contract, trybuild, check, fmt, and final `moon ci` at landing.

## 2. Trophy Allocation

| Group | Layer | Planned location | Rationale |
|---|---|---|---|
| Oracle guard and admission | Unit/static + integration harness | `crates/vb_codegen/src/tests.rs` | Prevents false acceptance via `run_until_blocked`, unsupported stubs, or inactive path changes. |
| Repeat parity | Integration | `crates/vb_codegen/src/tests.rs` | Requires emitted Rust and runtime oracle to agree on state/counters/taint/journal. |
| Reduce parity | Integration | `crates/vb_codegen/src/tests.rs` | Requires real list/reducer semantics and typed errors. |
| Together parity | Integration | `crates/vb_codegen/src/tests.rs` | Known incomplete join behavior; branch order/final append cannot be proved by static strings. |
| Collect parity | Integration | `crates/vb_codegen/src/tests.rs` | Highest stateful risk: page side state, duplicate/stale detection, materialization order. |
| Generated source contract | Static/integration | `crates/vb_codegen/src/tests.rs`, trybuild | Enforces generated Rust safety/governance constraints. |
| Focused gates | Static/workspace | command evidence | Required acceptance and landing evidence. |

## 3. BDD Scenarios

All scenarios are failure-first. Test-writer should add tests before implementation fixes. Tests must compare generated executable output to `vb_runtime::drive_deterministic_full`, not `vb_core::run_until_blocked`.

### 3.1 Oracle guard and harness identity

Placement: `crates/vb_codegen/src/tests.rs`.

- `family_parity_harness_uses_runtime_drive_deterministic_full_for_target_families`
  - Given representative workflows for `Repeat*`, `Reduce*`, `Together*`, and `Collect*`.
  - When the parity harness constructs the oracle execution.
  - Then the harness calls `vb_runtime::engine::drive::drive_deterministic_full` and never calls `vb_core::run_until_blocked` for these families.
  - And generated/runtime initialization uses identical workflow digest, constants, slots, inputs, limits, deterministic seeds, and action IDs.
- `family_parity_harness_fails_when_oracle_returns_not_yet_implemented`
  - Given a deliberately unsupported target-family workflow or injected oracle result equivalent to `UnsupportedPrimitive { primitive: "not_yet_implemented" }`.
  - When parity comparison runs.
  - Then the test fails with exact diagnostic naming the scenario and family; it must not record parity success.
- `target_family_admission_fails_closed_before_emission_when_unsupported`
  - Given a workflow containing each target node kind.
  - When `validate_generated_subset` / active `crates/vb_codegen/src/lib.rs` admission runs.
  - Then unsupported families return exact typed `UnsupportedIr` naming the family before source emission, while supported families require matching parity test ownership.

### 3.2 Repeat parity — `RepeatStart`, `RepeatAttempt`, `RepeatCheck`, `RepeatFinish`

Placement: `crates/vb_codegen/src/tests.rs`. Command owner: `cargo test -p vb_codegen repeat_generated_parity -- --nocapture`.

- `repeat_generated_parity_matches_runtime_for_first_attempt_state_and_routing`
  - Given a validated repeat workflow with attempt limit > 1, clean input slot, explicit attempt state slot, and deterministic seed.
  - When generated code and `drive_deterministic_full` execute one successful first-attempt path.
  - Then terminal value/error, final pc, attempt counter, route target, step states, slots, taints, and normalized journal signature match exactly.
- `repeat_generated_parity_matches_runtime_for_later_attempt_state_and_routing`
  - Given repeat state already containing a prior attempt and a runtime-equivalent retry path.
  - When both executions continue.
  - Then attempt counter increments with checked arithmetic, the next route matches runtime, and journal records equivalent attempt-state events.
- `repeat_generated_parity_routes_to_exhaustion_when_attempt_limit_reached`
  - Given attempt count at the finite explicit limit.
  - When `RepeatCheck` runs.
  - Then generated and runtime both route to the exhausted/done branch with exact pc/step-state parity and no extra attempt write.
- `repeat_generated_parity_returns_typed_error_when_attempt_counter_or_capacity_exceeds_limit`
  - Given near-bound counter/capacity state that runtime rejects.
  - When `RepeatAttempt` or `RepeatCheck` runs.
  - Then generated returns the same typed error variant and fields as runtime; no wrapping, saturating, panic, or generic semantic error is allowed unless runtime emits the same typed result.
- `repeat_generated_parity_preserves_taint_on_repeat_finish`
  - Given `Clean`, `DerivedFromSecret`, and `Secret` attempt outputs in separate subcases.
  - When `RepeatFinish` materializes final output.
  - Then generated final slot/result taint equals runtime exactly and journal signature includes the same taint.

### 3.3 Reduce parity — `ReduceStart`, `ReduceNext`, `ReduceFinish`

Placement: `crates/vb_codegen/src/tests.rs`. Command owner: `cargo test -p vb_codegen reduce_generated_parity -- --nocapture`.

- `reduce_generated_parity_matches_runtime_for_empty_input`
  - Given an empty list input and explicit reducer limits.
  - When generated and runtime reduce execute.
  - Then accumulator initialization, terminal output, pc, slots, taints, step states, and journal signature match runtime exactly.
- `reduce_generated_parity_matches_runtime_for_single_item_input`
  - Given a one-item list with known value/taint.
  - When the reducer executes.
  - Then item binding, accumulator state, output value, output taint, pc, and journal match runtime exactly.
- `reduce_generated_parity_matches_runtime_for_multiple_items_in_order`
  - Given a multi-item list whose result changes if order is reversed.
  - When generated and runtime reduce execute to finish.
  - Then per-iteration item binding, accumulator progression, materialized output, and journal event order match runtime.
- `reduce_generated_parity_returns_type_error_for_non_list_input`
  - Given scalar/object input where runtime expects a list.
  - When `ReduceStart` runs.
  - Then generated returns the exact runtime-compatible type error variant and fields.
- `reduce_generated_parity_returns_capacity_error_when_accumulator_or_output_exceeds_limit`
  - Given reducer input that exceeds explicit materialization/store capacity.
  - When reduction executes.
  - Then generated and runtime return the same capacity error; no panic or silent truncation is allowed.
- `reduce_generated_parity_preserves_accumulator_and_finish_taint`
  - Given mixed-taint list elements and accumulator inputs.
  - When reduce finishes.
  - Then generated accumulator/output taint lattice result and journal taint fields match runtime exactly.

### 3.4 Together parity — `TogetherStart`, `TogetherBranch`, `TogetherJoin`

Placement: `crates/vb_codegen/src/tests.rs`. Command owner: `cargo test -p vb_codegen together_generated_parity -- --nocapture`.

- `together_generated_parity_routes_branches_in_runtime_order`
  - Given a together workflow with at least three branches whose values expose ordering.
  - When both executions run.
  - Then generated branch route order, branch pc sequence, step states, and journal branch events match runtime semantic order.
- `together_generated_parity_preserves_branch_result_order`
  - Given branches returning distinct values.
  - When `TogetherJoin` materializes the aggregate.
  - Then final list/object/aggregate order equals runtime exactly, not sorted/reversed/hash order.
- `together_generated_parity_appends_final_join_result_like_runtime`
  - Given successful branches and an existing aggregate state.
  - When the final branch joins.
  - Then generated performs the same final append/join transition as runtime, with exact final pc, slots, step states, and journal signature.
- `together_generated_parity_matches_runtime_for_branch_typed_error_policy`
  - Given one branch returns a typed error while others can succeed.
  - When both executions run.
  - Then generated terminal error variant/fields and partial state/journal semantics match runtime failure policy exactly.
- `together_generated_parity_returns_capacity_error_when_branch_count_or_result_store_exceeds_limit`
  - Given fanout/result count beyond explicit limits.
  - When `TogetherStart`/`TogetherJoin` runs.
  - Then generated returns the same capacity/fanout error as runtime and does not allocate beyond contract.
- `together_generated_parity_preserves_join_taint_lattice`
  - Given branch outputs with clean/derived/secret taints.
  - When join completes.
  - Then generated final aggregate taint and journal taints match runtime exactly.

### 3.5 Collect parity — `CollectStart`, `CollectPage`, `CollectNext`, `CollectFinish`

Placement: `crates/vb_codegen/src/tests.rs`. Command owner: `cargo test -p vb_codegen collect_generated_parity -- --nocapture`.

- `collect_generated_parity_matches_runtime_for_single_page`
  - Given a collect workflow with one terminal page.
  - When generated and runtime collect execute.
  - Then page state, materialized items, final pc, slots, taints, step states, and journal signature match exactly.
- `collect_generated_parity_matches_runtime_for_multiple_pages_in_order`
  - Given two or more pages with values that expose order.
  - When `CollectPage`/`CollectNext` proceed to finish.
  - Then generated page cursors, next-page state, materialization order, and journal event order match runtime.
- `collect_generated_parity_returns_empty_collection_like_runtime`
  - Given a valid empty terminal page/collection.
  - When collect finishes.
  - Then generated output value, taint, page state, pc, and journal match runtime exactly.
- `collect_generated_parity_rejects_duplicate_page_like_runtime`
  - Given a duplicate page token/id already observed in collect state.
  - When generated and runtime handle the duplicate.
  - Then generated returns the exact duplicate-page error variant and fields, with matching state/journal semantics.
- `collect_generated_parity_rejects_stale_page_like_runtime`
  - Given a stale prior page after state has advanced.
  - When `CollectPage` runs.
  - Then generated returns the exact stale-page error variant and fields and does not materialize stale items.
- `collect_generated_parity_rejects_out_of_order_page_like_runtime`
  - Given a future/out-of-order page that runtime rejects.
  - When collect advances.
  - Then generated emits the same typed page-order error and journal mismatch is not laundered as success.
- `collect_generated_parity_returns_capacity_error_when_item_or_page_limit_exceeded`
  - Given pages whose item count/page count exceeds explicit limits.
  - When collect executes.
  - Then generated and runtime return identical capacity error fields; no unchecked indexing/slicing/allocation path is allowed.
- `collect_generated_parity_preserves_page_lineage_and_final_taint`
  - Given page items with mixed taints and lineage metadata observable in runtime state/signature.
  - When `CollectFinish` materializes output.
  - Then generated output taint, page lineage essentials, and journal signature match runtime exactly.

### 3.6 Normalized journal signature parity

Placement: `crates/vb_codegen/src/tests.rs` or shared helper used by each family suite.

- `journal_signature_generated_parity_matches_repeat_reduce_together_collect_observables`
  - Given representative success and error workflows for each family.
  - When generated and runtime journal observations are normalized.
  - Then event kind/order, step id, slot id, value kind, taint, family state/counter essentials, collect page essentials, and terminal event match exactly.
- `journal_signature_generated_parity_reports_exact_mismatch_dimension`
  - Given an intentionally perturbed generated signature in the comparison helper.
  - When comparison runs.
  - Then the failure reports exact dimension among result, error, pc, slots, taints, step state, attempts/counters, collect page state/materialization, or journal.

### 3.7 Generated source static contract

Placement: `crates/vb_codegen/src/tests.rs`; compile-fail fixtures only if implementation touches trybuild surfaces.

- `generated_source_contract_checks_repeat_reduce_together_collect_workflows`
  - Given representative generated source for all target families.
  - When static source contract scan runs.
  - Then source contains no forbidden constructs, no `not_yet_implemented`, no unsupported stubs, no string action/reference lookup, and no runtime YAML/JSON/HTTP path.
- `generated_source_contract_contains_no_unchecked_indexing_slicing_casts_or_arithmetic_for_target_families`
  - Given emitted family source.
  - When static scan runs.
  - Then unchecked `[]` indexing/slicing, ` as ` casts, and unchecked arithmetic patterns are rejected unless a repository-approved checked helper makes the operation safe and explicit.
- `generated_source_contract_compiles_and_formats_representative_target_family_source`
  - Given emitted representative workflows.
  - When compile/rustfmt/check gates run.
  - Then generated source compiles and formats without relying on panic/unwrap/unsafe escapes.

## 4. Proptest Invariants

These are recommended only if implementation exposes pure helper/comparison surfaces. They are not acceptance substitutes for executable generated-vs-runtime parity.

1. `target_family_observable_comparison_prop`
   - Invariant: equality requires all observable dimensions to match: terminal result/error, pc, slots, taints, step states, family counters/state, collect page state, and normalized journal signature.
   - Strategy: bounded generated pairs of synthetic observable summaries.
   - Anti-invariant: dropping any dimension must produce a mismatch.
2. `repeat_attempt_counter_checked_arithmetic_prop`
   - Invariant: incrementing attempts either matches runtime next counter or returns exact typed overflow/capacity error; never wraps silently.
   - Strategy: counters near 0, 1, limit-1, limit, and max bound.
   - Anti-invariant: saturating/wrapping behavior when runtime errors must fail.
3. `reduce_together_collect_materialization_order_prop`
   - Invariant: materialized output order equals runtime/source order for bounded lists/branches/pages.
   - Strategy: small non-empty sequences with distinct values.
   - Anti-invariant: reverse, sort, deduplicate, or hash-order materialization must fail.
4. `taint_lattice_generated_matches_runtime_prop`
   - Invariant: generated taint joins/copies match runtime for `Clean`, `DerivedFromSecret`, and `Secret` contributors.
   - Strategy: all taint combinations across final output contributors.
   - Anti-invariant: weakening secret/derived taint or inventing secret for clean-only inputs must fail.
5. `collect_page_state_monotonicity_prop`
   - Invariant: accepted page state advances monotonically according to runtime, while duplicate/stale/out-of-order pages return exact typed errors.
   - Strategy: bounded page-id/token sequences with valid, duplicate, stale, and future cases.
   - Anti-invariant: accepting duplicate/stale pages or materializing them must fail.

## 5. Fuzz Targets

No required fuzz target is introduced by this bead because the target is generated-vs-runtime execution parity, not a new raw parsing/deserialization boundary.

Conditional only: if implementation adds a raw generated-workflow/source comparison parser or new fuzz file, add/build the relevant fuzz target and run `cargo fuzz build <target>`. Fuzz success must not be claimed as parity proof.

## 6. Kani / TLA+ / Verus Harnesses

No Kani, TLA+, or Verus acceptance harness is planned for `vb-2b4g`. The contract explicitly scopes this specialist away from formal proof artifacts. Do not add vacuous formal harnesses, and do not weaken runtime parity based on formal deferral.

## 7. Mutation Checkpoints

Minimum focused mutation kill target: >=90% for touched `vb_codegen` generated-family surfaces; all critical mutants below must be killed.

| Mutant | Must be killed by |
|---|---|
| Replace `drive_deterministic_full` oracle with `vb_core::run_until_blocked`. | `family_parity_harness_uses_runtime_drive_deterministic_full_for_target_families`; all family parity suites. |
| Treat `UnsupportedPrimitive { primitive: "not_yet_implemented" }` as pass/equivalent. | `family_parity_harness_fails_when_oracle_returns_not_yet_implemented`. |
| Mark `Repeat*` supported while emitting unsupported/no-op stubs. | Repeat parity suite; generated source contract. |
| Wrap/saturate repeat attempt counter incorrectly. | `repeat_generated_parity_returns_typed_error_when_attempt_counter_or_capacity_exceeds_limit`. |
| Special-case reduce empty input to wrong initial output. | `reduce_generated_parity_matches_runtime_for_empty_input`. |
| Reverse or skip reducer item binding/materialization. | `reduce_generated_parity_matches_runtime_for_multiple_items_in_order`. |
| Accept non-list reducer input. | `reduce_generated_parity_returns_type_error_for_non_list_input`. |
| Reverse Together branch result order or omit final append. | `together_generated_parity_preserves_branch_result_order`; `together_generated_parity_appends_final_join_result_like_runtime`. |
| Ignore Together fanout/result capacity. | `together_generated_parity_returns_capacity_error_when_branch_count_or_result_store_exceeds_limit`. |
| Accept duplicate/stale/out-of-order Collect pages. | Collect duplicate/stale/out-of-order tests. |
| Materialize Collect pages in wrong order or drop empty page semantics. | Collect single/multiple/empty tests. |
| Drop or invent taint in any family finish/join/materialization. | Family taint tests and `taint_lattice_generated_matches_runtime_prop`. |
| Omit journal event, taint, counter, page, or terminal event from signature. | Journal signature parity tests. |
| Emit `unsafe`, `unwrap`, `expect`, `panic`, `todo`, `unimplemented`, `dbg`, unchecked `[]`/`as`/arithmetic, YAML/JSON/HTTP, or string lookup. | Generated source contract tests; trybuild if fixtures are touched. |

## 8. Combinatorial Coverage Matrix

| Scenario | Input class | Expected output | Layer |
|---|---|---|---|
| Harness identity | Same workflow/inputs/limits/seeds/action IDs | Generated and runtime initialized identically | Integration |
| Oracle guard | Runtime/core unsupported sentinel | Exact failure; no `not_yet_implemented` pass | Integration |
| Admission unsupported | Unsupported target family before implementation | Exact typed unsupported before emission | Unit/static |
| Repeat first attempt | Valid first attempt | Exact result/error, pc, slots, taints, attempt state, journal parity | Integration |
| Repeat later attempt | Prior attempt state | Exact counter/routing/state/journal parity | Integration |
| Repeat exhausted | Attempt count at limit | Exact exhausted/done routing parity | Integration |
| Repeat overflow/capacity | Near-bound counter/store | Exact typed error fields | Integration |
| Repeat taint | Clean/derived/secret output | Exact final taint parity | Integration |
| Reduce empty | Empty list | Exact accumulator/output/journal parity | Integration |
| Reduce single | One item | Exact item binding/output/taint parity | Integration |
| Reduce multi | Distinct ordered items | Exact iteration/materialization order parity | Integration |
| Reduce wrong type | Non-list input | Exact runtime-compatible type error | Integration |
| Reduce capacity | Accumulator/output over limit | Exact capacity error | Integration |
| Together branch order | >=3 distinct branches | Exact route/result order parity | Integration |
| Together final append | Last branch joins aggregate | Exact final append/join state parity | Integration |
| Together branch error | One branch typed error | Exact failure policy/error fields | Integration |
| Together capacity | Fanout/result over limit | Exact capacity/fanout error | Integration |
| Together taint | Mixed-taint branch outputs | Exact join taint parity | Integration |
| Collect single page | One terminal page | Exact page/output/journal parity | Integration |
| Collect multi-page | Ordered pages | Exact cursor/materialization order parity | Integration |
| Collect empty | Empty terminal collection | Exact empty output/page state parity | Integration |
| Collect duplicate | Repeated page token/id | Exact duplicate-page error fields | Integration |
| Collect stale | Prior page after advancement | Exact stale-page error fields | Integration |
| Collect out-of-order | Future page before expected page | Exact page-order error fields | Integration |
| Collect capacity | Page/item count over limit | Exact capacity error fields | Integration |
| Collect taint/lineage | Mixed-taint pages | Exact output taint and lineage essentials | Integration |
| Journal | Success and error per family | Exact normalized signature parity | Integration |
| Generated source | Representative target-family source | Static scan/compile/format pass | Static |
| Focused commands | Whole bead evidence | Required command stdout captured | Static/workspace |

## 9. Required Commands / Evidence Gates

Focused gates required for this bead after tests/implementation exist:

```bash
cargo test -p vb_codegen repeat_generated_parity -- --nocapture
cargo test -p vb_codegen reduce_generated_parity -- --nocapture
cargo test -p vb_codegen together_generated_parity -- --nocapture
cargo test -p vb_codegen collect_generated_parity -- --nocapture
cargo test -p vb_codegen generated_source_contract -- --nocapture
cargo test -p vb_codegen --test trybuild_tests
cargo check -p vb_codegen --all-targets
cargo fmt --all -- --check
```

Final landing gate remains:

```bash
moon ci
```

Static grep/review gates that must be represented by tests or review evidence:

- Target-family parity helpers must reference `drive_deterministic_full`.
- Target-family parity helpers must not reference `run_until_blocked` as oracle.
- Generated target-family source and tests must not launder `not_yet_implemented` as success.
- Active implementation changes must touch `crates/vb_codegen/src/lib.rs` path for admission/emission, not only duplicate inactive files.

## 10. Highest-Risk Tests

1. `collect_generated_parity_rejects_duplicate_page_like_runtime` and `collect_generated_parity_rejects_stale_page_like_runtime` — highest state-machine risk; codebase map flags `CollectNext` as incomplete and runtime has subtle page side state.
2. `together_generated_parity_appends_final_join_result_like_runtime` — codebase map flags `TogetherJoin` as incomplete; easy to pass branch success while losing final append semantics.
3. `family_parity_harness_fails_when_oracle_returns_not_yet_implemented` — prevents the known false-green path from `vb_core::run_until_blocked`/unsupported sentinels.
4. `repeat_generated_parity_returns_typed_error_when_attempt_counter_or_capacity_exceeds_limit` — catches unsafe arithmetic/wrap/saturate divergence.
5. `reduce_generated_parity_matches_runtime_for_multiple_items_in_order` — catches item binding/order bugs that shallow tests miss.
6. `journal_signature_generated_parity_matches_repeat_reduce_together_collect_observables` — catches silent semantic mismatches after terminal values appear correct.

## 11. Open Questions / Acceptance Traps

- Exact public error variant names/fields must be taken from current `vb_runtime`/`vb_codegen` types during test writing; this plan intentionally requires exact typed assertions rather than guessing names.
- If any target family remains unsupported, only exact fail-closed admission tests may pass for that family; bead acceptance cannot claim generated parity complete without an approved scope change.
- Static/source-substring tests alone never satisfy parity. Every target family requires executable generated-vs-`drive_deterministic_full` evidence.
- No test or production files should be changed by this State 7 planner. Next state is test-reviewer review of this plan before any State 9 test writing.
