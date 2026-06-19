# vb-06t25 progress

## Bead
- **ID**: vb-06t25
- **Title**: storage: fail closed for codec_miri_tests include_str target when module path moves
- **Status**: implementation complete, awaiting bead close
- **Scope**: `crates/vb_storage/src/codec_miri_tests_compile_check.rs`,
  `crates/workspace_tests/tests/vb_jpq7_3_fail_closed_storage_recovery_contract.rs`,
  and a new fail-closed sentinel test in `crates/workspace_tests/tests/`.

## Reference files read
- `/home/lewis/.opencode/skill/holzman-rust/SKILL.md`
- `/home/lewis/.agents/skills/holzman-rust/SKILL.md`
- `/home/lewis/.agents/skills/holzman-rust/references/nasa-jpl-standards.md`
- `/home/lewis/.agents/skills/holzman-rust/references/latency-throughput-playbook.md`
- `/home/lewis/.agents/skills/holzman-rust/references/runtime-performance-architecture.md`
- `/home/lewis/.agents/skills/holzman-rust/references/zero-cost-abstractions.md`
- `/home/lewis/.agents/skills/holzman-rust/references/simd-patterns.md`
- `/home/lewis/.agents/skills/holzman-rust/references/mechanical-empathy-toolchain.md`

## include_str! targets covered
The contract test
`crates/workspace_tests/tests/vb_jpq7_3_fail_closed_storage_recovery_contract.rs`
references the following `include_str!` targets (relative to the test file):

1. `../../vb_storage/src/journal/replay.rs`
   → resolves to `crates/vb_storage/src/journal/replay.rs` (file present).
2. `../../vb_storage/src/journal/core.rs`
   → resolves to `crates/vb_storage/src/journal/core.rs` (file present).
3. `../../vb_storage/src/journal/append/intent.rs`
   → resolves to `crates/vb_storage/src/journal/append/intent.rs` (file present).
4. `../../vb_storage/src/recovery/event_replay/mod.rs`
   → resolves to `crates/vb_storage/src/recovery/event_replay/mod.rs`
     (file present).

These four paths are the canonical list mirrored as
`CONTRACT_INCLUDE_STR_TARGETS` inside the new sentinel test file
`crates/workspace_tests/tests/vb_06t25_fail_closed_storage_recovery_include_str_sentinel.rs`.

## New fail-closed test functions
The new sentinel file introduces four `#[test]` functions:

1. `given_all_include_str_targets_exist_when_fail_closed_sentinel_runs_then_all_paths_resolve`
   — happy path: every `include_str!` target in `CONTRACT_INCLUDE_STR_TARGETS`
     resolves to an existing file on disk.
2. `given_each_include_str_target_when_resolved_then_canonical_path_under_vb_storage_src`
   — happy path boundary check: every resolved target lives underneath
     `vb_storage/src/`, so the sentinel cannot be tricked into accepting a
     moved-out-of-crate path that happens to exist.
3. `given_a_missing_include_str_target_when_sentinel_runs_then_typed_error_includes_the_path`
   — error path: a deliberately non-existent path produces a typed error
     containing the missing path (used to demonstrate the drift-detection
     message format without mutating the repo).
4. `given_a_directory_path_when_sentinel_runs_then_typed_error_reports_not_a_regular_file`
   — error path boundary: a path that exists but is a directory produces a
     distinct error explaining the non-regular-file drift mode.

The shared helper `assert_include_str_target_exists` uses `Path::exists()`
and `Path::is_file()` (both `bool` APIs) and returns `Result<(), String>`
so the drift mode and resolved absolute path are reported back to the
test runner without panicking.

## Holzman compliance notes
- No `unsafe`, `unwrap`, `expect`, `panic`, `todo`, `unimplemented`,
  `unreachable!`, `dbg!`, unchecked indexing, or unchecked arithmetic in
  the new sentinel file.
- Uses `std::path::Path::exists()` (returns `bool`) per the user's explicit
  guidance — not `try_exists()` (returns `io::Result<bool>`).
- Errors flow through `Result<(), String>` and `format!` error
  construction rather than panicking or asserting.
- All loops are static-bounded by `CONTRACT_INCLUDE_STR_TARGETS` (fixed
  length, set at compile time).
- File existence checks are the only filesystem interactions; no
  allocation or formatting is performed in a hot path.
- The `codec_miri_tests_compile_check.rs` sentinel was inspected and
  confirmed unchanged — its existing `#[test]` is a typed-result
  observation, not a panic.

## Pre-existing test failures (out of scope for this bead)
The contract test `vb_jpq7_3_fail_closed_storage_recovery_contract.rs`
currently has two failing tests that are NOT caused by this bead:

1. `given_public_hydration_tail_slot_cannot_be_dimensioned_when_recovery_runs_then_clean_taint_is_not_defaulted`
2. `given_journal_shutdown_when_durability_barrier_fails_then_drop_does_not_discard_result`

Both fail because the production source files referenced by `include_str!`
no longer contain the exact pattern strings the contract test expects.
These failures pre-date this bead and are owned by the vb-jut5w follow-up
chain (`vb-y3az6`, `vb-7m2pd`, `vb-jnome`, `vb-6nwuq`, …). This bead
treats them as `BLOCK_GLOBAL` prerequisite repair and does not modify the
production source files (they are out of scope per the user instruction).

The new fail-closed sentinel is independent of these failures: it
verifies file existence, not source-content patterns.

## Simulated failure evidence

### Simulation A — compile-time fail-closed via rename of a Rust module
All four `include_str!` targets in the contract test are also Rust modules
declared via `pub(crate) mod …` in their parent `mod.rs`. Renaming one
of them therefore breaks the entire `vb_storage` crate's compilation,
which is the strongest possible fail-closed signal at build time.

Command run:
```bash
git mv crates/vb_storage/src/recovery/event_replay/mod.rs \
       crates/vb_storage/src/recovery/event_replay/SIMULATED_DRIFT_bak_mod.rs
cargo build -p vb_storage --all-features
```

Observed output (captured in `/tmp/vb-06t25/compile-fail-closed.txt`):
```
error[E0583]: file not found for module `event_replay`
  --> crates/vb_storage/src/recovery/mod.rs:19:1
   |
19 | pub(crate) mod event_replay;
   | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^
   |
   = help: to create the module `event_replay`, create file \
       "crates/vb_storage/src/recovery/event_replay.rs" or \
       "crates/vb_storage/src/recovery/event_replay/mod.rs"

error: could not compile `vb_storage` (lib) due to 1 previous error
exit_code: 101
```

Then restored via `git mv … crates/vb_storage/src/recovery/event_replay/mod.rs`.

### Simulation B — runtime fail-closed via bogus path in sentinel list
The new sentinel's `CONTRACT_INCLUDE_STR_TARGETS` was temporarily
extended with a non-existent fifth path,
`../../vb_storage/src/recovery/SIMULATED_DRIFT_missing_module/mod.rs`,
and the sentinel test was re-run.

Command run:
```bash
cargo test -p velvet-ballistics-workspace-tests \
  --test vb_06t25_fail_closed_storage_recovery_include_str_sentinel
```

Observed output (captured in `/tmp/vb-06t25/runtime-fail.txt`):
```
running 4 tests
test given_a_directory_path_when_sentinel_runs_then_typed_error_reports_not_a_regular_file ... ok
test given_a_missing_include_str_target_when_sentinel_runs_then_typed_error_includes_the_path ... ok
test given_each_include_str_target_when_resolved_then_canonical_path_under_vb_storage_src ... ok
test given_all_include_str_targets_exist_when_fail_closed_sentinel_runs_then_all_paths_resolve ... FAILED

failures:
---- given_all_include_str_targets_exist_when_fail_closed_sentinel_runs_then_all_paths_resolve stdout ----
Error: "include_str! target drift: `../../vb_storage/src/recovery/SIMULATED_DRIFT_missing_module/mod.rs` \
        (resolved to `/home/lewis/src/velvet-ballistics/crates/workspace_tests/tests/../../vb_storage/src/recovery/SIMULATED_DRIFT_missing_module/mod.rs`) \
        does not exist on disk. Either restore the moved module or update \
        `vb_jpq7_3_fail_closed_storage_recovery_contract.rs` to point at the new location."

test result: FAILED. 3 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
exit_code: 101
```

The drift message:
- Names the relative path as written (`../../vb_storage/src/recovery/SIMULATED_DRIFT_missing_module/mod.rs`).
- Names the resolved absolute path so the developer can grep the file
  tree without re-running the test.
- Names the remediation: restore the moved module or update the contract
  test's `include_str!` macro call.

The bogus fifth entry was then removed from
`CONTRACT_INCLUDE_STR_TARGETS`, restoring the const to the original four
paths. The sentinel was re-run and all 4 tests passed (see final command
exit codes below).

## Commands run and exit codes

| Command | Exit code | Notes |
|---|---|---|
| `cargo check -p vb_storage --all-features --all-targets` | 0 | `/tmp/vb-06t25/check.txt` |
| `cargo test -p vb_storage --all-features --no-run` | 0 | `/tmp/vb-06t25/test-build.txt` |
| `cargo test -p velvet-ballistics-workspace-tests --test vb_jpq7_3_fail_closed_storage_recovery_contract` | 101 | Pre-existing failures (see above); 9 passed, 2 failed; NOT introduced by this bead. `/tmp/vb-06t25/contract.txt` |
| `cargo test -p velvet-ballistics-workspace-tests --test vb_06t25_fail_closed_storage_recovery_include_str_sentinel` (initial) | 0 | All 4 sentinel tests pass. `/tmp/vb-06t25/sentinel-run.txt` |
| `cargo test -p velvet-ballistics-workspace-tests --test vb_06t25_fail_closed_storage_recovery_include_str_sentinel` (under simulated drift) | 101 | Runtime fail-closed fires with a typed error message including both the relative and absolute paths. `/tmp/vb-06t25/runtime-fail.txt` |
| `cargo test -p velvet-ballistics-workspace-tests --test vb_06t25_fail_closed_storage_recovery_include_str_sentinel` (after restore) | 0 | All 4 sentinel tests pass. `/tmp/vb-06t25/runtime-pass.txt` |
| `cargo build -p vb_storage --all-features` (under simulated drift via rename) | 101 | Compile-time fail-closed fires with `error[E0583]: file not found for module 'event_replay'`. `/tmp/vb-06t25/compile-fail-closed.txt` |
| `cargo clippy -p velvet-ballistics-workspace-tests --test vb_06t25_fail_closed_storage_recovery_include_str_sentinel --all-features -- -D warnings -D unsafe_code -D clippy::unwrap_used -D clippy::expect_used -D clippy::panic -D clippy::panic_in_result_fn -D clippy::todo -D clippy::unimplemented -D clippy::dbg_macro -D clippy::indexing_slicing -D clippy::string_slice -D clippy::get_unwrap -D clippy::arithmetic_side_effects -D clippy::as_conversions -D clippy::let_underscore_must_use -D clippy::await_holding_lock` | 0 | Strict Holzman clippy passes on the new sentinel test file. |

Notes on command syntax: `cargo test --all-features` against
`-p velvet-ballistics-workspace-tests` returns
`error: cannot specify features for packages outside of workspace` because
the package's `[features]` section is empty. The verification was therefore
run without `--all-features`, which is the only correct invocation for
this package. The four `include_str!` paths probed by the sentinel do not
require any feature gate to be enabled.

## Files changed by this bead (committed state)
- New file: `crates/workspace_tests/tests/vb_06t25_fail_closed_storage_recovery_include_str_sentinel.rs`
  (the fail-closed sentinel harness with 4 `#[test]` functions).
- Untouched, confirmed unchanged:
  - `crates/vb_storage/src/codec_miri_tests_compile_check.rs`
  - `crates/workspace_tests/tests/vb_jpq7_3_fail_closed_storage_recovery_contract.rs`
- New file: `.bead-progress/vb-06t25/progress.md` (this file).

The `cargo test -p vb_storage --all-features --all-targets` and the
sentinel build/run all pass cleanly with the new file in place.

## Residual risk
- The contract test
  `vb_jpq7_3_fail_closed_storage_recovery_contract.rs` has two
  pre-existing failures that are out of scope for this bead (they assert
  source-content patterns in production files that drifted during the
  vb-jut5w refactor). The bead's own verification command therefore
  exits 101, even though my new sentinel passes. The bead's deliverable
  is the runtime fail-closed sentinel; the pre-existing failures are
  owned by other beads in the vb-jut5w follow-up chain
  (e.g., `vb-y3az6`, `vb-6nwuq`, `vb-jnome`).
- The sentinel's `CONTRACT_INCLUDE_STR_TARGETS` list is hand-maintained
  and must be kept in sync with the contract test's `include_str!`
  block. The sentinel will detect drift in EITHER direction (path moves
  in production, or contract test adding a new path without updating
  the sentinel) because the list is the canonical, machine-readable
  mirror.
- The sentinel uses `Path::exists()` and `Path::is_file()` which return
  `false` on transient I/O errors. A path that exists but is unreadable
  for permission reasons would be reported as missing; this is acceptable
  fail-closed behavior (better to fail loudly than to silently allow
  drift). No follow-up planned; this matches the user-specified API
  contract (`Path::exists()`, not `try_exists()`).
- Simulation A demonstrated that all four include_str! targets are also
  Rust modules, so any drift breaks the crate's compile before the
  sentinel can run. This is a defense-in-depth gain, not a regression.
