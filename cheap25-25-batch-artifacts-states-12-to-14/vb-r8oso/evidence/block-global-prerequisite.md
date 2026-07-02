# BLOCK_GLOBAL Prerequisite — vb-r8oso

**bead_id:** vb-r8oso
**owner:** landing-skill
**captured_at:** 2026-07-01T20:30:00Z

## Pre-existing failure observed in the workdir's parent commit

`cargo test -p vb_core --test aggregate_resource_budget_properties_red proptest_admission_with_budget_has_runtime_capacity_rejection_surface` fails in the workdir's parent commit (`1d6c017f`, AGENTS.md round10 forward-port).

```
thread 'proptest_admission_with_budget_has_runtime_capacity_rejection_surface'
  panicked at crates/vb_core/tests/aggregate_resource_budget_properties_red.rs:6:1:
Test failed: assertion failed: `(left == right)`
  left: `false`,
 right: `true` at crates/vb_core/tests/aggregate_resource_budget_properties_red.rs:73.
minimal failing input: requested = 1
```

Line 73 is:

```rust
prop_assert_eq!(ADMISSION_RS.contains("ResourceCapacityExceeded"), true);
```

The `ADMISSION_RS` constant is the result of `include_str!("../../vb_runtime/src/admission.rs")`. The admission module was split into focused chunks (under `admission/parts/`) and the literals `admit_run_with_budget` and `ResourceCapacityExceeded` now live in the chunk files, not the shell.

## Resolution on main

The fix is committed on `main` as commit `93d1d9026`:

```
fix(proptest+ipc): BLOCK_GLOBAL aggregate_resource_budget proptest + IPC inspect-run handler
```

The fix concatenates the focused chunks via `admission_production_surface()` so the proptest sees the same surface the production binary sees. After the fix, the test passes.

The workdir's parent commit `1d6c017f` does not contain this fix; the bead delivery is therefore working from a parent commit that has a known pre-existing `BLOCK_GLOBAL` failure. The failure is not introduced by this bead.

## Classification

- `BLOCK_GLOBAL` — pre-existing repo-wide failure on a parent commit.
- Not a regression introduced by `vb-r8oso`.
- Holzman Rust acceptance gate (`cargo test -p vb_storage`) is fully green.
- Downstream `landing-skill` must either rebase on `main` (which contains the fix) or cherry-pick `93d1d9026` before the final landing.

## Evidence

```
$ cd /home/lewis/src/velvet-ballistics && git log --oneline 1d6c017f..HEAD \
    -- crates/vb_core/tests/aggregate_resource_budget_properties_red.rs
b160d3a8b style(fmt): apply cargo fmt changes
93d1d9026 fix(proptest+ipc): BLOCK_GLOBAL aggregate_resource_budget proptest + ...
```

```
$ cd /home/lewis/src/velvet-ballistics && cargo test -p vb_core \
    --test aggregate_resource_budget_properties_red
cargo test: 5 passed (1 suite, 0.09s)        # clean on main
```

```
$ cd /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-r8oso \
    && cargo test -p vb_core --test aggregate_resource_budget_properties_red
test result: FAILED. 0 passed; 1 failed; ... # pre-existing failure on parent
```

## Action

- **Owner:** landing-skill.
- **Action:** rebase the bead delivery onto a parent commit that includes `93d1d9026` (or cherry-pick the fix into the landing commit) before merging the bead to `main`.
- **Status:** open. Non-blocking for the Holzman Rust stage but blocking for the landing stage.
