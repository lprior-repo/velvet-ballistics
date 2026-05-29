# Proof Plan Review: vb-xi2f.13 — Nested Choose Primitive Body Lowering

**reviewer_skill:** proof-plan-reviewer
**reviewer_invocation_id:** ppr-vb-xi2f.13-20260529-001
**review_state:** 4 (proof-plan-review)
**bead_id:** vb-xi2f.13
**reviewed_at:** 2026-05-29T00:00:00Z

## Re-review Context

This is a re-review following the first rejection (artifacts now materialized). All five previously-missing artifacts now exist on disk with correct schema versions and content.

## Reviewed Artifacts

| Artifact | Path | SHA-256 |
|---|---|---|
| contract.md | `.beads/vb-xi2f.13/contract.md` | a363d37fe2fc29bf8dce077a2fdfcd0dd35cc9e67df95881dd52063c415fac4b |
| domain-model.md | `.beads/vb-xi2f.13/domain-model.md` | 2a130e69ec0f6a4d4c578d13bd40bbb0715f3cf047e985c24dbc264ca794fede |
| error-taxonomy.md | `.beads/vb-xi2f.13/error-taxonomy.md` | e581d269d943a38f7df2bb73597a91e9db21b37ce0f9c6244bd3497be9ef0df0 |
| hazard-analysis.md | `.beads/vb-xi2f.13/hazard-analysis.md` | 9575d0e4e3c82f28422382808085474a847f618ab55f114f6a51288b42d94685 |
| proof-seeds.jsonl | `.beads/vb-xi2f.13/proof-seeds.jsonl` | 8dde2306f3a69c18ddcd070c4a1d76c30bbd0a23e38e72062191178684215a36 |
| traceability-matrix.jsonl | `.beads/vb-xi2f.13/traceability-matrix.jsonl` | 246c59047edd8d8cde6e4c2f5163dabd0e0d21ed9dd2225eb94ab30a9bb3f98f |
| type-contracts.md | `.beads/vb-xi2f.13/type-contracts.md` | 1fbdf663bc6d5167c9983423764c9e7c2988e1cf453c101f366a51d1309065ea |
| workflow-model.md | `.beads/vb-xi2f.13/workflow-model.md` | 69b401525e92377552cb98ee6e4a227fd67da85d896d1560886842f1c334852d |
| boundary-map.md | `.beads/vb-xi2f.13/boundary-map.md` | a4935d1ba5d8fa4779b112c5192b823fdee582f6df9e06a6c79c6fcf0c95e054 |
| proof-strategy.md | `.beads/vb-xi2f.13/proof-strategy.md` | bc4c00ac537e7ef199ea79210580b84438ec3938d8008bde30047063cb024f04 |
| verifier-lane-decisions.jsonl | `.beads/vb-xi2f.13/verifier-lane-decisions.jsonl` | ab193acf66fdd8bfd2f1dd5b8ed7884054635e035a145c4f77a6342ec38f7392 |
| proof-obligations.planned.jsonl | `.beads/vb-xi2f.13/proof-obligations.planned.jsonl` | fddba31b3b1a35b2d67ab41369dcbc7073741879effd89d957d877db25c4735b |
| trusted-base-plan.md | `.beads/vb-xi2f.13/trusted-base-plan.md` | 691638a44600a68ee27d7ba4c77755471d582a60721c234c78167a258c2de471 |
| waiver-candidates.jsonl | `.beads/vb-xi2f.13/waiver-candidates.jsonl` | 58258dabc972bbbc58542adcb8985e919875b7f9c1f64365e3c88f661768e3b4 |
| agent-invocation-ledger.jsonl | `.beads/vb-xi2f.13/agent-invocation-ledger.jsonl` | d3f7f4d522935ab912665e39cca3189f9c2d310c4b9cd4796a098da93eecb4b2 |

## Provenance Check

- **Planner invocation:** `proof-planner-vb-xi2f.13-20260529-001` (ledger entry 2, skill: proof-planner)
- **Reviewer invocation:** `ppr-vb-xi2f.13-20260529-001` (this review, skill: proof-plan-reviewer)
- **Planner/reviewer distinct:** YES — different invocation IDs, different skills
- **Invocation ledger entries:** 2 entries (femdation-controller + proof-planner)
- **Planner recorded output artifacts:** All 5 core artifacts with hashes matching current disk content

## Schema Compliance

| Artifact | Schema Version | Rows | Valid |
|---|---|---|---|
| proof-seeds.jsonl | proof-seed/v1 | 15 | PASS |
| verifier-lane-decisions.jsonl | verifier-lane-decision/v1 | 62 | PASS |
| proof-obligations.planned.jsonl | proof-obligation/v1 | 23 | PASS |
| waiver-candidates.jsonl | waiver-candidate/v1 | 2 | PASS |
| trusted-base-plan.md (ledger rows) | trusted-base-ledger/v1 | 5 | PASS |

No legacy alias fields detected. All `target` fields are canonical (no `layer`, `checker`, or alias-only `claim` fields). Commands use exact flags, workdirs, and expected evidence paths.

## Lane Decision Coverage Summary

### Default Profile Coverage (Verus, Kani, Flux, proptest) for Behavior-Affecting Seeds

| Seed | Verus | Kani | Flux | proptest | TLA+ | Loom | cargo-fuzz |
|---|---|---|---|---|---|---|---|
| PS-TEMPORAL-001 | required | required | required | required | not_applicable | — | — |
| PS-TEMPORAL-002 | required | required | required | required | not_applicable | — | — |
| PS-TEMPORAL-003 | required | required | not_applicable | required | not_applicable | — | — |
| PS-ARITH-001 | required | required | not_applicable | not_applicable | — | — | — |
| PS-ARITH-002 | required | required | not_applicable | not_applicable | — | — | — |
| PS-INVARIANT-001 | required | required | required | required | — | — | — |
| PS-INVARIANT-002 | required | required | required | required | — | — | — |
| PS-FANOUT-001 | not_applicable | required | not_applicable | not_applicable | — | — | — |
| PS-TYPE-001 | not_applicable | not_applicable | not_applicable | not_applicable | — | — | — |
| PS-LIVENESS-001 | required | required | not_applicable | not_applicable | — | — | — |
| PS-CONCURRENCY-001 | — | — | — | — | — | not_applicable | — |
| PS-INPUT-001 | required | required | required | required | — | — | required |
| PS-INPUT-002 | required | required | not_applicable | required | — | — | required |
| PS-EMISSION-PARITY | required | required | required | required | — | — | — |
| PS-YAML-FREE-IR | not_applicable | required | not_applicable | not_applicable | — | — | — |

**Coverage summary:**
- Verus: 10 required, 3 not_applicable, 1 out-of-scope (PS-TYPE-001), 1 non-behavior (PS-CONCURRENCY-001)
- Kani: 12 required, 1 not_applicable, 1 out-of-scope, 1 non-behavior
- Flux: 7 required, 6 not_applicable, 1 out-of-scope, 1 non-behavior
- proptest: 7 required, 6 not_applicable, 1 out-of-scope, 1 non-behavior
- TLA+: 3 not_applicable (all with compile_time_not_temporal evidence) — correct
- Loom: 1 not_applicable (no_concurrency_hazard) — correct
- cargo-fuzz: 2 required (hostile-input risk tag) — correct

### Not-Applicable Justification Assessment

All `not_applicable` decisions cite concrete evidence references (contract clauses, hazard analysis, type-system guarantees, or risk-tag analysis) and have specific `limitation_kind` values. No decision relies on "too hard" or "not practical" reasoning. The justifications are:

- **compile_time_not_temporal** (TLA+ for PS-TEMPORAL-001/002/003): Correct — these are compile-time layout invariants, not runtime temporal protocols.
- **single_inequality_not_refinement** (Flux for PS-TEMPORAL-003): Acceptable — Verus/Kani provide complete coverage for a single inequality.
- **single_op_not_refinement** (Flux for PS-ARITH-001): Acceptable — single checked_add call.
- **deterministic_not_statistical** (proptest for PS-ARITH-001/002): Reasonable — overflow is deterministic, Kani provides bounded coverage.
- **type_system_guarantee** (Flux for PS-ARITH-002, Verus/Flux/proptest for PS-YAML-FREE-IR): Strong — type system already enforces the invariant.
- **simple_bound_check** (Verus/Flux for PS-FANOUT-001): Acceptable — integer comparison len()<=64.
- **error_path_not_refinement** (Flux for PS-LIVENESS-001): Acceptable — error path is not a refinement property.
- **parser_boundary_not_lowering** (Flux for PS-INPUT-002): Correct — depth enforcement is at parser level.
- **contract_non_goal_exclusion** (all lanes for PS-TYPE-001): Correct — Contract Non-Goals item 4 explicitly excludes boolean slot type validation.

## Obligation Quality Assessment

All 23 obligations have:
- ✅ Exact command with flags (`--features`, `--harness`, `--unwind`, `--package`, etc.)
- ✅ Concrete workdir paths
- ✅ Expected evidence artifact paths under `.evidence/`
- ✅ Model bounds (unwind limits, branch counts, step counts, durations)
- ✅ Tool metadata with minimum versions
- ✅ Trusted base references where applicable
- ✅ GOD RULE compliance annotations (kani::Arbitrary, no hardcoded shapes, exec-fn binding)

Kani obligations (12): Harnesses target production functions with `kani::Arbitrary` per GOD RULE 1. Unwinds range from 64-256, appropriate for bounded model checking of choose lowering.
Verus obligations (4): Spec functions model production `exec fn` behavior per GOD RULE 2.
Flux obligations (3): Refinement annotations on production source files.
proptest obligations (2): 10,000 cases with shrinking enabled.
cargo-fuzz obligations (2): libFuzzer + ASAN, 600s max duration.

## Trusted Base Assessment

Five trust boundaries identified and ledgered:

| ID | Component | Trust Kind | Compensating Evidence | Status |
|---|---|---|---|---|
| TB-001 | SlotCompiler::record_slot | external_body | PO-KANI-006, PO-FLUX-001, PO-PROPTEST-002 | accepted |
| TB-002 | body_width | external_body | PO-KANI-005, PO-VERUS-004, PO-KANI-001 | accepted |
| TB-003 | step_idx | external_body | PO-KANI-005, PO-VERUS-004 | accepted |
| TB-004 | lower_choose | external_body | Existing tests, PO-KANI-006, PO-KANI-008 | accepted |
| TB-005 | vb_validate | external_body | Validator test suite, AC7 | accepted |

All trust markers have:
- Clear scope boundaries
- Impact assessment (HIGH/MEDIUM)
- Compensating evidence (Kani/Verus/Flux/proptest or existing test suites)
- Future expiry dates (2026-12-31)
- No unledgered implicit assumptions

Non-trusted components (`choose_width`, `lower_canonical_choose`, `slot_from_text`, `replay_choose_slot`) are fully within verification scope with multiple obligation coverage.

## Waiver Assessment

| Waiver | Seed | Behavior Affecting | Assessment |
|---|---|---|---|
| WC-001 | PS-TYPE-001 | false (per waiver) | Accepted. Contract Non-Goals item 4 explicitly excludes compile-time boolean slot type validation. Runtime safety net exists (replay_choose_slot rejects non-bool). Hazard H9 documented. Bound to this bead; expires 2026-12-31. **Note:** PS-TYPE-001 seed has behavior_affecting=true, creating a minor classification tension. The seed correctly identifies the property as behavior-relevant, but the contract non-goal excludes it from this bead's scope. The waiver's behavior_affecting=false is correct for this bead's delivery scope. |
| WC-002 | PS-INPUT-002 (partial) | false | Accepted. Only defers parser-level fuzzing (vb_yaml crate). Compiler-level fuzzing (PO-FUZZ-002) and Kani harness (PO-KANI-011) remain in scope. Depth limits enforced at parser boundary before lowering. Expires 2026-12-31. |

Both waivers are non-behavior-affecting, have boundary proofs, compensating evidence, and future expiry. No behavior-affecting waivers present.

## Non-Vacuity Assessment

Proof-strategy.md Section 7 explicitly addresses GOD RULES:
- Rule 1 (no hardcoded Kani shapes): Kani obligations specify kani::Arbitrary usage
- Rule 2 (no vacuum Verus proofs): Verus obligations require exec fn binding via requires/ensures
- Rule 4 (no loop oscillations): If verification exposes a flaw, fix implementation, not harness

Kani obligations use `kani::Arbitrary`/`kani::any()` for ChooseBranch and StepAst structures. No hardcoded structural inputs detected.

## Bridge Strategy

Proof-strategy.md Section 6 documents bridge planning:
- Each proof claim mapped to exact Rust source refs
- Independent behavior tests specified per acceptance criteria (AC1-AC10)
- Refinement harness refs documented
- Proof-to-implementation bridge prepared in `proof-to-implementation-input.md`

Bridge coverage is adequate for proof-writer and proof-to-implementation phases.

## Acceptance Criteria Coverage

All 10 acceptance criteria (AC1-AC10) map to specific proof obligations and seeds (proof-strategy.md Section 8). Coverage is complete with no orphaned ACs.

## Hazard Coverage

All 14 hazards (H1-H14) have corresponding proof obligations or documented mitigation (hazard H9 → WC-001, H11 → not_applicable, H14 → deferred non-behavior). No uncovered hazards.

## Decision

**STATUS: APPROVED**

The proof plan is comprehensive, schema-compliant, and sufficiently precise for proof-writer and proof-to-implementation phases. All five previously-missing artifacts are now materialized with correct schemas. Lane decisions cover all 15 proof seeds across the required default profile (Verus, Kani, Flux, proptest) plus conditional lanes (TLA+, Loom, cargo-fuzz) where risk tags mandate them. All 23 proof obligations have exact commands, workdirs, bounds, and expected evidence. The five trust boundaries are ledgered with compensating evidence. The two waivers are non-behavior-affecting with proper lifecycle fields.

**Resolved from first review:** All five missing artifacts (proof-strategy.md, verifier-lane-decisions.jsonl, proof-obligations.planned.jsonl, trusted-base-plan.md, waiver-candidates.jsonl) are now materialized. Planner invocation recorded in agent-invocation-ledger.jsonl (entry 2). Verus coverage added for 10 of 14 behavior-affecting seeds with not_applicable evidence for the remaining 4. Flux coverage extended to 7 seeds. TLA+ assessed for all temporal-tagged seeds with concrete non-applicability evidence. PS-TYPE-001 reconciled with contract Non-Goals item 4 via waiver WC-001.

**Minor finding noted (see proof-plan-findings.jsonl):** PS-TYPE-001 seed behavior_affecting flag (true) conflicts with waiver WC-001 behavior_affecting flag (false). This is substantively resolved by contract Non-Goals exclusion but creates a schema-level tension. Recommend adding a note to PS-TYPE-001 clarifying that the property is behavior-affecting in general but out of scope for this bead.
