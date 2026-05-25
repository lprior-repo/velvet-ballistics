# Worktree Inventory — vb-ccyi

Generated: 2026-05-24

## Summary

Six worktrees/directories identified as containing uncommitted diffs from
closed proof follow-up work. All six still exist on disk. Five are linked git
worktrees of velvet-ballistics; one (vb-jpq7-48-evidence-checker-gpt55-recovered)
is a plain directory.

---

## 1. vb-utvm-vb-validate-kani-gpt55

- **Path:** `/home/lewis/src/vb-utvm-vb-validate-kani-gpt55`
- **Type:** Git worktree (linked to main repo)
- **Branch:** `vb-utvm-vb-validate-kani-gpt55`
- **HEAD:** `e81c38e94`
- **Modified (tracked):**
  - `crates/vb_core/src/ids/kani_id_arbitrary.rs` (+20/-1)
    - Added `Arbitrary` impls for `ExprIdx`, `AccessorIdx`, `ConstIdx`
  - `crates/vb_validate/src/kani_gate_08_structural.rs` (+3/-2)
    - Import reorder, removed unused `mut`, added fallback match arm
- **Untracked:** `.evidence/vb-utvm/`
- **Status vs main: ALL CHANGES ALREADY IN MAIN** — `diff` between worktree and
  `../../../velvet-ballistics` returns `[ok] Files are identical`.

---

## 2. vb-2tpu-recovery-replay-tla-gpt55

- **Path:** `/home/lewis/src/vb-2tpu-recovery-replay-tla-gpt55`
- **Type:** Git worktree (linked to main repo)
- **Branch:** `vb-2tpu-recovery-replay-tla-gpt55`
- **HEAD:** `e81c38e94`
- **Modified (tracked):**
  - `specs/tla/RecoveryReplayFull.tla` (+127/-82)
    - Major bounded-finite rewrite: added `GeneratedEventType`,
      `RecoveryErrors`, `NoScheduleAfterResolved`, `workflow_verified`,
      `ir_verified`; tightened journal constraints
  - `specs/tla/RecoveryReplayFull.cfg` (+6/-10)
    - Reduced constant sets for bounded checking
- **Untracked:**
  - `.evidence/vb-2tpu/` (TLC logs, nonvacuity evidence, recovery-replay-evidence.md)
  - `specs/tla/RecoveryReplayErrors.{tla,cfg}`
- **Status vs main: ALL CHANGES ALREADY IN MAIN** — TLA+ files identical; Errors
  files and evidence directory already present in main repo.

---

## 3. vb-y2vn-recovery-replay-scale-gpt55

- **Path:** `/home/lewis/src/vb-y2vn-recovery-replay-scale-gpt55`
- **Type:** Git worktree (linked to main repo)
- **Branch:** `vb-y2vn-recovery-replay-scale-gpt55`
- **HEAD:** `e81c38e94`
- **Modified (tracked):**
  - Same TLA+ diffs as vb-2tpu (identical base model rewrite)
- **Untracked:**
  - `.evidence/vb-y2vn/` (scaled TLC runs)
  - `specs/tla/RecoveryReplayFull.scaled-seq3-events3.cfg`
  - `specs/tla/RecoveryReplayFull.scaled-two-actions-events2.cfg`
  - `specs/tla/RecoveryReplayFull.scaled-two-attempts-events2.cfg`
  - `specs/tla/RecoveryReplayFull.scaled-two-runs-events2.cfg`
- **Status vs main: ALL CHANGES ALREADY IN MAIN** — TLA+ files identical; all
  scaled cfg files already present in main.

---

## 4. vb-rga1-verusfmt-gpt55

- **Path:** `/home/lewis/src/vb-rga1-verusfmt-gpt55`
- **Type:** Git worktree (linked to main repo)
- **Branch:** `vb-rga1-verusfmt-gpt55`
- **HEAD:** `e81c38e94`
- **Modified (tracked):**
  - `verification/verus/vb_jpq724_events_for_run_production.rs` (+30/-38)
    - Pure formatting changes (verusfmt): line-wrapping, whitespace, brace
      placement. No logic or spec changes.
- **Untracked:** `.evidence/vb-rga1/`
- **Status vs main: ALL CHANGES ALREADY IN MAIN** — `diff` returns
  `[ok] Files are identical`.

---

## 5. vb-jpq7-48-evidence-checker-gpt55-recovered

- **Path:** `/home/lewis/src/vb-jpq7-48-evidence-checker-gpt55-recovered`
- **Type:** Plain directory (NOT a git repo)
- **Contents:** Full clone of velvet-ballistics workspace including:
  - `accepted_*` binary artifacts (Verus .smt2 dumps, 4.1MB each)
  - `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml`
  - `crates/`, `specs/`, `verification/`, `.evidence/`
  - Dozens of test plans, test reviews, black-hat reviews, formal reports
  - `velvet-ballistics-MASTER.md` (243.6KB)
  - Bead tracking data in `.beads/`
- **Status:** Recovered from a corrupted/failed worktree. All canonical
  artifacts already present in the main `velvet-ballistics` repo (test plans,
  reviews, evidence). The `accepted_*` files are large binary artifacts
  (Verus SMT2 output) that are not tracked in git.

---

## 6. vb-jpq7-49-anti-laundering-gpt55

- **Path:** `/home/lewis/src/vb-jpq7-49-anti-laundering-gpt55`
- **Type:** Git worktree (linked to main repo)
- **Branch:** Detached HEAD at `829d8bcd1` (no branch)
- **HEAD:** `829d8bcd1` ("test: rounds 8-10 - exhaustive behavior tests")
- **Modified (tracked):** None
- **Untracked:** `.evidence/vb-jpq7.49/`
  - Contains `anti-laundering-closure.md` (7.8KB) and
    `verification-ledger.jsonl` (2.3KB)
- **Status:** `.evidence/vb-jpq7.49/` already exists in main repo.

---

## Disposition Matrix

| Worktree | Tracked Changes | Already in Main? | Evidence in Main? | Disposition |
|---|---|---|---|---|
| vb-utvm | Kani Arbitrary impls + harness fixes | Yes | Yes | RETIRED |
| vb-2tpu | TLA+ bounded rewrite | Yes | Yes | RETIRED |
| vb-y2vn | Same TLA+ rewrite + scaling cfgs | Yes | Yes | RETIRED |
| vb-rga1 | Verusfmt formatting | Yes | Yes | RETIRED |
| vb-jpq7-48-recovered | Full workspace copy (non-git) | N/A | Yes | RETIRED |
| vb-jpq7-49 | Detached HEAD, only evidence | N/A | Yes | RETIRED |
