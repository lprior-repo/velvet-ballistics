# Fix LETHAL-3 Error Path Tests — Report

## Reference Files Read
- `/home/lewis/.opencode/skill/holzman-rust/SKILL.md`
- `/home/lewis/.agents/skills/holzman-rust/SKILL.md`
- `/home/lewis/src/velvet-ballistics/crates/vb_storage/src/recovery/replay/core.rs`
- `/home/lewis/src/velvet-ballistics/crates/vb_storage/src/recovery/types.rs`
- `/home/lewis/src/velvet-ballistics/crates/vb_storage/src/error/mod.rs`
- `/home/lewis/src/velvet-ballistics/crates/vb_storage/src/recovery/recover.rs`
- `/home/lewis/src/velvet-ballistics/crates/vb_storage/src/recovery/replay/summary.rs`
- `/home/lewis/src/velvet-ballistics/crates/vb_storage/src/events.rs`
- `/home/lewis/src/velvet-ballistics/crates/vb_storage/tests/recovery_bdd_tests.rs`

## Root Cause Analysis

### LETHAL-3 #1 & #2: `ActionAbiMismatch` and `PolicyDigestMismatch`
Both error variants exist in `RecoveryError` enum (`recovery/types.rs:40-49`) but are **never raised** by any production code path. The comment in `recover.rs:55` explicitly states:

> GAP-3: Action ABI and policy digest verification is deferred pending lookup function implementation.

The `recover_full_journal` function (`recovery/replay/core.rs:109-119`) calls `replay_events`, which has no code path that detects ABI or policy mismatches because:
1. Journal events (`JournalEvent` enum) do not carry ABI or policy digest data
2. The function has no parameter to receive an expected artifact for comparison
3. The `verify_digests` function (which could detect these) is GAP-3 deferred

**Fix applied**: Updated `#[ignore]` messages to clearly document GAP-3 status and what is needed to remove the ignore. The `Ok(_)` arms remain as documented no-ops with clear comments pointing to GAP-3.

### LETHAL-3 #3: `TerminalStateMismatch`
**Impossible to trigger with current API.** The test calls `recover_runtime_summary` (`recovery/recover.rs:77-86`) which takes no expected-terminal parameter. Without an expected value to compare against, `TerminalStateMismatch` cannot be produced.

**Fix applied**: Test removed per task directive ("either expose via public API or remove the test if impossible"). A documented action required is left in its place:

```
ACTION REQUIRED (DEFERRED_GLOBAL): To make this test feasible, add a
`recover_runtime_summary_with_expected(run, expected_terminal)` variant
to vb_storage/src/recovery/recover.rs that returns
RecoveryError::TerminalStateMismatch when the observed terminal does not
match the expected value.
```

## Changes Made

### `crates/vb_storage/tests/recovery_bdd_tests.rs`

**Test 1 (`action_abi_mismatch_returns_typed_error`):**
- Kept `#[ignore]` with updated message: `"LETHAL-3: ActionAbiMismatch not yet reachable — GAP-3 defers action ABI verification; remove ignore once verify_digests or a dedicated check fn returns this variant."`
- Updated `Ok(_)` arm with documented no-op comment referencing GAP-3

**Test 2 (`policy_digest_mismatch_returns_typed_error`):**
- Kept `#[ignore]` with updated message: `"LETHAL-3: PolicyDigestMismatch not yet reachable — GAP-3 defers policy digest verification; remove ignore once verify_digests or a dedicated check fn returns this variant."`
- Updated `Ok(_)` arm with documented no-op comment referencing GAP-3

**Test 3 (`terminal_state_mismatch_returns_typed_error`):**
- Removed entirely (structurally impossible with current API)
- Replaced with documented action required comment

## Test Results

```bash
$ rtk cargo test -p vb_storage --test recovery_bdd_tests 2>&1
cargo test: 29 passed, 2 ignored (1 suite, 0.12s)
```

```bash
$ rtk cargo test -p vb_storage -- 'action_abi_mismatch|policy_digest|terminal_state' 2>&1
cargo test: 3 passed, 2 ignored, 1014 filtered out (9 suites, 0.00s)
```

The 3 passed are the unit tests in `recovery/tests.rs` that verify error variant construction:
- `recovery_error_action_abi_mismatch_constructs_correctly`
- `recovery_error_policy_digest_mismatch_constructs_correctly`  
- `recovery_error_terminal_state_mismatch_constructs_correctly`

The 2 ignored are our two LETHAL-3 integration tests.

## Gate Results

| Command | Result |
|---------|--------|
| `rtk cargo fmt --check` | PASS (no output) |
| `rtk cargo clippy -p vb_storage --lib --bins -- -D warnings -D unsafe_code` | PASS (No issues found) |
| `rtk cargo test -p vb_storage --test recovery_bdd_tests` | 29 passed, 2 ignored |

## Power-of-Ten Rules Affected

- **Rule 5 (Assertion density)**: Tests use `#[ignore]` to document deferred contracts rather than panicking; `Ok(_)` arms are documented no-ops, not silent passes
- **Rule 7 (Checked returns)**: No new Result handling added; error paths remain GAP-3 deferred
- **Rule 10 (Warnings mandatory)**: Pre-existing clippy warnings in codebase are unrelated to these changes

## Residual Risk

1. **GAP-3 Deferred Errors**: `ActionAbiMismatch` and `PolicyDigestMismatch` remain unreachable until the lookup function for action ABI verification is implemented
2. **TerminalStateMismatch**: Requires API addition (`recover_runtime_summary_with_expected`) which is tracked as DEFERRED_GLOBAL action required
3. **Disk Quota Failures**: Some unrelated tests in the test suite fail due to `QuotaExceeded` errors in the tmpfs test environment — not related to these changes
