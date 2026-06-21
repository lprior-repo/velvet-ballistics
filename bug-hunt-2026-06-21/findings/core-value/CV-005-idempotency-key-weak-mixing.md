# CV-005: `compute_action_idempotency_key` uses non-cryptographic wrapping math — collisions possible

- **Severity**: Low
- **Category**: correctness
- **Location**: `crates/vb_core/src/action/key.rs:8-18`
- **Confidence**: confirmed

## Description

The idempotency key is computed as `((run * M1 + seq) * M2 + action) * M3` with three fixed `u128` multipliers and wrapping arithmetic. The mixing is a single multiplicative chain with no diffusion, no avalanche, and no non-linear step. Two distinct `(run, seq, action)` triples that produce the same intermediate value at any stage collide.

## Evidence

```rust
// key.rs:8
pub fn compute_action_idempotency_key(run: RunId, seq: SeqNo, action: ActionId) -> u128 {
    let run_part = u128::from(run.get());
    let seq_part = u128::from(seq.get());
    let action_part = u128::from(action.get());
    run_part
        .wrapping_mul(0x6c62272e07bb0143_u128)
        .wrapping_add(seq_part)
        .wrapping_mul(0x3b4f1a5b6c2d8e7f_u128)
        .wrapping_add(action_part)
        .wrapping_mul(0x5bd1e9956c7b4d3a_u128)
}
```

Because `wrapping_mul` is invertible for odd multipliers (the multipliers here are all odd), the function is a linear map `f(run, seq, action) = (run * M1 + seq) * M2 + action) * M3 mod 2^128`. Linearity means:

- `f(0, 0, 0) == 0`
- `f(run + a, seq + b, action + c) ≡ f(run, seq, action) + (a*M1 + b) * M2 + c) * M3` (mod 2^128)

Trivial collisions exist: any `(run, seq, action)` and `(run + a, seq + b, action + c)` where the linear combination above is `0 mod 2^128` collide. For u128, finding such inputs is feasible with extended GCD on the multipliers.

## Adversarial Check

The key is consumed by `action_ticket_has_valid_key` (key.rs:22) which is a checksum-style equality check — collisions are not exploitable there because the verifier recomputes from the same `(run, seq, action)` triple. The real risk is if downstream code uses the key for deduplication (e.g., a future idempotency-replay cache that keys on `idempotency_key` alone). The doc on `ActionTicket::idempotency_key` (model.rs:88) calls it "Idempotency key for deduplication and replay", which implies uniqueness — but the math does not provide it.

For current engine code paths, no collision is exploitable, hence Low severity. The finding is preserved because the doc-contract implies uniqueness that the implementation does not deliver, and any future dedup-by-key consumer will inherit the collision risk.

## Suggested Fix

Either:

1. **Use a real hash**: feed `(run, seq, action)` into `blake3` (already in dev-deps, available workspace-wide) and truncate to `u128`. This gives collision resistance of ~2^-60.
2. **Drop the key entirely**: the ticket already carries `(run, seq, action)` as identifying fields. A composite key `(run, seq, action)` IS unique. The `idempotency_key` field is redundant — it could be removed, with callers computing a hash on demand when they need one.
3. **Document the limitation**: change the doc on `ActionTicket::idempotency_key` to "checksum derived from (run, seq, action); not collision-resistant — use the tuple for deduplication, not the key alone".

Option 1 is the safest.
