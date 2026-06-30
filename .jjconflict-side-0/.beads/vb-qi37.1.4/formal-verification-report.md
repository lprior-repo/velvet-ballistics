# Formal Verification Report — vb-qi37.1.4

## STATUS: REJECTED — Implementation GAP-2 Bug

---

## Inputs

| Artifact | Status |
|----------|--------|
| proof-obligations.jsonl | ✓ Present |
| delivery-scope.jsonl | ✓ Present |
| baseline-report.md | ✓ Present |
| tla-spec.md | ✓ Present |
| contract-verification-review.md | ✓ Present |

---

## Tool Availability

| Tool | Available | Path |
|------|-----------|------|
| verus | ✓ Yes | /home/lewis/.local/bin/verus |
| tlc | ✓ Yes | /home/lewis/.local/share/mise/installs/http-tla2tools/1.7.4/tlc |
| cargo | ✓ Yes | /home/lewis/.cargo/bin/cargo |
| moon | Unknown | Not checked |

---

## Obligation Results

### VERUS-GAP1-001
- **id**: VERUS-GAP1-001
- **risk**: critical
- **scope**: bead-local
- **layer**: verus
- **checker**: verus
- **command**: `verus verification/verus/recovery_verification.rs`
- **required**: true
- **owner_state**: 3
- **rerun_from**: 3
- **result**: PASS
- **evidence**: 7 verified, 0 errors

### VERUS-GAP2-001
- **id**: VERUS-GAP2-001
- **risk**: critical
- **scope**: bead-local
- **layer**: verus
- **checker**: verus
- **command**: `verus verification/verus/recovery_verification.rs`
- **required**: true
- **owner_state**: 3
- **rerun_from**: 3
- **result**: PASS
- **evidence**: 7 verified, 0 errors

### VERUS-GAP3-001
- **id**: VERUS-GAP3-001
- **risk**: critical
- **scope**: touched-crate
- **layer**: verus
- **checker**: verus
- **command**: `verus verification/verus/recovery_verification.rs`
- **required**: true
- **owner_state**: 3
- **rerun_from**: 3
- **result**: PASS
- **evidence**: 7 verified, 0 errors

### VERUS-GAP3-002
- **id**: VERUS-GAP3-002
- **risk**: critical
- **scope**: touched-crate
- **layer**: verus
- **checker**: verus
- **command**: `verus verification/verus/recovery_verification.rs`
- **required**: true
- **owner_state**: 3
- **rerun_from**: 3
- **result**: PASS
- **evidence**: 7 verified, 0 errors

### WAIVER-GAP3-ABI
- **id**: WAIVER-GAP3-ABI
- **layer**: waiver
- **result**: WAIVED
- **evidence**: Formal waiver (expiry 2026-07-01) in contract.md

### WAIVER-LEAN
- **id**: WAIVER-LEAN
- **layer**: waiver
- **result**: WAIVED
- **evidence**: All clauses Verus-expressible

### UNIT-GAP1-SLOT-TAINT
- **id**: UNIT-GAP1-SLOT-TAINT
- **risk**: critical
- **scope**: touched-crate
- **layer**: unit-test
- **checker**: cargo test
- **command**: `cargo test -p vb_runtime -- recovery`
- **required**: true
- **result**: FAIL_LOCAL
- **evidence**: Tooling limitation — verus dependency not on crates.io

### UNIT-GAP2-PENDING
- **id**: UNIT-GAP2-PENDING
- **risk**: critical
- **scope**: touched-crate
- **layer**: unit-test
- **checker**: cargo test
- **command**: `cargo test -p vb_runtime -- recovery`
- **required**: true
- **result**: FAIL_LOCAL
- **evidence**: Tooling limitation — verus dependency not on crates.io

### UNIT-GAP3-ACTION-ABI
- **id**: UNIT-GAP3-ACTION-ABI
- **risk**: critical
- **scope**: touched-crate
- **layer**: unit-test
- **checker**: cargo test
- **command**: `cargo test -p vb_storage --lib -- recovery`
- **required**: true
- **result**: FAIL_LOCAL
- **evidence**: Tooling limitation — verus dependency not on crates.io

### UNIT-GAP3-POLICY
- **id**: UNIT-GAP3-POLICY
- **risk**: critical
- **scope**: touched-crate
- **layer**: unit-test
- **checker**: cargo test
- **command**: `cargo test -p vb_storage --lib -- recovery`
- **required**: true
- **result**: FAIL_LOCAL
- **evidence**: Tooling limitation — verus dependency not on crates.io

### INTEG-GAP1
- **id**: INTEG-GAP1
- **risk**: critical
- **scope**: touched-crate
- **layer**: integration-test
- **checker**: cargo test
- **command**: `cargo test -p vb_storage --test recovery_integration slot_taint`
- **required**: true
- **result**: FAIL_LOCAL
- **evidence**: Tooling limitation — verus dependency not on crates.io

### INTEG-GAP2
- **id**: INTEG-GAP2
- **risk**: critical
- **scope**: touched-crate
- **layer**: integration-test
- **checker**: cargo test
- **command**: `cargo test -p vb_storage --test recovery_integration pending_actions_unsupported_empty`
- **required**: true
- **result**: FAIL_LOCAL
- **evidence**: Tooling limitation — verus dependency not on crates.io

### KANI-CODEC
- **id**: KANI-CODEC
- **risk**: high
- **scope**: touched-crate
- **layer**: kani
- **checker**: cargo kani
- **command**: `cargo kani --workspace --no-default-features --features=kani`
- **required**: true
- **result**: FAIL_LOCAL
- **evidence**: Tooling limitation — verus dependency not on crates.io

---

## Waivers

### WAIVER-GAP3-ABI
- **Owner**: contract
- **Reason**: verify_digests currently has deferred implementation (action_abi_digests, policy_digests parameters)
- **Expiry**: 2026-07-01
- **Limitation**: GAP-3 implementation must add parameters
- **Compensating evidence**: VERUS-GAP3-001/002 + unit tests

### WAIVER-LEAN
- **Owner**: contract
- **Reason**: All clauses Verus-expressible
- **Compensating evidence**: Verus proofs for GAP-1/GAP-2

---

## Residual Risk

### GAP-2 Bug (BLOCKING)
**Location**: `crates/vb_runtime/src/recovery.rs:84`

**Issue**: The condition `(!seed.pending_actions.is_empty() && seed.unsupported.pending_actions)` means that when `unsupported.pending_actions=true` AND `pending_actions IS EMPTY`, the guard does NOT fire.

**Contract violation**: POST-002 requires `reject_unsupported_live_frame_state` to return Err when `unsupported.pending_actions` is true REGARDLESS of whether `pending_actions` is empty.

**Truth table**:
| `unsupported.pending_actions` | `pending_actions.is_empty()` | Guard fires? | Correct? |
|---|---|---|---|
| true | true | NO | ✗ BUG |
| true | false | YES | ✓ |
| false | true | NO | ✓ |
| false | false | NO | ✓ |

**Fix required**: Change line 84 from:
```rust
|| (!seed.pending_actions.is_empty() && seed.unsupported.pending_actions)
```
to:
```rust
|| seed.unsupported.pending_actions
```

---

## Summary

| Category | Count |
|----------|-------|
| Total obligations | 13 |
| PASS | 4 (Verus) |
| WAIVED | 2 |
| FAIL_LOCAL | 7 (tooling + GAP-2 bug) |

**STATUS: REJECTED** — GAP-2 bug present, tooling blocks test execution.

---

*formal-verification-report: state 11 (formal-verifier) for vb-qi37.1.4*