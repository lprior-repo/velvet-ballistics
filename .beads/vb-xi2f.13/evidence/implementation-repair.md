# vb-xi2f.13 implementation repair evidence

## Production repair

- `choose_width` now computes `1 + sum(body_width(branch.steps))` with `checked_add`.
- `lower_canonical_choose` now lowers non-empty branch bodies instead of rejecting them.
- `lower_canonical_choose` emits the `ChooseSlot` before generated branch-body nodes so `CompiledWorkflow::try_from_parts` sees dense `StepIdx == node table index` ordering.
- `emit_choose_branch_body` emits bounded `Set` and `Do` body nodes, chains intermediate nodes to the next body step, and chains the final body node to the common fallthrough step.
- `part_14.rs` holds the choose-body lowering helpers so `part_02.rs` and `part_06.rs` stay below the source-length ceiling.
- `part_02.rs` and `part_06.rs` re-export compatibility shims for existing tests/Kani imports.

## Behavior tests added

- `choose_width_counts_branch_body_steps`
- `lower_canonical_choose_single_body_set_targets_body_start`
- `lower_canonical_choose_multi_body_steps_chain_to_common_next`
- `compile_workflow_choose_branch_body_emits_dense_order`

## Commands

- `rtk cargo fmt --check` — passed
- `bash scripts/check-source-length.sh` — passed
- `rtk cargo test -p vb_compile` — passed: `662 passed, 5 ignored (31 suites, 6.31s)`
- `verus --crate-type=lib verification/verus/vb_compile/src/choose_bool_invariant.rs` — passed: `verification results:: 2 verified, 0 errors`
- `cargo kani -p vb_compile --harness kani_choose_body_fallthrough --unwind 256` — blocked before target harness by pre-existing `vb_compile` Kani compile errors in unrelated legacy harnesses.
- `moon ci` — passed: `Tasks: 32 completed (4 cached)`, `Time: 9m 22s 51ms`
  - Raw output: `/home/lewis/.local/share/opencode/tool-output/tool_e74f59883001K1Bi8ajXOkw03J`

## Residual risk

- `.beads/vb-xi2f.13/proof-review.md` is a stale rejection from before this implementation repair. It correctly captured that the previous artifact set was missing, but it has not been superseded by an independent proof-reviewer pass.
- Flux refinement commands were not executed in this pass.
- The planned choose-specific Kani harnesses exist, but `cargo kani -p vb_compile` is blocked by unrelated pre-existing harness compilation errors before the target harness can run.
