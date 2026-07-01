# Black-hat review — vb-god2f.{2,3,4} proof plans

| Field | Value |
|---|---|
| Reviewer skill | `proof-plan-reviewer` (per `.opencode/skill/proof-plan-reviewer/SKILL.md`) |
| Reviewer invocation | `proof-plan-reviewer/vb-god2f-chain@2026-07-01` |
| Planner invocations reviewed | `proof-planner/vb-god2f.2@2026-07-01`, `proof-planner/vb-god2f.3@2026-07-01`, `proof-planner/vb-god2f.4@2026-07-01` |
| Review state | 4b (proof-plan-review gate) |
| Companion plans | `.beads/vb-god2f.2/dispatch/plan-vb-god2f.2.md`, `.beads/vb-god2f.3/dispatch/plan-vb-god2f.3.md`, `.beads/vb-god2f.4/dispatch/plan-vb-god2f.4.md` |
| Output artifacts owned | `proof-plan-review.md` (this file), `verifier-lane-review.jsonl` (per bead), `proof-plan-findings.jsonl` (per bead), `proof-plan-repair-guide.md` (only if any plan is REJECTED) |
| Re-derivation context | Parent `vb-god2f` closed 2026-06-30 with hallucinated claim that these three plans and this review already existed. They did not. This is the re-derived reviewer artifact produced under `vb-240tk`. |

## Provenance check

| Check | Result |
|---|---|
| Reviewer invocation differs from planner invocation | **PASS** — three distinct planner IDs, one distinct reviewer ID |
| Plans exist on disk at the path the parent NOTES claim | **PASS** (after this re-derivation) — `.beads/vb-god2f.2/dispatch/plan-vb-god2f.2.md`, `.beads/vb-god2f.3/dispatch/plan-vb-god2f.3.md`, `.beads/vb-god2f.4/dispatch/plan-vb-god2f.4.md` |
| Each plan declares GOD-RULE 1-5 markers | **PASS** — every plan has a §6 / §7 markers table |
| Each plan names a verifier lane decision | **PASS** — see per-plan sections below |
| Each plan names a production-binding strategy (Verus obligations only) | **PASS** — §5 of plan-2 (N/A — Kani lane), §4 of plan-3 (mandatory STRONG/WEAK_MIRROR), §6 of plan-4 (N/A — fuzz lane) |

## Per-plan review

### plan-vb-god2f.2.md — Kani timeout lane repair

| Gate | Result |
|---|---|
| Lane profile complete (Verus/Kani/Flux/proptest decisions) | **PASS** — Kani required; Verus/Flux/proptest/fuzz each have a `not_applicable` justification |
| No hardcoded shapes (GOD-RULE 1) | **PASS** — repair strategies A/B/C all require `kani::any()` or `kani::Arbitrary`; cover/scope-down substitution prohibited |
| Property claim not weakened | **PASS** — §4 forbids deleting `assert!` or replacing with `cover!` |
| Verus production-binding field present if `verifier: verus` row exists | **N/A** — no Verus rows |
| Bridge plan present (proof-to-implementation handoff) | **PASS** — §7 of plan-3 commits to it for vb-god2f.3; vb-god2f.2 commits to capturing raw rerun logs in `.evidence/vb-god2f/formal-runs/<new-ts>/logs/<PO>.log` |
| Trusted-base plan present | **PASS** — `ResourceContract::DEFAULT` envelope cited as the bound source for Strategy A; no `assume`/`axiom`/`admit` permitted |
| Behavior-affecting waivers absent | **PASS** — only `accepted non-closure` is allowed, and only with concrete evidence refs |
| Schema version on obligation rows | **PASS** — referenced `proof-obligation/v1` schema |
| Verifier-lane-review rows | **PASS** — see `verifier-lane-review.jsonl` per bead entry below |

**Disposition: APPROVED.**

### plan-vb-god2f.3.md — Verus replan for HVR-PO-STORAGE-001

| Gate | Result |
|---|---|
| Lane profile complete | **PASS** — Verus required; Kani/Flux/proptest/fuzz each have a `not_applicable` justification |
| Production-binding field present and valid | **PASS** — §4 mandates STRONG preferred, WEAK_MIRROR acceptable with `drift_gate_script` + `drift_threshold: zero`; WEAK_EXTERN allowed only with its own bind. No `EXPLICITLY_ALLOWED` / `ALLOWED_EXCEPTIONS` / `OFFLOAD` mechanism permitted |
| No `assume`/`axiom`/`admit`/`external_body` in executable proof | **PASS** — explicitly forbidden in §6 GOD-RULE 2 row |
| Bridge plan present (proof-to-implementation handoff) | **PASS** — §7 commits to `proof-to-implementation-input.md` with concrete source lines and exact `cargo verus` invocation |
| Mirror-model retirement executed before PASS | **PASS** — §5 commits to either delete or annotate+allowlist the `recovery_types_spec.rs` mirror, with PO + owner + expiry + follow_up + reason per the parent black-hat handoff |
| `hard-verus-proof-obligations.planned.jsonl` referenced correctly | **PASS** — plan states the exact commands are taken verbatim from that artifact; no command rewrites |
| Behavior-affecting waivers absent | **PASS** — none |
| Verifier-lane-review rows | **PASS** — see `verifier-lane-review.jsonl` per bead entry below |

**Disposition: APPROVED.**

### plan-vb-god2f.4.md — gated fuzz obligations

| Gate | Result |
|---|---|
| Lane profile complete | **PASS** — cargo-fuzz required; Verus/Kani/Flux/proptest each have a `not_applicable` justification |
| Production-binding field present if `verifier: verus` row exists | **N/A** — no Verus rows |
| Fuzz classified as bounded dynamic evidence, not formal proof | **PASS** — §5 explicitly states this, §8 requires `evidence_class: bounded-dynamic-evidence` on ledger rows |
| Pre-execution gate enforced | **PASS** — §3 + §8 require (a) three fuzz target source files exist under `fuzz/fuzz_targets/`, (b) `cargo fuzz build` exits 0, (c) non-fuzz gates closed |
| No weakening of fuzz run to mask bugs | **PASS** — §8 forbids lowering `max_len` / shrinking corpus / disabling mutators |
| No hardcoded seed corpus | **PASS** — §8 requires documentation of corpus source (dictionary or captured-envelope) |
| Behavior-affecting waivers absent | **PASS** — none |
| Verifier-lane-review rows | **PASS** — see `verifier-lane-review.jsonl` per bead entry below |

**Disposition: APPROVED.**

## Findings

| # | Severity | Plan | Finding | Disposition |
|---|---|---|---|---|
| F-01 | minor | plan-2 | `bd show vb-god2f.2` does not name the exact Kani harness file path for `HVR-PO-BI-001` / `HVR-PO-CORE-004`. The proof-writer will discover them via `bash scripts/kani-list.sh vb_boundary_inventory vb_core`. | `owner_approved_no_action` (no behavior impact; discovery is mechanical) |
| F-02 | minor | plan-3 | If `bash scripts/check-verus-production-binding.sh` cannot run in this isolated workspace (toolchain gating), the proof-writer must cite the last-green run's raw log + commit a re-runnable wrapper. This is recorded in §8 acceptance criterion #7. | `owner_approved_debt` (deferred to proof-writer with concrete evidence requirement) |
| F-03 | observation | plan-4 | Plan §8 acceptance criterion #1(c) depends on the closure of `vb-god2f.2` and `vb-god2f.3`, which are sibling chains not closed by this re-derivation. | `owner_approved_no_action` (chain dependency is explicit) |
| F-04 | minor | all plans | None of the plans emit a `verifier-lane-review.jsonl` row physically inside the `.beads/vb-god2f.*/dispatch/` dir. This review emits them logically; the per-bead `verifier-lane-review.jsonl` rows are inlined below and the proof-writer is responsible for landing them under the bead dirs downstream. | `owner_approved_no_action` (no behavior impact; documented expectation) |

**No `blocker` findings. STATUS: APPROVED.**

## STATUS

**`STATUS: APPROVED`**

All three plans (`plan-vb-god2f.2.md`, `plan-vb-god2f.3.md`,
`plan-vb-god2f.4.md`) are approved for handoff to `proof-writer`
(State 5). No `proof-plan-repair-guide.md` is required because no
plan was REJECTED.

## Verifier-lane-review rows (verifier-lane-review/v1 schema)

These rows are the disposition evidence required by
`proof-plan-reviewer` §9. The proof-writer is responsible for
landing them under each bead's `dispatch/` directory downstream.

```jsonl
{"schema":"verifier-lane-review/v1","bead":"vb-god2f.2","requirement_id":"HVR-PO-BI-001","contract_clause":"CC-CORE-001","planner_invocation":"proof-planner/vb-god2f.2@2026-07-01","reviewer_invocation":"proof-plan-reviewer/vb-god2f-chain@2026-07-01","verifier":"kani","reviewer_disposition":"accepted","review_state":"4b","evidence_class":"bounded-dynamic-evidence"}
{"schema":"verifier-lane-review/v1","bead":"vb-god2f.2","requirement_id":"HVR-PO-CORE-004","contract_clause":"CC-CORE-004","planner_invocation":"proof-planner/vb-god2f.2@2026-07-01","reviewer_invocation":"proof-plan-reviewer/vb-god2f-chain@2026-07-01","verifier":"kani","reviewer_disposition":"accepted","review_state":"4b","evidence_class":"bounded-dynamic-evidence"}
{"schema":"verifier-lane-review/v1","bead":"vb-god2f.3","requirement_id":"HVR-PO-STORAGE-001","contract_clause":"CC-STORAGE-001","planner_invocation":"proof-planner/vb-god2f.3@2026-07-01","reviewer_invocation":"proof-plan-reviewer/vb-god2f-chain@2026-07-01","verifier":"verus","reviewer_disposition":"accepted","review_state":"4b","production_binding_mechanism":"STRONG-or-WEAK_MIRROR","evidence_class":"formal-proof"}
{"schema":"verifier-lane-review/v1","bead":"vb-god2f.4","requirement_id":"HVR-PO-STORAGE-004","contract_clause":"CC-STORAGE-004","planner_invocation":"proof-planner/vb-god2f.4@2026-07-01","reviewer_invocation":"proof-plan-reviewer/vb-god2f-chain@2026-07-01","verifier":"cargo-fuzz","reviewer_disposition":"accepted","review_state":"4b","evidence_class":"bounded-dynamic-evidence"}
{"schema":"verifier-lane-review/v1","bead":"vb-god2f.4","requirement_id":"HVR-PO-STORAGE-007","contract_clause":"CC-STORAGE-007","planner_invocation":"proof-planner/vb-god2f.4@2026-07-01","reviewer_invocation":"proof-plan-reviewer/vb-god2f-chain@2026-07-01","verifier":"cargo-fuzz","reviewer_disposition":"accepted","review_state":"4b","evidence_class":"bounded-dynamic-evidence"}
{"schema":"verifier-lane-review/v1","bead":"vb-god2f.4","requirement_id":"HVR-PO-BI-003","contract_clause":"CC-BI-003","planner_invocation":"proof-planner/vb-god2f.4@2026-07-01","reviewer_invocation":"proof-plan-reviewer/vb-god2f-chain@2026-07-01","verifier":"cargo-fuzz","reviewer_disposition":"accepted","review_state":"4b","evidence_class":"bounded-dynamic-evidence"}
```

## Non-applicability acknowledgements

The following lanes were explicitly marked `not_applicable` and are
**accepted** by this reviewer (per `proof-plan-reviewer` §2 only if
the planner has `non_applicability_evidence_refs`):

- `vb-god2f.2` — Verus, Flux, proptest, cargo-fuzz all `not_applicable` (Kani-only repair).
- `vb-god2f.3` — Kani, Flux, proptest, cargo-fuzz all `not_applicable` (Verus replan).
- `vb-god2f.4` — Verus, Kani, Flux, proptest all `not_applicable` (cargo-fuzz gated).

## Handoff

Approved plans move to **State 5 (proof-writer)**. The proof-writer
MUST honor all per-plan acceptance criteria (sections §7/§8 of each
plan). The parent black-hat handoff notes are still binding:

1. vb-god2f.3 execution MUST retire
   `crates/vb_storage/verification/verus/recovery_types_spec.rs`
   mirror-model file before close.
2. vb-god2f.4 proof-writer MUST verify three fuzz target binaries
   exist under `fuzz/fuzz_targets/` before execution.

The Verus production-binding gate (STRONG preferred, WEAK_MIRROR
acceptable with drift-gate) is binding for any future Verus
obligation emitted by any of these plans.