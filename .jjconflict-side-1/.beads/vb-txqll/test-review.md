# Test Review: vb_79ffw_ticket_authority_probe

**Date:** 2026-10-21
**Test file:** crates/workspace_tests/tests/vb_79ffw_ticket_authority_probe.rs
**Reviewer:** local5090-ninfer/qwen3.8-27b-uncensored
**Status:** APPROVED

## Test Plan Review

The test suite covers the complete vb-79ffw ticket authority behavior contract:
- Run-attempt ticket generation and progression
- Stale completion rejection with exact error type
- Queued stale failure tick rejection with exact error type
- Cold recovery preserves current generation
- Idempotency key uniqueness across run attempts
- Ticket serialization roundtrip integrity

## Assertion Strength Analysis

### Strong assertions (field-level, typed, byte-exact)
- Exact `RuntimeError::StaleAttempt { incoming, current }` with field value checks
- Exact `RuntimeError::InvalidActionCompletion` type
- Full serialized `JournalEvent` vector byte-equality (not count >=)
- Full `Option<ActionTicket>` equality for pending ticket state
- Full `InspectSnapshot` equality for public snapshot surface
- Specific event field matches (`run`, `result`, `ticket`, `output`, decoded `SlotValue::I64(42)`)

### Public API surface
The test exclusively uses public Runtime/Fjall API:
- `Runtime::new()`, `Runtime::submit_direct_with_inputs_grants_and_contracts()`
- `Runtime::tick_all()`, `Runtime::fail_action()`, `Runtime::complete_action_with_output()`
- `Runtime::recover_retry()`, `Runtime::lookup_pending_action_ticket()`
- `Runtime::snapshot_run(run, correlation)`

No private field access or internal state manipulation.

## Limitations

1. **Public snapshot fields only**: `InspectSnapshot` exposes a public surface, not all private run state. The test correctly limits claims to observable public fields.

2. **No property-based testing**: Uses concrete values, not `proptest`-driven generation. This is appropriate for state transition verification but doesn't exhaustively cover input space.

3. **Single shard count**: Tests use `shard_count() = 1`. Multi-shard routing not covered (out of scope for ticket authority).

4. **No real time passage**: All retry checkpoints occur in immediate succession. Timer-based expiry behavior not tested.

## Determinism

Tests are deterministic:
- Fixed RunId values (90001-90006)
- No sleeps or wall-clock dependencies
- No random values
- Tempdir cleanup via RAII

## Mutation Resistance

- Delete stale rejection logic → `stale_completion_rejected_after_two_retries` fails at exact error type check
- Allow state mutation on stale rejection → serialized journal vector equality fails
- Allow state mutation on stale tick → serialized journal vector equality fails
- Reset pending ticket on stale events → Option equality check fails
- Change snapshot on stale events → InspectSnapshot equality fails
- Change run_attempt generation → run_attempt=1/2/3 assertions fail
- Change idempotency key computation → key difference assertions fail

## Test Infrastructure

Explicit Cargo.toml registration at crates/workspace_tests/Cargo.toml:198-200. No implicit test discovery.

## STATUS: APPROVED

All assertion strength, coverage, and determinism criteria met. The test suite provides strong evidence of ticket authority behavior through exact typed assertions, full state vector equality, and public API surface verification.