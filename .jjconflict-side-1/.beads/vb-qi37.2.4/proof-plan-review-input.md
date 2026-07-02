# Proof Plan Review Input: vb-qi37.2.4

## Review Question
Does this proof plan cover the State 3 repaired contract for bounded nested `collect`/`reduce`/`repeat`/`together` composition, aggregate budget acceptance/rejection, and structural growth diagnostics without overclaiming existing proof coverage?

## Contract Sources
- `.beads/vb-qi37.2.4/contract.md`
- `.beads/vb-qi37.2.4/traceability-matrix.jsonl`
- `.beads/vb-qi37.2.4/verification-layers.md`
- `.beads/vb-qi37.2.4/tla-spec.md`

## Code And Proof Surfaces Discovered
- `crates/vb_core/src/budget.rs`: `WholeWorkflowBudget::compute`, `BoundednessPolicy::validate`, `AggregateResourceBudget::from_workflow`, `validate_aggregate_budget`, `count_total_steps`, `count_and_push_loop_body`, `count_nested_for_region`, `compute_fanout_and_depth`, `update_workflow_metrics`.
- `crates/vb_validate/src/gate_12_14_15.rs`: current gate file is action/slot/determinism oriented and is not itself a bounded nested composition proof surface.
- `verification/verus/budget_bounded.rs`: existing simple step/add/overflow lemmas only; nested composition lemmas are not present yet.
- `specs/tla/BoundedAdmission.tla`: existing reservation model; lacks explicit verified-budget state required by `INV-006`.

## Planned Obligations
- TLA required: `TLA-ADM-001`, `TLA-ADM-002`.
- Verus required: `VERUS-BUD-001`, `VERUS-BUD-002`, `VERUS-BUD-003`, `VERUS-AGG-001`, `VERUS-DIAG-001`.
- Kani required: `KANI-BUD-001`.
- Proptest/deep required: `PROP-BUD-001`, `PROP-DIAG-001`.
- Rollup gates required: `GATE-BUD-001`, `GATE-BUD-002`, `GATE-BUD-003`.

## Known Blockers To Preserve
- Existing TLA model should produce `BLOCK_LOCAL` for verified-budget admission until the model is repaired.
- Existing Verus file should produce `BLOCK_LOCAL` for missing nested/together/refinement/diagnostic proof surfaces until repaired.
- Existing property/Kani harness coverage is not assumed. Later states must either add harnesses or record precise missing-surface blockers.

## Reviewer Checks Requested
- Confirm every planned obligation maps to a contract clause in the State 3 repair.
- Reject any obligation that claims current PASS evidence without executable proof output.
- Confirm no nested lifecycle, production edit, test edit, proof-code edit, or go-skill/femdation/master/Red Queen invocation is required by this plan.
- Confirm `proof-obligations.planned.jsonl` is valid JSONL and includes `command`, `expected_evidence`, `required`, `assumptions`, `owner_state`, and `rerun_from` on every row.
