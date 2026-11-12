# Codebase Map: vb-f04l

bead_id: `vb-f04l`
title: `compiler: Safe v1 primitive source lowering`
state: `2 artifact repair`
workspace: `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-f04l`
source_checkout_write_policy: `forbidden`

## Scope Summary

This State 2 scout maps the isolated workspace snapshot for safe v1 primitive source lowering. The bead requires canonical YAML AST to numeric IR parity for v1 primitives including `ForEach`, `Together`, `Collect`, `Reduce`, `Repeat`, plus related `Wait` and `Ask`, while preserving existing compiler/schema/lowering tests and not deleting legacy compiler paths.

## Bead Evidence

- Command used as requested: `bd --db /home/lewis/src/velvet-ballistics/.beads/dolt show vb-f04l --json`.
- Result: bead exists, status `in_progress`, title `compiler: Safe v1 primitive source lowering`.
- Key contract: compile all v1 primitives from AST to numeric IR and emit mathematically equivalent `CompiledNodeKind` IR.
- Key invariants: numeric IR indices are dense and valid; no untested primitives are reachable.
- Research paths requested by bead text: `crates/vb_compile/src/lower.rs` and `crates/vb_compile/src/api_build2.rs`.
- Isolated snapshot correction: `crates/vb_compile/src/lower.rs` is MISSING; `crates/vb_compile/src/lower/mod.rs` exists and re-exports lowering functions from `crates/vb_compile/src/lib.rs`.
- Isolated snapshot correction: `crates/vb_compile/src/api_build2.rs` is MISSING.

## Relevant Files Read

- `crates/vb_compile/src/lib.rs`: canonical compile entry points, `compile_source`, current supported primitive match, lowering helpers, `CompileError`, and validation handoff.
- `crates/vb_compile/src/lower/mod.rs`: lowering module is only a re-export shim; implementation is in `lib.rs`.
- `crates/vb_compile/src/ast/types.rs`: legacy compiler AST contains low-level primitive variants for `ForEach`, `Together`, `Collect`, `Reduce`, `Repeat`, `Wait`, `Ask`, and `Finish`.
- `crates/vb_compile/src/ast/parse.rs`: legacy `parse_ast` can parse low-level numeric fields for the listed primitives.
- `crates/vb_compile/src/control_flow.rs`: reachability/target validation recognizes the listed legacy primitive AST variants and `Together` branch targets.
- `crates/vb_compile/src/type_taint.rs`: type/taint validation records or reads slots for `ForEach`, `Collect`, `Reduce`, `Repeat`, `Wait`, and `Ask`.
- `crates/vb_core/src/nodes.rs`: runtime `CompiledNodeKind` includes IR variants for `ForEachStart`, `ForEachNext`, `ForEachJoin`, `TogetherStart`, `TogetherBranch`, `TogetherJoin`, `CollectStart`, `CollectPage`, `CollectNext`, `CollectFinish`, `ReduceStart`, `ReduceNext`, `ReduceFinish`, `RepeatStart`, `RepeatAttempt`, `RepeatCheck`, `RepeatFinish`, `WaitUntil`, `WaitEvent`, `Ask`, and `AskResume`.
- `crates/vb_core/src/validation.rs`: `CompiledWorkflow::try_from_parts` path validates resource bounds, node IDs, node-specific invariants, reachability, and forward edges.
- `crates/vb_validate/src/gate_11_loop.rs`: loop/body graph validation covers the same loop/fanout primitive IR families.
- `crates/vb_validate/src/gates.rs`: slot and pairing validation handles loop/fanout/repeat/wait/ask IR variants.
- `crates/vb_yaml/src/ast/types.rs`: canonical authoring AST has high-level source-shape `StepPrimitive` variants for all target primitives.
- `crates/vb_compile/src/control_flow/tests.rs`: existing tests cover legacy parse/control-flow diagnostics, not canonical source lowering parity for all target primitives.
- `crates/vb_compile/src/ast/tests.rs`: existing tests cover AST surface, primitive identity for run/do/save, expression diagnostics, and hardening cases.

## Key Findings

- `compile_source` currently accepts canonical `vb_yaml::ast::WorkflowSource` but only lowers `StepPrimitive::Set` and `StepPrimitive::Finish`; any other canonical primitive returns `CompileError::UnsupportedStepPrimitive` with primitive names from `canonical_primitive_name`.
- Lowering helper functions already exist for `lower_for_each`, `lower_together`, `lower_collect`, `lower_reduce`, `lower_repeat`, `lower_wait`, and `lower_ask`, but they take low-level numeric slots/step indices rather than high-level canonical source bodies from `vb_yaml::ast::StepPrimitive`.
- Legacy `YamlCompiler::parse_ast` can parse low-level numeric primitive forms into `StepKindAst`, but `YamlCompiler::compile` now routes through `vb_yaml::parse_workflow_source` and `compile_source`, so the current canonical compiler path does not use the legacy AST for full primitive lowering.
- Canonical `vb_yaml::ast::StepPrimitive` stores high-level body/source fields for `ForEach`, `Together`, `Collect`, `Reduce`, and `Repeat`; downstream implementation must decide how nested source bodies allocate dense step IDs, slots, constants, and branch/join/done nodes.
- Runtime and validation layers already have structural IR variants and gates for the target primitive families, so the main gap is source-to-IR lowering, dense indexing, and acceptance/error coverage rather than adding runtime node kinds.
- Existing validation should remain in the path through `vb_validate::shared::validate` and `CompiledWorkflow::try_from_parts` after lowering.

## Important Symbols

- `vb_compile::compile_source`: current canonical AST handoff and primary implementation target.
- `vb_compile::YamlCompiler::compile`: public YAML byte entry point that delegates to canonical `vb_yaml` parser and `compile_source`.
- `vb_compile::lower_for_each`: helper emits `ForEachStart` and `ForEachNext`; no join node is emitted despite the comment mentioning `ForEachJoin`.
- `vb_compile::lower_together`: helper emits `TogetherStart` and `TogetherJoin`; no `TogetherBranch` nodes are emitted.
- `vb_compile::lower_collect`: helper emits `CollectStart`, `CollectPage`, and `CollectFinish`.
- `vb_compile::lower_reduce`: helper emits `ReduceStart`, `ReduceNext`, and `ReduceFinish`.
- `vb_compile::lower_repeat`: helper emits `RepeatStart`, `RepeatAttempt`, and `RepeatFinish`; no `RepeatCheck` node is emitted.
- `vb_yaml::ast::StepPrimitive`: source primitive enum for source-shape data.
- `vb_compile::ast::StepKindAst`: legacy low-level compiler AST enum for numeric primitive data.
- `vb_core::CompiledNodeKind`: runtime IR node-kind enum.
- `vb_validate::shared::validate`: shared structural validation gate run before `CompiledWorkflow::try_from_parts`.

## Existing Tests And Gaps

- Existing `vb_compile` AST/control-flow tests exercise legacy low-level parse paths and diagnostics.
- Existing `vb_validate` tests exercise structural loop/fanout/repeat IR gate behavior.
- Missing in this scope: canonical `compile_source` happy-path tests for `for_each`, `together`, `collect`, `reduce`, `repeat`, `wait`, and `ask` from `vb_yaml::ast::WorkflowSource` into concrete `CompiledNodeKind` arrays.
- Missing in this scope: invalid-shape/error-path tests proving canonical AST primitives with missing fields, invalid body topology, invalid references, or unsupported nested structures fail before runtime.
- Missing in this scope: coverage matrix evidence proving parser/validator/compiler parity for all v1 primitives.

## Risk Tags

- `parser-codec`: canonical YAML AST and legacy compiler AST differ.
- `public-api`: `compile_source` and `compile_workflow` are public compiler entry points.
- `temporal`: loop/fanout/retry primitives encode execution order and bounded progress.
- `concurrency`: `Together` expresses bounded parallel branches.
- `performance`: lowering must preserve dense numeric IR and bounded resource contracts.
- `migration`: legacy `parse_ast`/low-level compiler grammar coexists with canonical `vb_yaml` handoff.
- `user-visible-behavior`: YAML workflows currently reject valid v1 primitives through `UnsupportedStepPrimitive`.

## Downstream Owner Recommendations

- `rust-contract`: specify canonical source primitive to numeric IR contracts, especially nested body expansion, dense step ordering, slot allocation, and done/join targets.
- `proof-planner`: require Kani/property or focused model tests for dense indices, valid targets, and bounded loop/fanout shapes; TLA+ only if whole-workflow temporal semantics change.
- `test-planner`: write acceptance tests for each primitive and negative tests for invalid body/topology/field/reference shapes before implementation.
- `holzman-rust`: implement minimal safe lowering in `vb_compile` without `unwrap`, `expect`, unchecked indexing, unchecked arithmetic, or source checkout writes.

## Open Questions

- UNKNOWN: final canonical lowering representation for nested `body: Vec<StepAst>` inside `ForEach`, `Collect`, `Reduce`, `Repeat`, and branch steps inside `Together`.
- UNKNOWN: whether `lower_for_each`, `lower_together`, and `lower_repeat` helper outputs intentionally omit `ForEachJoin`, `TogetherBranch`, and `RepeatCheck` nodes or are incomplete relative to runtime IR variants.
- UNKNOWN: whether legacy `parse_ast` is to remain as a supported low-level compiler path or only as transitional compatibility.
- UNKNOWN: exact required source syntax for `Wait`/`Ask` canonical lowering in this bead versus child bead scope.

## Exclusions

- No production code, tests, proof artifacts, dependency files, or CI config were modified.
- No writes were made under `/home/lewis/src/velvet-ballistics`.
- No implementation or verification gates were run beyond artifact existence and JSONL syntax checks required for State 2 repair.
