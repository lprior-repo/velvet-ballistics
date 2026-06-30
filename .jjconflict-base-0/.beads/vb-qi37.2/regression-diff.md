# Regression Diff: vb-qi37.2 State 11

## Changed Files

- `crates/vb_core/src/budget.rs`
- `crates/vb_core/src/value_store.rs`
- `.beads/vb-qi37.2/*` evidence and lifecycle artifacts

## Risk

- Runtime production behavior is unchanged outside `cfg(kani)` and `cfg(miri)` test/proof paths.
- Fuzz and `moon ci` remain unexecuted because of global toolchain/workspace blockers, so landing is blocked.
