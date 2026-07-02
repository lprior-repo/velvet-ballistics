---
bead_id: vb-7akm0
bead_title: "Lint: remove #[allow(unreachable_pub)] suppressions by narrowing visibility (P1 bug)"
phase: 15
state: landing
skill: landing-skill
generated_at: 2026-07-02T05:19:00Z
ledger_seq: 12
---

# Landing Report — vb-7akm0

## STATUS: LANDED

The bead `vb-7akm0` is **CLOSED** and synced to the Dolt remote. All
landing preconditions from `final-evidence-decision.md` (STATUS:
APPROVED) were satisfied before closure.

## Landing Summary

| Item | Value |
|------|-------|
| Bead | vb-7akm0 |
| Prior state | 14 (evidence-packaging + truth-serum, APPROVED) |
| Work commit | qvlkvsyy `d4476627ff28` |
| Bookmark | cheap25-vb-7akm0 (isolate workspace) |
| Diff | 25 files changed, 85 insertions(+), 755 deletions(-) |
| Close result | ✓ CLOSED (exit 0) |
| Dolt push | Push complete (exit 0) |

## Precondition Gate (from State 14)

- `final-evidence-decision.md` — **STATUS: APPROVED** (sha256 `34f7f87f649b381dc21ab6fff63362443d4b61f4921fcfb5f94507b222186860`)
- `assurance-bundle.md` present (sha256 `63e7ca28fbcfac288e763e1cd59fc546574853f175e0152e39022900f6468ec6`)
- `truth-serum-report.md` present (sha256 `17282c2e2fa9729f68f83f59b45da90d1e4d2ab6afd3a9df419642f5d1dfe8c1`)
- `moon run :lint-src` exit 0 (State 11 evidence `run-001/lint-src-exit-code.txt`)
- Zero runtime-panic surface in all 25 touched files (truth-serum audit)
- `agent-invocation-ledger.jsonl` chain intact through State 14 (entry 11, `c8578abf0ddc`)

## Landing Commands (coord checkout `/home/lewis/src/velvet-ballistics`)

```
$ bd close vb-7akm0 --reason "25 visibility-narrowing edits landed; moon run :lint-src exit 0; orphan test retired per source-length-exceptions.txt:221 plan; zero production-source regression."
✓ Closed vb-7akm0 — Lint: remove allow unreachable_pub suppressions by narrowing visibility: 25 visibility-narrowing edits landed; moon run :lint-src exit 0; orphan test retired per source-length-exceptions.txt:221 plan; zero production-source regression.
# EXIT=0

$ bd dolt push
Pushing to Dolt remote...
Push complete.
# EXIT=0

$ bd show vb-7akm0
✓ vb-7akm0 [BUG] ... [● P1 · CLOSED]  Updated: 2026-07-02
```

Raw evidence: `.beads/vb-7akm0/evidence/landing-state15/`
(`bd-close-output.log`, `bd-close-exit-code.txt`,
`bd-dolt-push-output.log`, `bd-dolt-push-exit-code.txt`,
`bd-show-after-close.log`, `bd-dolt-status-after.log`,
`jj-diff-stat.log`, `jj-work-commit.log`).

## Landed Change Inventory (25 files)

- Group A — vestigial suppression delete-allow: `diag_tests.rs`, `schema_tests.rs`, `fact_table.rs` (xtask crate-root suppression RESTORED, Deviation 1).
- Group B — `pub fn` → `pub(crate) fn`/`fn` (gate validators): `gate_07_stack`, `gate_08_accessor`, `gate_09_slots`, `gate_10_node`, `gate_11_loop`, `gate_12_14_15`, `gate_13_cycles`, `taint_prop`, `type_check`, `secret_leak`.
- Group C — `pub` → `pub(crate)` (schema/type): `type_sigs.rs`, `schema_doc.rs`, `schema_id.rs`, `schema_fields.rs`.
- Group D — delete-allow, externally reachable: `diagnostic.rs`, `diag_render.rs`, `lifecycle.rs`.
- Group E — orphan-test retirement + narrowing: `vb_test_cli_diff_incident_behavior.rs` (deleted, 646 lines), `.config/source-length-exceptions.txt:221` (row removed), `commands_diff.rs`, `commands_incident.rs`.
- Companion: `vb_cli/src/lib.rs` (`pub mod` → `pub(crate) mod` for `commands_diff`/`commands_incident`).

## Regression Attribution

The 25-file scope introduces **zero regressions**. Two non-PASS
obligations are pre-existing global defects (verified identical on the
parent commit) and are owned by separate beads:

| Defect | Pre-existing | New | Blocks landing |
|--------|--------------|-----|----------------|
| PO-TEST-001 `proptest_admission_with_budget_...` (vb_core red test) | YES | NO | NO |
| PO-EXTERN-001 12 production_inner drifts | YES | NO | NO |

## Code Integration Note

Bead-DB landing (close + dolt push) is complete. The code commit
`d4476627` remains on the isolate bookmark `cheap25-vb-7akm0` for
integration into `origin/main` by the femdation cheap25 batch
orchestrator. This report records the commit reference for traceability.

## Gate

- final-evidence-decision APPROVED: **PASS**
- `bd close` exit 0: **PASS**
- `bd dolt push` exit 0: **PASS**
- bead status CLOSED: **PASS**
- ledger chain intact (state 15 appended): **PASS**

**STATUS: LANDED**
