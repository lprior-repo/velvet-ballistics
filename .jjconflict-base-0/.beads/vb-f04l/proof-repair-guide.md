# Proof Repair Guide: vb-f04l State 6 Attempt 3

## Route

Return to State 5 proof-writer repair. Do not edit production code, tests, dependencies, CI, or source checkout files for this repair.

## Required Repairs

1. Replace assumption-decomposition Verus proofs with constructive proof obligations.
2. Add or map exact primitive-shape proof functions for `POST-006-VERUS` through `POST-012-VERUS`.
3. Strengthen or honestly narrow the TLA+ model and obligations.
4. Refresh `proof-writer-report.md` and `proof-evidence.md` with current raw command output and exact canonical obligation mapping.

## Verus Repair Detail

- `constructor_inputs_valid(plan)` must not be the only source of the ensured properties for dense IDs, target range, slot coverage, bound checks, and shape preservation.
- Add abstract constructors or transition lemmas such as `construct_foreach_plan`, `construct_together_plan`, `construct_collect_plan`, `construct_reduce_plan`, `construct_repeat_plan`, `construct_wait_plan`, and `construct_ask_plan`, or one equivalent tagged constructor.
- Prove checks occur before narrowing/allocation by modeling the check result and deriving bounded fields from it.
- Prove dense IDs using a sequence/list model where position implies `StepIdx(position)`, not by reducing the obligation to `node_count > 0`.
- Prove slot coverage from a modeled list/set of slot references, not only from a supplied `max_slot_ref` field.
- Prove determinism from equal abstract source inputs through a deterministic construction function, not from a precondition equating every output field.
- Ensure the proof function names in `.beads/vb-f04l/proof-obligations.jsonl` exactly match the artifact, or update the JSONL/evidence after repair.

## TLA+ Repair Detail

- Do not fix all target fields to a single representative layout unless every TLA+ obligation is narrowed to that exact representative claim.
- If retaining the broader obligations, allow bounded variation for target fields and include invalid shape cases that invariants reject.
- Make `GraphShapePrevalidated` meaningful by connecting each primitive's required route fields to the lifecycle actions that use them.
- Preserve deadlock and temporal checks for all seven primitives.
- Record TLC state counts and seed after repair.

## Required Rerun Commands

- `pwd -P`
- `test -s .beads/vb-f04l/proof-obligations.jsonl && test -s .beads/vb-f04l/proof-obligations.planned.jsonl && test -s .beads/vb-f04l/proof-writer-report.md && test -s .beads/vb-f04l/proof-evidence.md`
- `jq -c . .beads/vb-f04l/proof-obligations.jsonl >/dev/null && jq -c . .beads/vb-f04l/proof-obligations.planned.jsonl >/dev/null && jq -c . .beads/vb-f04l/traceability-matrix.jsonl >/dev/null`
- `verus verification/verus/v1_primitive_lowering.rs`
- `tlc -config verification/tla/V1PrimitiveLowering.cfg verification/tla/V1PrimitiveLowering.tla`

## Acceptance Bar

Next proof-review can approve only if every required State 5 obligation has exact artifact mapping, raw rerun evidence, and a non-vacuous proof/model whose assumptions are narrower than the property being claimed.
