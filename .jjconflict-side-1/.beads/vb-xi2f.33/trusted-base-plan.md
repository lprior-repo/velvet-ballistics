# Trusted Base Plan — vb-xi2f.33

**Bead**: `vb-xi2f.33` / P1: digest covers ask semantics
**Schema**: `trusted-base-ledger/v1`
**Scope**: All trust assumptions that proof obligations depend on.

## Trusted Base Ledger

| ID | Obligation | Artifact | Marker | Kind | Reason |
|----|-----------|----------|--------|------|--------|
| TB-001 | PO-KANI-001..PO-KANI-006, PO-PROPTEST-001..PO-PROPTEST-004, PO-FUZZ-001 | blake3 crate (external dependency) | trusted dependency | `trusted-dependency` | Cryptographic hash determinism is a foundational assumption for all digest properties. blake3 is audited and well-known. No internal verification planned. |
| TB-002 | PO-KANI-001..PO-KANI-006 | Rust stdlib `String::as_bytes()` | trusted stdlib | `trusted-stdlib` | String bytes are deterministic for same String value. This is guaranteed by Rust language semantics. |
| TB-003 | PO-KANI-004 | `b"no_timeout"` sentinel design | design assumption | `design-assumption` | The sentinel `b"no_timeout"` does not collide with `b"timeout"` prefix + any valid timeout expression bytes. Verified by PO-KANI-004 (bounded proof). |
| TB-004 | All obligations | YAML parser type safety (vb_yaml) | trusted boundary | `trusted-boundary` | `parse_ask()` guarantees `prompt: String` and `timeout: Option<String>`. Digest functions assume these invariants without re-validating. |
| TB-005 | Delegated (S8) | Golden Set/Finish digest values | trusted golden | `trusted-golden` | Set/Finish regression tests (delegated to test-planner State 8) depend on captured digest values from before the fix. These must be captured in the test setup. |
| TB-006 | Delegated (S8) | Both copies receive identical fix | process assumption | `process-assumption` | The active path (`part_05.rs`) and legacy path (`compile/mod.rs`) must both receive the same Ask arm addition. Parity test (delegated to S8) enforces this. |
| TB-007 | PO-FUZZ-001 | WorkflowSource reconstruction from fuzz bytes | trusted boundary | `trusted-boundary` | Fuzz target must safely reconstruct a valid WorkflowSource from arbitrary bytes. Invalid inputs must be rejected gracefully (return, not panic). |

## Trusted Dependency: blake3

- **Crate**: `blake3` (external)
- **Version**: per `Cargo.lock`
- **Properties assumed**: Deterministic output for identical input, correct 32-byte finalization, no side effects.
- **Risk**: If blake3 were non-deterministic (contrary to its specification), all digest invariants would be violated.
- **Mitigation**: blake3 is a well-reviewed cryptographic hash with published specification and test vectors. Its determinism is fundamental to its design.
- **Compensating evidence**: proptest determinism tests (PO-PROPTEST-003) will catch any determinism violation at the integration level.
- **Expiry**: No expiry — foundational assumption of the hash-based identity system.

## Design Assumption: Timeout Sentinels

- **Sentinel for `None`**: `b"no_timeout"` (9 bytes)
- **Prefix for `Some`**: `b"timeout"` (7 bytes)
- **Collision risk**: Could a valid timeout expression start with `b"no_timeout"`? Hypothetically yes, but timeout expressions are short strings (seconds, cron expressions). The sentinel is 9 bytes of ASCII text unlikely to appear as a timeout value.
- **Verification**: PO-KANI-004 proves `b"no_timeout"` vs `b"timeout"` + `b""` produce distinct hash states for the bounded case. The general case (arbitrary timeout values) cannot be exhaustively proven by Kani but is covered by proptest (PO-PROPTEST-002).
- **Alternative design**: Use a length-delimited encoding (`[0u8]` for None, `[1u8][len][bytes]` for Some) to guarantee non-collision. This is a more robust design for a future bead.

## Process Assumption: Fix Applied to Both Copies

- **Risk**: If only `part_05.rs` is fixed and `compile/mod.rs` is not, the two copies diverge, violating INV-ASK-006.
- **Mitigation**: PO-UT-003 explicitly tests parity. The fix description in type-contracts.md names both files.
- **Future resolution**: A separate bead should extract `canonical_digest` to a shared location, eliminating the duplication entirely.

## Non-Trusted Items (Verified by Obligations)

The following are NOT trusted but are verified by proof obligations:

| Property | Verified By |
|----------|------------|
| Prompt sensitivity (INV-ASK-001) | PO-KANI-001, PO-PROPTEST-001, PO-FUZZ-001 |
| Timeout sensitivity (INV-ASK-002) | PO-KANI-002, PO-PROPTEST-002 |
| Determinism (INV-ASK-003) | PO-PROPTEST-003 |
| Empty prompt edge case (INV-ASK-004) | PO-KANI-003 |
| None vs Some("") sentinel (INV-ASK-005) | PO-KANI-004 |
| Field ordering (TC-002) | PO-KANI-005, PO-PROPTEST-004 |
| Panic-freedom (TC-007) | PO-KANI-006 |
| Explicit arm (TC-001) | Delegated to S8 test-planner + static review |
| Set/Finish regression (INV-ASK-007) | Delegated to S8 test-planner |
| Duplicate parity (INV-ASK-006) | Delegated to S8 test-planner |
