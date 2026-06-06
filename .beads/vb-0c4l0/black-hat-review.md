**Bead**: vb-0c4l0
**State**: 13 (black-hat-reviewer)
**Reviewer**: black-hat-reviewer
**Source checkout**: /home/lewis/src/velvet-ballistics
**Branch**: process/vb-63st6.2-worktree-loom-route
**Commit under review**: 39f685822
**Attempt**: 1

---

# Black Hat Reviewer Response

## Gate Result
**STATUS: REJECTED**

---

## PHASE 1: Contract & Bead Parity

| Requirement | Status | Evidence |
|-------------|--------|----------|
| Bead description exists | ❌ | `bd show vb-0c4l0` → `DESCRIPTION (none)`, `comment_count: 0` |
| Original failure cited | ❌ | No CI log, no commit-referenced test output, no worktree path with PID/repro |
| Fix touches only test files | ✅ | `git show 39f685822 --stat` → only `impl_tests.rs` + `ipc_magic_gate.rs` (test files) |
| Path lengths under sun_path=108 | ✅ | All 6 changed paths are 26–37 chars (worst case 28 chars with 7-digit PID) |
| All vb_ipc tests pass | ✅ | `cargo test --package vb_ipc` → 631 passed (6 suites, 0.22s) |
| Target test target compiles and runs | ❌ | 4 of 6 changed paths are in a file that is **NEVER compiled** (see Finding-01) |

---

## PHASE 2: Farley Engineering Rigor

| Function | Lines | Limit | Status |
|----------|-------|-------|--------|
| `bind_to_nested_directory_fails` | 11 | 25 | ✅ |
| `bind_fails_when_path_is_existing_directory` | 18 | 25 | ✅ |

No function grew. No parameters added.

---

## PHASE 3: Holzman Rust (The Big 6)

| Rule | Status |
|------|--------|
| Zero `unsafe` | ✅ (no `unsafe` introduced; `#![forbid(unsafe_code)]` already on file) |
| Zero `.unwrap()`/`.expect()` | ✅ (no new `unwrap`/`expect` introduced by diff) |
| Zero `panic!`/`todo!`/`dbg!` | ✅ |
| Checked arithmetic | ✅ (no arithmetic in diff) |
| Unwrapped primitives in domain | n/a (test code) |
| Parse, Don't Validate | n/a |

---

## PHASE 4: Ruthless Simplicity & DDD

| Check | Status |
|-------|--------|
| No Option-based state machines | ✅ |
| CUPID compliant | ⚠️ Inconsistent: fix uses hardcoded `/tmp/...` paths instead of the existing `temp_socket_path(name)` helper (which uses `pid + counter`). Same pattern in 3 sibling test files (`vb_ipc/src/server/dispatch_tests.rs:22`, `vb_ipc/src/client/tests.rs:209`, `workspace_tests/tests/vb_te1i_binary_ipc_acceptance.rs:36`). The fix diverges from established convention without justification. |
| No clever abstractions | ✅ |
| Cleanup of stale files | ⚠️ `bind_to_nested_directory_fails` path has no `CleanupPath` guard. Not catastrophic (test expects failure), but the sibling `bind_fails_when_path_is_existing_directory` correctly uses `CleanupDir`. |

---

## PHASE 5: The Bitter Truth

The commit is a textbook example of a "fix" that *looks* correct and *passes tests* without *fixing anything real*. The headline claim "All 631 vb_ipc tests pass" is technically true, but **631 is the count of tests in `vb_ipc`, which does not include the 4 supposedly-fixed paths in `ipc_magic_gate.rs`** — those tests are not in any registered test target and are not compiled or run by `cargo test` at all. The 2 changes in `impl_tests.rs` are in real, executed tests, but the bead's failure scenario cannot be reproduced with the worktree path mentioned in the commit message.

---

## Findings (Ordered by Severity)

| Finding | Severity | File:Line | Status |
|---------|----------|-----------|--------|
| FINDING-01: 4 of 6 changed paths are in dead code | CRITICAL | `crates/workspace_tests/ipc_magic_gate.rs:63, 106, 152, 213` | open |
| FINDING-02: Bead has no description, no original failure citation | HIGH | `vb-0c4l0` bead | open |
| FINDING-03: Claimed failure is not reproducible with stated worktree path | HIGH | commit 39f685822 message | open |
| FINDING-04: Fix diverges from established `temp_socket_path(name)` pattern | MEDIUM | `impl_tests.rs:129, 1111` | open |
| FINDING-05: `bind_to_nested_directory_fails` lacks cleanup guard | LOW | `impl_tests.rs:129` | open |
| FINDING-06: `/tmp/...` paths are not pid-qualified for the nested test | LOW | `impl_tests.rs:129` | open |

### [FINDING-01]: 4 of 6 changed paths are in code that is never compiled

**Location**: `crates/workspace_tests/ipc_magic_gate.rs:63, 106, 152, 213`

**Problem**: The file `crates/workspace_tests/ipc_magic_gate.rs` exists at the crate root but is **NOT registered as a `[[test]]` target** in `crates/workspace_tests/Cargo.toml` and is **NOT included via `mod`** in `src/lib.rs`. Therefore the file is never compiled, never linked, never executed. The 4 changed paths in this file have zero functional impact.

**Evidence**:
- `crates/workspace_tests/Cargo.toml` has 41 `[[test]]` entries (lines 53-267); none references `ipc_magic_gate`.
- `crates/workspace_tests/src/lib.rs` declares only `acceptance_catalog`, `bdd_runner`, `boundary_inventory`, `quality`.
- `cargo metadata --no-deps` enumerates all test targets; `ipc_magic_gate` is absent.
- `cargo test --package velvet-ballistics-workspace-tests --test ipc_magic_gate` → `error: no test target named 'ipc_magic_gate'`.
- `cargo test --package velvet-ballistics-workspace-tests` stdout contains zero references to `ipc_magic`, `invalid_magic`, `frame_error`, `buffer_cap`, or `chunk_bound` test names.
- `rtk cargo test --package velvet-ballistics-workspace-tests -- --list` produces 0 lines — the test list is suppressed because the harness lists targets per-binary; no binary exists for `ipc_magic_gate`.

The commit's "All 631 vb_ipc tests pass" is true but misleading: those 631 tests live in `crates/vb_ipc`, which does not include `ipc_magic_gate.rs`.

**Required Fix**: Either (a) register `ipc_magic_gate.rs` as a real `[[test]]` target in `crates/workspace_tests/Cargo.toml` so the fix is actually exercised, or (b) drop the 4 changes from the commit since they are dead-code edits with no behavioral effect.

### [FINDING-02]: Bead has no description and no evidence of original failure

**Location**: `vb-0c4l0` bead record

**Problem**: `bd show vb-0c4l0 --json` reports `"description": null`, `"comment_count": 0`, and `dependency_type: "discovered-from"` for the two parent beads. The bead title is the only context. The commit message claims "Worst-case path length after fix: 37 chars (was 113 in a long worktree)" but cites no specific worktree path, PID, or test output.

**Evidence**: `bd show vb-0c4l0 --json` output above; commit message text.

**Required Fix**: Either populate the bead with a real reproduction (worktree path + failing test name + error text) or down-prioritize and merge with FINDING-01's option (b).

### [FINDING-03]: Claimed failure is not reproducible with the path in the commit message

**Location**: commit 39f685822 body, claim `was 113 in a long worktree`

**Problem**: The commit message cites `/home/.../worktrees/<branch>/target/tmp/` as the example. The most plausible concrete path on this branch is `/home/lewis/src/velvet-ballistics/worktrees/vb-63st6.2-worktree-loom-route/target/tmp` (80 chars). With this prefix:
- `bind_fails_when_path_is_existing_directory` (PID-suffixed, max 7-digit PID): 80 + 1 + 17 + 7 = **105 chars** (under 108). The test passes pre-fix and post-fix.
- `bind_to_nested_directory_fails` (nested `dir/sock`): 80 + 1 + 30 = **111 chars** (over 108). But the test only asserts `Err(_)`, so it passes regardless of error type.
- The 4 paths in `ipc_magic_gate.rs` are dead code; cannot fail.

The fix is **semantically correct** (all paths now safely under 108) but **does not address a reproducible failure** with the example path. The author's claim of "was 113" requires a 30+ char path component beyond the example, which is not documented.

**Evidence**:
- Computed path lengths shown above.
- `TMPDIR=/home/lewis/src/velvet-ballistics/worktrees/vb-63st6.2-worktree-loom-route/target/tmp cargo test --package vb_ipc bind_fails_when_path_is_existing_directory` → `1 passed, 630 filtered out`.
- Same for `bind_to_nested_directory_fails` → passes.

**Required Fix**: Either (a) document the actual failing worktree + test pair with raw output, or (b) admit the fix is defensive (preventing future breakage with longer worktree paths) rather than reactive.

### [FINDING-04]: Fix diverges from established `temp_socket_path(name)` pattern

**Location**: `crates/vb_ipc/src/server/impl_tests.rs:129, 1111`

**Problem**: The crate already has a `temp_socket_path(_name)` helper at `impl_tests.rs:37` that uses `format!("/tmp/vbi{}_{}.sock", std::process::id(), sequence)` with an `AtomicUsize` counter. Three sibling test files use the same pattern (`dispatch_tests.rs:22`, `client/tests.rs:209`, `workspace_tests/tests/vb_te1i_binary_ipc_acceptance.rs:36`). The fix bypasses this helper and hardcodes specific paths. This is an inconsistency: the fixed tests use bespoke strings while their siblings use the dynamic helper.

**Evidence**:
- `impl_tests.rs:37-40` defines `temp_socket_path` with PID+counter.
- Sibling files use the same `format!("/tmp/vb...{}_{}.sock", pid, seq)` pattern.
- The two changed sites at lines 129 and 1111 use literal `PathBuf::from(...)` and `PathBuf::from(format!("/tmp/vb_ipc_dir_test_{}", std::process::id()))` — different from siblings.

**Required Fix**: Use the existing `temp_socket_path(name)` helper for both sites. This is a one-line refactor that produces consistent, pid-qualified, counter-suffixed paths and removes the hand-rolled string construction.

### [FINDING-05]: `bind_to_nested_directory_fails` lacks cleanup guard

**Location**: `crates/vb_ipc/src/server/impl_tests.rs:129`

**Problem**: The hardcoded path `/tmp/vb_ipc_nonexistent_dir_test/sock` has no `CleanupPath` guard. If a previous test run (or a parallel cargo test binary) leaves a stale socket file, the test still passes (it only checks `Err(_)`), but the leftover file accumulates in `/tmp/`. Sibling tests at the same site use the `CleanupPath` RAII guard.

**Evidence**:
- `impl_tests.rs:809-817` defines `CleanupPath` Drop.
- `impl_tests.rs:1937-1945` defines `CleanupDir` Drop.
- The changed test at line 129 instantiates neither.

**Required Fix**: Wrap the path in `_cleanup = CleanupPath(&path);` for parity with sibling tests. Low severity because the test still passes, but it is the established pattern.

### [FINDING-06]: Nested test path is not PID-qualified

**Location**: `crates/vb_ipc/src/server/impl_tests.rs:129`

**Problem**: `/tmp/vb_ipc_nonexistent_dir_test/sock` is a fully hardcoded path with no PID or counter. If `cargo test` ever runs two test binaries that both include this test (impossible today, but brittle), they would race on the same socket path. The sibling path on line 1111 uses `std::process::id()`; the one on line 129 does not.

**Evidence**: line 129 vs. line 1111.

**Required Fix**: Use `temp_socket_path("nonexistent_dir_test")` (which includes PID+counter).

---

## Quality Gates

| Gate | Result | Evidence |
|------|--------|----------|
| `cargo test -p vb_ipc` | ✅ | 631 passed (6 suites, 0.22s) |
| `cargo test -p velvet-ballistics-workspace-tests` | ⚠️ | 1 unrelated FAILED (`vb_8ma2_workspace_assertions::valid_workspace_passes_sharpened_assertions` — `Cargo.toml: workspace.exclude missing ["crates/vb_ajc40_flux"]` and `vb_core/Cargo.toml: features missing [...]`). Pre-existing, not caused by this commit. |
| `cargo clippy -p vb_ipc --tests --all-features` | ⚠️ | 559 pre-existing errors (e.g. `used unwrap`, `indexing may panic`, `as` conversion). None on changed lines (126, 1109-1111). |
| `cargo fmt --check` | not run | n/a (no production formatting expected to change) |

---

## Verdict

**STATUS: REJECTED**

### Summary

Two of the six changed paths (`impl_tests.rs:129, 1111`) are technically correct and in real, executed tests. The remaining four changes (`ipc_magic_gate.rs:63, 106, 152, 213`) are **dead-code edits** in a file that is not registered as a test target and is never compiled. The commit's headline "All 631 vb_ipc tests pass" is technically true but misleading — those 631 tests are unrelated to the 4 dead-code changes. The bead has no description, no original failure citation, and the example worktree path in the commit message does not reproduce a failure. The fix is defensive (preventing future breakage with longer worktree paths) but is presented as a reactive fix. The fix also diverges from the established `temp_socket_path(name)` helper pattern used in three sibling test files. The bulk of the commit is wasted work, and the small portion that is real is incomplete.

---

## Required Repair Actions

1. **[CRITICAL] FINDING-01**: Either register `crates/workspace_tests/ipc_magic_gate.rs` as a real `[[test]]` target in `crates/workspace_tests/Cargo.toml` (so the 4 changes actually run), OR drop those 4 lines from the commit. Current state is wasted work presented as a verified fix.
2. **[HIGH] FINDING-02 + FINDING-03**: Populate the bead with the actual failing worktree path, PID, test name, and raw error output. If no concrete failure exists, reclassify the bead as a defensive hardening task and re-target the title/description.
3. **[MEDIUM] FINDING-04**: Replace the two hardcoded `PathBuf::from(...)` constructions in `impl_tests.rs:129, 1111` with calls to the existing `temp_socket_path(name)` helper, restoring consistency with the three sibling test files.
4. **[LOW] FINDING-05**: Add `let _cleanup = CleanupPath(&path);` to `bind_to_nested_directory_fails` for parity with the `CleanupDir` used in `bind_fails_when_path_is_existing_directory`.
5. **[LOW] FINDING-06**: Resolved by FINDING-04 (the helper provides PID+counter automatically).
