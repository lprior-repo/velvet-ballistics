# Proof Review Report — Updates

## Corrections (from adversarial audit → actual production code)

### Finding 5 RETRACTED: vb_y9d3v_action_fence.rs overflow mismatch
**Original claim:** `spec_retry_attempt_after` uses `wrapping_add(1)` but production uses `checked_add(1)`.
**Retraction:** In the `base < max_attempts` branch, `max_attempts ≤ u16::MAX` (it IS u16), so `base < u16::MAX`. Therefore `base.wrapping_add(1) == base.checked_add(1).unwrap()`. The spec is correct.
**Status:** No fix needed. File passes verus: 24 verified, 0 errors.

### Finding 4 PARTIALLY CORRECTED: vb_kzz99_action_completion.rs
**Original claim:** `spec_validate_input_bytes` — production function doesn't exist.
**Correction:** `validate_input_bytes` DOES exist at `action.rs:206-217`. The spec is a valid abstraction (raw u32/u16 params instead of ActionInput/ActionContract structs).

**Original claim:** `spec_advance_after_action_completion` — production function doesn't exist at claimed path.
**Correction:** `advance_after_action_completion` DOES exist at `helpers/action.rs:102-116`. The spec is a valid abstraction (bool params instead of &mut RunState, finer-grained local error types instead of production's single `RuntimeError::InvalidActionCompletion`).

**Original claim:** `TerminalStep` error variant never produced by production.
**Correction:** Spec models terminal step as `Ok(())` which MATCHES production (`None => Ok(())`). The `TerminalStep` error variant in the local enum is dead code but doesn't affect correctness.

**Status:** File is CONNECTED to production. Minor cleanup applied (updated header comments to accurately reflect bindings). 16 verified, 0 errors.

### Finding 4 CORRECTED: `scheduled_attempt_after_spec` is IDENTICAL to production
Production (helpers/action.rs:225-234) and spec logic are byte-for-byte identical:
```
ticket_attempt == 0 → current
None → Some(ticket_attempt)
Some(c) if c == 0 || ticket_attempt > c → Some(ticket_attempt)
Some(c) → Some(c)
```
This is the ONLY proof file with a true production binding.

## Final Disposition

| File | Disposition | Verified | Errors |
|------|------------|----------|--------|
| vb_rxru0_action_verus.rs | Fixed — all `assert(true)` replaced | 8 | 0 |
| runtime_facade_typed_errors.rs | Deleted — local enum copy | — | — |
| runtime_module_topology.rs | Deleted — duplicate spec_shard_index + unbound topology | — | — |
| vb_kzz99_action_completion.rs | Connected — header comments corrected | 16 | 0 |
| vb_y9d3v_action_fence.rs | Connected — no fix needed (overflow analysis was false positive) | 24 | 0 |
| vb-0l9k0/helpers.rs | Connected — minor auto-simplification, documented | 4 | 0 |
| vb-0l9k0/numeric_timer.rs | Connected | 6 | 0 |
| vb-0l9k0/pending_timer.rs | Connected | 5 | 0 |
| vb-0l9k0/timer_wheel.rs | Connected | 5 | 0 |
| runtime_facade_api.rs | Connected — the ONLY well-bound proof | 6 | 0 |
