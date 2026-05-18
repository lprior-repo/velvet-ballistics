# Test Plan: vb-f04l — Safe v1 Primitive Source Lowering

## Startup Sources Cited

- `/home/lewis/.claude/skills/test-planner/SKILL.md` lines 8-10 require this role to write only `test-plan.md` and not implementation or test code.
- `/home/lewis/.claude/skills/test-planner/SKILL.md` lines 41-171 require behavior inventory, trophy allocation, BDD scenarios, proptest invariants, fuzz targets, Kani harnesses, mutation checkpoints, and exact-value/error assertions.
- `/home/lewis/.agents/skills/test-planner/SKILL.md` lines 8-10 and 41-171 contain the same requirements; no conflict observed, and the agents copy controls if a conflict exists.
- `/home/lewis/.agents/skills/test-planner/references/testing-philosophy.md` lines 5-16 require behavior-focused public API tests and ATDD separation of intent from implementation.
- `/home/lewis/.agents/skills/test-planner/references/testing-philosophy.md` lines 32-50 require the Testing Trophy with integration as the widest layer.
- `/home/lewis/.agents/skills/test-planner/references/testing-philosophy.md` lines 106-116 reject shallow `is_ok()`/`is_err()`, brittle interaction tests, and sleep-based tests.

## Summary

- Behaviors identified: 42 traceability-backed behaviors.
- Trophy allocation: 18 unit / 18 integration / 3 E2E-acceptance / 3 static gates, plus cross-cutting property, fuzz, Kani, mutation, and formal rerun gates.
- Proptest invariants: 12 required property groups.
- Fuzz targets: 2 boundary groups; one is an existing parser boundary regression, one is conditional if new raw parsing/string-to-index logic appears.
- Kani harnesses/checkpoints: 4 concrete-candidate harness groups, despite current proof-plan waiver; required if implementation introduces unchecked casts/arithmetic/indexing or if Verus bridge is insufficient.
- Mutation threshold: >= 90% killed overall, 100% killed for the seven newly accepted primitive dispatch/shape/error branches.
- Assertion rule: no planned test may assert only `is_ok()` or `is_err()`; every scenario must assert exact `CompiledNodeKind`, target/slot/index values, digest equality, or exact `CompileError` variant and payload.

## Inputs Reviewed

- Approved State 6 proof review: `.beads/vb-f04l/proof-review.md` (`STATUS: APPROVED`).
- Approved State 6 contract verification review: `.beads/vb-f04l/contract-verification-review.md` (`STATUS: APPROVED`).
- Contract: `.beads/vb-f04l/contract.md`.
- Traceability: `.beads/vb-f04l/traceability-matrix.jsonl` with 42 rows.
- Proof obligations: `.beads/vb-f04l/proof-obligations.jsonl` and `.beads/vb-f04l/proof-obligations.planned.jsonl`.
- Scope: `.beads/vb-f04l/delivery-scope.jsonl`.
- Isolation command run: `pwd -P && test "$(pwd -P)" = "/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-f04l" && case "$(pwd -P)" in "/home/lewis/src/velvet-ballistics"|"/home/lewis/src/velvet-ballistics"/*) exit 1;; *) exit 0;; esac` -> exit 0, output `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-f04l`.

## 1. Behavior Inventory

| ID | Trace | Behavior | Primary layer |
|---|---|---|---|
| B01 | PRE-001 / ERR-001 | `compile_source` rejects an empty canonical source with `CompileError::EmptySteps` before IR construction. | unit + proptest |
| B02 | PRE-002 | canonical admission accepts only `velvet-ballastics/v1` plus supported canonical trigger forms. | integration + proptest |
| B03 | PRE-003 / ERR-002 | unsupported top-level declarations and top-level `result` return exact unsupported top-level errors. | unit + proptest |
| B04 | PRE-004 / ERR-004 | duplicate top-level or nested step IDs return `DuplicateStepId` before runtime validation. | integration + proptest + Kani checkpoint |
| B05 | PRE-005 / ERR-003 | unsupported step control fields return `UnsupportedStepControlField` unless a future primitive contract consumes them. | unit + proptest |
| B06 | PRE-006 / ERR-007 | empty or malformed variable/output/branch/loop/prompt/event/reference fields return `StepFieldShape`. | unit + proptest |
| B07 | PRE-007 / ERR-008 | primitive bounds are checked before narrowing/allocation and return range/limit errors. | unit + proptest + Kani |
| B08 | POST-001 / INV-006 / ERR-011 | each in-scope primitive (`ForEach`, `Together`, `Collect`, `Reduce`, `Repeat`, `Wait`, `Ask`) lowers instead of returning `UnsupportedStepPrimitive`. | integration + E2E |
| B09 | POST-002 / ERR-009 | successful lowering passes shared validation and validation failures are wrapped as `CompileError::Workflow`. | integration |
| B10 | POST-003 / INV-005 | equal canonical `WorkflowSource` values produce deterministic digest and IR. | integration + proptest |
| B11 | POST-004 / INV-001 / INV-002 | every emitted node ID is dense and every target is in range. | integration + proptest + Kani |
| B12 | POST-005 / INV-003 | `slot_count` covers every emitted slot reference with deterministic no-gap allocation. | integration + proptest + Kani |
| B13 | POST-006 | valid `ForEach` source lowers to bounded loop graph with body, next/join, done, and finish route. | integration + property |
| B14 | POST-007 | valid `Together` source lowers to bounded fan-out/fan-in graph whose join waits for exactly branch count. | integration + property |
| B15 | POST-008 | valid `Collect` source lowers to bounded paginated graph preserving collector slot and completion route. | integration + property |
| B16 | POST-009 | valid `Reduce` source lowers to bounded reduction graph preserving input, accumulator, initial value, body, and finish route. | integration + property |
| B17 | POST-010 | valid `Repeat` source lowers to bounded retry graph preserving attempts, body route, check/finish, and exhaustion. | integration + property |
| B18 | POST-011 | valid `Wait` source lowers to exactly one legal `WaitUntil` or `WaitEvent` shape; invalid event/timeout forms are rejected. | integration + unit |
| B19 | POST-012 | valid `Ask` source lowers to paired `Ask`/`AskResume` nodes with prompt, answer slot, optional timeout, and resume target. | integration + unit |
| B20 | POST-013 / ERR-005 / ERR-006 | existing `Set` and terminal `Finish` behavior remains compatible, including duplicate and unknown output diagnostics. | regression integration |
| B21 | POST-014 / INV-010 | legacy compiler files and tests are not deleted without a reviewed migration bead. | static gate |
| B22 | INV-004 | bounds near numeric limits either produce valid IR or exact checked errors; never wrap, truncate, panic, or allocate unboundedly. | proptest + Kani |
| B23 | INV-007 | every newly accepted primitive has at least one positive and one negative executable scenario. | coverage gate |
| B24 | INV-008 | production lowering code has no `unsafe`, `unwrap`, `expect`, `panic`, `todo`, `unimplemented`, `dbg`, unchecked indexing/slicing/casts/arithmetic. | static gate |
| B25 | INV-009 | runtime crates remain free of YAML/JSON/HTTP dependencies; YAML stays in compiler boundary. | static gate |
| B26 | ERR-010 | `YamlCompiler::compile` maps canonical YAML parse rejection to `CompileError::CanonicalYaml`. | integration + fuzz boundary |
| B27 | ERR-011 | out-of-scope `Save`, `Do`, and `Choose` remain the only legal `UnsupportedStepPrimitive` results. | integration |
| B28 | PRE-006 / POST-006 | malformed `ForEach` input/source/body shape fails with `StepFieldShape`; valid shape emits exact loop nodes. | unit + integration |
| B29 | PRE-006 / POST-007 | malformed `Together` branches fail with `StepFieldShape`; valid branches emit exact fan-out/fan-in nodes. | unit + integration |
| B30 | PRE-006 / POST-008 | malformed `Collect` pagination/body fields fail; valid fields emit exact pagination nodes. | unit + integration |
| B31 | PRE-006 / POST-009 | malformed `Reduce` input/initial/body fields fail; valid fields emit exact reduction nodes. | unit + integration |
| B32 | PRE-006 / POST-010 | malformed `Repeat.max_attempts` or body fails; valid fields emit exact retry nodes. | unit + integration |
| B33 | PRE-006 / POST-011 | ambiguous `Wait` event/deadline/timeout combinations fail; valid forms emit exact wait node kind. | unit + integration |
| B34 | PRE-006 / POST-012 | missing `Ask.prompt` or invalid answer target fails; valid prompt emits exact ask/resume pair. | unit + integration |
| B35 | POST-004 / POST-006..012 | nested body expansion preserves dense preorder node allocation across all scoped primitives. | property + Kani |
| B36 | POST-005 / POST-008..012 | collector, accumulator, prompt, answer, timeout, iterator, and result slots are all counted. | property + Kani |
| B37 | POST-002 / POST-004 | compiler validation bridge rejects intentionally malformed generated parts and preserves underlying `WorkflowError`. | integration |
| B38 | POST-003 | deterministic traversal is independent of heap allocation order and repeated invocation. | proptest |
| B39 | INV-006 / mutation | primitive dispatch cannot accidentally fall through to unsupported branch. | mutation |
| B40 | ERR-001..011 / mutation | every error variant branch is killed by exact variant tests. | mutation |
| B41 | POST-006..012 / formal rerun | TLA+ and Verus approved model/proof reruns remain green after implementation changes. | formal static |
| B42 | POST-014 / regression | existing parser/control-flow/type-taint tests remain present and passing; no hidden regression by deletion. | static + CI |

## 2. Trophy Allocation

| Layer | Planned coverage | Behaviors | Rationale |
|---|---:|---|---|
| Static / formal gates | 3 static + 1 formal rerun lane | B21, B24, B25, B41, B42 | Cheap safety net for forbidden constructs, dependency boundary, legacy inventory, and approved proof/model reruns. |
| Unit / calc | 18 focused groups | B01, B03, B05, B06, B07, B18, B19, B22, B28-B34, B40 | Exact diagnostics, field validators, and numeric-bound decisions are pure enough for fast exhaustive checks. |
| Integration | 18 groups | B02, B04, B08-B20, B26, B27, B35-B38 | Main risk is public `compile_source` lowering through real `vb_yaml`, `vb_validate`, and `CompiledWorkflow::try_from_parts`. Integration is intentionally widest. |
| E2E / acceptance | 3 black-box workflows | B08, B20, B27 | Compile complete YAML bytes via `YamlCompiler::compile` to prove user-facing path and existing Set/Finish compatibility. |
| Property / fuzz / Kani / mutation overlays | 12 proptest + 2 fuzz + 4 Kani + mutation threshold | B01-B42 as mapped below | These are not separate trophy layers; they strengthen assertions and kill shallow implementations. |

Deviation note: the ratio is near the Testing Trophy target but tilts heavier toward unit/property because this bead is a compiler lowering boundary with many exact error variants and checked numeric invariants.

## 3. BDD Scenarios

### B01 — Empty source rejection (`PRE-001`, `ERR-001`)

`fn compile_source_returns_empty_steps_when_source_has_no_steps()`

- Given: a canonical `WorkflowSource` with version `velvet-ballastics/v1`, supported trigger, and zero steps.
- When: `compile_source(&source)` is called.
- Then: result is `Err(CompileErrors)` containing exactly `CompileError::EmptySteps`.
- And: no `CompiledWorkflow`, nodes, digest, or validation side effects are produced.

### B02 — Canonical admission (`PRE-002`)

`fn compile_source_admits_only_v1_supported_triggers_when_source_is_canonical()`

- Given: valid and invalid canonical sources spanning accepted version/trigger and rejected version/trigger forms.
- When: each source enters `compile_source` or `YamlCompiler::compile` as appropriate.
- Then: accepted cases proceed to scoped lowering or later exact errors; rejected parse/admission cases return the exact canonical admission error, never a primitive lowering placeholder.

### B03 — Unsupported top-level declarations (`PRE-003`, `ERR-002`)

`fn compile_source_returns_unsupported_top_level_error_when_declarations_are_present()`

- Given: otherwise valid sources with one of `inputs`, `vars`, `secrets`, `examples`, or top-level `result` populated.
- When: `compile_source` runs.
- Then: declaration cases return `CompileError::UnsupportedTopLevelDeclaration` naming the field, and `result` returns `CompileError::UnsupportedTopLevelResult`.

### B04 — Duplicate step IDs (`PRE-004`, `ERR-004`)

`fn compile_source_returns_duplicate_step_id_when_top_level_or_nested_ids_repeat()`

- Given: a source with duplicate top-level IDs, and separate sources with duplicates inside each nested body primitive.
- When: `compile_source` runs.
- Then: each case returns `CompileError::DuplicateStepId` with the duplicated ID and scope; no validation-wrapper error masks it.

### B05 — Unsupported step control fields (`PRE-005`, `ERR-003`)

`fn compile_source_returns_unsupported_control_field_when_step_control_field_is_present()`

- Given: scoped primitive steps containing `name`, `if`, `with`, `try_again`, `on_error`, or `then` where no contract extension consumes the field.
- When: `compile_source` runs.
- Then: each case returns `CompileError::UnsupportedStepControlField` naming the exact field.

### B06 — Empty primitive fields (`PRE-006`, `ERR-007`)

`fn compile_source_returns_step_field_shape_when_required_primitive_field_is_empty()`

- Given: each scoped primitive with an empty output, variable, branch label, loop variable, prompt, event, reference, or body where required.
- When: `compile_source` runs.
- Then: each case returns `CompileError::StepFieldShape` with primitive and field context.

### B07 — Bound checks (`PRE-007`, `INV-004`, `ERR-008`)

`fn compile_source_returns_range_or_limit_error_when_primitive_bounds_exceed_ir_width()`

- Given: sources whose branch count, max attempts, page limits, generated steps, slots, constants, or action references exceed `u16`/`u32` target widths.
- When: `compile_source` runs.
- Then: result is one of the exact contracted errors: `StepIndexOutOfRange`, `SlotIndexOutOfRange`, or `PrimitiveLoweringLimitExceeded`.
- And: no panic, truncation, wraparound, or partial valid workflow occurs.

### B08 — In-scope primitive support (`POST-001`, `INV-006`, `ERR-011`)

`fn compile_source_emits_supported_ir_when_each_scoped_primitive_is_valid()`

- Given: seven minimal valid workflows, one each for `ForEach`, `Together`, `Collect`, `Reduce`, `Repeat`, `Wait`, and `Ask`, plus `Finish`.
- When: `compile_source` runs.
- Then: each returns `Ok(CompiledWorkflow)` with at least one expected primitive-specific `CompiledNodeKind`.
- And: none returns `CompileError::UnsupportedStepPrimitive`.

### B09 — Validation bridge (`POST-002`, `ERR-009`)

`fn compile_source_wraps_workflow_error_when_generated_parts_fail_validation()`

- Given: an instrumented or minimal seam that forces generated `WorkflowParts` to violate shared validation after lowering.
- When: `compile_source` reaches the validation bridge.
- Then: it returns `CompileError::Workflow` preserving the underlying `WorkflowError` variant.
- And: success cases prove `vb_validate::shared::validate` and `CompiledWorkflow::try_from_parts` remain on the return path.

### B10 — Determinism (`POST-003`, `INV-005`)

`fn compile_source_returns_equal_digest_and_ir_when_sources_are_equal()`

- Given: two equal canonical `WorkflowSource` values with nested primitives and slots.
- When: both are compiled multiple times.
- Then: digest, entry, resource contract, step names, constants, expressions, accessors, slots, nodes, and `slot_count` are byte-for-byte/equality equal.

### B11 — Dense targets (`POST-004`, `INV-001`, `INV-002`)

`fn compile_source_emits_dense_in_range_targets_when_primitive_lowering_succeeds()`

- Given: successful lowerings for all seven scoped primitives and nested combinations.
- When: emitted nodes and target fields are inspected through public `CompiledWorkflow` accessors.
- Then: for every position `i`, `node.id == StepIdx(i)`.
- And: every `next`, `body`, `done`, `join`, branch, resume, exhausted, or handler target is `< nodes.len()`.

### B12 — Slot coverage (`POST-005`, `INV-003`)

`fn compile_source_sets_slot_count_to_cover_all_references_when_lowering_succeeds()`

- Given: successful lowerings using iterator, collector, accumulator, prompt, answer, timeout, result, and named output slots.
- When: slot references are inspected.
- Then: every slot reference is `< slot_count`.
- And: `slot_count` is zero when no slots exist, otherwise exactly `max(slot_ref) + 1`.

### B13 — ForEach shape (`POST-006`)

`fn compile_source_emits_for_each_loop_graph_when_for_each_body_is_valid()`

- Given: a valid `ForEach` with input/source expression, iterator output, nested body, and following `Finish`.
- When: `compile_source` runs.
- Then: IR contains the expected `ForEachStart` plus body/next/join/done route accepted by validation.
- And: source body execution route completes exactly once into the following step or terminal finish.

### B14 — Together shape (`POST-007`)

`fn compile_source_emits_together_join_when_all_branches_are_valid()`

- Given: a valid `Together` with N branches, each branch having a body and completion.
- When: `compile_source` runs.
- Then: IR contains fan-out branch entries and one join whose count equals N.
- And: join completion routes once to the next source step.

### B15 — Collect shape (`POST-008`)

`fn compile_source_emits_collect_pagination_when_collect_fields_are_valid()`

- Given: valid `Collect` source with page limit, page size, collector slot/output, body, and finish route.
- When: `compile_source` runs.
- Then: IR contains collect start/page/next-or-finish shape preserving collector slot and done route.

### B16 — Reduce shape (`POST-009`)

`fn compile_source_emits_reduce_graph_when_reduce_fields_are_valid()`

- Given: valid `Reduce` source with input, accumulator, initial value, body, and following finish.
- When: `compile_source` runs.
- Then: IR contains reduce start/next/finish shape preserving input slot, accumulator slot, initial constant, body route, and finish route.

### B17 — Repeat shape (`POST-010`)

`fn compile_source_emits_repeat_retry_graph_when_repeat_fields_are_valid()`

- Given: valid `Repeat` with max attempts and body.
- When: `compile_source` runs.
- Then: IR contains repeat start/attempt/check/finish or exhaustion route.
- And: max attempts in IR equals source max attempts and is checked before narrowing.

### B18 — Wait shape (`POST-011`)

`fn compile_source_emits_exact_wait_shape_when_wait_form_is_valid()`

- Given: one deadline-only wait and one event wait with optional timeout.
- When: `compile_source` runs.
- Then: deadline-only emits exactly `WaitUntil`; event form emits exactly `WaitEvent` with valid continuation and optional timeout route.

`fn compile_source_returns_step_field_shape_when_wait_form_is_ambiguous_or_empty()`

- Given: invalid wait combinations such as missing both deadline/event, empty event, or incompatible timeout shape.
- When: `compile_source` runs.
- Then: result is exact `CompileError::StepFieldShape`.

### B19 — Ask shape (`POST-012`)

`fn compile_source_emits_ask_resume_pair_when_prompt_is_valid()`

- Given: valid `Ask` with prompt, answer output, optional timeout, and following finish.
- When: `compile_source` runs.
- Then: IR contains `Ask` and paired `AskResume` nodes with answer slot covered by `slot_count` and resume target in range.

### B20 — Set and Finish regression (`POST-013`, `ERR-005`, `ERR-006`)

`fn compile_source_preserves_set_and_terminal_finish_when_existing_workflow_is_valid()`

- Given: a currently supported workflow using `Set` and terminal `Finish` with named output lookup.
- When: `compile_source` runs before and after primitive-lowering implementation.
- Then: emitted IR and exact named-output behavior remain compatible.

`fn compile_source_returns_duplicate_output_name_when_outputs_repeat()`

- Given: two outputs with the same name in one lowering scope.
- When: `compile_source` runs.
- Then: exact `CompileError::DuplicateOutputName` is returned.

`fn compile_source_returns_unknown_output_name_when_finish_references_absent_output()`

- Given: `Finish` or source expression references an absent output.
- When: `compile_source` runs.
- Then: exact `CompileError::UnknownOutputName` is returned.

### B21 — Legacy inventory (`POST-014`, `INV-010`)

`fn review_diff_preserves_legacy_compiler_files_when_no_migration_bead_exists()`

- Given: the bead diff after implementation.
- When: static inventory checks run.
- Then: legacy compiler files and existing tests identified in `delivery-scope.jsonl` remain present or a reviewed migration bead is cited.

### B22 — Numeric limits (`INV-004`)

`fn compile_source_returns_valid_ir_or_checked_error_when_bounds_are_near_limits()`

- Given: boundary cases at `0`, `1`, `u16::MAX`, `u16::MAX + 1`, `u32::MAX`, and `u32::MAX + 1` where constructible.
- When: `compile_source` runs.
- Then: each case produces exact valid IR or exact range/limit error; never panic or truncation.

### B23 — Primitive coverage matrix (`INV-007`)

`fn primitive_coverage_matrix_lists_positive_and_negative_cases_for_each_scoped_primitive()`

- Given: the implemented test suite metadata or explicit matrix.
- When: coverage matrix check runs.
- Then: every one of the seven in-scope primitives has at least one positive scenario and one negative scenario.

### B24 — Forbidden production constructs (`INV-008`)

`fn static_scan_finds_no_forbidden_constructs_in_production_lowering_diff()`

- Given: modified production files under `crates/vb_compile`.
- When: static scan and `moon ci` run.
- Then: no forbidden constructs exist in production paths.

### B25 — Runtime dependency boundary (`INV-009`)

`fn dependency_scan_keeps_runtime_crates_free_of_yaml_json_http()`

- Given: workspace dependency graph after implementation.
- When: dependency boundary scan runs.
- Then: runtime core crates have no new YAML/JSON/HTTP dependency; parser dependencies remain in `vb_compile`/`vb_yaml` boundary only.

### B26 — YAML compiler parse mapping (`ERR-010`)

`fn yaml_compiler_compile_returns_canonical_yaml_when_source_parse_fails()`

- Given: invalid canonical YAML bytes rejected by `vb_yaml`.
- When: `YamlCompiler::compile(&bytes)` runs.
- Then: exact `CompileError::CanonicalYaml` is returned with parse context preserved.

### B27 — Unsupported primitive policy (`ERR-011`)

`fn compile_source_returns_unsupported_step_primitive_only_for_save_do_choose()`

- Given: valid in-scope primitive workflows and separate out-of-scope `Save`, `Do`, and `Choose` workflows.
- When: `compile_source` runs.
- Then: in-scope primitives never return `UnsupportedStepPrimitive`; out-of-scope primitives return exact `UnsupportedStepPrimitive` naming the primitive.

## 4. Proptest Invariants

| Property ID | Trace | Invariant | Strategy | Anti-invariant |
|---|---|---|---|---|
| P01 | PRE-001 / ERR-001 | Any source with zero steps returns exactly `EmptySteps`. | Generate valid header/trigger with empty step vec. | Non-empty valid Set/Finish source must not return `EmptySteps`. |
| P02 | PRE-003 / ERR-002 | Any unsupported top-level declaration triggers the matching exact top-level error. | Generate one unsupported declaration at a time. | Empty unsupported declarations absent must not trigger top-level error. |
| P03 | PRE-004 / ERR-004 | Duplicate IDs in any scope are rejected before validation. | Generate nested bodies with duplicate/nonduplicate IDs. | Unique IDs in all scopes must not return `DuplicateStepId`. |
| P04 | PRE-005 / ERR-003 | Any unsupported control field returns exact field-specific error. | Generate one control field at a time across primitives. | No unsupported field means no `UnsupportedStepControlField`. |
| P05 | PRE-006 / ERR-007 | Empty required primitive string/reference fields always fail as `StepFieldShape`. | Generate empty/whitespace/minimal non-empty field values if parser exposes them. | Non-empty valid fields must not fail as field shape. |
| P06 | PRE-007 / INV-004 / ERR-008 | Bound conversion is total: valid bound -> exact IR value; overflow bound -> exact range/limit error. | Generate values around `u16`/`u32` edges and generated graph sizes. | No wrapping/truncation allowed. |
| P07 | POST-003 / INV-005 | Equal source values compile to equal digest and equal IR. | Generate valid small workflows including each primitive and nested body depth <= configured bound. | Source with semantically changed step ID/field should change relevant digest/IR. |
| P08 | POST-004 / INV-001 / INV-002 | Successful lowering always emits dense IDs and in-range targets. | Generate valid primitive graphs within bounds. | Any intentionally corrupted target must be rejected by validation, not accepted. |
| P09 | POST-005 / INV-003 | `slot_count == 0` iff no slots; otherwise `slot_count == max_ref + 1`. | Generate workflows with combinations of outputs/iterator/collector/accumulator/prompt/answer/timeout slots. | Any emitted slot `>= slot_count` must be impossible in success path. |
| P10 | POST-006..012 | Primitive shape tags are preserved in emitted node families. | Generate one valid source for each primitive with small bounded bodies/branches. | In-scope primitive must never lower solely to Set/Finish or unsupported placeholder. |
| P11 | POST-013 | Set/Finish regression behavior is stable under adding other primitive support. | Generate Set/Finish-only workflows with output names and finish refs. | Unknown/duplicate output must still exact-error. |
| P12 | ERR-011 | Unsupported policy partitions primitives exactly into in-scope vs out-of-scope. | Generate primitive enum variants. | ForEach/Together/Collect/Reduce/Repeat/Wait/Ask must not produce `UnsupportedStepPrimitive`. |

## 5. Fuzz Targets

### FZ01 — Canonical YAML bytes to `YamlCompiler::compile` (`ERR-010`, `PRE-002`, `PRE-006`)

- Input type: bytes.
- Boundary: existing raw YAML parse/admission path; no new parser should be added in this bead.
- Risk: parser panic, pathological allocation, wrong error mapping, malformed primitive field accepted.
- Corpus seeds: empty file, invalid UTF-8, valid minimal Set/Finish, each scoped primitive minimal YAML, missing step ID, duplicate step ID, empty prompt/event/output, huge branch list, huge repeat attempts, unsupported Save/Do/Choose.
- Assertion: invalid bytes return exact `CanonicalYaml` or exact compile error; valid in-scope primitive bytes compile to exact IR shape; no panic/OOM.

### FZ02 — Conditional new parser/string-to-index boundary (`PRE-007`, `ERR-008`, `INV-008`)

- Input type: str/bytes; only required if implementation introduces a new raw parser, string-to-slot/index conversion, or ad hoc numeric decoding inside `vb_compile`.
- Risk: unchecked cast, overflow, panic, mismatch between parser and canonical AST validation.
- Corpus seeds: `""`, whitespace, `0`, `1`, `65535`, `65536`, `4294967295`, `4294967296`, negative-looking strings, Unicode identifiers, repeated IDs.
- Assertion: exact checked error or valid bounded value; no unchecked narrowing or panic.

## 6. Kani Harnesses / Checkpoints

Current `proof-obligations.planned.jsonl` records `NA-KANI-001` because dense index and bounds are assigned to Verus plus cargo tests. State 7 still requires these Kani checkpoints for escalation. If implementation introduces unchecked arithmetic/casts/indexing or if the Verus-to-production bridge remains review-insufficient, State 8/11 must add these harnesses rather than relying only on proptest.

| Kani ID | Trace | Property | Bound | Rationale |
|---|---|---|---|---|
| K01 | PRE-007 / INV-004 / ERR-008 | checked bound conversion never wraps or truncates generated step, slot, branch, attempt, page, or item counts. | primitive counts <= 8 plus edge symbolic `u16/u32` values. | Exhaustive bounded arithmetic proof is stronger than sampled property tests. |
| K02 | POST-004 / INV-001 / INV-002 | dense node allocator preserves `node.id == position` and every emitted target is `< node_count`. | node_count <= 12, branch/body depth <= 4. | Mirrors approved TLA+/Verus finite bounds against concrete helper functions. |
| K03 | POST-005 / INV-003 | slot allocator returns zero for no refs and max+1 for refs; every emitted slot ref is in range. | slot refs <= 12. | Prevents off-by-one and missing prompt/answer/collector slots. |
| K04 | PRE-004 / ERR-004 | duplicate ID detection is complete for bounded top-level and nested bodies. | steps <= 8, nested depth <= 2. | Duplicate checks are combinatorial and easy to miss in nested bodies. |

## 7. Mutation Checkpoints

Threshold: >= 90% mutation kill rate overall; 100% kill rate for primitive dispatch, exact error taxonomy, dense target/slot checks, and existing Set/Finish regression branches.

Critical mutants that must be killed:

- Replace `UnsupportedStepPrimitive` guard for any of `ForEach`, `Together`, `Collect`, `Reduce`, `Repeat`, `Wait`, `Ask` -> killed by B08/B27 scenarios.
- Swap primitive dispatch arms (`WaitUntil` vs `WaitEvent`, `Ask` vs `AskResume`, `Collect` vs `Reduce`) -> killed by B13-B19 exact `CompiledNodeKind` assertions.
- Remove validation bridge call or ignore `WorkflowError` -> killed by B09/B37.
- Change dense ID assignment from position to constant/one-based -> killed by B11/P08/K02.
- Change target comparison from `< len` to `<= len` -> killed by B11/P08/K02.
- Change `slot_count` from `max + 1` to `max` or count of refs -> killed by B12/P09/K03.
- Remove branch-count join check or change N to N-1/N+1 -> killed by B14 and TLA+/formal rerun B41.
- Change `Repeat.max_attempts` bound or exhaustion route -> killed by B17/B22/P06/K01.
- Accept empty prompt/event/output or map it to unknown output -> killed by B06/B18/B19.
- Collapse `DuplicateOutputName` and `UnknownOutputName` into a generic error -> killed by B20/B40.
- Delete legacy tests/files or weaken static inventory -> killed by B21/B42.
- Introduce `unwrap`/`expect`/unchecked indexing in production -> killed by B24/static gate.

## 8. Unit Test Coverage Matrix

| Scenario group | Trace | Input class | Expected output | Layer |
|---|---|---|---|---|
| empty source | PRE-001 / ERR-001 | zero steps | exact `CompileError::EmptySteps` | unit/proptest |
| unsupported declarations | PRE-003 / ERR-002 | one top-level unsupported field | exact unsupported top-level variant and field | unit/proptest |
| unsupported control fields | PRE-005 / ERR-003 | one control field per primitive | exact `UnsupportedStepControlField(field)` | unit/proptest |
| duplicate IDs | PRE-004 / ERR-004 | duplicate top-level and nested IDs | exact `DuplicateStepId(id)` | unit/proptest/Kani |
| malformed primitive fields | PRE-006 / ERR-007 | empty/missing/incompatible primitive fields | exact `StepFieldShape(primitive, field)` | unit/proptest |
| bound overflow | PRE-007 / ERR-008 | counts/indexes above target widths | exact range/limit error | unit/proptest/Kani |
| Wait form selection | POST-011 | deadline-only vs event+timeout | exact `WaitUntil` or `WaitEvent`; invalid combo exact `StepFieldShape` | unit/integration |
| Ask prompt/answer | POST-012 | valid prompt/answer/timeout and invalid prompt | exact `Ask` + `AskResume` or exact `StepFieldShape` | unit/integration |
| output registry | POST-013 / ERR-005 / ERR-006 | duplicate/unknown output names | exact `DuplicateOutputName` or `UnknownOutputName` | unit/regression |
| unsupported policy | ERR-011 | in-scope and out-of-scope primitive variants | in-scope Ok exact IR; out-of-scope exact `UnsupportedStepPrimitive` | unit/integration |

## 9. Integration and E2E Coverage Matrix

| Scenario group | Trace | Input class | Expected output | Layer |
|---|---|---|---|---|
| compile all scoped primitives | POST-001 / INV-006 | seven valid canonical AST workflows | `Ok(CompiledWorkflow)` with exact primitive-specific nodes | integration |
| validation bridge | POST-002 / ERR-009 | valid and forced-invalid generated parts | success passes validation; failure wraps exact `WorkflowError` | integration |
| determinism | POST-003 / INV-005 | equal nested sources compiled repeatedly | equal digest and IR fields | integration/proptest |
| dense target inspection | POST-004 / INV-001 / INV-002 | successful primitive workflows | every ID equals position; all targets in range | integration/proptest/Kani |
| slot coverage inspection | POST-005 / INV-003 | workflows with all slot-bearing fields | `slot_count` covers max slot ref | integration/proptest/Kani |
| ForEach lifecycle shape | POST-006 | valid loop body and following finish | exact loop node family and done route | integration |
| Together lifecycle shape | POST-007 | N branches | exact branch entries and join count N | integration |
| Collect lifecycle shape | POST-008 | page/body/collector workflow | exact pagination and collector slot route | integration |
| Reduce lifecycle shape | POST-009 | input/accumulator/body workflow | exact reduction slots and finish route | integration |
| Repeat lifecycle shape | POST-010 | max attempts/body workflow | exact retry/exhaustion route | integration |
| YAML bytes happy path | POST-001 / ERR-010 | complete YAML per in-scope primitive | compile bytes to exact IR or exact parse error | E2E/fuzz |
| Set/Finish legacy path | POST-013 | existing supported YAML and AST workflows | exact preexisting behavior preserved | integration/E2E |
| out-of-scope primitive path | ERR-011 | Save/Do/Choose workflows | exact `UnsupportedStepPrimitive` only for those variants | integration/E2E |

## 10. Static, Formal, and CI Gates

| Gate | Trace | Command/evidence expected | Must assert |
|---|---|---|---|
| source lint / forbidden constructs | INV-008 | `moon ci` plus targeted production diff scan | no forbidden production constructs; test helper style not treated as production lint. |
| dependency boundary | INV-009 | dependency graph/boundary scan under `moon ci` | runtime core adds no YAML/JSON/HTTP dependency. |
| legacy inventory | POST-014 / INV-010 | inventory diff of scoped legacy files/tests | no unapproved deletion. |
| proof rerun | POST-006..012 / INV-001..005 | `TMPDIR=target/tmp verus verification/verus/v1_primitive_lowering.rs`; `tlc -config verification/tla/V1PrimitiveLowering.cfg verification/tla/V1PrimitiveLowering.tla` | approved State 6 proof/model still pass after implementation. |
| focused cargo tests | PRE/POST/ERR owner_state 8 rows | commands listed in `proof-obligations.jsonl` for `cargo test -p vb_compile ...` | exact scenario names pass and emit exact value/error evidence. |
| full CI | POST-001 / POST-014 / INV-006 / INV-008 / INV-009 / ERR-011 | `moon ci` | workspace gate exits 0; no hidden regressions. |
| coverage/mutation | INV-007 + mutation plan | `cargo llvm-cov`/coverage report and `cargo-mutants` report when available | primitive matrix positive/negative coverage and >=90% mutation kill. |

## 11. Traceability Crosswalk

| Contract clauses | Required scenario/test groups | Property/fuzz/Kani/static/mutation overlays |
|---|---|---|
| PRE-001, ERR-001 | B01 | P01, mutation error branch |
| PRE-002 | B02, B26 | FZ01, admission property |
| PRE-003, ERR-002 | B03 | P02, mutation exact field branch |
| PRE-004, ERR-004 | B04 | P03, K04, mutation duplicate path |
| PRE-005, ERR-003 | B05 | P04, mutation field dispatch |
| PRE-006, ERR-007 | B06, B28-B34 | P05, FZ01/FZ02, mutation malformed-field path |
| PRE-007, INV-004, ERR-008 | B07, B22 | P06, K01, FZ02 conditional, mutation bound comparisons |
| POST-001, INV-006, ERR-011 | B08, B27 | P10, P12, mutation primitive dispatch, `moon ci` |
| POST-002, ERR-009 | B09, B37 | mutation validation bridge |
| POST-003, INV-005 | B10, B38 | P07, formal rerun |
| POST-004, INV-001, INV-002 | B11, B35 | P08, K02, formal rerun, mutation target comparisons |
| POST-005, INV-003 | B12, B36 | P09, K03, mutation slot count |
| POST-006 | B13 | P10, formal rerun, mutation loop route |
| POST-007 | B14 | P10, formal rerun, mutation branch count/join |
| POST-008 | B15 | P10, formal rerun, mutation collector/page route |
| POST-009 | B16 | P10, formal rerun, mutation accumulator/input route |
| POST-010 | B17 | P10, K01, formal rerun, mutation attempt/exhaustion route |
| POST-011 | B18 | P10, FZ01, formal rerun, mutation wait-kind selection |
| POST-012 | B19 | P10, formal rerun, mutation ask/resume pairing |
| POST-013, ERR-005, ERR-006 | B20 | P11, mutation output registry |
| POST-014, INV-010 | B21, B42 | static inventory, CI |
| INV-007 | B23 | coverage report, mutation threshold |
| INV-008 | B24 | static scan, `moon ci` |
| INV-009 | B25 | dependency scan, `moon ci` |
| ERR-010 | B26 | FZ01 |

## 12. Acceptance Rules for Test Writer

- Test through public APIs (`compile_source`, `compile_workflow`, `YamlCompiler::compile`) unless a small pure helper is deliberately exposed for bound/slot/target calculation.
- Prefer real `vb_yaml`/`vb_validate`/`vb_core` integration over mocks. Use fakes only for forcing validation failure if no public invalid-parts seam exists.
- Use exact assertions: exact error variant and payload, exact node kind, exact target indexes, exact slot count, exact digest/IR equality.
- No sleeps, no ordering dependence, no external services, no source checkout dependency.
- Every in-scope primitive must have one positive and one negative executable scenario before implementation can pass State 8.
- If exact source-expression-to-slot policy remains unresolved from `contract.md` OQ-001, tests must pin the implemented policy explicitly and update traceability before accepting behavior.

## Open Questions / Test-Writer Clarifications

- OQ-001 remains from contract: exact source-expression-to-slot policy for `input`, `source`, `initial`, `event`, `timeout`, and `prompt` must be pinned by tests once implementation chooses it.
- OQ-002 remains from contract: if optional runtime variants (`ForEachJoin`, `TogetherBranch`, `CollectNext`, `RepeatCheck`) are not emitted, tests must assert the intentional legal alternative shape and trace it to implementation rationale.
- Validation failure injection may need a small test-only seam or lower-level helper exposure; if none exists, use a public helper returning `WorkflowParts` rather than mocking validation interactions.

## Completion Evidence

- State 7 test plan written in isolated workspace only: `.beads/vb-f04l/test-plan.md`.
- Production code edits: none.
- Test code edits: none.
- Proof/model edits: none.
- Required inputs were read and State 6 proof/contract reviews were approved.
- Isolation verified by command output `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-f04l` and guard excluding `/home/lewis/src/velvet-ballistics`.
