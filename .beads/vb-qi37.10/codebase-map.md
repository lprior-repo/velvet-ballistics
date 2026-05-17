# Codebase Map — vb-qi37.10

Bead: `vb-qi37.10` — `codegen: Complete remaining final IR coverage and parity`  
State: 2 / explore  
Workspace: `/tmp/opencode/go-skill-vb-qi37-10`  
Source checkout reference: `/home/lewis/src/velvet-ballistics`

## Scope Contract

- Master Phase 32 requires generated Rust mode: codegen, compile checks, equivalence tests, and compile-fail tests.
- Master Phase 33+ depends on generated-mode parity but is not the direct scope of this bead.
- Final IR surface is every `CompiledNodeKind` in `crates/vb_core/src/workflow/mod.rs:543-708`.
- Runtime oracle is `crates/vb_runtime/src/engine/execute.rs:45-360+` plus primitive modules under `crates/vb_runtime/src/primitives/`.
- Generated Rust must match IR/runtime on terminal result, typed error, final pc, slot values, taints, step states, journal sequence, suspension semantics, action tickets, retry counts, wait/ask scheduling, and replay-observable behavior.
- Do not broaden into all release blockers: `vb-qi37.11` owns suspension-error parity expansion; `vb-gvmt` owns broader generated semantic parity evidence after this coverage bead unblocks it.

## Required Inputs Read

- `/tmp/opencode/go-skill-vb-qi37-10/velvet-ballistics-MASTER.md`
  - Phase table: lines 1445-1488.
  - Round 2 gaps: lines 1500-1514.
  - Generated Rust contract: lines 1071-1107.
  - Final IR contract: lines 613-661.
  - Final DoD generated parity: lines 1977-1978.
- `/tmp/opencode/go-skill-vb-qi37-10/.beads/vb-qi37.10/STATE.md`
- `/tmp/opencode/go-skill-vb-qi37-10/.beads/vb-qi37.10/baseline-report.md`
- `/tmp/opencode/go-skill-vb-qi37-10/.beads/vb-scxh/scope-control-audit.md`
- `/tmp/opencode/go-skill-vb-qi37-10/.beads/vb-gvmt/parity-test-report.md`
- `bd --db "/home/lewis/src/velvet-ballistics/.beads/dolt" show vb-qi37.10 --json` from isolated workspace.

## Master/Bead Reality

- `vb-qi37.10` remains open and is a release blocker for generated Rust / maxperf.
- Closed child beads claim partial implementation, but current master still names generated-mode coverage/parity as unfinished.
- `vb-scxh` recorded the important scope marker: `vb-qi37.10` still owns `append/append_if/merge/sum/unique generated expressions`, `accessor traversal`, `Together/Reduce/Repeat nodes`, and semantic parity evidence.
- Current source has moved since that marker: generated expressions now include `Append`, `AppendIf`, `Merge`, `Sum`, `Count`, `Unique`, and accessor traversal is emitted. This bead should verify and harden those paths, not assume closure from code presence.
- The active remaining codegen coverage gap in source is final IR families intentionally rejected by generated subset: `Collect*`, `Together*`, `Reduce*`, `Repeat*`, plus text helpers `Contains`, `StartsWith`, `EndsWith`.

## Primary Implementation Hotspots

### `/tmp/opencode/go-skill-vb-qi37-10/crates/vb_codegen/src/lib.rs`

This is the main production codegen implementation. `Cargo.toml` points at the crate default lib; there is no `mod codegen;` export, so this file is the active surface.

Key APIs:

- `emit_rust_workflow(workflow: &CompiledWorkflow) -> CodegenResult<String>` — top-level generator.
- `validate_generated_subset(workflow: &CompiledWorkflow) -> CodegenResult<()>` — fail-closed coverage gate.
- `unsupported_node_feature(kind: &CompiledNodeKind) -> Option<&'static str>` — current generated-mode acceptance matrix.
- `unsupported_expr_feature(op: ExprOp) -> Option<&'static str>` — expression helper acceptance matrix.
- `emit_step_function`, `emit_step_body`, `emit_*_step` — per-node generated Rust emission.
- `emit_expr_function`, `emit_accessor_eval`, `emit_accessor_traversal` — generated expression/accessor execution.
- `emit_generated_runtime_api`, `emit_run_until_blocked`, `emit_action_resume_api`, `emit_ask_resume_api` — generated execution API and journal/suspension surfaces.
- `compare_generated_to_ir(source, workflow)` — currently static source-pattern/count guard, not full semantic equivalence.

Current coverage from `unsupported_node_feature`:

- Accepted: `Nop`, `SetConst`, `Copy`, `EvalExpr`, `BuildObject`, `BuildList`, `Do`, `Choose`, `ChooseSlot`, `WaitUntil`, `WaitEvent`, `Ask`, `AskResume`, `ErrorHandler`, `RetryCheck`, `ForEachStart`, `ForEachNext`, `ForEachJoin`, `Jump`, `Finish`.
- Rejected: `TogetherStart`, `TogetherBranch`, `TogetherJoin`, `ReduceStart`, `ReduceNext`, `ReduceFinish`, `RepeatStart`, `RepeatAttempt`, `RepeatCheck`, `RepeatFinish`, `CollectStart`, `CollectPage`, `CollectNext`, `CollectFinish`.

Current expression coverage from `unsupported_expr_feature`:

- Accepted: load ops, comparison/logical/arithmetic, `Has`, `Exists`, `Length`, `Empty`, `Append`, `AppendIf`, `Merge`, `Sum`, `Count`, `Unique`.
- Rejected: `Contains`, `StartsWith`, `EndsWith` because generated code has no runtime symbol/string store semantics.

Important drift trap:

- `/tmp/opencode/go-skill-vb-qi37-10/crates/vb_codegen/src/codegen/mod.rs` is a near-duplicate of active `src/lib.rs` but appears unreferenced by the crate. Avoid changing only `src/codegen/mod.rs`; either leave it alone or explicitly consolidate/remove in a separate cleanup bead. For this bead, production implementation should target `src/lib.rs` and tests.

### `/tmp/opencode/go-skill-vb-qi37-10/crates/vb_codegen/src/generated_storage_helpers.rs.txt`

Generated runtime helper text included into generated Rust by `include_str!`. It provides bounded in-generated stores for list/object handles.

Key APIs/behaviors:

- `ListStore`, `ObjectStore`, `ListRecord`, `ObjectRecord`, `ObjectField`.
- `insert_items_with_taints`, `insert_items_prefix`, `value_at`, `tail`, `field`.
- Expression helper support: `append_list_item`, `clone_list_items`, `unique_list_items`, `sum_list_items`, `merge_object_records`, `object_field_count`, `list_contains_item`.

Risk: generated helper semantics must match `vb_core::ValueStore` / runtime primitive behavior exactly. Code presence is insufficient; parity tests must execute generated output.

### `/tmp/opencode/go-skill-vb-qi37-10/crates/vb_codegen/src/tests.rs`

Large existing generated-mode test harness. This is the best place for red/green tests for this bead.

Useful surfaces:

- Workflow builders around lines 153+, 825+, 920+, 1025+, 1120+, 2702+, 3397+, 3440+, 3480+, 3523+, 3566+.
- Unsupported control primitive tests around lines 2511+, 5550+, 6074+, 6092+.
- Post-regression executable generated-mode tests around lines 12123-12619+.
- `generated_step_stdout`, `generated_drive_stdout`, `generated_trace_stdout` style helpers execute generated Rust as a real binary and compare stdout.
- Existing tests already cover BuildObject/BuildList, expression taint, action/ask journal surfaces, retry, accessors, and some rejection cases.

Needed additions for this bead:

- Positive executable parity tests if adding `Collect*`, `Together*`, `Reduce*`, or `Repeat*` to generated mode.
- If any final IR family remains intentionally unsupported, exact rejection tests must remain and the master/bead acceptance must decide whether this still satisfies `Complete remaining final IR coverage`. Given title and Phase 32 wording, leaving unsupported final IR likely does not satisfy bead closure.
- Replace/augment static `compare_generated_to_ir` with semantic comparison for new supported shapes or route tests through generated executable + IR/runtime oracle.

### `/tmp/opencode/go-skill-vb-qi37-10/crates/vb_codegen/tests/trybuild_tests.rs`

Compile-fail test harness currently passes when no compile-fail fixtures exist:

- `trybuild_compile_fail_tests` returns `Ok(())` with only an `eprintln!` if `tests/compile-fail/*.rs` is empty.
- Master lines 1106-1107 and 1588-1593 require compile-fail coverage for generated Rust contracts.

Risk: this is an acceptance loophole. `vb-qi37.10` should either add real compile-fail fixtures for generated-code contract violations or explicitly scope that to a follow-up bead. Because Phase 32 includes compile-fail tests, this is likely in scope.

### `/tmp/opencode/go-skill-vb-qi37-10/crates/vb_core/src/workflow/mod.rs`

Authoritative IR definitions and validation.

Key APIs/types:

- `CompiledNodeKind` final IR variants: lines 543-708.
- `CompiledWorkflow::try_from_parts` and validation: lines 35+, 711+.
- `ExprProgram`, `ExprOp`, `AccessorProgram`, `ResourceContract` definitions.

Use as shape oracle only. Avoid modifying unless generated-mode support exposes missing IR metadata. Any IR shape change is high risk and likely out of scope.

### `/tmp/opencode/go-skill-vb-qi37-10/crates/vb_runtime/src/engine/execute.rs`

Runtime full primitive dispatch, the semantic oracle for generated parity.

Relevant dispatches:

- `ForEachStart/Next/Join`: lines 56-102.
- `TogetherStart/Branch/Join`: lines 104-144.
- `CollectStart/Page/Next/Finish`: lines 146-210.
- `ReduceStart/Next/Finish`: lines 212-261.
- `RepeatStart/Attempt/Check/Finish`: lines 263-289.
- `WaitUntil`, `WaitEvent`, `Ask`, `AskResume`, `Do`, `RetryCheck`, `ErrorHandler`: lines 291-360+.

Use as the expected behavior for generated-mode implementations.

### `/tmp/opencode/go-skill-vb-qi37-10/crates/vb_runtime/src/primitives/`

Primitive-specific runtime oracle files:

- `collect.rs` — pagination state, `CollectStates`, durable extras, ordering/stale/duplicate classification. This is the hardest generated-mode family because current runtime uses a side table, serialized extras, and journal hydration semantics.
- `together.rs` — branch fanout/join semantics, accumulator behavior, ordering/failure policy.
- `reduce.rs` — accumulator initialization from const pool, item binding, tail iteration, finish taint copy.
- `repeat.rs` — packed repeat state in `I64`, attempt increment/routing, finish copy.
- `for_each.rs` — already supported in codegen; useful pattern for list iteration/tail semantics.
- `wait_ask.rs`, `retry.rs`, `helpers.rs` — suspension/retry/helper semantics; mostly adjacent to `vb-qi37.11`.

### `/tmp/opencode/go-skill-vb-qi37-10/crates/vb_storage/src/`

Storage is not a primary implementation target for `vb-qi37.10`, but generated parity claims include journal-observable behavior. Relevant storage surfaces:

- `events.rs` / `journal/*` — `JournalEvent` families such as `SlotWrittenEvent`, `StepStarted`, `RunFinished`, `ActionScheduled`, etc.
- `records.rs`, `codec/*`, `types.rs` — binary envelope and record-kind validation.
- `recovery/*` — replay/hydration expectations.

Expected changes: none unless tests need to compare generated lightweight journal signatures against storage/runtime event names. Avoid storage behavior changes in this bead.

### `/tmp/opencode/go-skill-vb-qi37-10/crates/vb_expr/src/`

Expression engine oracle for helper semantics.

Useful files:

- `bytecode/mod.rs`, `bytecode/fold.rs`, `builtin_eval.rs`, `slot_eval.rs`, `eval/*`, `typecheck/*`.
- Existing Kani/Miri/proptest surfaces for expression stack and parser.

Expected changes: none unless generated expression helper semantics reveal a mismatch with expression bytecode. Prefer fixing generated code/tests, not expression semantics, unless a real interpreter bug is found.

### `/tmp/opencode/go-skill-vb-qi37-10/crates/workspace_tests/`

Cross-crate tests and benchmarks. Relevant existing surfaces:

- `benches/velvet_ballastics.rs` has generated-mode and IR-vs-generated benchmark groups.
- `tests/phase0_scaffold_test.rs` checks generated benchmark group presence.
- No focused workspace integration test currently owns full generated final-IR parity; most codegen coverage is in `vb_codegen/src/tests.rs`.

Expected changes: possibly add a focused workspace integration test only if cross-crate runtime/storage parity cannot be proven inside `vb_codegen` tests. Keep scope narrow.

### `/tmp/opencode/go-skill-vb-qi37-10/fuzz/src/lib.rs` and `/tmp/opencode/go-skill-vb-qi37-10/fuzz/src/bin/generated_compare.rs`

Existing `fuzz_generated_compare` is shallow:

- It decodes `WorkflowParts`, validates/constructs `CompiledWorkflow`, and selects a workflow, but does not execute generated-vs-IR parity.

If this bead expands generated final-IR support, fuzz should at least build/gate generated compare over small accepted workflows. Full fuzz campaign evidence can be deferred if recorded, but Phase 37 lists `generated_compare` as required.

## Current Coverage Matrix Summary

| IR / surface | Current generated status | Main files | Bead action |
|---|---:|---|---|
| `Nop`, `SetConst`, `Copy`, `Jump`, `Finish` | emitted | `vb_codegen/src/lib.rs` | Keep; semantic tests should remain. |
| `EvalExpr` scalar/logical/arithmetic | emitted | `vb_codegen/src/lib.rs`, `generated_storage_helpers.rs.txt` | Keep; add parity for any untested helpers. |
| `BuildObject`, `BuildList` | emitted | `vb_codegen/src/lib.rs`, helpers txt | Verify keyed/taint parity with runtime/value store. |
| `LoadAccessor` / path traversal | emitted | `vb_codegen/src/lib.rs:emit_accessor_*` | Verify object/list/missing/type/taint parity. |
| `Append`, `AppendIf`, `Merge`, `Sum`, `Count`, `Unique`, `Has`, `Exists`, `Length`, `Empty` | emitted | `vb_codegen/src/lib.rs`, helpers txt | Verify helper parity and generated bounded capacity errors. |
| `Contains`, `StartsWith`, `EndsWith` | rejected | `unsupported_expr_feature` | Decide: implement symbol/text store semantics or keep exact rejection with acceptance waiver. Title suggests implementation may be required if final expression coverage includes text helpers. |
| `ForEachStart`, `ForEachNext`, `ForEachJoin` | emitted | `vb_codegen/src/lib.rs`, runtime `for_each.rs` | Keep; use as pattern for Reduce/Collect. |
| `CollectStart`, `CollectPage`, `CollectNext`, `CollectFinish` | rejected | `unsupported_node_feature`, runtime `collect.rs` | Highest complexity; likely either implement bounded generated side table or retain fail-closed with explicit non-closure. |
| `ReduceStart`, `ReduceNext`, `ReduceFinish` | rejected | `unsupported_node_feature`, runtime `reduce.rs` | Good candidate for implementation; similar to ForEach + accumulator. |
| `RepeatStart`, `RepeatAttempt`, `RepeatCheck`, `RepeatFinish` | rejected | `unsupported_node_feature`, runtime `repeat.rs` | Good candidate for implementation; compact packed-state semantics. |
| `TogetherStart`, `TogetherBranch`, `TogetherJoin` | rejected | `unsupported_node_feature`, runtime `together.rs` | High risk due branch accumulator/join semantics; less hard than Collect if no true parallelism in generated mode. |
| `Do`, `WaitUntil`, `WaitEvent`, `Ask`, `AskResume`, `RetryCheck`, `ErrorHandler` | emitted subset | `vb_codegen/src/lib.rs`, `vb_runtime/src/engine/execute.rs` | Do not broaden unless needed; `vb-qi37.11` owns detailed suspension-error parity. |
| Generated compile-fail fixtures | weak | `vb_codegen/tests/trybuild_tests.rs` | Add non-empty compile-fail fixtures or track as blocker. |
| `compare_generated_to_ir` | static guard | `vb_codegen/src/lib.rs` | Do not treat as semantic evidence; use executable harness. |

## Highest Risks / Blockers

1. **Collect generated parity**: runtime `CollectStates` uses per-run side state, pagination lineage, durable extras, stale/duplicate page detection, and hydration. Generated mode currently has only lightweight in-source journal/state; implementing this faithfully is non-trivial.
2. **Semantic parity evidence loophole**: `compare_generated_to_ir` rejects bad source patterns and counts functions/actions but does not compare actual execution semantics.
3. **Compile-fail loophole**: trybuild compile-fail passes with zero fixtures.
4. **Duplicate codegen implementation file**: active `src/lib.rs` and unreferenced `src/codegen/mod.rs` can drift. Touching the wrong file would produce no production effect.
5. **Expression F64 generated code risk**: generated arithmetic emits unchecked `as` conversions for some F64 paths in generated source strings; `compare_generated_to_ir` rejects ` as ` in generated output. Any F64 expression tests may expose current generated code as unacceptable under master no unchecked-cast rules.
6. **Text helper gap**: `Contains`, `StartsWith`, `EndsWith` are final expression helpers but generated mode rejects them due missing symbol/text store. Needs a decision before claiming “complete remaining final IR coverage”.
7. **Journal parity mismatch risk**: generated lightweight `JournalEvent` is not the same as `vb_storage::JournalEvent`; tests must compare an agreed semantic signature, not merely event count.

## Recommended Narrow Delivery Plan

1. Write failing tests first in `crates/vb_codegen/src/tests.rs` for the remaining rejected node families, one family at a time: `Repeat*`, `Reduce*`, `Together*`, then `Collect*`.
2. Add exact compile-fail fixtures under `crates/vb_codegen/tests/compile-fail/` so trybuild no longer passes empty.
3. Upgrade semantic parity tests to execute generated Rust and compare against runtime/IR oracle for result, taint, final pc, slot values, and journal signature for each newly supported family.
4. Keep dependency/config unchanged unless compile-fail fixture wiring or test helpers require dev-only changes. Current expected dependency delta: none.
5. If `Collect*` cannot be implemented faithfully in this bead, stop and file/update blocker instead of closing `vb-qi37.10`.

## Suggested Verification Modes

- Focused red/green: `cargo test -p vb_codegen <new_test_name> -- --nocapture`.
- Codegen package: `cargo test -p vb_codegen --lib` and `cargo test -p vb_codegen --test trybuild_tests`.
- Runtime primitive regression: `cargo test -p vb_runtime primitives` or narrower test names for touched primitive oracle comparisons.
- Core expression/value regression if helper semantics touched: `cargo test -p vb_core expr` and/or `cargo test -p vb_expr` focused helpers.
- Fuzz build/smoke for generated compare if fuzz target changes: `cargo fuzz build generated_compare` or repository moon/just equivalent.
- Final bead gate after implementation: `moon ci` per repo contract.
