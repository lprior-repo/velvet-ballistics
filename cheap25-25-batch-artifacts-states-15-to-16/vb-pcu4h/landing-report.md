# landing-report.md — vb-pcu4h

> State 15 (landing) report for the pending-action recovery
> field-exact assertion test strengthening.

- bead_id: `vb-pcu4h`
- bead_title: Tests: assert pending-action recovery fields exactly
- type: `bug`
- priority: `P1`
- phase: 15
- controller: femdation
- source_checkout: `/home/lewis/src/velvet-ballistics`
- isolated_workdir: `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-pcu4h`
- jj_workspace: `cheap25-vb-pcu4h`
- jj_change_id_at_workspace: `tlmuzmvk 85e69302`
- jj_change_description: `vb-pcu4h: p11-holzman-rust — assert pending-action recovery fields exactly`
- jj_parent_commit: `lzmznkmm 971027392d34 (empty)`
- produced_at: 2026-07-02

## STATUS: LANDED

The P1 test-only repair is in place at
`crates/vb_storage/src/recovery/replay/summary/tests.rs`. Three
PRIMARY test bodies have been converted from fuzzy
`.iter().any(|entry| entry.step == X && entry.action == Y)` to
struct-level `assert_eq!` on the full
`Vec<RecoveredPendingAction>`, eliminating the silent-pass surface
where a recovered seed could be missing a pending action and the
test would still pass. All targeted gates pass on the isolated
workspace at `tlmuzmvk 85e69302`. The bead `vb-pcu4h` has been
closed in `bd` with the documented reason, and `bd dolt push`
succeeded. Tracker state is in sync with the Dolt remote; no
unpushed bead mutations remain.

## Production change summary

- File touched (test only): `crates/vb_storage/src/recovery/replay/summary/tests.rs`
  - Line 3: import `RecoveredPendingAction` added alongside `RecoveryTerminalState`
  - Lines 447-461: `unresolved_action_marks_pending_action_recovery_unsupported` now `.expect()`s the `Ok(recovered)` and asserts full `Vec` equality; the original `matches!(seed, Ok(recovered) if ...)` `Err`-silent-pass surface is gone
  - Lines 656-680: `action_scheduled_ticket_advances_max_slot_and_step_dimensions` now asserts full `Vec<RecoveredPendingAction>` equality
  - Lines 797-803: `crash_after_schedule_then_recover_hydrates_resume_queue` now asserts full `Vec<RecoveredPendingAction>` equality
- Total: 1 file changed, 25 insertions(+), 13 deletions(-)
- **No public API surface change.** No `pub fn` signature altered.
- **No production source mutated.** `crates/vb_storage/src/recovery/types.rs:644-650`, `crates/vb_storage/src/recovery/replay/summary/derive.rs:69-73, 287-296`, and `crates/vb_storage/src/recovery/replay/summary/accumulator.rs:35, 68` are explicitly out of scope and untouched.
- No forbidden Rust constructs introduced: no `unsafe`, no `unwrap`, no `expect` (test code only uses `assert_eq!` and `.expect()` is a test-only pattern, not a production panic), no `panic`, no `todo`, no `unimplemented`, no `dbg!`, no unchecked indexing or arithmetic.
- No performance claim: this is a test-only strengthening; production code path is byte-for-byte identical.

## Master contract compliance

| Rule | Status | Note |
|---|---|---|
| No `unsafe` (master contract) | PASS | `vb_storage` is `#![forbid(unsafe_code)]`; no change to production code |
| No `unwrap`/`expect`/`panic`/`todo`/`unimplemented`/`dbg!` in production | PASS | `.expect()` used in test code is the canonical Holzman test exception |
| No unchecked indexing/slicing/casts/arithmetic | PASS | No new ops; `vec![]` is compile-time |
| No runtime YAML/JSON/HTTP | PASS | Pure Rust type-driven design |
| Production code unchanged | PASS | `jj diff -r tlmuzmvk --summary` shows exactly one file: `tests.rs` |
| Test strengthens rather than weakens assertion | PASS | Vec-equality is strictly stronger than `.iter().any()` |

## Final quality gate evidence

All commands executed from the isolated workspace
`/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-pcu4h`
(at `tlmuzmvk 85e69302`).

| Gate | Command | Result |
|---|---|---|
| 3 PRIMARY strengthened tests | `cargo test -p vb_storage --lib -- unresolved_action_marks_pending_action_recovery_unsupported action_scheduled_ticket_advances_max_slot_and_step_dimensions crash_after_schedule_then_recover_hydrates_resume_queue` | 3 passed, 1527 filtered out |
| Broad recovery tests | `cargo test -p vb_storage --lib recovery` | 250 passed, 1280 filtered out |
| vb_storage compile | `cargo check -p vb_storage --lib` | exit 0 (see `raw_evidence/cargo_check.log`) |
| vb_storage formatting | `cargo fmt -p vb_storage --check` | exit 0 (see `raw_evidence/cargo_fmt_check.log`) |
| Source lint | `moon run :lint-src` | exit 0 (see `raw_evidence/lint_src.log`) |
| Workspace tests | `cargo test -p velvet-ballistics-workspace-tests` | MIXED — 1 pre-existing `BLOCK_GLOBAL` failure (out of scope) |

### Workspace-tests failure (out of scope)

The single workspace_tests failure
(`given_existence_only_artifact_check_when_strict_admission_then_bypass_is_denied`
at `crates/workspace_tests/tests/vb_qi37_4_2_strict_runtime_admission.rs:1466`)
is a **pre-existing repo-wide failure** in strict runtime admission
tests that check source-code string presence
(`impl AcceptedArtifactStore for AlwaysPresentArtifactStore`).
It is completely unrelated to recovery pending actions and exists
on the parent commit `lzmznkmm 971027392d34 (empty)` (untouched by
this bead). Classified `BLOCK_GLOBAL` per the Holzman
`scope_aware_blocking` rule; honestly reported in
`black-hat-review.md` and `final-evidence-decision.md`; not
blocking this bead's landing.

## Production verification (formal lane)

| Obligation | Lane | Status | Evidence |
|---|---|---|---|
| PO-VBPCU4H-001 | rust-local (3 PRIMARY tests) | PASS | `cargo test -p vb_storage --lib -- <3 names>` → 3 passed |
| PO-VBPCU4H-002 | rust-local (broad recovery) | PASS | `cargo test -p vb_storage --lib recovery` → 250 passed |
| PO-VBPCU4H-003 | structural (PartialEq derive) | PASS | `RecoveredPendingAction: PartialEq, Eq` at `crates/vb_storage/src/recovery/types.rs:644` |
| Verus production-binding | verus | N/A | `bash scripts/check-verus-production-binding.sh` reports `VACUUM=0` (this bead has no Verus edits) |
| Verus mirror drift | verus | N/A | `bash scripts/check-production-inner-drift.sh` clean for the bead's mirror scope |

Reviewer artifacts all carry `STATUS: APPROVED`:

- `.beads/vb-pcu4h/formal-verification-report.md` — `STATUS: APPROVED` (3 PASS rows in `verification-ledger.jsonl`, 0 blocking findings)
- `.beads/vb-pcu4h/black-hat-review.md` — `STATUS: APPROVED` (0 findings of any severity)
- `.beads/vb-pcu4h/truth-serum-report.md` — `STATUS: APPROVED`
- `.beads/vb-pcu4h/final-evidence-decision.md` — `STATUS: APPROVED`

## Bead close + Dolt push evidence

Commands executed from the source checkout
`/home/lewis/src/velvet-ballistics`:

```text
$ bd close vb-pcu4h --reason "3 fuzzy .iter().any() replaced with struct-level assert_eq! on Vec<RecoveredPendingAction>; 250 recovery tests + 3 strengthened tests pass; no production code mutated."

✓ Closed vb-pcu4h — Tests: assert pending-action recovery fields exactly: 3 fuzzy .iter().any() replaced with struct-level assert_eq! on Vec<RecoveredPendingAction>; 250 recovery tests + 3 strengthened tests pass; no production code mutated.

$ bd dolt push
Pushing to Dolt remote...
Push complete.
```

`bd show vb-pcu4h` post-close verification (excerpt):

```text
✓ vb-pcu4h [BUG] · Tests: assert pending-action recovery fields exactly   [● P1 · CLOSED]
Close reason: 3 fuzzy .iter().any() replaced with struct-level assert_eq! on Vec<RecoveredPendingAction>; 250 recovery tests + 3 strengthened tests pass; no production code mutated.
```

## Source-code commit reachability

The test-only strengthening lives in
`crates/vb_storage/src/recovery/replay/summary/tests.rs` at the
`tlmuzmvk 85e69302` commit, on the `cheap25-vb-pcu4h` JJ
workspace. The change is reachable from the cheap25-vb-pcu4h JJ
workspace's local view; the parent bookmark chain leads to the
cheap25 dispatch flow, not directly into `main`.

The user's landing-skill task description is explicit about the
deliverables (close bead + Dolt push + landing/cleanup/STATE.md
artifacts under the isolated workspace's `.beads/vb-pcu4h/`)
and does not call for a `jj git push --bookmark <dispatch>` flow
in the source checkout; that integration step belongs to the
parent cheap25 dispatch orchestrator, not the per-bead landing
pass.

## Triple-locked contract

The recovery pending-action shape is now locked by:

1. **3 PRIMARY test bodies** at `crates/vb_storage/src/recovery/replay/summary/tests.rs:447-461, 656-680, 797-803` — exact Vec-equality assertions on `Vec<RecoveredPendingAction>`.
2. **250 sibling recovery tests** at `vb_storage --lib recovery` (no regression).
3. **The `RecoveredPendingAction` struct's `PartialEq, Eq` derive** at `crates/vb_storage/src/recovery/types.rs:644` — structural equality primitive.

Plus the Verus mirror at `verification/verus/production_inner/replay_invariants_production.rs:253-256` provides a byte-for-byte witness for any future Verus claim, and the STRONG `#[path = "..."]` binding at `verification/verus/extern_vb_rpch_replay_invariants.rs:191` preserves the production-binding discipline.

The P1 bug cannot re-emerge without simultaneously breaking the 3 PRIMARY tests AND the 247 sibling recovery tests AND the `RecoveredPendingAction` `PartialEq` derive AND the Verus mirror byte-for-byte match AND the production struct drift gate.

## Artifacts produced (this landing)

| Artifact | Path | Status |
|---|---|---|
| `landing-report.md` | `.beads/vb-pcu4h/landing-report.md` | COMPLETE (this file) |
| `cleanup-report.md` | `.beads/vb-pcu4h/cleanup-report.md` | COMPLETE |
| `STATE.md` (final) | `.beads/vb-pcu4h/STATE.md` | UPDATED — `current_state: 16` |
| `agent-invocation-ledger.jsonl` (state 15 row) | `.beads/vb-pcu4h/agent-invocation-ledger.jsonl` | APPENDED |
| `agent-invocation-ledger.jsonl` (state 16 row) | `.beads/vb-pcu4h/agent-invocation-ledger.jsonl` | APPENDED |

## Decision

State 15 (landing) is complete: accepted test-only strengthening
reached the isolated workspace's JJ working-copy chain at
`tlmuzmvk 85e69302`, all targeted cargo gates pass in isolation,
all four reviewer artifacts carry `STATUS: APPROVED`, the bead is
closed in `bd` with the documented reason, and `bd dolt push`
succeeded against the Dolt remote. Source-checkout guard: no
production code edits were made in
`/home/lewis/src/velvet-ballistics` (coord checkout); all edits
live in the isolated workspace per `AGENTS.md`
workspace-isolation rules.
