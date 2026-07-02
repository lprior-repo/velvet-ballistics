---
bead_id: vb-7akm0
bead_title: "Lint: remove #[allow(unreachable_pub)] suppressions by narrowing visibility (P1 bug)"
phase: 16
state: cleanup
skill: landing-skill (cleanup phase)
generated_at: 2026-07-02T05:22:00Z
ledger_seq: 13
---

# Cleanup Report — vb-7akm0

## STATUS: CLEAN

Post-landing cleanup for `vb-7akm0` is complete. No stray artifacts, no
abandoned work commit, deferred scope routed to backlog, and pre-existing
defects attributed to their owning beads.

## 1. Deferred-Scope Backlog

These items are documented deviations/residual risks from
`implementation.md`. They are **out of scope** for this lint-cleanup bead
and carried forward as follow-up work:

| Item | Reason deferred | Source |
|------|-----------------|--------|
| xtask inner-module `unreachable_pub` cleanup (~173 items) | Removing `xtask/src/main.rs` crate-root suppression cascades ~173 pre-existing errors; suppression RESTORED with NOTE | implementation.md Deviation 1 |
| `diag_codes.rs` 60+ `CODE_*` `pub const` → `pub(crate)` | Larger refactor; zero external consumers confirmed | implementation.md Residual risk 2 |
| `diag_convert.rs` suppression | Only item `pub(super) fn all_variants` not subject to lint; left in place per bead scope | implementation.md §Evidence |

Net result recorded in State 11: 22 of 25 originally-listed
`#[allow(unreachable_pub)]` attributes removed; 3 remaining are
documented deviations with explicit rationale.

## 2. Pre-existing Global Defects (separate-bead owners)

| Obligation | Defect | Owner |
|------------|--------|-------|
| PO-TEST-001 | `proptest_admission_with_budget_has_runtime_capacity_rejection_surface` (`crates/vb_core/tests/aggregate_resource_budget_properties_red.rs:73`) fails on parent commit | vb_core/vb_runtime admission resource-string repair bead |
| PO-EXTERN-001 | 12 `production_inner` drifts in `verification/verus/production_inner/*.rs` | production_inner mirror-refresh bead |

vb-7akm0 touches zero `verification/verus/` files and did not introduce
either defect.

## 3. Orphan-Test Retirement

- `crates/workspace_tests/tests/vb_test_cli_diff_incident_behavior.rs` — DELETED (646 lines).
- `.config/source-length-exceptions.txt:221` (`vb-jpq7.47|split-or-retire-before-release`) — row REMOVED.
- Disposition: RetireOrphanTest (default), per `decision-ack.md`.
- Rationale: no `[[test]]` registration in `crates/workspace_tests/Cargo.toml`; retirement prevents `scripts/check-source-length.sh` failing on a phantom row.

## 4. Workspace Cleanup

- jj workspace `cheap25-vb-7akm0` root: `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-7akm0`.
- Work commit `qvlkvsyy d4476627` **retained** on bookmark `cheap25-vb-7akm0` for batch integration — NOT abandoned.
- No temporary/backup/patch files introduced by states 15/16.
- `.beads/vb-7akm0/` artifact set complete; `agent-invocation-ledger.jsonl` extended through state 16 with intact hash chain.

## 5. Bead-DB Final State

- `vb-7akm0` status: **CLOSED** (Updated 2026-07-02).
- Dolt push to `origin` (`doltremoteapi.dolthub.com/priorlewis43/velvet-ballistics`): complete.

## Gate

- deferred scope documented for backlog: **PASS**
- pre-existing defects attributed to separate beads: **PASS**
- orphan test + source-length row retired: **PASS**
- work commit preserved for integration: **PASS**
- STATE.md advanced to current_state 16: **PASS**
- ledger chain intact through state 16: **PASS**

**STATUS: CLEAN**
