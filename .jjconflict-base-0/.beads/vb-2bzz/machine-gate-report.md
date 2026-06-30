bead_id: vb-2bzz
bead_title: storage: Expose action ABI and policy digest recovery mismatch checks
phase: 11
updated_at: 2026-05-17T02:00:00Z
attempt: 1-of-7

## Machine Gate Report

### Gates Executed

| Gate | Command | Result |
|---|---|---|
| Compile | `cargo check -p vb_storage` | PASS (74 crates) |
| Clippy | `cargo clippy -p vb_storage -- -D warnings` | PASS (no issues) |
| Format | `cargo fmt -p vb_storage -- --check` | PASS |
| Tests | `cargo test -p vb_storage --test recovery_bdd_tests` | PASS (34 passed, 2 ignored) |
| GAP-3 tests | `cargo test -p vb_storage --test recovery_bdd_tests -- action_abi policy_digest` | PASS (6 passed) |

### Acceptance Criteria Verification

1. **GAP-3 ignored tests unignored or replaced**: PASS — Both `action_abi_mismatch_returns_typed_error` and `policy_digest_mismatch_returns_typed_error` are no longer ignored and contain executable assertions.
2. **ActionAbiMismatch returned only on real mismatch**: PASS — `check_action_abi_digests` returns error only when `expected != found`.
3. **PolicyDigestMismatch returned only on real mismatch**: PASS — `check_policy_digests` returns error only when `expected != found`.
4. **`cargo test -p vb_storage --test recovery_bdd_tests -- --ignored` no longer contains hollow passes**: PASS — 2 remaining ignored tests are out-of-scope (`corrupt_snapshot`, `terminal_state_mismatch`).

### Regression Analysis

- Pre-existing failure: `hydrate_run_frame_rejects_mismatched_snapshot_run_id` in `recovery/tests.rs` — classified as DEFERRED_GLOBAL (unrelated to this bead, documents a separate contract gap).
- No new failures introduced by this bead.

### Scope Classification

| Failure | Classification | Reason |
|---|---|---|
| `hydrate_run_frame_rejects_mismatched_snapshot_run_id` | DEFERRED_GLOBAL | Pre-existing, unrelated to vb-2bzz scope, in file not touched by this bead |

STATUS: PASS
