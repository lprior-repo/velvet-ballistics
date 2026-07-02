# Proof Plan Review — vb-7akm0

**Bead:** vb-7akm0
**Title:** Lint: remove `#[allow(unreachable_pub)]` suppressions by narrowing visibility (P1 bug)
**Workspace:** `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-7akm0`

## Provenance

- `reviewer_skill`: `proof-plan-reviewer`
- `reviewer_invocation_id`: `proof-plan-reviewer-vb-7akm0-state4b`
- `planner_invocation_id`: `proof-planner-vb-7akm0-state4` (synthetic — planner did not record a ledger row)
- `review_state`: `state4b`
- `host_session_id`: `femdation-cheap25-batch`
- `started_at`: `2026-07-01T17:00:00Z`
- `completed_at`: `2026-07-01T17:00:30Z`

## Reviewed Artifacts (with SHA-256)

| Artifact | SHA-256 |
|----------|---------|
| `.beads/vb-7akm0/proof-strategy.md` | `10241596c50bcc9b3844dd68b0e4407ac2137ee4e548713641d02f36b52e279d` |
| `.beads/vb-7akm0/verifier-lane-decisions.jsonl` | `892c5dcb06b2c20aed46bbf049af2447d548cba1dec5e3c6b6406210a41d949a` |
| `.beads/vb-7akm0/proof-obligations.planned.jsonl` | `a5a03321ca16dca48e0e4d72a763fc5ec331b3e3e94071e849f1f1e3a0787334` |
| `.beads/vb-7akm0/trusted-base-plan.md` | `2530fc7c74259d9a4ffc79b852d15df1e4c64d20a906e184a002a83ecac631fe` |
| `.beads/vb-7akm0/waiver-candidates.jsonl` | `7ffafde4299832d2b9612ed69f04505e8b3a81eb50a1fa288c5920e2c45f774f` |
| `.beads/vb-7akm0/verifier-lane-matrix.md` | `e9d961644456c855877e1bfa2b2e1f60527093f0dd4ecd6fe51079cefb8e8c72` |
| `.beads/vb-7akm0/proof-coverage-matrix.md` | `33959fb73667f13b2f413caada17418a15df2a62ffa38e74d812f0ca41418697` |
| `.beads/vb-7akm0/proof-seeds.jsonl` | `d8b4daee69ab12c2eaa12491ece51e92d7462441c0e640a4d563fe3f5c687b86` |

## Reviewer Disposition Summary

| # | Verifier | Applicability | Reviewer Disposition | Lane-Decision Row |
|---|----------|---------------|----------------------|-------------------|
| 1 | `moon-lint-src` | required | accepted | `LD-vb-7akm0-001-moon-lint-src` |
| 2 | `cargo-check` | required | accepted | `LD-vb-7akm0-002-cargo-check` |
| 3 | `cargo-test` | required | accepted | `LD-vb-7akm0-003-cargo-test` |
| 4 | `grep-externality` | required | accepted | `LD-vb-7akm0-004-grep-externality` |
| 5 | `check-verus-production-binding` | required | accepted | `LD-vb-7akm0-005-check-verus-production-binding` |
| 6 | `check-production-inner-drift` | required | accepted | `LD-vb-7akm0-006-check-production-inner-drift` |
| 7 | `decision-ack` | required | accepted | `LD-vb-7akm0-007-decision-ack` |
| 8 | `grep` | required | accepted | `LD-vb-7akm0-008-decision-grep` |
| 9 | `verus` | not_applicable | accepted | `LD-vb-7akm0-009-verus` |
| 10 | `kani` | not_applicable | accepted | `LD-vb-7akm0-010-kani` |
| 11 | `flux-rs` | not_applicable | accepted | `LD-vb-7akm0-011-flux_rs` |
| 12 | `loom` | not_applicable | accepted | `LD-vb-7akm0-012-loom` |
| 13 | `proptest` | not_applicable | accepted | `LD-vb-7akm0-013-proptest` |
| 14 | `cargo-fuzz` | not_applicable | accepted | `LD-vb-7akm0-014-cargo-fuzz` |
| 15 | `miri` | not_applicable | accepted | `LD-vb-7akm0-015-miri` |
| 16 | `tla-plus` | not_applicable | accepted | `LD-vb-7akm0-016-tla-plus` |

16 verifier-lane-review rows total (8 required + 8 not_applicable).

## Plan Structure

- **6 proof obligations** (within 4-6 budget):
  - `PO-LINT-001` (`moon-lint-src`): zero `#[allow(unreachable_pub)]` surviving.
  - `PO-COMPILE-001` (`cargo-check`): all narrowed items compile.
  - `PO-TEST-001` (`cargo-test`): same test count as pre-change baseline.
  - `PO-EXTERN-001` (`grep-externality` + Verus binding gates): externally-reachable items remain `pub`.
  - `PO-DECISION-001` (`decision-ack`): pre-condition gate — orphan-test decision recorded before `ApplyTreatment`.
  - `PO-DECISION-GREP-001` (`grep`): pre-condition gate — `grep IncidentReport verification/verus/production_inner/` returns no results before narrowing.
- **Rust-local lanes only** — no Verus, no Kani, no Flux, no Loom, no TLA+. Eight formal/concurrency verifiers explicitly marked `not_applicable` with concrete evidence refs.
- **Pre-conditions clearly gated:** `PO-DECISION-001` requires `.beads/vb-7akm0/decision-ack.md` to exist with a valid `Decision:` line BEFORE `ApplyTreatment` runs on categories G.1/G.2. `PO-DECISION-GREP-001` requires the grep to return empty BEFORE category G.2 narrowing. Both are in `mode=pre-condition`, `owner_state=4`, `rerun_from=4`.
- **Trusted base plan** enumerates 12 trusted items (workspace lint policy, moon task, Rust visibility rules, Verus binding gates) and 6 verified items. Boundary is explicit.
- **Waiver candidates:** single sentinel `W-NONE-001` (no behavior-affecting waivers). All 30 proof seeds and 25 delivery-scope rows are `behavior_affecting=false`.

## Findings

Three low-severity observations, all `owner_approved_no_action`. See `.beads/vb-7akm0/proof-plan-findings.jsonl` for full text. None are blockers.

- **F-vb-7akm0-plan-001 (`E_SCHEMA_MISSING_FIELD`, low):** `proof-obligations.planned.jsonl` rows lack schema_version / target / domain_claim / risk_tags / workdir / model_bounds / tool_metadata / trusted_base_refs. Will be filled in at materialisation step.
- **F-vb-7akm0-plan-002 (`E_INVOCATION_LEDGER_MISSING`, low):** host-side gap — proof-planner invocation not in ledger. Review uses synthetic planner invocation ID; reviewer and writer invocation IDs differ.
- **F-vb-7akm0-plan-003 (`E_PROOF_PLAN_MISSING_NONVACUITY`, low):** non-vacuity applies only to formal-verifier obligations. This bead has zero; satisfied by construction.

## Decision

The plan is precise enough for proof-writer and proof-to-implementation. All required gates are present. Pre-conditions are clearly gated. Behaviour-affecting flag is consistently false. No waivers needed. No formal-verifier obligations introduced.

**Approved.**

STATUS: APPROVED
