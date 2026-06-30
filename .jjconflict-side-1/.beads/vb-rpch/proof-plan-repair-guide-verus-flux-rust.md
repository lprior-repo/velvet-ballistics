# Proof Plan Repair Guide — vb-rpch Verus/Flux/Rust

Nearest rerun state: **State 3 / proof-planner repair**, then rerun **State 4 / proof-plan-reviewer**.

## Required repairs

1. Replace `.beads/vb-rpch/verifier-lane-decisions.verus-flux-rust.jsonl` with canonical `verifier-lane-decision/v1` JSONL.
   - One row per `(requirement_id, contract_clause, proof_seed_id, verifier)` tuple.
   - Include the full core verifier set: `tla-plus`, `verus`, `kani`, `flux-rs`, `loom`, `miri`, `proptest`, `cargo-fuzz`.
   - Use canonical `applicability`: `required`, `not_applicable`, or `blocked_tooling`.
   - Include exact `required_obligation_ids` for required lanes and concrete `non_applicability_evidence_refs` for not-applicable lanes.

2. Replace `.beads/vb-rpch/proof-obligations.verus-flux-rust.planned.jsonl` with canonical `proof-obligation/v1` rows.
   - Add `schema_version`, `domain_claim`, `risk_tags`, `model_bounds`, `tool_metadata`, `trusted_base_refs`, and `behavior_affecting` to every row.
   - Remove alias-only `bounds`; use `model_bounds`.
   - Keep exact commands and workdir `/home/lewis/src/vb-jpq7-jj-fix`.

3. Repair Kani lane decisions.
   - Do not treat suffix scope as non-applicability evidence.
   - Either plan Kani harness obligations for the high-risk clauses already identified in `verification-layers.md`, or justify per tuple why Kani is genuinely not applicable.

4. Preserve these acceptable elements.
   - Keep Flux as `blocked_tooling` until `cargo flux --version` succeeds; do not claim Flux proof success.
   - Keep production Rust proof-attachment and behavior repair in State 11 / Holzman.
   - Keep TLC round-3 approval scoped to bounded TLA/TLC evidence only; do not cite it as Rust/Flux proof evidence.

## Rerun checklist

- Run JSONL syntax validation after rewrite.
- Verify every planner lane has a stable `id` for reviewer rows.
- Verify no behavior-affecting waiver is introduced.
- Recompute hashes and resubmit to State 4 proof-plan-reviewer.
