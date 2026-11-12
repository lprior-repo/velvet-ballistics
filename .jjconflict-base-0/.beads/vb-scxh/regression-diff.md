# Regression Diff: vb-scxh State 11

STATUS: REJECTED

## Scope

- Isolated workspace: `/home/lewis/src/vb-scxh`.
- Forbidden workspace: `/home/lewis/src/Velvet-ballistics` was not modified.
- Allowed State 11 writes: `.beads/vb-scxh/` evidence artifacts only.

## Diff Evidence

- Pre-artifact command: `git diff --name-only` from `/home/lewis/src/vb-scxh` produced no stdout before State 11 reports were written.
- State 11 writes are limited to evidence/report files under `.beads/vb-scxh/`.

## Regression Classification

- No production, test, proof-model, or implementation file edits were made.
- Current command evidence after rerun:
  - Moon CI source/workspace blocker is repaired: `TMPDIR=/home/lewis/src/vb-scxh/target/tmp RUSTC_WRAPPER= moon ci --force --summary normal` exited 0 with `Actions: 21 completed` and `8185 passed, 6 skipped`.
  - Remaining local evidence blocker: missing/unopenable safety bundle path `/home/lewis/src/Velvet-ballistics-rescue-20260513T022011Z.bundle`.
  - TLC prior repo-local rerun remains PASS; no new TLC regression observed in this rerun.
