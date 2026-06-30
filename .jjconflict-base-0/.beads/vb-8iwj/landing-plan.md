bead_id: vb-8iwj
phase: State 15 landing policy
updated_at: 2026-05-11T00:00:00Z

# Safe Landing Plan

STATUS: LANDING_BLOCKED

## Current safe state

The original integration directory `/home/lewis/src/Velvet-ballistics-vb-8iwj-wave3-integration` was absent when this session resumed. The JJ integration change still exists as:

```text
tqypyqys 57f44923 vb-8iwj: integrate wave 3 CLI workspaces
```

This session created a non-landing preflight workspace on top of that change:

```text
/home/lewis/src/Velvet-ballistics-vb-8iwj-wave3-preflight
zmryxnnv e3b5bb45 (empty) vb-8iwj: run wave 3 landing preflight
parent: tqypyqys 57f44923
```

No source push was performed. Root workspace was not used for source operations.

## Recommended landing sequence once policy is explicitly approved

1. Choose the landing target:
   - Option A: create a local JJ bookmark at `tqypyqys`/or the preflight child and hand off a push command without pushing.
   - Option B: allow the orchestrator to push a named bookmark to remote.
   - Option C: keep blocked and let a human integrate the merge change.
2. Before any close:
   - verify `jj workspace list` includes all original workspaces and the integration/preflight workspace;
   - verify no unresolved conflicts;
   - run at minimum `moon run :quick`, `moon run :test`, and scoped CLI tests from this preflight evidence;
   - classify `moon ci` failures as `DEFERRED_GLOBAL` only if they still match `vb-w823`.
3. Land only the integrated merge change, not the three sibling changes separately, to avoid reintroducing the EOF append conflict.
4. After source landing is proven, update/close `vb-qi37.13.4`, `vb-qi37.15.1`, `vb-qi37.15.2`, and `vb-8iwj`.
5. Only after landing and bead sync, forget/remove the original three workspaces and verify both:
   - `jj workspace list` no longer contains them;
   - their isolated directories no longer exist.

## Exact policy question needed

Which landing policy should be used for integrated change `tqypyqys 57f44923` / preflight child `zmryxnnv e3b5bb45`?

1. **Bookmark only, no push**: create a local bookmark (suggested: `go/vb-8iwj-wave3-integration`) at the integrated/preflight change and report the exact `jj git push --bookmark ...` command for a human to run.
2. **Push bookmark**: create and push a remote bookmark for review/merge, without touching `main` directly.
3. **Human landing**: leave all workspaces and beads open; human integrates `tqypyqys` manually.
4. **Different policy**: provide exact bookmark name, remote, base, and whether source push is allowed.

Until one option is approved, State 15 remains `LANDING_BLOCKED`.
