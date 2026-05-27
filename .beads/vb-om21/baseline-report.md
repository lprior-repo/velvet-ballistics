# Baseline Report - vb-om21

Generated: 2026-05-25T15:14:41.035216+00:00

## Commands / Evidence
- Skill existence gate: PASS for femdation, go-skill, and delegated specialist skill/agent files listed by femdation mandatory gate.
- `bd update vb-om21 --claim`: PASS in fleet claim wave.
- Git worktree: `/home/lewis/isolated/femdation-velvet-ballistics/vb-om21` on branch `femdation/vb-om21-20260525-h1` at source HEAD `96e2518b5`.
- Source checkout: `/home/lewis/src/velvet-ballistics`. Source checkout is control-plane only; no specialist may write there.
- Isolation rule: workspace path is outside source checkout and not under it.

## Baseline Notes
- Initial `/tmp/opencode/femdation-velvet-ballistics` worktree attempt failed while writing checkout files; abandoned and cleaned.
- `/home/lewis/isolated/femdation-velvet-ballistics` selected for all active beads.
- Git hook emitted `env: 'sh': No such file or directory` during worktree creation, but each worktree was registered and checked out by Git. Treat as infrastructure note; State 2 may proceed if validator passes.

## BD Server Metadata Repair
- Timestamp: 2026-05-25T15:29:58.434909+00:00
- Mirrored server route: 127.0.0.1:37421 / velvet_ballistics.
- Set `/home/lewis/isolated/femdation-velvet-ballistics/vb-om21/.beads` permissions to 0700.
