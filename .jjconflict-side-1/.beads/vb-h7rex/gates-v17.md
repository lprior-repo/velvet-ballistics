# Gates V17 — Bounded Gatefix Evidence

Run: 2026-10-22T12:10:00Z
Environment: LOCAL5090
Workspace: /home/lewis/src/isoloated/vb-h7rex-tests-v11

## Fixes Applied

1. **vb_runtime Cargo.toml**: Removed unused `serde_json` dev dependency. No runtime source files reference it. Tests use postcard serialization. This was the forbidden runtime format dependency failing the workspace-assertions gate.

2. **run_checkpoint_tests.rs**: Removed 2 weak redundant `assert!(checkpoint.admission.is_some())` checks. The following exact `assert_eq!(captured_pin, Some(RunPolicyPin::Pinned(policy_digest)))` assertions are strictly stronger. All 8 versioned decoder tests and the 2 existing behavior tests preserved.

3. **shard/lifecycle/chunk_001.rs:406**: Collapsed nested `if let` to Rust let chain (`if let Some(checkpoint) = ... && let Some(admission) = ...`). No `#[allow]` suppression; compiler-supported idiom. Same guard preseed semantics.

4. **vb_ko29_5_public_idempotency.rs:737**: Added `policy_pin: vb_storage::RunPolicyPin::Unverified` field to legacy `JournalEvent::RunAdmission` fixture. This was the only test file missing the field — all production admission paths and other test files already specify `policy_pin`.

## Files Modified

| File | Lines | Purpose |
|------|-------|---------|
| crates/vb_runtime/Cargo.toml | 1 | Remove unused serde_json dev dep |
| crates/vb_runtime/src/run_checkpoint_tests.rs | 2 | Remove weak redundant assertions |
| crates/vb_runtime/src/shard/lifecycle/chunk_001.rs | 5 | Let chain refactor |
| crates/workspace_tests/tests/vb_ko29_5_public_idempotency.rs | 1 | Add missing policy_pin field |

Total: 4 files, 9 lines changed.

## Targeted Verification

- `cargo build --lib -p vb_runtime -p vb_core -p vb_storage --features "vb_runtime/vb-y9d3v-attempt-fence"`: **PASS** (8.56s)
- `cargo test -p velvet-ballistics-workspace-tests --test vb_ko29_5_public_idempotency --no-run`: **PASS** (5.92s compile)
- `cargo test -p vb_runtime --lib --features "vb_runtime/vb-y9d3v-attempt-fence" -- run_checkpoint_tests`: **PASS** — 10/10 tests passed (8 versioned decoder + 2 existing behavior)
- `cargo test -p velvet-ballistics-workspace-tests --test vb_ko29_5_public_idempotency`: **PASS** — 19/19 tests passed

## Full CI Gate Run

Command:
```bash
TEST_INTEGRITY_BASE=58a1f9580ca8beb32f80118ce08df521688a249a \
TEST_INTEGRITY_ROOT=/home/lewis/src/isoloated/vb-h7rex-tests-v11 \
bash /tmp/vb-run-gate.py 600 .beads/vb-h7rex/evidence/GATES_V17_01 \
  moon ci :workspace-assertions :test-integrity :check :lint-src :fmt \
  --downstream none --force --concurrency 2
```

Status: RUNNING (background job bg_1)

Expected result: All 5 CI tasks pass. No proof obligations, benchmarks, or full nextest suite.