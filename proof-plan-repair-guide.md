# Proof Plan Repair Guide — vb-mrwe.6

Smallest rerun: State 4 proof-planner for `vb-mrwe.6`, then rerun State 4 proof-plan-reviewer.

Required repairs:

1. Produce a `vb-mrwe.6` proof-plan artifact set in this isolated workspace. Do not reuse `vb-jpq7.21` artifacts.
2. Rewrite `proof-seeds.jsonl` to `proof-seed/v1` with all required fields: `schema_version`, `id`, `requirement_id`, `contract_clause`, `domain_claim`, `risk_tags`, `suggested_layers`, `behavior_affecting`, `model_boundary`, and `notes`.
3. Emit `verifier-lane-decision/v1` rows per seed and per verifier for exactly these lanes: `verus`, `kani`, `flux-rs`, `proptest`, `cargo-fuzz`, and `loom`.
4. Use singular schema fields (`requirement_id`, `proof_seed_id`) and include `contract_clause` on every lane row.
5. Required lanes must name planned obligation ids. Not-applicable lanes require concrete source/tooling evidence and are not accepted until reviewer-stamped.
6. Do not waive behavior-affecting verification. Remove behavior waivers and schema-invalid waiver candidates.
7. Regenerate `proof-obligations.planned.jsonl` with exact commands, model bounds, assumptions, expected evidence, trusted-base refs, and workdirs under `/home/lewis/isolated/go-skill-batch-20260604/vb-mrwe.6`.
8. Add explicit proof-to-implementation bridge planning for every behavior-affecting obligation, including production source refs, independent behavior-test refs, refinement harness refs, and exact evidence commands.
9. Ensure `trusted-base-plan.md` lists every planned assume/axiom/admit/trusted/ignore/stub/model reduction and its compensating evidence.
10. Record planner invocation provenance independent from reviewer invocation provenance.

After repair, rerun this reviewer with planner invocation id `vb-mrwe.6-state04-proof-planner-20260604` or the updated planner invocation id if State 4 is rerun under a new id.
