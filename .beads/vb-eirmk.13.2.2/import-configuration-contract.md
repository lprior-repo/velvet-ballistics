# Import Configuration Alignment Contract
# Bead: vb-eirmk.13.2.2

## Scope
Align import `#[cfg]` attributes so Kani compilation succeeds on
`crates/vb_core/src/budget/aggregate.rs` and `crates/vb_core/src/budget/compute.rs`
without altering runtime logic, partition harnesses, or production behavior.

## Problem
Five imports are pulled unconditionally but used exclusively within
`#[cfg(not(kani))]` blocks. Under the Kani profile they become unused imports,
failing strict compilation (`-D warnings`).

## Fix
Add `#[cfg(not(kani))]` before exactly these five import statements:

1. `crates/vb_core/src/budget/aggregate.rs`:
   - `use crate::workflow::WorkflowError;`

2. `crates/vb_core/src/budget/compute.rs`:
   - `use super::depth::compute_structural_depth;`
   - `use super::loop_body::count_total_steps;`
   - `use super::reachable::compute_reachable_metrics;`
   - `use super::successors::bounded_tracking_vec;`

## Constraints
- No `#[allow(unused_imports)]` or other suppressions.
- No changes to `#[cfg]` on existing functions, branches, Drop impls, or code blocks.
- No deletion of imports from the native (non-Kani) path.
- No changes to partition harnesses, dispatch logic, math, production behavior,
  gates, settings, or verification evidence.
- No proof edits beyond these five attribute lines.

## Verification
1. `rustup run nightly-2026-04-28 rustfmt --edition 2024 --style-edition 2024`
   on both changed files.
2. Strict native compile:
   `TMPDIR=/home/lewis/.vbs CARGO_TARGET_DIR=/home/lewis/src/isoloated/velvet-ballistics-core-current-review/target CARGO_BUILD_JOBS=2 RUSTC_WRAPPER= RUSTFLAGS=-Dwarnings rustup run nightly-2026-04-28 cargo check --lib -p vb_core --all-features`
   Expected: exit 0.
3. Kani strict compilation will be run by the parent agent after review.

## Limits
- Maximum 10 lines of source changed (5 attributes + formatting).
- Two files touched: aggregate.rs, compute.rs.
- No new dependencies, no public API changes, no schema changes.
