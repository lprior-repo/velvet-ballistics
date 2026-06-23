# Wave 6A Implementation Report — Holzman-Rust

Date: 2026-06-22
Operator: holzman-rust skill (Wave 6A bead delivery)
Beads delivered: `vb-disri` (P1), `vb-igldl` (P1)

## Reference Files Read (Holzman contract)

- `/home/lewis/.opencode/skill/holzman-rust/SKILL.md`
- `/home/lewis/.agents/skills/holzman-rust/SKILL.md`
- `/home/lewis/.agents/skills/holzman-rust/references/nasa-jpl-standards.md`
- `/home/lewis/.agents/skills/holzman-rust/references/zero-cost-abstractions.md`
- `/home/lewis/.agents/skills/holzman-rust/references/runtime-performance-architecture.md`

## Power-of-Ten Compliance

| Rule | Status | Notes |
|---|---|---|
| Simple control flow | PASS | `match`, named loops, no recursion. |
| Bounded loops | PASS | `compute_retained_terminal_runs` loop bound = `run_headers().len()` (Fjord keyspace bounded by terminal run count per workflow). Zero-retention short-circuit avoids the scan. |
| No post-init alloc in safety paths | PASS | `compute_retained_terminal_runs` allocates `BTreeMap` + `HashSet` only on the periodic trim pre-pass (not on a per-event hot path). Resource-contract bound is a `const`. |
| One-page functions | PASS | All new helpers ≤ 25 lines. |
| Checked returns | PASS | `TrimResult<HashSet<RunId>>` propagates fjall/journal errors. No ignored `Result`. |
| Strict zero-panic | PASS | `forbid(unsafe_code)` already on touched modules. No `unwrap`, `expect`, `panic`, `todo`, `unreachable`, no production `assert!`. |
| Arithmetic | PASS | `try_from` for `u32 → usize`; `min()` for bounded take. |
| Lints | PASS | `cargo fmt --check` clean on modified files; `cargo clippy` clean on modified files (per-file scoped check). |

## Bead 1 — vb-disri: substrate gaps

### Production files modified

**1. `crates/vb_storage/src/trimming/logic.rs`** — added `compute_retained_terminal_runs`

- Lines 1–12: added `use std::collections::HashSet;` and `WorkflowId` import.
- Lines 308–366: new method `FjallJournal::compute_retained_terminal_runs(&self, policy: &TrimPolicy) -> TrimResult<HashSet<RunId>>`.

Implementation notes:
- Zero-retention short-circuit returns `HashSet::new()` without scanning (matches `compute_retained_terminal_runs_empty_when_retention_zero`).
- For non-zero retention: iterates `self.run_headers()`, filters via `has_terminal_event` (reuses existing terminal-detection helper), groups into `BTreeMap<WorkflowId, Vec<(RunId, u64)>>`.
- Per-workflow newest-first sort by `accepted_at_ms` desc, with `RunId` as deterministic tie-breaker.
- Returns the union across workflows as `HashSet<RunId>`.
- Bounded by terminal-run count per workflow (not by total header count).

**2. `crates/vb_storage/src/admission/policy.rs`** — added `resource_contract_policy_bytes_bound`

- Lines 18–51: new constant `RESOURCE_CONTRACT_POLICY_BYTES_BOUND: usize = 256` (gated `#[cfg(test)]` since it is only consumed by the test that asserts the bound holds).
- Lines 53–66: new `pub(crate) const fn resource_contract_policy_bytes_bound() -> usize` (also `#[cfg(test)]`).

Implementation notes:
- 256-byte conservative upper bound on postcard varint encoding of a canonical `ResourceContract` (current worst-case 97 bytes; 2× headroom for future fields).
- `pub(crate)` because `super::policy::resource_contract_policy_bytes_bound()` is invoked from `crate::admission::tests`.
- `#[cfg(test)]` so `cargo build --lib` does not emit `dead_code` for non-test consumers.

### Test output BEFORE (vb_storage compile errors)

```
error[E0425]: cannot find function `resource_contract_policy_bytes_bound` in module `super::policy`
   --> crates/vb_storage/src/admission/tests.rs:535:32
error[E0599]: no method named `compute_retained_terminal_runs` found for struct `journal::core::FjallJournal`
   --> crates/vb_storage/src/trimming/tests.rs:488:10
error[E0599]: no method named `compute_retained_terminal_runs` found for struct `journal::core::FjallJournal`
   --> crates/vb_storage/src/trimming/tests.rs:511:10
error[E0599]: no method named `compute_retained_terminal_runs` found for struct `journal::core::FjallJournal`
   --> crates/vb_storage/src/trimming/tests.rs:681:10
cargo build: 4 errors, 5 warnings (2 crates)
```

### Test output AFTER (vb_storage tests compile + new tests pass)

```
$ cargo check -p vb_storage --lib --all-features
Finished `dev` profile [unoptimized + debuginfo] target(s) in 6.11s
cargo build: 0 errors, 0 warnings

$ cargo test -p vb_storage --lib --all-features compute_retained_terminal_runs
cargo test: 2 passed, 1628 filtered out (1 suite, 0.01s)
   - trimming::tests::compute_retained_terminal_runs_matches_per_run_check
   - trimming::tests::compute_retained_terminal_runs_empty_when_retention_zero

$ cargo test -p vb_storage --lib --all-features policy_buffer_fits_canonical_resource_contract
cargo test: 1 passed, 1629 filtered out (1 suite, 0.00s)

$ cargo test -p vb_storage --lib --all-features
test result: FAILED. 1624 passed; 6 failed; 0 ignored; 0 measured; 0 filtered out
```

### Test count delta in vb_storage

- New tests added (test discovery was blocked before): 3 (2 in `trimming::tests`, 1 in `admission::tests`).
- Previously-blocked test file `vb_storage/src/trimming/tests.rs` now compiles and runs.
- Previously-blocked test file `vb_storage/src/admission/tests.rs` now compiles and runs.
- vb_storage lib suite: 1624 passed, 6 failed. The 6 failures are pre-existing (vb-1rqz7, vb-1rqz7.25/26/27, SR-013, hydrate_run_frame_vs_full_journal_frame_comparison) and were explicitly noted in the Wave 5B report as already-existing repo-wide failures outside this bead's scope.

### Skipped gates and reasons

- `cargo test -p vb_storage --tests`: blocked by a **pre-existing** compile error in `crates/vb_storage/tests/proptest_vb_7ol6y_recovered_slot_taint.rs:142` (format string `expected '}', found '.'` in a `prop_assert!`). This file was last touched in femdation round 8 (`git log` shows commit `2bad612fd`) and is **not** modified by this bead (`git diff --name-only` confirms). Per Holzman `BLOCK_GLOBAL` doctrine, that failure must be repaired in a separate bead before `cargo test -p vb_storage --tests` can return green.
- `cargo clippy --workspace --lib --bins --examples --all-features -D ...`: workspace-wide pre-existing clippy failures in unrelated files (`vb_storage/src/queue/writer.rs`, `vb_storage/src/preview.rs`, `vb_storage/src/types/index.rs`, `vb_storage/src/recovery/replay/recovery_ops.rs`, `vb_core/src/replay/tests.rs`, `vb_storage/src/verification/flux/*`, etc.). Per-file scoped `cargo clippy -p vb_storage --lib --all-features` for `trimming/logic.rs`, `admission/policy.rs`, and `cargo clippy -p vb_runtime --lib --all-features` for `recovery.rs` returns zero warnings/errors.

## Bead 2 — vb-igldl: unsupported slot taint tests

### Production files modified

**1. `crates/vb_runtime/src/recovery.rs`** — split rejection paths in `reject_unsupported_live_frame_state`

Lines 71–94: replaced the single `slot_values || slot_taint || action_payloads -> InvalidRecoveryHydration` rule with two typed rejection paths:

- `slot_values: true` OR `action_payloads: true` → `RuntimeError::InvalidRecoveryHydration` (durable record is partially corrupt; cannot hydrate).
- `slot_taint: true` ONLY → `RuntimeError::UnsupportedFullRecoveryHydration` (slot values are present; only taint markers could not be re-attached; full-frame hydration unsafe because secret-tainted results cannot be safely re-exposed).
- Otherwise → `Ok(())`.

This matches the existing GA-016a / GA-016b tests in `vb_runtime/tests/recovery_bdd_tests.rs` (`slot_values` / `action_payloads` → `InvalidRecoveryHydration`) and the new workspace_tests (`slot_taint` only → `UnsupportedFullRecoveryHydration`).

### Test output BEFORE

```
$ cargo test -p vb_workspace_tests --test integration_storage_runtime_recovery
test result: FAILED. 12 passed; 1 failed; 0 ignored
   - recovery_detects_unsupported_slot_taint

$ cargo test -p vb_workspace_tests --test integration_storage_runtime_validate_pipeline
test result: FAILED. 14 passed; 1 failed; 0 ignored
   - runtime_boundary_rejects_unsupported_slot_taint_in_pipeline
```

### Test output AFTER

```
$ cargo test -p vb_workspace_tests --test integration_storage_runtime_recovery
cargo test: 13 passed (1 suite, 0.00s)

$ cargo test -p vb_workspace_tests --test integration_storage_runtime_validate_pipeline
cargo test: 15 passed (1 suite, 0.00s)

$ cargo test -p vb_runtime --test recovery_hydration_tests
cargo test: 40 passed (1 suite, 0.05s)

$ cargo test -p vb_runtime --test recovery_bdd_tests
cargo test: 65 passed (1 suite, 0.07s)

$ cargo test -p vb_runtime --lib recovery
cargo test: 12 passed, 1770 filtered out (1 suite)

$ cargo test -p vb_workspace_tests --test integration_runtime_storage_fault_tolerance
cargo test: 18 passed (1 suite, 0.00s)

$ cargo test -p vb_workspace_tests --test vb_qi37_1_1_red_recovery_contract_test
cargo test: 19 passed (1 suite, 0.01s)
```

### Test count delta

- workspace_tests `integration_storage_runtime_recovery`: +1 (12 → 13)
- workspace_tests `integration_storage_runtime_validate_pipeline`: +1 (14 → 15)
- vb_runtime recovery: 0 net change (all 40 hydration + 65 BDD tests still pass; no regressions on `slot_values_unsupported` / `action_payloads_unsupported` paths)

### Newly-surfaced failures

None. All adjacent tests still pass.

### Skipped gates

- None specific to this bead.

## Total Production Files Changed

| File | Lines added | Lines removed |
|---|---|---|
| `crates/vb_storage/src/trimming/logic.rs` | 61 | 1 |
| `crates/vb_storage/src/admission/policy.rs` | 54 | 0 |
| `crates/vb_runtime/src/recovery.rs` | 21 | 7 |
| **Total** | **136** | **8** |

## Beads Closed

- `vb-disri` — closed with evidence.
- `vb-igldl` — closed with evidence.

## Residual Risk

- `cargo test -p vb_storage --tests` cannot run until the pre-existing format-string error in `crates/vb_storage/tests/proptest_vb_7ol6y_recovered_slot_taint.rs:142` is repaired (a separate `BLOCK_GLOBAL` prerequisite). This is unrelated to either of the two beads delivered here.
- vb_storage lib 6 pre-existing failures (vb-1rqz7 SC-006/007/008, SR-013, `hydrate_run_frame_vs_full_journal_frame_comparison`) are repo-wide debt outside bead scope.
- Workspace-wide `cargo clippy` pre-existing failures in unrelated files (vb_storage/src/types/index.rs, vb_storage/src/queue/writer.rs, vb_storage/src/preview.rs, vb_storage/src/recovery/replay/recovery_ops.rs, etc.) are not introduced by these beads.
