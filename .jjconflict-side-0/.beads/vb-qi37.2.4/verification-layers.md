# Verification Layers

## Boundary
- Verus-owned kernel: `crates/vb_core/src/budget.rs` budget arithmetic and refinement from `WholeWorkflowBudget` to `AggregateResourceBudget`.
- TLA+ temporal model: `specs/tla/BoundedAdmission.tla` admission ordering and reservation-before-run invariants.
- Kani/proptest: bounded state exploration for nested collect/reduce/repeat/together compositions and diagnostics.
- Runtime shell: admission capacity fit and error propagation; no YAML/JSON/HTTP in runtime core.
- Theorem projection: waived; see `lean-contract.md`.

## Layer Assignment
- PRE-001 -> static-scan + unit/property construction coverage.
- PRE-002 -> Verus + Kani + proptest.
- PRE-003 -> proptest + static-scan + mutation.
- PRE-004 -> TLA+ + Verus refinement.
- POST-001 -> Verus + Kani + gauntlet-proof.
- POST-002 -> Verus + proptest.
- POST-003 -> Verus + proptest.
- POST-004 -> Verus + Kani.
- POST-005 -> proptest + Kani.
- POST-006 -> proptest + fuzz-smoke where authored input generates collect IR.
- POST-007 -> Verus + proptest.
- POST-008 -> Verus + proptest.
- POST-009 -> proptest + mutation + snapshot/golden diagnostics if approved by later bead.
- POST-010 -> TLA+ + gauntlet-proof.
- INV-001 -> TLA+.
- INV-002 -> Verus.
- INV-003 -> Verus + Kani.
- INV-004 -> proptest + mutation.
- INV-005 -> proptest + mutation.
- INV-006 -> TLA+ + Verus refinement.

## Verus Scope
- Rust target: `crates/vb_core/src/budget.rs`.
- Existing proof file: `verification/verus/budget_bounded.rs`.
- Spec/proof surface: `spec_count_total_steps_bounded`, `proof_steps_bounded`, `proof_sequential_add_bounded`, and required extensions for nested multiplication/max/refinement.
- Invariants: checked sum, checked multiplication, monotone aggregate dimensions, policy upper bounds, no overflow success path.
- Trusted boundary: `CompiledWorkflow`/`WorkflowParts` shape is structurally valid or returns typed `WorkflowError`; runtime shell excluded.
- Evidence command: `verus verification/verus/budget_bounded.rs` and rollup `moon run :verify-proof`.

## TLA+ Scope
- Module/model path: `specs/tla/BoundedAdmission.tla`.
- Config: `specs/tla/BoundedAdmission.cfg`.
- Variables: `admitted_runs`, `shard_runs`, `reserved_resources`, `pending_admission`, plus required future explicit verified-budget state if missing.
- Actions: `Init`, `RequestAdmission`, `AdmitRun`, `RejectAdmission`, `RunCompleted`.
- Safety invariants: no admission without reservation; no admission without verified budget; shard capacity bounded.
- Temporal properties: pending admission eventually resolves under weak fairness; no deadlock.
- Refinement boundary: Rust verification produces aggregate budget before `AdmitRun` abstraction.
- Evidence command: `tlc -config specs/tla/BoundedAdmission.cfg specs/tla/BoundedAdmission.tla`.

## Defense in Depth
- Kani: bounded model checks for overflow/rejection on nested budget composition in `crates/vb_core/src/budget.rs` or existing Kani budget harnesses if present.
- Proptest: generated IR shapes containing nested `collect`, `reduce`, `repeat`, `together`; accepted cases remain under policy and rejected cases identify the growth source.
- Miri: `moon run :miri` for interpreter-level UB/panic smoke in core/validation crates.
- Fuzz: `moon run :fuzz-smoke` to ensure malformed authored inputs/IR builders do not bypass bounds once harnesses cover this path.
- Mutation: `moon run :mutants-smoke` plus later targeted mutants for diagnostics and overflow branches.
- Static scan/lint: `moon run :lint-src`, `moon run :check`, `moon run :nightly-feature-gate`.
- Rollups: `moon run :verify-standard`, `moon run :verify-deep`, `moon run :verify-proof`.

## Waivers
- Lean/Aeneas/Hax waived pending independent review.
- No performance or assembly obligation: this bead makes correctness/admission claims only and no speed claim.

## Required Verifier Lanes
1. Proof lane: `moon run :verify-proof`.
2. Deep defense lane: `moon run :verify-deep`.
3. Standard CI confidence lane: `moon run :verify-standard`.

## Status / Evidence Summary
- Status: planned. This file assigns verification layers; it does not execute them.
