bead_id: vb-qi37.6
phase: 1
attempt: 1-of-7

# Baseline report

Source checkout: `/home/lewis/src/Velvet-ballistics`
Isolated workspace: `/home/lewis/src/vb-qi37-6`

Baseline was captured before local repair edits in this replacement worktree.

## Commands

```text
$ pwd -P
/home/lewis/src/vb-qi37-6

$ path guard
PATH_GUARD_PASS

$ git status --short && git rev-parse HEAD && git rev-parse --show-toplevel
[no git status --short output]
c6272854a341ff3e5017db2aae703aa6d1483d7f
/home/lewis/src/vb-qi37-6

$ BD_DB=/home/lewis/src/.beads/dolt bd show vb-qi37.6 --json
bead exists: verifier/runtime: Capability model enforcement; status open; assignee Lewis
```

## Baseline caveats

- `jj workspace list` failed because the replacement checkout is not a usable jj workspace in this environment (`/home/lewis/.jj/repo/store/type` missing).
- User explicitly supplied this path as the clean replacement isolated worktree.
- Repo-wide `cargo fmt --check` later failed on pre-existing unrelated formatting/parse issues, including `fuzz/src/bin/step_budget_new.rs:2:1 expected item, found '!'`; this is not a clean State 11 result and is not accepted as a pass.
