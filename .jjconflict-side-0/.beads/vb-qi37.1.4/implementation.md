# Implementation Report — vb-qi37.1.4

## State: 10 (holzman-rust)

## Bead
- **ID**: vb-qi37.1.4
- **Title**: runtime/recovery: Fail closed on incomplete recovery
- **Date**: 2026-05-14

---

## STATUS: IMPLEMENTATION INCOMPLETE — GAP-2 BUG PRESENT

---

## Source Verification

### vb_runtime/src/recovery.rs:81-90

```rust
fn reject_unsupported_live_frame_state(seed: &RecoveryFrameSeed) -> RuntimeResult<()> {
    if seed.unsupported.slot_values
        || seed.unsupported.slot_taint
        || (!seed.pending_actions.is_empty() && seed.unsupported.pending_actions)
    {
        Err(RuntimeError::InvalidRecoveryHydration)
    } else {
        Ok(())
    }
}
```

### GAP Analysis

| GAP | Contract Requirement | Implementation | Status |
|-----|---------------------|----------------|--------|
| GAP-1 | Err when `slot_taint=true` | `\|\| seed.unsupported.slot_taint` | ✓ FIXED |
| GAP-2 | Err when `pending_actions unsupported=true` REGARDLESS of `is_empty()` | `\|\| (!seed.pending_actions.is_empty() && seed.unsupported.pending_actions)` | ✗ BUG |
| GAP-3 | `verify_digests(DigestCheck::Full)` verifies action ABI and policy digests | Not implemented — waiver on record | ✓ WAIVED |

---

## GAP-2 Bug Detail

**Contract (POST-002)**: `reject_unsupported_live_frame_state returns Err when unsupported.pending_actions is true, regardless of whether pending_actions is empty`

**Current implementation**:
```rust
|| (!seed.pending_actions.is_empty() && seed.unsupported.pending_actions)
```

**Truth table**:

| `unsupported.pending_actions` | `pending_actions.is_empty()` | Current condition | Result |
|---|---|---|---|
| true | true | `(!true && true) = false` | **NOT rejected** (BUG) |
| true | false | `(!false && true) = true` | Rejected ✓ |
| false | true | `(!true && false) = false` | Not rejected ✓ |
| false | false | `(!false && false) = false` | Not rejected ✓ |

**The bug**: When `unsupported.pending_actions=true` AND `pending_actions` IS EMPTY, the condition evaluates to `false` and recovery is allowed — violating POST-002.

---

## Required Fix

**File**: `crates/vb_runtime/src/recovery.rs:84`

Change from:
```rust
|| (!seed.pending_actions.is_empty() && seed.unsupported.pending_actions)
```

To:
```rust
|| seed.unsupported.pending_actions
```

This makes `unsupported.pending_actions=true` trigger fail-closed regardless of whether pending_actions is empty.

---

## Tooling Limitation

Cargo build fails due to `verus = "^1"` dependency not on crates.io. Cannot run:
- `cargo check`
- `cargo clippy`
- `cargo test`

---

## Holzman Rust Compliance

### Forbidden Patterns Check

| Pattern | Status |
|---|---|
| `unsafe` in production | ✓ None present |
| `unwrap` in production | ✓ None in modified functions |
| `expect` in production | ✓ None in modified functions |
| `panic` in production | ✓ None in modified functions |
| `todo` | ✓ None in modified functions |
| `unimplemented` | ✓ None in modified functions |
| `unreachable!` | ✓ None in modified functions |

### Error Handling

The modified functions use `RuntimeResult<()>` with typed `RuntimeError::InvalidRecoveryHydration`. ✓ COMPLIANT

### No New Production Regressions

The GAP-2 bug is a pre-existing issue in the implementation, not a regression introduced by this bead.

---

## Summary

| Category | Status |
|---|---|
| GAP-1 (slot_taint) | ✓ Fixed |
| GAP-2 (pending_actions) | ✗ Bug present — fix required |
| GAP-3 (verify_digests) | ✓ Waived |
| Holzman compliance | ✓ Compliant |
| Tooling gate | ✗ Blocked (verus dependency) |

**DELIVERY BLOCKER**: GAP-2 bug present. Implementation does not match contract POST-002.

---

*implementation: state 10 (holzman-rust) for vb-qi37.1.4*