bead_id: vb-qi37.15.3
bead_title: cli: Add trace command
phase: 1
updated_at: 2026-05-18T00:00:00Z
attempt: 1

## Baseline Report - Pre-Edit State

### Source Checkout
- path: /home/lewis/src/velvet-ballistics
- HEAD: a120d107 (main)
- source is control-plane only

### Isolated Workspace
- path: /home/lewis/src/vb-qi37-15-3
- HEAD: 139d9193 (vb-eydg-kani branch)
- note: workspace is a separate git repo, NOT a jj worktree of source checkout

### Pre-Existing Build Errors
```
error[E0428]: the name `contracts` is defined multiple times
  --> xtask/src/lib.rs:25:1
   |
3 | pub mod contracts;
   | ------------------ previous definition of the module `contracts` here
```

### Workspace Cargo State
- Build fails with duplicate module error in xtask
- 229 crates total
- 8 warnings, 1 error

### Bead Context
This bead implements the `trace` command for CLI with:
- Structured event stream inspection
- Filters by run, step, shard, action, status, time range, diagnostic severity
- Tests for empty, filtered, and invalid requests

### Prior Artifacts (from previous partial run)
The workspace already contains some artifacts that may be stale:
- contract.md
- lean-contract.md
- proof-obligations.jsonl
- tla-spec.md
- traceability-matrix.jsonl
- verification-layers.md

These artifacts were NOT produced by a completed State 1 (no STATE.md exists).

### Verification Commands
```bash
# Build verification (fails with duplicate module error)
rtk cargo build --manifest-path /home/lewis/src/vb-qi37-15-3/Cargo.toml --all

# Source checkout at:
# 139d9193 (vb-eydg-kani) test: mark failing tests as #[ignore]
```

### Notes
- This baseline was captured BEFORE any bead-specific implementation work
- The duplicate contracts module error is a pre-existing issue in the workspace
- STATE.md was missing - this is a fresh State 1 initialization
