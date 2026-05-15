# Baseline Report — vb-qi37.12.2

bead_id: vb-qi37.12.2
phase: 1
attempt: 1-of-7

## Baseline reality
- Recovery worktree existed before this session at `/home/lewis/src/vb-qi37-12-2`, detached at current main per user context.
- Existing artifacts at start: `STATE.md`, `test-plan.md`, `test-writer-report.md` only.
- `bd show vb-qi37.12.2 --json` against workspace-local `.beads` failed due missing `issues` table.
- Shared Dolt database `/home/lewis/src/.beads/dolt` resolved bead metadata.
- `cargo fmt --check` baseline/global gate is red due unrelated workspace fmt drift and a syntax error in `fuzz/src/bin/step_budget_new.rs`.

## Baseline command evidence
- `bd --db /home/lewis/src/.beads/dolt show vb-qi37.12.2 --json`: PASS, bead open before claim.
- `jj workspace list`: FAIL, replacement worktree not jj-managed.
- `cargo fmt --check`: FAIL, global format drift/syntax outside bead scope.
