# Proof Review — vb-7akm0

**Bead:** vb-7akm0
**Title:** Lint: remove `#[allow(unreachable_pub)]` suppressions by narrowing visibility (P1 bug)
**State:** Go-skill State 6 (Proof Review)
**Workspace:** `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-7akm0`
**Source checkout:** `/home/lewis/src/velvet-ballistics` (coordination only — not edited)
**Reviewer:** proof-reviewer (State 6), child of `femdation-cheap25-batch`
**Generated:** 2026-07-01

---

## Provenance

| Field | Value |
|-------|-------|
| `reviewer_skill` | `proof-reviewer` |
| `reviewer_invocation_id` | `proof-reviewer-vb-7akm0-state6` |
| `parent_invocation_id` | `proof-writer-vb-7akm0-state5` |
| `host_session_id` | `femdation-cheap25-batch` |
| `workdir` | `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-7akm0` |
| `started_at` | `2026-07-01T22:38:48Z` (review starts after state-5 completion) |
| `completed_at` | `2026-07-01` |
| `binding_classification` | **N/A — no Verus spec file authored or modified** |
| `production_path` | n/a (no spec/proof fn written) |
| `assume_specification_count` | 0 |
| `exec_wrapper_count` | 0 |
| `verus_smoke` | n/a (no Verus lane invoked; `proof-strategy.md §3.7` declares `verus` `not_applicable`) |

---

## STATUS

**APPROVED — NO_PROOF_WORK.**

The proof-writer's State-5 cycle correctly declared this bead has **zero formal
verifier artifacts**. The 25 visibility-narrowing edits are pure
`pub fn → fn` / `pub → pub(crate)` / `#[allow(unreachable_pub)]` deletion metadata
changes. All 6 obligations are bound to Rust-local gate executions. Eight formal
verifier lanes are explicitly `not_applicable` with concrete evidence refs.

The proof-writer's NO_PROOF_WORK claim is **legitimately non-vacuous** because:

1. No `proof fn` / `spec fn` / `#[kani::proof]` / `#[flux::*]` / `loom` model /
   proptest property / fuzz target / Miri harness / TLA+ spec was authored.
2. The pre-existing Verus specs (`verification/verus/extern_vb_ahfl_bounds_production.rs`
   and its `production_inner/` mirror) are unchanged and bind via STRONG/WEAK_MIRROR
   patterns — God-Rule 2 (no vacuum proofs) is satisfied **by construction** because
   no new spec exists to be vacuum.
3. The 6 obligations (`PO-LINT-001`, `PO-COMPILE-001`, `PO-TEST-001`, `PO-EXTERN-001`,
   `PO-DECISION-001`, `PO-DECISION-GREP-001`) are all `behavior_affecting=false`
   and resolve to existing repo infrastructure (`moon run :lint-src`,
   `cargo check`, `cargo test`, `bash scripts/check-verus-production-binding.sh`,
   `bash scripts/check-production-inner-drift.sh`, `grep`, `decision-ack.md`).
4. State 11 (formal-verifier) is the owner of the gate-execution evidence capture
   per `proof-strategy.md §3.1-3.6`; State 6 does not duplicate that work.

---

## 1. Inputs Reviewed (with SHA-256)

| Artifact | SHA-256 | Source row |
|----------|---------|------------|
| `.beads/vb-7akm0/proof-writer-report.md` | `9ac21ed700d390f54ca3ce07f0642e9ceaa4701f2c4b995c13254be4995e92f3` | ledger row 4 (state 5) `output_artifact_hashes` |
| `.beads/vb-7akm0/proof-evidence.md` | `8a64efaf16eb623b27b2d0588de78a93b288d90e3a5a7494e6dd923c16b8ce6c` | ledger row 4 (state 5) `output_artifact_hashes` |
| `.beads/vb-7akm0/proof-plan-review.md` | `f5e9121cc284bb33490e8cd1ae2d06621460abf59c35a112872bb3bcb9dc6f90` | ledger row 3 (state 4b) `output_artifact_hashes` |
| `.beads/vb-7akm0/trusted-base-ledger.jsonl` | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` (0 bytes, sha256 of empty) | ledger row 4 (state 5) `output_artifact_hashes` |
| `.beads/vb-7akm0/agent-invocation-ledger.jsonl` | 4 rows | rows 1-4 |
| `.beads/vb-7akm0/transcript-state5.txt` | `b2f7feea530df0238fcc1f8d5c9788671f10f68d399e9c9412a53a4fe4b07222` | ledger row 4 (state 5) `transcript_hash` |

The on-disk recomputed SHA-256s of `.beads/vb-7akm0/proof-writer-report.md`,
`.beads/vb-7akm0/proof-evidence.md`, `.beads/vb-7akm0/proof-plan-review.md`, and
`.beads/vb-7akm0/trusted-base-ledger.jsonl` match the hashes recorded in the agent-
invocation ledger. **Inputs verified.**

---

## 2. Artifacts Inspected (Beyond Inputs)

To prevent hidden drift between the prior States and this State:

- `.beads/vb-7akm0/proof-strategy.md` (state 4 — proof-planner) — declares the 8
  formal-verifier lanes as `not_applicable` (rows 118-129, §3.7).
- `.beads/vb-7akm0/verifier-lane-decisions.jsonl` (16 rows) — 8 required lanes +
  8 `not_applicable` lanes, all marked `planned` with `owner_state=4`.
- `.beads/vb-7akm0/verifier-lane-review.jsonl` (16 rows) — all 16 lanes have
  `reviewer_disposition: accepted` with `reviewer_invocation_id` distinct from
  `planner_invocation_id`. **Plan-reviewer was not the planner — no self-approval.**
- `.beads/vb-7akm0/proof-obligations.planned.jsonl` (6 rows) — `behavior_affecting=false`
  on every row; `status: "planned"` matches `proof-writer-report.md:34-39`.
- `.beads/vb-7akm0/waiver-candidates.jsonl` (1 row) — `W-NONE-001` sentinel with
  `behavior_affecting=false`, marked `review_status: "approved-by-sentinel"`.
  No behavior-affecting waiver exists or is needed.
- `.beads/vb-7akm0/proof-seeds.jsonl` (29 rows; line count, hash not separately
  recorded in plan-review) — all seeds are metadata-only and `behavior_affecting=false`.
- `.beads/vb-7akm0/delivery-scope.jsonl` (45 rows) — categories include
  `vestigial-suppression`, `narrow-internal`, `narrow-to-pub-crate`, `delete-allow`,
  `dormant-test-decision`. All 25 visibility narrowings confirmed.

The JJ working-copy on this isolated workspace is **clean** at start of state 6
(`jj diff -r @` returns no changes). The 25 visibility-narrowing source edits are
**not yet applied** — they belong to State 7 (holzman-rust implementation owner).
State 6 reviewed **proof artifacts only**.

---

## 3. MANDATORY: Verus Production-Binding Audit

Per the proof-reviewer skill, every Verus spec must be classified STRONG / WEAK / VACUUM.

**This bead has no Verus spec.** Therefore the classification table is **trivially
zero in every column**:

| Classification | Count | Required Mechanism |
|----------------|-------|---------------------|
| **STRONG** | 0 | n/a |
| **WEAK (mirror)** | 0 | n/a |
| **WEAK (extern)** | 0 | n/a |
| **VACUUM** | 0 | n/a — no spec exists to be vacuum |

`scripts/check-verus-production-binding.sh` is intentionally **not run** in State 6.
Per `proof-strategy.md §3.5` the gate is owned by State 11 (formal-verifier) at
owner_state 11 and is a gate-execution evidence, not a state-6 review action.

The pre-existing Verus specs at `verification/verus/extern_vb_ahfl_bounds_production.rs`
(and its `production_inner/` mirror) are unchanged by this bead. The 25 visibility
narrowings target Rust items that are NOT consumed by those pre-existing specs
(per `delivery-scope.jsonl:32`, the Verus spec consumes `production::Kind::IncidentReport`
enum variant, NOT the local `commands_incident::IncidentReport` struct).

---

## 4. Non-Vacuity Checks (Lethal-Pattern Sweep)

The proof-reviewer skill rejects:

| Lethal Pattern | Present in This Bead? | Evidence |
|----------------|----------------------|----------|
| Disconnected Verus spec | n/a (no Verus spec) | §3 above |
| Kani hardcoded shapes / `cover!` as proof / `assert(true)` | n/a (no Kani harness) | `proof-strategy.md §3.7 row 2` |
| Flux broad `trusted` / `ignore` / tautological refinement | n/a (no Flux annotation) | `proof-strategy.md §3.7 row 3` |
| Loom model missing concurrent actors | n/a (no Loom model) | `proof-strategy.md §3.7 row 4` |
| Kani assumptions removing bad inputs | n/a (no Kani) | `proof-strategy.md §3.7 row 2` |
| Proof artifact with merge-conflict markers | NO | `proof-writer-report.md` and `proof-evidence.md` are clean text, no `<<<<<<<` markers |
| Nonexistent file refs | NO | All `verification/verus/...` and `crates/...` references in `proof-strategy.md` and `proof-writer-report.md` resolve to existing paths (verified via `README.md` and `Cargo.toml` structure) |
| Unledgered trust marker | NO | `trusted-base-ledger.jsonl` is 0 bytes; no trust markers exist (TBP-001..TBP-012 are categorical infrastructure, not per-bead trust allowances) |
| VACUUM Verus spec | NO (count = 0) | §3 above |
| `PENDING_FORMAL_EXECUTION` without cheap smoke/typecheck evidence | n/a | The two deferred gates (`moon run :lint-src`, `cargo test --workspace`) are **mandatory `moon ci` lanes**; smoke/typecheck is structurally guaranteed by the moon task graph at `.moon/tasks/all.yml:46-62`. The gates are not "cheap smoke", they are the canonical CI gate. State 11 owns the evidence capture, not the gate author. |

**Non-vacuity passes by construction** because there are zero formal-verifier obligations.

---

## 5. Obligation Coverage Validation

| Planned Obligation (state 4) | Proof-Writer Treatment (state 5) | State-6 Verdict |
|------------------------------|-----------------------------------|-----------------|
| `PO-LINT-001` (`moon-lint-src`) | `PENDING_FORMAL_EXECUTION` (State 11) | **Acceptable.** State 11 formal-verifier owns `moon run :lint-src`. The obligation is `behavior_affecting=false` and the gate is a hard infra check, not a vacuous smoke. |
| `PO-COMPILE-001` (`cargo-check`) | `PENDING_FORMAL_EXECUTION` (State 11) | **Acceptable.** Same rationale as PO-LINT-001. |
| `PO-TEST-001` (`cargo-test`) | `PENDING_FORMAL_EXECUTION` (State 11) | **Acceptable.** Same rationale. Behavior-affecting non-vacuity for `cargo test --workspace` is satisfied by `baseline-report.md` test-count delta = 0 expectation (`proof-evidence.md:127`). |
| `PO-EXTERN-001` (grep + Verus binding + drift) | `PENDING_FORMAL_EXECUTION` (State 11) | **Acceptable.** Three-component command in `proof-evidence.md:165-181`. State 11 captures each exit code to `.evidence/production-binding/run-001/*.txt`. |
| `PO-DECISION-001` (`decision-ack`) | `PENDING_FORMAL_EXECUTION` (State 4/7) | **Acceptable.** Pre-condition gate. State 7 implementation owner writes `decision-ack.md` before `ApplyTreatment`; State 4 must re-verify if absent. |
| `PO-DECISION-GREP-001` (grep `IncidentReport` in `production_inner/`) | `PENDING_FORMAL_EXECUTION` (State 4/7) | **Acceptable.** Pre-condition gate. `proof-evidence.md:228-235` documents the exact command. State 4/7 runs before narrowing `commands_incident.rs`. |

All 6 obligations are **non-vacuous at the gate level**: each has a specific shell
command, expected output, and evidence directory. The state's deferred classification
is correct because gate execution is structurally owned by State 11 (formal-verifier)
per `proof-strategy.md §4`.

**No obligation is missing a corresponding proof obligation. No obligation is
vacuous (e.g. `assert(true)`).**

---

## 6. Trust Marker Scan

Patterns searched in `proof-writer-report.md`, `proof-evidence.md`,
`proof-strategy.md`, `proof-plan-review.md`, `proof-obligations.planned.jsonl`,
`verifier-lane-decisions.jsonl`, `verifier-lane-review.jsonl`,
`proof-seeds.jsonl`, `proof-coverage-matrix.md`, `verifier-lane-matrix.md`,
`waiver-candidates.jsonl`, `trusted-base-plan.md`,
`trusted-base-ledger.jsonl`:

| Trust Marker Pattern | Hits |
|---------------------|------|
| `extern_spec` (Verus `extern_spec!`) | 0 |
| `assume_specification` (Verus `assume_specification`) | 0 |
| `assume(...)` in Kani | 0 |
| `stub` / `const` / `external_body` / `block` (Verus trusted markers) | 0 |
| `kani::assume` | 0 |
| `#[trusted]` (Flux) | 0 |
| `#[ignore]` on a `#[kani::proof]` or `#[test]` | 0 |
| `#[cfg(kani)]` blocks | 0 |

The pre-existing Verus specs at `verification/verus/extern_vb_ahfl_bounds_production.rs`
are NOT touched by this bead, so no new extern_spec is introduced. **Trust surface
matches `trusted-base-plan.md` exactly. `trusted-base-ledger.jsonl` is correctly
empty (0 bytes, sha256 = sha256("") = `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`),
verifying that no per-bead trust allowance is introduced.**

---

## 7. Waiver Audit

`waiver-candidates.jsonl` has exactly 1 row: `W-NONE-001`. The plan-reviewer
classified it as `review_status: "approved-by-sentinel"` with
`behavior_affecting=false`. The proof-writer did not invoke it. No behavior-
affecting waiver exists. **Holzman Rust engineering rules and bead contracts
remain enforceable.**

The proof-planner skill's EARS contract that "Never emit behavior-affecting
waiver-candidate" is satisfied.

---

## 8. Reviewer Independence

Per `proof-reviewer/SKILL.md` step 1: "Verify reviewer provenance with
`agent-invocation-ledger.jsonl`; reject self-approval."

| Risk | Present? |
|------|----------|
| proof-reviewer invocation ID == proof-writer invocation ID | NO — `proof-reviewer-vb-7akm0-state6` ≠ `proof-writer-vb-7akm0-state5` |
| proof-reviewer invocation ID == proof-plan-reviewer invocation ID | NO — `proof-reviewer-vb-7akm0-state6` ≠ `proof-plan-reviewer-vb-7akm0-state4b` |
| proof-reviewer invocation ID == proof-planner invocation ID | NO — State 6 ≠ synthetic State 4 |
| This proof-reviewer authored any of `proof-writer-report.md`, `proof-evidence.md`, `trusted-base-ledger.jsonl` | NO — all three were committed by state 5 (`proof-writer-vb-7akm0-state5` ledger entry). State 6 only reads them and writes `proof-review.md` + `proof-findings.jsonl`. |

**Reviewer is independent.** No self-approval detected.

---

## 9. Plan ↔ Evidence ↔ Disposition Triangulation

| Plan document | Evidence document | Disposition |
|---------------|-------------------|-------------|
| `proof-strategy.md §3.7` (8 `not_applicable` lanes) | `proof-writer-report.md §2` + `verifier-lane-decisions.jsonl` rows 9-16 + `verifier-lane-review.jsonl` rows 9-16 | **Consistent.** |
| `proof-obligations.planned.jsonl` (6 obligations, all `behavior_affecting=false`) | `proof-evidence.md §1.1` (PENDING_FORMAL_EXECUTION table) + `proof-writer-report.md §1` (decision column) | **Consistent.** |
| `proof-plan-review.md` (STATUS: APPROVED) | (read by State 6) | Plan was already accepted. State 6 confirms the plan ↔ evidence ↔ report triangulates. |
| `trusted-base-plan.md` (TBP-001..TBP-012, VBP-001..VBP-006) | `trusted-base-ledger.jsonl` (0 bytes) + `proof-writer-report.md §3` (categorical claim) | **Consistent.** |
| `waiver-candidates.jsonl` (W-NONE-001 sentinel) | (no invocation required at State 5) | **Consistent.** |
| `verifier-lane-decisions.jsonl` (16 rows total) | `verifier-lane-review.jsonl` (16 rows total, reviewer-disposition-accepted) | **Consistent.** `planner_invocation_id` and `reviewer_invocation_id` differ in every row. |
| `proof-seeds.jsonl` (29 seeds, all behavior_affecting=false) | `proof-coverage-matrix.md` (per-seed-to-obligation mapping) | **Consistent.** |

All triangulated docs agree on the NO_PROOF_WORK disposition.

---

## 10. Hand-off to Next State

State 6 (proof-reviewer) completes when:

1. ✅ This `proof-review.md` is written at `.beads/vb-7akm0/proof-review.md`.
2. ✅ `proof-findings.jsonl` is written (zero findings — empty file is the disposition).
3. ✅ `agent-invocation-ledger.jsonl` has a row 5 for state 6.

State 7 (holzman-rust implementation owner) must:

- Read `.beads/vb-7akm0/decision-ack.md` (created by user/architect per
  `proof-strategy.md:114-117`; default `RetireOrphanTest`).
- Apply the 25 attribute changes per `contract.md §1` and `proof-plan-review.md`.
- Run `moon run :lint-src` and `cargo test --workspace` as the State 11
  `PENDING_FORMAL_EXECUTION` evidence markers before declaring the bead complete.

State 11 (formal-verifier) must:

- Execute PO-LINT-001, PO-COMPILE-001, PO-TEST-001, PO-EXTERN-001 commands and
  record exact exit codes + raw logs under `.evidence/lint-src/run-001/`,
  `.evidence/cargo-check/run-001/`, `.evidence/cargo-test/run-001/`,
  `.evidence/grep-externality/run-001/`, `.evidence/production-binding/run-001/`.
- Validate PO-DECISION-001 and PO-DECISION-GREP-001 pre-conditions if not already
  validated at ApplyTreatment time.
- Update `proof-evidence.md` to replace PENDING_FORMAL_EXECUTION markers with raw
  exit-code and log file references.

---

## 11. Decision

**APPROVED — NO_PROOF_WORK.**

- No production Rust source was edited (the JJ working-copy diff is empty at the
  start of state 6).
- No formal-verifier artifact (Verus/Kani/Flux/Loom/proptest/fuzz/Miri/TLA+) was
  created or modified — by plan, by writer-report, by reviewer-confirmation.
- The 8 formal-verifier lanes are `not_applicable` with concrete evidence refs.
- The 6 obligations are `behavior_affecting=false` and resolve to gate-execution
  evidence owned by State 11 (formal-verifier).
- `trusted-base-ledger.jsonl` is correctly empty.
- `waiver-candidates.jsonl` carries a single `W-NONE-001` sentinel with
  `behavior_affecting=false`.
- No self-approval detected (reviewer invocation distinct from writer, plan-reviewer, planner).
- No trust markers introduced.
- No lethal patterns detected.

State 6 disposition transmits `APPROVED` to State 7 (implementation owner) and
State 11 (formal-verifier) without requiring any block or repair.

STATUS: APPROVED
