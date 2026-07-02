# Regression Diff

Scoped diff effect:
- `vb_ipc` public facade now re-exports canonical modules instead of duplicating definitions in `lib.rs`.
- Command and payload split modules now match the previously public command surface.
- Kani harness compile drift repaired.

No dependency files changed.

Observed unrelated regression debt:
- Workspace-wide Moon check fails in `vb_storage` test warning debt, outside this bead's files.
