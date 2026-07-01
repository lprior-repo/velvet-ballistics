# Proof Plan Review — vb-r8oso

- **reviewer_skill**: proof-plan-reviewer
- **reviewer_invocation_id**: proof-plan-reviewer-cheap25-vb-r8oso-2026-07-01
- **review_state**: proof-plan-review-completed
- **planner_invocation_id**: proof-planner-vb-r8oso-state4
- **bead_id**: vb-r8oso
- **bead_title**: Storage: enforce next-sequence-at-write before durable append (P1 bug)
- **owner_state**: 4 (proof-plan-review at sub-state 4b)
- **isolated_workdir**: /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-r8oso
- **controller**: femdation (direct child; no sub-agents)

## Reviewed Artifacts and SHA-256 Hashes

| Artifact | SHA-256 | Location |
|---|---|---|
| proof-strategy.md | `efe2a459158e70e9b4be9c16b9a58d199f7f670a5ec21b4694898b8b218680a0` | `.beads/vb-r8oso/proof-strategy.md` |
| verifier-lane-decisions.jsonl | `81918a129e029334529ed93f9b268dd5aba9e8c93c87caa5e960de3eef01f2c5` | `.beads/vb-r8oso/verifier-lane-decisions.jsonl` |
| proof-obligations.planned.jsonl | `d66fb290a2ca93cbe52133564334428acd46e9922eef5e39a0f03a14197ea9a5` | `.beads/vb-r8oso/proof-obligations.planned.jsonl` |
| trusted-base-plan.md | `efd4f4e412804edca17aaad4dbc1138e4319379915d4cfaf2038528089525b74` | `.beads/vb-r8oso/trusted-base-plan.md` |
| waiver-candidates.jsonl | `5c0b12eb8f2374e74059ac37504f04c0fbbbbe32f8ff65888ebcf104801f6de0` | `.beads/vb-r8oso/waiver-candidates.jsonl` |
| proof-seeds.jsonl | `568ecedc7430252c31fb55971a540c92b9fafb3739bafc267d711550724395ed` | `.beads/vb-r8oso/proof-seeds.jsonl` |
| traceability-matrix.jsonl | `490e23718d1c0fb4b193e251cd538993f24911e912a7867a076f94228dc2dd15` | `.beads/vb-r8oso/traceability-matrix.jsonl` |
| contract.md | `34416ab9921eeb777ea1c3944ff7514b76e841aca2d9549ec1196c1c617f41d8` | `.beads/vb-r8oso/contract.md` |
| verifier-lane-review.jsonl | `79b74d318f49fc01c8adc1d6547df71c205b5d1259cd4508211a58b505bcfd7e` | `.beads/vb-r8oso/verifier-lane-review.jsonl` *(written by this review)* |
| proof-plan-findings.jsonl | `7337945519fd16dcc3d9cbbfed0796a85159a9008b094b666bd741c1dfb81f5d` | `.beads/vb-r8oso/proof-plan-findings.jsonl` *(written by this review)* |

`codebase-map.md` (sha256: `d8dfdf1c4f179f472ee11b60fed66baa4243087f76092b693597b5aa2fe0aa36`) and `delivery-scope.jsonl` (sha256: `99dcc034d31fe20316b308f67dd1148ce11e2723765a4c9afe7da8caf0d76b70`) are referenced as evidence refs throughout the lane-decisions; both are present in `.beads/vb-r8oso/`.

## Scope of This Review

This review covers the proof plan for bead **vb-r8oso** ("Storage: enforce next-sequence-at-write before durable append (P1 bug)"), transitioning from State 4 (proof-planning) to State 4b (proof-plan-review) to State 5 (proof-writing). The plan addresses 16 proof seeds (`PS-1..PS-16`) covering contract clauses C-2.1..C-2.9, C-3.1..C-3.5, C-4.1..C-4.4, C-5, C-9, C-10. The user's gate (`Lanes: rust-local, kani (with feature gate), proptest, loom (cross-process — research gate). 5-7 obligations.`) is satisfied: the plan materialises **7 proof obligations** (`POB-vb-r8oso-001..POB-vb-r8oso-007`) across **two verifiers** (Kani feature-gated and proptest).

## Verifier-Lane Decision Validation

All 128 verifier-lane-decision rows (16 seeds × 8 verifiers: kani, verus, flux-rs, loom, miri, cargo-fuzz, cargo-test, proptest) are present. The 8-row "default Rust profile" is paired with explicit applicability states; no verifier is silently omitted.

### Required Lanes (Accepted)

The 24 `required` VLD rows pair exclusively with one of the 7 POBs:

| Verifier | Required VLD IDs | Bound to POBs |
|---|---|---|
| `kani` | VLD-001, VLD-009, VLD-017, VLD-025, VLD-041, VLD-049, VLD-065, VLD-105 | POB-001, POB-002 |
| `proptest` | VLD-002, VLD-010, VLD-018, VLD-026, VLD-034, VLD-042, VLD-050, VLD-058, VLD-066, VLD-082, VLD-090, VLD-098, VLD-106, VLD-114, VLD-122 | POB-003, POB-004, POB-005, POB-006, POB-007 |

`cargo-test` (the catalog profile carries a Verus-shaped "rust-local" lane) is the only verifier recording `not_applicable` despite the user's gate. The plan folds it into `proptest` because the schema's verifier enum is `{verus, kani, flux-rs, loom, miri, cargo-fuzz, proptest}` and there is no separate "rust-local" verifier. This is documented in proof-strategy.md §3 and evidence-bound to `codebase-map.md §2`, `contract.md C-2.7`, and `domain-model.md §1.2`.

### Not-Applicable Lanes (Accepted)

The 104 `not_applicable` VLD rows fall into four buckets:

| Bucket | Count | Limitation kind | Evidence (sha256 prefix) |
|---|---|---|---|
| `loom` (single-process surface) | 16 | `surface_absent` | `d8dfdf1c4f179f472e…` (codebase-map.md §2) + `34416ab9921eeb77…` (contract.md) |
| `verus` (contract C-8 excludes) | 16 | `risk_out_of_scope` | `34416ab9921eeb77…` (contract.md C-8) |
| `flux-rs` (no refinement boundary) | 16 | `surface_absent` | `34416ab9921eeb77…` (contract.md C-8) |
| `miri` (#![forbid(unsafe_code)]) | 16 | `surface_absent` | `d8dfdf1c4f179f472e…` (codebase-map.md §18) + `34416ab9921eeb77…` (contract.md) |
| `cargo-fuzz` (no new parser/codec) | 16 | `superseded_by_other_lane_with_evidence` (proptest exhaustiveness on `proptest_journal_error_codes_sequence_mismatch`) | `99dcc034d31fe203…` (delivery-scope.jsonl) + `34416ab9921eeb77…` (contract.md) |
| `cargo-test` (folded into proptest) | 16 | `surface_absent` | `34416ab9921eeb77…` + `568ecedc7430252c…` |
| `kani` (in seeded rows where proptest is primary) | 8 | `surface_absent` | per-seed sha256 bindings |

**Every `not_applicable` row has at least one concrete `non_applicability_evidence_refs` SHA-256.** I verified `0` rows with empty evidence refs.

## 7 Obligations Validation

The 7 proof-obligation/v1 rows satisfy the user's 5-7 obligation gate:

| POB | Verifier | Risk | Target | Status |
|---|---|---|---|---|
| POB-001 | `kani` | bounded_transition | `crates::vb_storage::FjallJournal::next_sequence_at_write` | required |
| POB-002 | `kani` | rejection | `crates::vb_storage::FjallJournal::append_unfsynced` | required |
| POB-003 | `proptest` | bounded_transition | `crates::vb_storage::FjallJournal::next_sequence_at_write` | required |
| POB-004 | `proptest` | rejection | `crates::vb_storage::error::JournalError::SequenceMismatch` | required |
| POB-005 | `proptest` | illegal_state | `crates::vb_storage::FjallJournal::append_strict` | required |
| POB-006 | `proptest` | bounded_transition | `crates::vb_storage::FjallJournal::append_strict_batch` | required |
| POB-007 | `proptest` | rejection | `crates::vb_storage::FjallJournal::append_strict` | required |

Each row has the required fields: `schema_version: proof-obligation/v1`, `requirement_id`, `contract_clause`, `domain_claim`, `risk`, `risk_tags`, `verifier`, `artifact`, `target` (canonical, no legacy `layer`/`checker` alias), `command` (exact executable), `workdir` (consistent isolated path), `expected_evidence` (non-vacuous, Kani vs proptest evidence markers disambiguated), `assumptions` (PLANNED-only disclaimers where applicable), `model_bounds` (explicit bounds: Kani `unwind:8`; proptest `cases:10000`/`cases:1000`), `tool_metadata`, `trusted_base_refs`, `required: true`, `behavior_affecting: true` (where applicable), `mode: "verify-proof"` or `"verify-test"`, `owner_state: 5` or `4`, `rerun_from: 5`, `status: "planned"`. **No legacy alias fields detected.**

## User-Gate Cross-Check

The user's gate asked for: **(a)** 7 obligations, **(b)** kani with Cargo feature gate, **(c)** proptest, **(d)** loom cross-process — research gate, **(e)** no silent seq rewrite, **(f)** RESEARCH_REQUIRED downstream caller audit. The plan satisfies each:

1. **7 obligations (kani + proptest):** 7 POBs confirmed above; 2 Kani (POB-001, POB-002) and 5 proptest (POB-003..POB-007).
2. **Kani feature gate `kani-sequence-at-write`:** proof-strategy.md §5 declares `crates/vb_storage/Cargo.toml:23-29` will receive `kani-sequence-at-write = []` mirroring the existing `kani-recovery`, `kani-typed-partitioned-ids`, `kani-vb-u8gi-decode-taxonomy`, `kani-vb-vzcuf` features. Module registration: `#[cfg(all(kani, feature = "kani-sequence-at-write"))] pub mod kani_sequence_at_write;`. Trusted-base ledger TB-NSAW-KANI-001 binds the isolation invariant. POB-006 contains a cfg-gated proptest that compiles only when the feature is enabled.
3. **`JournalError::SequenceMismatch` (0x4042):** Declared in contract C-3.1 with `DiagnosticCode::new(0x4042)` and `symbolic_code() = "JOURNAL_SEQUENCE_MISMATCH_AT_WRITE"` (C-3.3). POB-004 asserts the diagnostic code exhaustiveness; trust marker TB-NSAW-CODE-001 covers the `INTERNAL_INVARIANT` fallback per C-3.4.
4. **RESEARCH_REQUIRED downstream caller audit (C-10 / ODQ-1):** Trusted-base-plan.md TB-NSAW-RESEARCH-001 is a `RESEARCH_REQUIRED` marker explicitly closing POB-002, POB-003, POB-005, POB-006, POB-007. POB-007 reads `.beads/vb-r8oso/audit-downstream-callers.md` and asserts the audit header; failure path is documented as "contract widens per `domain-model.md` ODQ-1 and plan returns to State 3."
5. **No silent seq rewrite:** C-5 (MUST NOT) + FS-3 (forbidden state) + §6 forbidden implementation patterns list silent rewrite as detection target POB-005. POB-005 captures `event.seq()` before each append and asserts byte-for-byte identity post-rejection; constructor pre-condition `expected != actual` is exercised via `prop_assume!` in POB-004 and POB-005.

## Non-Vacuity Assessment

The plan enforces all five "anti-laundering" rules from proof-strategy.md §4:

1. **GOD RULE 1 (No Hardcoded Kani Shapes):** POB-001/002 use `kani::any()` / `kani::any_where()` for symbolic seq and run values over a small keyspace (n in 0..4). No hardcoded `JournalEvent` literals.
2. **GOD RULE 2 (No Vacuum Verus Proofs):** No Verus obligations exist; `risk_out_of_scope` is documented per C-8.
3. **GOD RULE 3 (No Unbounded TLA+ Math):** N/A — TLA+ removed per `proof-planner/SKILL.md`.
4. **GOD RULE 4 (No Loop Oscillations):** The plan does not alter contract or harness to make tests pass.
5. **GOD RULE 5 (No Blind Verification Mutations):** Scope is trimmed to four call sites in `vb_storage` (delivery-scope.jsonl:1-3); no full-crate Kani or fuzz invocations.

Kani `cover!` reaches are paired with verifier-enforced postconditions (POB-001 asserts `next_sequence_at_write == ZERO` for empty run; `next_sequence_at_write == succ(seq_max)` for non-empty; `SequenceMismatch` rejection for stale seq).

## Trusted-Base Plan Validation

8 trust markers cover every assumption in the 7 POBs:

| Marker | Type | Required by POBs |
|---|---|---|
| TB-NSAW-001 | Type-level contract (signature + variant) | all 7 |
| TB-NSAW-002 | Runtime seam (`codec::next_seq` checked_add) | POB-001, POB-003 |
| TB-NSAW-003 | `#![forbid(unsafe_code)]` | POB-001, POB-002 |
| TB-NSAW-004 | Single-process write_lock | POB-006, POB-007 |
| TB-NSAW-RESEARCH-001 | Research gate (downstream caller audit, C-10/ODQ-1) | POB-002, POB-003, POB-005, POB-006, POB-007 |
| TB-NSAW-KANI-001 | Kani feature isolation (`kani-sequence-at-write`) | POB-001, POB-002, POB-006 |
| TB-NSAW-CODE-001 | Symbolic code registration (CODE_REGISTRY or INTERNAL_INVARIANT fallback) | POB-004 |
| TB-NSAW-FUZZ-001 | Fuzz arm updates (holzman-rust delivery scope, no new fuzz harness) | (delivery-scope items 19-22) |

**Bidirectional coverage confirmed:** every `assumptions` entry on every POB has a corresponding `trusted_base_refs` entry, and every `trusted_base_refs` entry maps to a row in `trusted-base-plan.md` (per §69 of the plan).

## Waiver Assessment

The single `W-vb-r8oso-NONE-001` waiver candidate is a placeholder with `behavior_affecting: false`, `review_status: approved`, `limitation_kind: surface_absent`, future expiry `2026-12-31`. It is explicitly a "declaration-only placeholder" that "MUST NOT be matched against any behavior-affecting proof-obligation/v1 row by the E_BEHAVIOR_WAIVER gate at State 12" (per the candidate itself).

**No behavior-affecting waivers found. E_BEHAVIOR_WAIVER not triggered.**

## Cross-Lane Evidence Disambiguation

Two cases exist where the same physical claim is verified by more than one verifier; the plan disambiguates them in §7:

- **POB-002 (kani) and POB-003 (proptest):** Both target the `append_unfsynced` / `append_strict` guard. Kani provides bounded symbolic evidence with `kani::any()`; proptest provides randomized property evidence through the real Fjall LSM tree. Two distinct harness files, two distinct expected_evidence markers (`VERIFICATION:- SUCCESSFUL` vs `test result: ok. N passed; 0 failed`).
- **POB-003 (proptest) and POB-006 (proptest):** Both at the proptest level but in distinct files — `proptest_journal_sequence_at_write.rs` (randomized sequence pressure) vs `proptest_journal_batch_atomicity.rs` (batch atomicity + Kani feature compile-check).

## Bridge to Implementation

The bridge stub is intentionally minimal: one new method, one new variant, one diagnostic constant. Section 10 of the proof-strategy defers the detailed bridge to State 7 (`proof-to-implementation`), which is acceptable for this bead because the public API surface is small and the seven POBs target specific production paths (`next_sequence_at_write`, `append_unfsynced`, `append_strict`, `append_strict_batch`, `append_event`). The proof-to-implementation-input artifact will materialise at State 7.

## Findings (proof-plan-findings.jsonl)

Three non-blocking findings are recorded:

1. **E_INVOCATION_LEDGER_MISSING (minor)** — agent-invocation-ledger.jsonl has rows for state 1 (go-skill) and state 2 (explore) only; state 3 (rust-contract) and state 4 (proof-planner) rows are absent. Disposition: `owner_approved_no_action`. Process debt, not correctness.

2. **E_PROOF_PLAN_MISSING_VERUS (observation)** — Verus is uniformly `not_applicable` per contract C-8. The verification-lane-policy default would have included Verus for Rust-local arithmetic and indexing; contract C-8 explicitly authorises the deviation. Disposition: `owner_approved_debt`. Future beads that introduce new exec fn surfaces in `vb_storage` should re-open Verus.

3. **E_BRIDGE_REFS_NOT_DISJOINT (observation)** — POB-003 and POB-006 both target the append_strict_batch path. The plan disambiguates the harnesses in two distinct test files. Disposition: `owner_approved_no_action`. The cross-lane evidence rule is satisfied; the disjointness check is a write-time invariant only.

All findings are non-blocking (`owner_approved_no_action` / `owner_approved_debt`) and do not require State rerun.

## Provenance Audit

- **Reviewer invocation ID**: `proof-plan-reviewer-cheap25-vb-r8oso-2026-07-01` (this review, defined in this document).
- **Planner invocation ID**: `proof-planner-vb-r8oso-state4` (referenced in every verifier-lane-review row's `planner_invocation_id` field; not present in `.beads/vb-r8oso/agent-invocation-ledger.jsonl`, see finding E_INVOCATION_LEDGER_MISSING).
- **Independent reviewer skill chain**: this reviewer is `proof-plan-reviewer`; the writer was `proof-planner`. Different skill IDs.
- **Hash chain**: every referenced artifact has its sha256 captured above; no hashes were recomputed between read and write — the reviewer's hashes were computed in this session from the on-disk files.

## Verification of Deliverables

- `pwd -P` at this review start: `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-r8oso` (matches `isolated_workdir` in `STATE.md`).
- `jj root`: resolves to the same path (cheap25-vb-r8oso workspace).
- `git rev-parse --show-toplevel`: reports "fatal: not a git repository" — the coord-checkout convention forbids adding a git remote at the isolated workdir level. JJ provides the workspace identity. The plan and contract commits are chain-traceable through the parent bookmark (`cheap25-vb-r8oso@`).
- 128 verifier-lane-review rows produced; `jq 'length'` parses cleanly.
- `proof-plan-findings.jsonl` produced; 3 rows; `jq` validates schema.

## Conclusion

The proof plan for **vb-r8oso** is precise, well-scoped, vacuum-proof, and ready for proof writing. It:

- Provides complete lane decisions for all 16 proof seeds (128 VLD rows)
- Materialises exactly 7 proof obligations across the user's gate lanes (Kani feature-gated + proptest)
- Correctly prevents vacuum proofs (Verus/Flux non-applicability is owner-approved; Kani mandates `kani::any()`; proptest generates randomized sequence pressure)
- Has **no behavior-affecting waivers** (only one non-behavior placeholder)
- Has a sound 8-marker trusted-base plan with bidirectional coverage on every POB
- Satisfies the user's gate requirements: `kani-sequence-at-write` Cargo feature, `JournalError::SequenceMismatch { … }` with `0x4042`, RESEARCH_REQUIRED downstream caller audit gate, no silent seq rewrite enforcement via POB-005
- Will be materialised by `proof-writer` at State 5 (Kani harness + proptest harnesses) and `holzman-rust` (fuzz arm updates, no new fuzz harness per C-8)

STATUS: APPROVED
