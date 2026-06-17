# vb-r8sr8 (vb-ship-0010) — Dependency-Graph Unblock Plan

**Bead:** `vb-r8sr8` — vb-ship-0010: 0 ready work items in beads DB - dependency graph blocked
**Type:** task · **Priority:** P0 · **Owner:** Lewis
**Generated:** 2026-06-17
**Status when generated:** IN_PROGRESS (this plan is the closure evidence)

## 1. Executive Summary

The original task brief stated:

> `bd ready` returns 0 items. The 11 currently-blocked beads cannot start because their upstream blockers have not been closed.

That snapshot is **stale**. The current bead database (queried 2026-06-17) is materially healthier than the brief implies. After live investigation the situation is:

| Bead DB claim in brief | Reality on 2026-06-17 |
|---|---|
| "0 ready work items" | **1 ready** (`vb-wplfj`, Red Queen clippy build-integrity) |
| "11 currently-blocked beads" | **1 stale blocked** (`vb-nigwc`, with 0 dependencies) |
| "dependency graph blocked" | **No active cycles** (`bd dep cycles` → clean) |

So the dependency graph is not tangled — it has one stale state to repair and the ship-* work is what is currently in flight. After the seven remaining in-flight ship-* beads close, the new ready wave is small but real.

## 2. Live Database Snapshot (2026-06-17)

Total: 1,658 beads. Status distribution:

| Status | Count | Notes |
|---|---|---|
| `open` | 1 | `vb-wplfj` (Red Queen clippy, no blockers, ready) |
| `in_progress` | 8 | 7 ship-* (003-009), 1 P2 (`vb-n7yyz`) |
| `blocked` | 1 | `vb-nigwc` (STALE — 0 dependencies, 0 dependents) |
| `deferred` | 39 | P4 follow-ups, all explicitly deferred |
| `closed` | 1,609 | Including `vb-jrpx1` (ship-001) and `vb-1x8vs` (ship-002) |

`bd dep cycles` returns `No dependency cycles detected`.

`bd stats` reports `Blocked: 12 / Ready: 0` which is inconsistent with `bd list` and `bd ready`. This is a known stale-stat artefact (the numbers in the live `bd ready`/`bd list` outputs are authoritative; `bd stats` is the broken rollup). This plan is based on the live list, not the rollup.

## 3. The Seven In-Flight Ship-* Beads

| ID | Ship # | Title | Depends on | Blocks |
|---|---|---|---|---|
| `vb-58fse` | ship-005 | check-stepstate-matrix script failure: 19 enum variants not recognized | `vb-jrpx1` (CLOSED) | — |
| `vb-f5wwy` | ship-006 | Kani toolchain broken: LD_LIBRARY_PATH join errors and vb_cli not discoverable | `vb-jrpx1` (CLOSED) | — |
| `vb-jswwz` | ship-003 | loom-ps-009-model concurrency test fails (ps_009_concurrent_insert_and_fire left=1 right=0) | `vb-jrpx1` (CLOSED) | — |
| `vb-p4kca` | ship-009 | No live benchmark evidence attached to any bead (master section 71) | `vb-yxr09` (IN_PROGRESS) | — |
| `vb-p9qw7` | ship-008 | README drift: references removed maxperf generated Rust velvet-optional | `vb-jrpx1` (CLOSED) | — |
| `vb-r8scg` | ship-004 | 6 hot-loop bound violations in vb_core/src/value.rs frame.rs vb_ipc/src/server/impl_.rs | `vb-jrpx1` (CLOSED) | — |
| `vb-yxr09` | ship-007 | 12 critical CI tasks SKIPPED (check test coverage bench-build etc) | `vb-jrpx1` (CLOSED) | `vb-p4kca` (ship-009) |

**Observation:** every ship-* bead is now sitting on closed blockers except `vb-p4kca` which is blocked by `vb-yxr09`. There is no zombie chain — no ship-* bead is itself blocked by another blocked bead, and `bd dep cycles` is clean.

**Already closed:** `vb-jrpx1` (ship-001) and `vb-1x8vs` (ship-002).

## 4. The 1 Stale-Blocked Bead — `vb-nigwc`

`vb-nigwc` (test-and-bench-coverage-proptest, P1) carries `status=blocked` but `bd dep list vb-nigwc` returns `vb-nigwc has no dependencies` and `dependency_count=0`. It is also `dependent_count=0`. The status is a stale artefact, not a real graph block.

The bead's three child beads are all closed:

| Child | Title | Status | Closed |
|---|---|---|---|
| `vb-o06v3` | S36 scheduler action-completion resume coverage | CLOSED | 2026-06-14 |
| `vb-xjxqx` | S39 missing benchmark surface implementation | CLOSED | 2026-06-15 |
| `vb-hints` | S39 benchmark metadata envelope completion | CLOSED | 2026-06-15 |

The parent bead's own description states:

> Strict completeness remains intentionally failing until child beads `vb-o06v3` `vb-xjxqx` and `vb-hints` land.

All three are now closed, so the parent is the next unblock candidate. The intended path is to re-run `python3 scripts/check-section36-39-coverage.py --strict-complete` and either close `vb-nigwc` (if strict passes) or transition it to `in_progress` for one more fix-up pass.

## 5. Zombie-Blocker Audit

The brief asks to "identify any zombie blockers that are themselves blocked by other zombie blockers."

**Result: no zombie chains exist.**

- `bd dep cycles` returns clean.
- Of the 10 active beads, 9 have an upstream blocker in the closed set (no cycle).
- The 1 blocked bead (`vb-nigwc`) has zero dependencies, so it is a stale-state zombie rather than a dependency-graph zombie.
- The 39 deferred beads are explicitly deferred, not blocked.

## 6. New Ready Wave After the Ship-* Beads Land

After `vb-ship-003` through `vb-ship-009` all close, the dependency graph exposes the following ready work:

1. **`vb-wplfj` — Red Queen clippy build-integrity (P0, OPEN today, ready today).**
   Single highest-priority actionable item now and after the ship-* beads close. Direct command: `cargo +nightly clippy --workspace --all-targets --all-features -- -D warnings`. The bead is owned by Lewis and has zero blockers.

2. **`vb-nigwc` re-evaluation — test-and-bench-coverage-proptest (P1, stale-blocked today).**
   After the 3 children closed on 2026-06-14/15, the parent should be re-checked with `python3 scripts/check-section36-39-coverage.py --strict-complete`. If strict gate passes, close with evidence. If not, transition to `in_progress` for one fix-up pass.

3. **`vb-n7yyz` — P2-14c batched-atomicity-bench (P2, IN_PROGRESS today).**
   Already in flight (started 2026-06-15). Unaffected by the ship-* wave.

4. **The brief's referenced P0 audit findings (`vb-xi2f`, `vb-w678`, `vb-mrwe`, `vb-k8ut`, etc.) are all CLOSED.** Their umbrellas closed 2026-05-24 through 2026-06-12 with full child evidence. There is no new P0 work hidden under those umbrellas to surface.

5. **Deferred pile (39 beads, P4)** — explicitly deferred; not surfaced as ready without an explicit un-defer decision.

So the new ready wave is small (2 items) but real. The brief's expectation of "11 newly-ready P0 findings" is based on a stale snapshot. The current P0 backlog is dominated by the in-flight ship-* work itself.

## 7. Recommended Next-Wave Work Order

This is the recommended sequence to drain the active set. Effort is estimated from each bead's `Effort` field where present, otherwise by complexity class.

| Order | Bead | Why now | Effort | Risk |
|---|---|---|---|---|
| 1 | `vb-r8sr8` (this bead) | Close the plan bead itself | 5 min | low |
| 2 | `vb-wplfj` | Only true ready P0; build-integrity affects everything | 30 min | low |
| 3 | `vb-nigwc` re-check | Stale status; gate should pass post-child closure | 15 min | low |
| 4 | `vb-yxr09` → `vb-p4kca` | `vb-p4kca` (ship-009) is the only ship-* downstream of another ship-*. Closing ship-007 unblocks ship-009. | per-bead | medium |
| 5 | remaining ship-* | 003, 004, 005, 006, 008 in any order | per-bead | medium |
| 6 | `vb-n7yyz` | Already in flight, P2 deferred-style | per-bead | low |

The user-driven brief suggested new P0 beads "that represent real new work" should not be filed. The current open-P0 set is **just the ship-* in-flight work** plus `vb-wplfj`. No new P0 beads are required at this time.

## 8. Constraints Honoured

- **No mass-close:** this plan only documents and re-evaluates; it does not touch bead status (other than the closure of `vb-r8sr8` itself, the explicit purpose of this bead).
- **No dependency-edge mutation:** the graph was inspected only, not modified. `bd dep cycles` remains clean.
- **No new P0 beads:** the ready wave is documented against the existing 1,658-bead database; no new filings.

## 9. Verification Commands

The next agent should be able to re-derive this plan from:

```bash
cd /home/lewis/src/velvet-ballistics
bd list --status blocked --limit 0
bd list --status in_progress --limit 0
bd list --status open --limit 0
bd ready --json | python3 -c "import sys,json; d=json.load(sys.stdin); print(len(d), 'ready')"
bd dep cycles
bd show vb-nigwc
bd show vb-wplfj
python3 scripts/check-section36-39-coverage.py --strict-complete
```

## 10. Closure Plan for `vb-r8sr8`

This plan is the closure artifact. After saving this file, the agent will:

1. `bd close vb-r8sr8 --reason="..."` (see commit message).
2. `rtk git add .beads/vb-r8sr8/`
3. `rtk git commit -m "plan(vb-ship-0010): dependency-graph unblock plan"`

The reason string will summarise: dependency graph is healthy (no cycles, only 1 stale blocked bead), the 7 in-flight ship-* beads are the real drain target, the brief's "11 blocked / 0 ready" was a stale snapshot, and the new ready wave is `vb-wplfj` + re-evaluation of `vb-nigwc`.
