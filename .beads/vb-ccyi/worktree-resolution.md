# Worktree Resolution — vb-ccyi

Generated: 2026-05-24
Bead: vb-ccyi ("P0: land or retire closed proof follow-up worktree diffs")

## Resolution: ALL WORKTREES RETIRED

All six worktrees/directories have been audited. Every substantive tracked change
has already been incorporated into the main `velvet-ballistics` repository
(branch `wip/active-verification-state-20260524`, HEAD `3d548fed1`).

No further landing is required. All worktrees can be safely removed.

---

### 1. vb-utvm-vb-validate-kani-gpt55 → RETIRED
- **Evidence of absorption:** `diff` between worktree and main repo confirms
  `kani_id_arbitrary.rs` and `kani_gate_08_structural.rs` are identical.
- **What was absorbed:** Kani `Arbitrary` impls for `ExprIdx`, `AccessorIdx`,
  `ConstIdx`; import reorder; removal of unused `mut`; fallback match arm.
- **Command evidence:**
  ```
  diff .../vb-utvm-vb-validate-kani-gpt55/crates/vb_core/src/ids/kani_id_arbitrary.rs \
       .../velvet-ballistics/crates/vb_core/src/ids/kani_id_arbitrary.rs
  [ok] Files are identical
  ```

### 2. vb-2tpu-recovery-replay-tla-gpt55 → RETIRED
- **Evidence of absorption:** `diff` confirms `RecoveryReplayFull.tla` and
  `.cfg` identical. `RecoveryReplayErrors.{tla,cfg}` and `.evidence/vb-2tpu/`
  already present in main repo.
- **What was absorbed:** Bounded finite TLA+ recovery replay model rewrite with
  `GeneratedEventType`, `RecoveryErrors`, `NoScheduleAfterResolved`,
  `workflow_verified`/`ir_verified` variables, tightened journal structure.

### 3. vb-y2vn-recovery-replay-scale-gpt55 → RETIRED
- **Evidence of absorption:** Same TLA+ base as vb-2tpu; all 4 scaled `.cfg`
  files (`scaled-seq3-events3`, `scaled-two-actions-events2`,
  `scaled-two-attempts-events2`, `scaled-two-runs-events2`) already in main.
- **Relation to vb-2tpu:** Scaling variant of the same TLA+ model. Both
  absorbed into main via the same upstream commit path.

### 4. vb-rga1-verusfmt-gpt55 → RETIRED
- **Evidence of absorption:** `diff` confirms
  `vb_jpq724_events_for_run_production.rs` identical.
- **What was absorbed:** Pure `verusfmt` reformatting — line wrapping,
  whitespace, brace placement. No logic or spec changes.

### 5. vb-jpq7-48-evidence-checker-gpt55-recovered → RETIRED
- **Nature:** Plain directory (not a git repo) — recovered workspace snapshot
  from corrupted/failed worktree.
- **What it contains:** Full clone including `accepted_*` Verus SMT2 dumps
  (4.1MB each), test plans, reviews, bead data, master doc.
- **Disposition rationale:** All canonical artifacts already exist in the main
  repo. The large binary `accepted_*` files are Verus output dumps, not part
  of the tracked codebase. This directory can be safely removed.

### 6. vb-jpq7-49-anti-laundering-gpt55 → RETIRED
- **Nature:** Detached HEAD at commit `829d8bcd1` ("test: rounds 8-10").
  No tracked modifications. Only untracked `.evidence/vb-jpq7.49/`.
- **Disposition rationale:** `.evidence/vb-jpq7.49/` already present in main
  repo. No branch to land. The `anti-laundering-closure.md` (7.8KB) and
  `verification-ledger.jsonl` (2.3KB) are evidence artifacts whose canonical
  copies reside in the main repo.

---

## Action Log

| Date | Action | Detail |
|---|---|---|
| 2026-05-24 | Inventory | All 6 worktrees audited, diffs vs main compared |
| 2026-05-24 | Resolution | All 6 retired — no unlanded changes remain |
| 2026-05-24 | Cleanup | Worktrees removed via `git worktree remove` |
| 2026-05-24 | Bead closed | vb-ccyi closed with RETIRED disposition |

## Follow-up

None required. No valuable unlanded changes remain. All worktree changes
were already incorporated into the main repository's active branch
(`wip/active-verification-state-20260524`).
