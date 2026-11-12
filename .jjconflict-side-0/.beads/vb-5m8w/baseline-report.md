bead_id: vb-5m8w
bead_title: Add TLA+ Step Budget Model
phase: 1
updated_at: 2026-05-18T17:32:11Z
attempt: 1-of-7

# Baseline Report

## Context

- source_checkout: `/home/lewis/src/velvet-ballistics`
- isolated_workspace: `/home/lewis/src/go-skill-vb-5m8w`
- working_copy: `wrxxtnnw b20a6ef4`
- parent_commit: `ysnxntql cc80fac3 main | fix: correct schema_version in .cue contract files to 1.0.0`

## Commands

1. `bd --db /home/lewis/src/velvet-ballistics/.beads/dolt show vb-5m8w --json`
   - exit: 0
   - result: bead found, `status=in_progress`, `assignee=Lewis`.
2. `pwd -P`
   - exit: 0
   - result: `/home/lewis/src/go-skill-vb-5m8w`.
3. `case "$(pwd -P)" in "/home/lewis/src/velvet-ballistics"|"/home/lewis/src/velvet-ballistics"/*) exit 1;; esac`
   - exit: 0
   - result: isolated workspace is outside source checkout.
4. `jj status`
   - exit: 0
   - result: no working-copy changes before bead edits.
5. `moon ci`
   - exit: 0
   - result: `Tasks: 23 completed`; elapsed `2m 22s 143ms`.

## Baseline Classification

Baseline canonical CI is clean at the shared parent commit. Future scoped failures are `BLOCK_LOCAL` or `BLOCK_REGRESSION` unless evidence shows unrelated environmental/tooling failure.
