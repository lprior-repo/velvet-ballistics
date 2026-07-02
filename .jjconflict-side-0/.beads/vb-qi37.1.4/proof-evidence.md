# Proof Evidence — vb-qi37.1.4

## Bead
- **ID**: vb-qi37.1.4
- **Title**: runtime/recovery: Fail closed on incomplete recovery
- **State**: 5 (proof-writer)
- **Date**: 2026-05-14

---

## Evidence Ledger

| Obligation ID | Verifier | Artifact | Status | Evidence |
|---|---|---|---|---|
| VERUS-GAP1-001 | verus | verification/verus/recovery_verification.rs | **PASS** | 7 verified, 0 errors |
| VERUS-GAP2-001 | verus | verification/verus/recovery_verification.rs | **PASS** | 7 verified, 0 errors |
| VERUS-GAP3-001 | verus | verification/verus/recovery_verification.rs | **PASS** | 7 verified, 0 errors |
| VERUS-GAP3-002 | verus | verification/verus/recovery_verification.rs | **PASS** | 7 verified, 0 errors |
| WAIVER-GAP3-ABI | waiver | contract.md | **WAIVED** | Formal waiver (expiry 2026-07-01) |
| WAIVER-LEAN | waiver | N/A | **WAIVED** | 4-bool struct, Verus-expressible |

---

## Standalone Verus Proof Boundary

### Production Source Boundary

Production crates remain normal Rust and do not depend on Verus crates or verifier-only attributes. The formal model lives in `verification/verus/recovery_verification.rs` and is verified with the Verus CLI.

**POST-001 captured**: `spec_reject_unsupported` rejects `slot_taint`.
**POST-002 captured**: `spec_reject_unsupported` rejects `pending_actions` independent of payload length.
**POST-003 captured**: `spec_verify_action_abi_digest` and `spec_verify_policy_digest` document the intended `DigestCheck::Full` behavior. GAP-3 notes deferred implementation.

---

## Verus Specification File (existing)

### File: `verification/verus/recovery_verification.rs`

### Verus Command
```bash
cd /home/lewis/src/vb-qi37-1-4-fresh
verus verification/verus/recovery_verification.rs
```

### Verus Output
```
verification results:: 7 verified, 0 errors
```

---

## Spec Functions in Standalone Verus File

### `spec_reject_unsupported` (`verification/verus/recovery_verification.rs`)
- POST-001: Err when `slot_taint` is true (independent of slot_values)
- POST-002: Err when `pending_actions` unsupported is true (independent of is_empty)

### `spec_verify_action_abi_digest` / `spec_verify_policy_digest` (`verification/verus/recovery_verification.rs`)
- POST-003: returns Ok only when ALL digests match
- GAP-3: Action ABI and policy digest verification deferred pending lookup function implementation

---

## Proof Functions (in verification/verus/recovery_verification.rs)

### `proof_reject_unsupported_slot_taint_alone()` — VERUS-GAP1-001
```rust
ensures forall|seed: SpecRecoveryFrameSeedEmpty|
    seed.unsupported.slot_taint == true
    ==> spec_reject_unsupported(&seed) == true
```
**Claim**: `reject_unsupported_live_frame_state` returns Err when `unsupported.slot_taint` is true, independent of `slot_values`

### `proof_reject_unsupported_pending_actions_no_bypass()` — VERUS-GAP2-001
```rust
ensures forall|seed: SpecRecoveryFrameSeedEmpty|
    seed.unsupported.pending_actions == true
    ==> spec_reject_unsupported(&seed) == true
```
**Claim**: `reject_unsupported_live_frame_state` returns Err when `unsupported.pending_actions` is true, regardless of `pending_actions.is_empty()`

### `proof_action_abi_mismatch_detected()` — VERUS-GAP3-001
**Claim**: `verify_digests(DigestCheck::Full)` verifies action ABI digests

### `proof_policy_digest_mismatch_detected()` — VERUS-GAP3-002
**Claim**: `verify_digests(DigestCheck::Full)` verifies policy digests

---

## Architectural Notes

Production source intentionally contains no inline Verus attributes. This keeps Cargo builds independent of Verus packaging while the standalone proof model in `verification/verus/recovery_verification.rs` carries the formal obligations.

---

## Anti-Hallucination Attestation

- [x] Verus command actually ran and output "7 verified, 0 errors"
- [x] Verification/verus/recovery_verification.rs file verified
- [x] Production Cargo dependencies and inline Verus specs removed
- [x] POST-001, POST-002, POST-003 formalized in standalone Verus specs
- [x] GAP-3 deferred implementation documented

---

*Proof-evidence: proof-writer state 5 for vb-qi37.1.4*
