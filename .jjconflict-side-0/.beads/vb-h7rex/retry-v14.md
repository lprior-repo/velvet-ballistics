# vb-h7rex: Policy Pin Durable Recovery Validation

**Bead ID:** vb-h7rex
**Attempt:** v14
**Status:** Complete
**Date:** 2026-10-22

## Problem

`Runtime::recover_retry` validates checkpoint value equality but did not validate the checkpoint's admission policy pin against the artifact's `policy_digest`. A forged checkpoint with a wrong pin could pass durable chain validation and be restored, violating the admission authority contract.

## Fix

Added policy pin validation in `Shard::handle_recover` (crates/vb_runtime/src/shard/lifecycle/chunk_001.rs) BEFORE journal seeding and checkpoint restore. The validation:

- Checks if the checkpoint has admission authority (`Some(admission)`)
- If the admission has a `Pinned` policy pin, compares it against `artifact.policy_digest`
- Returns `RecoveryCannotResume { reason: "admission_policy_pin_mismatch" }` on mismatch
- `Unverified` (legacy) pins are accepted as historical records per existing contract

## Test

Replaced the vacuous `admission_policy_pin_mismatch_not_reachable_with_current_semantics` test with `admission_policy_pin_mismatch_rejected_by_recover_retry` which:

1. Runs a real workflow to failure, producing a checkpoint with the correct policy pin
2. Forges a checkpoint with a wrong policy pin (`[0xEE; 32]`) using the same test-local serde DTO pattern as existing tests
3. Persists the forged checkpoint as the durable checkpoint by appending a new `RunFailedEvent` (attempt 2) and forged `RetryCheckpoint` (attempt 3) to the journal
4. Calls `recover_retry` (enqueues command) then `tick_all` (shard processes)
5. Asserts exact error type `RecoveryCannotResume` with reason `admission_policy_pin_mismatch`
6. Verifies journal was not mutated by the rejected recovery

All original four authoritative tests remain unchanged:
- `strict_admission_policy_pin_survives_durable_recovery` — PASS
- `journaled_admission_policy_pin_survives_durable_recovery` — PASS
- `legacy_version_1_checkpoint_is_explicitly_rejected` — PASS
- `tampered_policy_pin_rejected_by_recover_retry` — PASS

## Evidence

- Raw RED (before production fix): Test failed with "forged policy pin must be rejected during tick processing" — forged checkpoint was accepted because no policy pin validation existed in retry recovery flow.
- Raw GREEN (after production fix): All 5 tests pass. Debug output confirms forged pin (`[238; 32]`) is compared against artifact policy_digest (`[218, 104, 8, ...]`) and rejected with the exact expected error.

## Changes

1. `crates/vb_runtime/src/shard/lifecycle/chunk_001.rs` — Added policy pin validation in `handle_recover` (Step 4)
2. `crates/vb_runtime/tests/policy_pin_durable_recovery.rs` — Replaced vacuous test with real validation test; added `RuntimeError` import
3. `crates/vb_storage/tests/policy_pin_wire_contract.rs` — Removed unused `JournalError` import (fmt cleanup)

## Verification

```bash
cargo test --release -p vb_runtime --test policy_pin_durable_recovery
cargo test --release -p vb_storage --test policy_pin_wire_contract
cargo fmt --all -- --check
```

All pass (exit code 0).