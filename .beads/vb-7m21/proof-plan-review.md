# Proof Plan Review — vb-7m21 State 4 Replan (Reduced Scope, Second Review)

reviewer_skill: proof-plan-reviewer
reviewer_invocation_id: proof-plan-reviewer-vb-7m21-state4-replan-002
writer_invocation_id: proof-planner-vb-7m21-state4-replan-001
review_state: 4
bead_id: vb-7m21
replan: true
replan_sequence: 2
prior_replan_review: proof-plan-reviewer-vb-7m21-state4-replan-001

## Reviewed Replanned Artifacts

These are the artifacts produced by `proof-planner-vb-7m21-state4-replan-001` (ledger sequence 19), which executed the reduced-scope replan prescribed by the prior review `proof-plan-reviewer-vb-7m21-state4-replan-001` (sequence 18).

| Artifact | SHA-256 |
|---|---|
| proof-strategy.md | `91d443f0efc0b54c79a8accfa309355c6dcfcd3ba1583c85aa5c9a461a734f3a` |
| verifier-lane-decisions.jsonl | `84cf002c42b9c9c87fb0f0c7da342094861ffa866ce7304f93b85bc6cf47b4d2` |
| proof-obligations.planned.jsonl | `0753d9fdf8ff1e519baeca3822f6135312a3d1ebb0adf769812cb4535beea2b2` |
| trusted-base-plan.md | `6ba86e0a256b5cbaa46b8f7df23ad31368ff538131e4cebcd31f59f98ad60d3b` |
| contract.md (unchanged) | `b7336db37c9a55b6b1d7554b7be2f395963b0a70efdf03a977eba7672bdfe697` |
| proof-seeds.jsonl (unchanged) | `99698744bca651be4ff4489ed12195a216dc24fcaa32ab1bddfc2de00d4c70a8` |
| boundary-map.md (unchanged) | `b7fc8066a7e606fc4f0b38a880d3f1e3d83b27ede765943ada1957fce5dad840` |
| traceability-matrix.jsonl (unchanged) | `a34b31a615fe7a1d9793613e91ded2eabea89e50a4e5b33d388222a2335cf788` |

## Provenance

- **Prior replan review**: `proof-plan-reviewer-vb-7m21-state4-replan-001` (sequence 18) REJECTED the original 39-obligation plan and prescribed a reduced-scope replan with 14 obligations (3 Kani + 8 proptest + 3 fuzz).
- **Replanning planner**: `proof-planner-vb-7m21-state4-replan-001` (sequence 19) executed the prescription exactly.
- **Current reviewer**: `proof-plan-reviewer-vb-7m21-state4-replan-002` — independent from the replanning planner invocation.
- **State 6 rejection root cause**: Original plan over-scoped for a test-first bead, demanding Verus/Flux/TLA+/full Kani for a bead whose primary deliverable is a test fixture corpus file. The prior replan review correctly identified this as a scope classification error.

## Replanned Plan Baseline

```
72 lane decisions = 9 proof seeds × 8 verifiers
14 required proof obligations (all required=true)
   3 Kani    (PS-001, PS-002, PS-003 — codec-boundary seeds only)
   8 proptest (PS-001 through PS-008 — all behavior-affecting seeds)
   3 cargo-fuzz (PS-001, PS-002, PS-003 — binary-envelope seeds only)
0 blocked tooling
0 behavior waivers
```

## Lane Decision Verification

### Required Lanes (14)

| VLD ID | Seed | REQ | Verifier | Obligation ID | 
|---|---|---|---|---|
| VLD-vb-7m21-003 | PS-001 | REQ-5 | kani | PO-vb-7m21-kani-001 |
| VLD-vb-7m21-007 | PS-001 | REQ-5 | proptest | PO-vb-7m21-prop-001 |
| VLD-vb-7m21-008 | PS-001 | REQ-5 | cargo-fuzz | PO-vb-7m21-fuzz-001 |
| VLD-vb-7m21-011 | PS-002 | REQ-3 | kani | PO-vb-7m21-kani-002 |
| VLD-vb-7m21-015 | PS-002 | REQ-3 | proptest | PO-vb-7m21-prop-002 |
| VLD-vb-7m21-016 | PS-002 | REQ-3 | cargo-fuzz | PO-vb-7m21-fuzz-002 |
| VLD-vb-7m21-019 | PS-003 | REQ-6 | kani | PO-vb-7m21-kani-003 |
| VLD-vb-7m21-023 | PS-003 | REQ-6 | proptest | PO-vb-7m21-prop-003 |
| VLD-vb-7m21-024 | PS-003 | REQ-6 | cargo-fuzz | PO-vb-7m21-fuzz-003 |
| VLD-vb-7m21-031 | PS-004 | REQ-4 | proptest | PO-vb-7m21-prop-004 |
| VLD-vb-7m21-039 | PS-005 | REQ-8 | proptest | PO-vb-7m21-prop-005 |
| VLD-vb-7m21-047 | PS-006 | REQ-9 | proptest | PO-vb-7m21-prop-006 |
| VLD-vb-7m21-055 | PS-007 | REQ-10 | proptest | PO-vb-7m21-prop-007 |
| VLD-vb-7m21-063 | PS-008 | REQ-11 | proptest | PO-vb-7m21-prop-008 |

All 14 required rows have decision reasons, obligation references, and risk tags. ✓

### Not-Applicable Lanes (58)

Every NA row has `limitation_kind: not_applicable_by_contract_evidence` with concrete evidence references. Breakdown by verifier:

| Verifier | NA Count | Primary Evidence |
|---|---|---|
| tla-plus | 9 (all seeds) | boundary-map.md:36-39, local deterministic synchronous corpus |
| verus | 9 (all seeds) | contract.md:26-27, codebase-map.md:7-8, no exec/spec binding targets |
| flux-rs | 9 (all seeds) | contract.md:26-27, no new behavior-affecting Rust code |
| loom | 9 (all seeds) | boundary-map.md:36-39, no concurrency surfaces |
| miri | 9 (all seeds) | boundary-map.md:41-44, no unsafe/FFI/raw-pointer surfaces |
| kani | 5 (PS-004 through PS-008) | codebase-map.md:71-79, integration seeds lack bounded codec properties |
| cargo-fuzz | 6 (PS-004 through PS-009) | boundary-map.md, no parser/codec/hostile byte-input surface |
| proptest | 1 (PS-009) | no-copy fence is review/integration, not proptest |

All NA decisions are justified with contract-level, boundary-level, or codebase-level evidence. No NA decision relies solely on the prior review document as circular evidence. ✓

## Obligation Quality Verification

### Schema Compliance
All 14 obligations use `schema_version: proof-obligation/v1`. No legacy alias fields (`layer`, `checker`, `claim`) present. Every required field per `proof-schemas.md:60-62` is populated. ✓

### Command Precision
- **Kani (3)**: `cargo kani -p vb_storage --harness vb_7m21_<NNN>_harness` — exact harness names, package specification, no vague flags. ✓
- **Proptest (8)**: `cargo nextest run -p velvet-ballistics-workspace-tests --test restate_storage_blackhat_fixture_corpus` — unified command targeting the single test fixture file. ✓
- **Cargo-fuzz (3)**: `cargo fuzz run vb_7m21_storage_envelope -- -max_total_time=60 -runs=10000` — explicit time/run bounds. ✓

### Model Bounds
Every obligation has concrete model bounds. Key examples:
- Kani: bounded byte arrays ≤ RECORD_HEADER_BYTES + 4, kani::Arbitrary/kani::any generators, no hardcoded structure-only proof
- Proptest: deterministic seeds recorded, temporary stores only, fixture IDs unique, typed key constructors
- Fuzz: 60-second smoke + future deep run, seed corpus from VB APIs/constants only

### Non-Vacuity
- Kani obligations require `kani::Arbitrary` or `kani::any()` generators; explicitly prohibit hardcoded structure-only proofs. ✓
- Proptest obligations require deterministic seed recording and exact typed outcome assertions. ✓
- Fuzz obligations require seed corpus generated from VB APIs/constants and expected-typed-rejection non-crash behavior. ✓

### Trusted Base References
All 14 obligations have `trusted_base_refs` matching entries in `trusted-base-plan.md`. All trust references are marked "No behavior-affecting trust permitted." The single non-behavior external reference (`TB-vb-7m21-REST-001`) for Restate source unavailability is correctly classified as non-behavior-affecting and has compensating evidence. ✓

### Workdir
All obligations specify `workdir: /home/lewis/isolated/femdation-velvet-ballistics/vb-7m21`. ✓

## Excluded Verifier Justification

### Verus — Not Applicable (9/9 seeds)
No production implementation in scope until State 11. No new `exec fn` targets to bind `spec fn`/`proof fn` contracts to. The `contract.md:26-27` acceptance boundary and `codebase-map.md:7-8` confirm the bead outputs test files only. Every `proof-seeds.jsonl` entry excludes `verus` from `suggested_layers`. Evidence is concrete, non-circular, and grounded in delivery scope. ✓

### Flux — Not Applicable (9/9 seeds)
No new behavior-affecting Rust code to annotate with `#[sig]`/`#[refined_by]`. The bead adds test fixtures against existing APIs, not new production functions. Evidence: `contract.md:26-27`, `proof-seeds.jsonl` suggested_layers. ✓

### TLA+ — Not Applicable (9/9 seeds)
No temporal protocol, retry, lease, lifecycle, distributed, or interleaving behavior. The bead is local, deterministic, synchronous test infrastructure per `boundary-map.md:36-39`. The temporal behaviors (journal gaps, duplicate events, snapshot recovery) are observable through deterministic behavior test assertions. Per `proof-schemas.md:37`: TLA+ is not part of the default Rust-local profile and never substitutes for implementation-bound Rust evidence. ✓

### Loom — Not Applicable (9/9 seeds)
No implementation concurrency, cancellation, shutdown, atomics, channels, locks, task ownership, or interleaving risk. Evidence: `boundary-map.md:36-39`. ✓

### Miri — Not Applicable (9/9 seeds)
No first-party unsafe, FFI, raw pointer, aliasing, provenance, layout, or UB-sensitive code. Evidence: `boundary-map.md:41-44`. ✓

### Kani — Not Applicable for Integration Seeds (5/9 seeds: PS-004 through PS-008)
These seeds operate through higher-level public storage APIs (FjallJournal, events_for_run, has_*_index_entry) where Kani's bounded model checking adds minimal value over proptest + behavior test assertions. The existing Kani harness infrastructure (`codebase-map.md:71-79`) already covers bounded codec properties. ✓

### Cargo-Fuzz — Not Applicable for Non-Codec Seeds (6/9 seeds: PS-004 through PS-009)
No parser/codec/hostile byte-input surface. Model boundary is persistence invariants, not `decode_record`/`decode_record_header` hostile bytes. ✓

### Proptest — Not Applicable for PS-009 (1/9)
REQ-16 no-copy fence is a provenance/source-boundary contract, not a mathematical verifier target. Evidence: `proof-seeds.jsonl:9`, `boundary-map.md:31-34`, `hazard-analysis.md H4`. ✓

## Waiver Review

The single non-behavior waiver candidate for external Restate source/layout comparison unavailability remains valid. The replan does not change this assessment. No behavior-affecting waivers exist. ✓

## Bridge Readiness

This replanned plan is precise enough for proof-writer (State 5) to produce:
- 3 Kani harness files (`kani_vb_7m21_001.rs` through `kani_vb_7m21_003.rs`) with `kani::Arbitrary`-based generators
- 8 proptest fixture sections in the corpus test file with deterministic seed recording
- 3 cargo-fuzz targets with VB-derived seed corpora

The obligations specify exact artifacts, commands, workdir, and expected evidence. The trusted-base plan ledgeres all model bounds. Bridge from proof to implementation (State 7) needs no additional scope adjustment. ✓

## Findings

No findings. The replanned artifacts conform exactly to the reduced-scope prescription. All 72 lane decisions and 14 obligations are valid. No schema drift, missing fields, circular evidence, self-stamped fields, or behavior-affecting waivers detected.

## Lane Review Disposition Summary

- **Accepted**: 72 lane decisions (all)
- **Rejected**: 0
- **Blocked tooling**: 0
- **Behavior waivers**: 0

Full per-lane dispositions in `verifier-lane-review.jsonl`.

## Reviewer Disposition

The replanned proof plan produced by `proof-planner-vb-7m21-state4-replan-001` is APPROVED. It faithfully executes the reduced-scope prescription from `proof-plan-reviewer-vb-7m21-state4-replan-001`. The plan contains exactly 14 obligations (3 Kani + 8 proptest + 3 fuzz) with precise commands, concrete model bounds, non-vacuity constraints, and evidence-backed NA decisions for the remaining 58 lanes. It is ready for proof-writer (State 5) execution.

STATUS: APPROVED
