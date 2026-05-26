reviewer_skill: proof-plan-reviewer
reviewer_invocation_id: inv-proof-plan-reviewer-s4

STATUS: APPROVED

# Proof Plan Review: vb-xi2f.4 — Route Compiler Emission through try_from_parts

## Review Metadata

- **reviewer_skill**: proof-plan-reviewer
- **reviewer_invocation_id**: inv-proof-plan-reviewer-s4
- **review_state**: 6
- **planner_invocation_id**: inv-004
- **review_date**: 2026-05-24
- **bead_id**: vb-xi2f.4

## Reviewed Artifacts

| Artifact | SHA-256 |
|----------|---------|
| proof-strategy.md | cb2f38b45200c897a1a23152dba6181e44cd123138ab5f81cfee4b658f6f3ffe |
| verifier-lane-decisions.jsonl | 88bf95b18c0a005d124e645c59a8d6b4b66f36f0400c29b81fbe9119ceb8743c |
| proof-obligations.planned.jsonl | 529f27f075443f9369b340f5c7a53d7e6f6fe76c9ef823b4a1215d45193d968b |
| trusted-base-plan.md | f83fc26da1e297a714c0f36f29c211255e9dc8d1bed033800d572de089927b18 |
| waiver-candidates.jsonl | 56bb974a11214c7b569a854e316b9d023ebfd0baeaef4421469b84957e8793bb |

## Review Summary

- **Total lane decisions reviewed**: 16
- **Accepted**: 16
- **Rejected**: 0
- **Required lanes accepted**: 8
- **Not-applicable lanes accepted**: 8
- **Blocked tooling lanes**: 0
- **Behavior-affecting waivers**: 0
- **Findings**: 0

## Required Lane Review

| Review ID | Decision ID | Seed | Verifier | Obligation | Disposition | Notes |
|-----------|-------------|------|----------|------------|-------------|-------|
| VLR-001 | VL-001 | seed-001 | verus | PO-001 | accepted | Postcondition spec binding compile_source to try_from_parts is well-scoped and targets the exact production function. |
| VLR-002 | VL-002 | seed-001 | kani | PO-002 | accepted | Bounded panic-freedom with explicit bounds (steps≤5, slots≤8, unwind=6). Model reduction (TB-002) is documented and compensated by proptest (PO-003). |
| VLR-003 | VL-003 | seed-001 | proptest | PO-003 | accepted | 10,000 cases with real YAML inputs; directly compensates Kani model reduction and tests the public API surface. |
| VLR-004 | VL-004 | seed-001 | flux-rs | PO-004 | accepted | Refinement on compile_source return path is appropriate for enforcing validated-construction invariant. |
| VLR-009 | VL-009 | seed-002 | verus | PO-005 | accepted | Error-mapping spec (WorkflowError → CompileError::Workflow) is a clean boundary invariant with minimal trusted base. |
| VLR-010 | VL-010 | seed-002 | kani | PO-006 | accepted | Explicit kani::any() usage for WorkflowParts generation; error-variant coverage for bounded invalid inputs is well-scoped. |
| VLR-011 | VL-011 | seed-002 | proptest | PO-007 | accepted | Invalid-class coverage (out-of-bounds indices, backward edges, empty nodes, etc.) is comprehensive. |
| VLR-012 | VL-012 | seed-002 | flux-rs | PO-008 | accepted | Return-type refinement for error typing is appropriately scoped to try_from_parts in vb_core. |

## Not-Applicable Lane Review

All not-applicable lanes cite concrete evidence refs and provide domain-specific rationale. No weak "out of scope" hand-waving.

| Review ID | Decision ID | Seed | Verifier | Disposition | Evidence Quality |
|-----------|-------------|------|----------|-------------|------------------|
| VLR-005 | VL-005 | seed-001 | tla-plus | accepted | Cites contract.md#postconditions and hazard-analysis.md#H1. Temporal/distributed rationale is sound. |
| VLR-006 | VL-006 | seed-001 | loom | accepted | Cites type-contracts.md#typestates and codebase-map.md. Single-threaded rationale is correct. |
| VLR-007 | VL-007 | seed-001 | miri | accepted | Cites #![forbid(unsafe_code)] in both vb_compile and vb_core source files. Direct evidence. |
| VLR-008 | VL-008 | seed-001 | cargo-fuzz | accepted | Cites delivery-scope.jsonl and proof-seeds.jsonl#seed-002. Parser/codec scope boundary is justified. |
| VLR-013 | VL-013 | seed-002 | tla-plus | accepted | Cites error-taxonomy.md#structural-validation-errors. Functional-property rationale is consistent with seed-001. |
| VLR-014 | VL-014 | seed-002 | loom | accepted | Cites type-contracts.md#typestates. Synchronous error paths rationale is correct. |
| VLR-015 | VL-015 | seed-002 | miri | accepted | Cites specific source file (crates/vb_core/src/workflow/mod.rs#L1) with unsafe-forbid declaration. |
| VLR-016 | VL-016 | seed-002 | cargo-fuzz | accepted | Cites vb_core proptest artifacts as owning fuzz scope. Call-site replacement rationale is correct. |

## Waiver Review

- **WC-001**: Accepted as non-behavior-affecting (`behavior_affecting: false`). The waiver addresses test-only `from_parts_unchecked` usage in `workspace_tests` and `vb_core/tests`, gated by `#[cfg(test)]`. Boundary proof and compensating evidence (grep + Cargo.toml dependency pruning) are adequate. No behavior-affecting waivers are present in the plan.

## Trusted Base Review

All 6 trusted base entries have documented scope, impact, kind, and compensating evidence:

| ID | Kind | Behavior Affecting | Compensating Evidence | Assessment |
|----|------|-------------------|----------------------|------------|
| TB-001 | external_verified | true | section36 tests + prior Kani/Verus artifacts | Strong. try_from_parts is owned by vb_core with extensive existing verification. |
| TB-002 | model_reduction | true | PO-003 proptest with real YAML input | Strong. Acknowledges Kani harness manual construction and compensates with property-based testing. |
| TB-003 | external_verified | true | vb_yaml tests | Adequate. Parser correctness is a standard external assumption. |
| TB-004 | tool_soundness | false | Flux project test suite | Acceptable. Non-behavior-affecting tool assumption. |
| TB-005 | type_system | false | thiserror tests + Rust compiler | Acceptable. Non-behavior-affecting derive assumption. |
| TB-006 | model_reduction | true | section36 explicit boundary cases | Strong. Arbitrary impl limitations are compensated by explicit unit tests. |

Additional static analysis compensating evidence (CI lint for `from_parts_unchecked` absence) is documented and integrated into the build pipeline.

## Non-Vacuity Assessment

- **Kani PO-002**: Explicit bounds (steps≤5, slots≤8, unwind=6) and named harness. Model reduction (TB-002) is documented and compensated.
- **Kani PO-006**: Explicitly uses `kani::any()` for WorkflowParts generation. Bounds (nodes≤4, slots≤4, unwind=5) are stated.
- **Proptest PO-003 / PO-007**: 10,000 cases provide probabilistic non-vacuity. Real YAML inputs avoid synthetic input starvation.
- **Verus PO-001 / PO-005**: Target specific postconditions and error mappings on concrete functions, not vacuous universal quantification.

The plan addresses non-vacuity through explicit model bounds, named harnesses, compensating real-input testing, and documented model reductions.

## Bridge Planning Assessment

- Every obligation maps to an exact Rust artifact path and target function.
- Obligations reference: `compile_source` (vb_compile), `YamlCompiler::compile` (vb_compile), `CompiledWorkflow::try_from_parts` (vb_core).
- Trusted base refs are explicitly linked to obligations.
- Commands include exact package names, harness names, toolchain versions (`nightly-2026-04-28`), and flags (`--quiet`, `--harness`).
- Workdir is consistently specified as the repo root.

## Obligation Schema Review

All 8 planned obligations (PO-001 through PO-008):
- Use schema_version `proof-obligation/v1`.
- Contain all required fields: id, requirement_id, contract_clause, domain_claim, risk, risk_tags, verifier, artifact, target, command, workdir, expected_evidence, assumptions, model_bounds, tool_metadata, trusted_base_refs, required, behavior_affecting, mode, owner_state, rerun_from, status.
- Contain no legacy alias fields (`layer`, `checker`, `claim`).
- Use canonical `target` values referencing production source paths.

## Findings

None.

## Approval Rationale

The proof plan for vb-xi2f.4 is **precise, complete, and ready for proof-writer execution**.

- All core verifier lanes (verus, kani, flux-rs, proptest, tla-plus, loom, miri, cargo-fuzz) have decisions for both proof seeds.
- Required lanes (8) are justified with concrete, well-scoped obligations.
- Not-applicable lanes (8) cite specific evidence references; no hand-waving.
- The trusted base is comprehensive with compensating evidence for every behavior-affecting assumption.
- No behavior-affecting waivers are present.
- Commands are exact and reproducible.
- Bounds are explicit and appropriate for the problem domain.
- Bridge planning is present: every claim maps to a specific production function and artifact path.
- Non-vacuity is addressed through model bounds, named harnesses, kani::any() usage, and compensating real-input proptest.

The plan passes all proof-plan-reviewer gates and may proceed to proof-writer.
