# Trusted Base Plan — vb-xi2f.34: Finish Digest Coverage

**Bead**: vb-xi2f.34  
**Phase**: p4-proof-planner  
**Date**: 2026-05-24  

---

## Purpose

This document registers every assumption, stub, bound, trusted surface, and model reduction that the proof obligations depend on. Each assumption carries a risk rating and a mitigation strategy.

---

## Trusted Dependencies

### T-1: `blake3` crate (v1.x)

| Property | Trust Level | Rationale |
|---|---|---|
| Determinism | TRUSTED | `blake3::Hasher` is specified to be deterministic; well-known cryptographic hash |
| Collision resistance | TRUSTED | 2^-128 collision probability; trusted for workflow identity (not cryptographic security) |
| No IO/random | TRUSTED | blake3 is seeded deterministically; no entropy source |
| `Hasher::update` accumulates | TRUSTED | Standard hash API; bytes fed in sequence produce deterministic final output |

**Risk**: Medium. If blake3 has a bug, digests change.  
**Mitigation**: blake3 is a widely-used, audited crate. Digest change would be a breaking change detectable by integration tests.  
**Kani impact**: Kani harnesses use a tracking mock, not real blake3. The mock records byte slices passed to `update()`. The proof is about the *inputs to blake3*, not blake3's internal logic.

### T-2: Rust Standard Library — `i64::to_le_bytes()`

| Property | Trust Level | Rationale |
|---|---|---|
| Bijective | TRUSTED | `to_le_bytes()` is specified to return the little-endian byte representation. Different i64 values produce different `[u8; 8]`. This is provable by construction. |
| Platform-independent | TRUSTED | LE is explicit; same bytes on all platforms. |

**Risk**: None. Rust standard library invariant.

### T-3: Rust Standard Library — `String::as_bytes()`

| Property | Trust Level | Rationale |
|---|---|---|
| Returns UTF-8 bytes of string content | TRUSTED | Standard library guarantee. |
| Deterministic for same String value | TRUSTED | String is immutable; `as_bytes()` returns a borrow. |

**Risk**: None.

### T-4: Rust Compiler — `#[non_exhaustive]` semantics

| Property | Trust Level | Rationale |
|---|---|---|
| Downstream crates must handle unknown variants | TRUSTED | Rust language guarantee. |
| Match exhaustiveness checking | TRUSTED | Compiler enforces all variants matched or `_` arm present. |

**Risk**: None. Fundamental Rust feature.

---

## Stubs and Mocks

### S-1: Kani blake3 Hasher Tracking Mock

For Kani harnesses (PO-KANI-001, 002, 003), the `blake3::Hasher` is replaced with a tracking mock:

```
Mock behavior:
- hasher.update(bytes) appends bytes to an internal Vec<u8> accumulator
- Callers verify that distinct inputs produce distinct accumulator contents
```

**Justification**: Proving blake3's internal hash function correctness is out of scope. The proof obligation is about the *inputs to the hasher* — i.e., that different Finish result values produce different byte sequences fed to `update()`. This is a valid model reduction because:
1. blake3 is collision-resistant (T-1).
2. If the byte sequences differ, the final hashes differ (with overwhelming probability).

**Risk**: Low. The reduction is sound for the property being proven (input discrimination).

### S-2: Proptest WorkflowSource Generator

For proptest obligations (PO-PROPTEST-001 through 004), a `WorkflowSource` generator creates valid AST structures.

**Generator requirements**:
- Generates `WorkflowSource` with version, name, trigger, and 1-10 steps
- At least one step is `Finish { result: ScalarValue }`
- `ScalarValue` can be `String` (arbitrary UTF-8 content) or `Integer` (arbitrary i64)
- Other steps can include `Set` and other primitive types

**Justification**: The digest function operates on `WorkflowSource`; the generator must cover the relevant input space. The parser boundary (YAML → AST) is assumed correct (out of scope).

**Risk**: Low. The generator is for statistical testing, not proof. False negatives (generator doesn't produce edge cases) are mitigated by Kani's exhaustive exploration of bounded spaces.

---

## Bounds and Model Reductions

### B-1: String Value Bound (Kani)

**Bound**: String values ≤ 256 bytes in Kani harnesses.  
**Reason**: Kani cannot exhaustively explore unbounded strings. 256 bytes covers all realistic output name references.  
**Risk**: Low. If a String > 256 bytes produces a collision undetected by Kani, proptest (PO-PROPTEST-002) catches it statistically.

### B-2: No Deep AST Recursion

**Bound**: Kani unwind limit of 3 (covers: `digest_step_primitive` → `match primitive` → `match result`).  
**Reason**: The function has bounded recursion (none) and shallow match nesting.  
**Risk**: None. The function is verified to have no loops or recursion.

### B-3: Blast Radius

**Scope**: Only the `Finish` arm of `digest_step_primitive` (lines 150-156 canonical, lines 250-255 legacy).  
**Excluded**: Other primitives (Set, Ask, Together, etc.) are not in scope for this bead. Their digest behavior is covered by existing implementation, not this proof plan.

---

## Trusted Surfaces (External to Proof)

### Surface: YAML Parser (`vb_yaml`)

**Trust**: The YAML parser produces a correct `WorkflowSource` AST for valid YAML input.  
**Justification**: Parser correctness is a separate contract (not in this bead's scope). Digest proofs operate on typed AST values, bypassing the parser.  
**Risk**: Parser bugs could cause digests to differ between textually equivalent YAML inputs. Out of scope for this bead.

### Surface: Postcard Serialization (`vb_storage`)

**Trust**: Postcard serialization of `WorkflowParts` preserves `WorkflowDigest` bytes.  
**Justification**: Storage serialization is downstream of digest computation. Not in scope.  
**Risk**: Serialization bugs could corrupt persisted digests. Separate concern.

### Surface: `canonical_primitive_name()`

**Trust**: Known bugs (Together → "parallel", Aggregate → "aggregate") are **waived** for this bead (WC-001).  
**Justification**: `Finish` has its own explicit match arm in `digest_step_primitive()` that writes `b"finish"`, bypassing `canonical_primitive_name()`.  
**Risk**: None for Finish digest.

---

## Assumption Risk Summary

| ID | Assumption | Risk | Impact if Wrong | Mitigation |
|---|---|---|---|---|
| T-1 | blake3 determinism | Medium | Digest instability | Industry-standard crate; integration tests would catch |
| T-2 | i64::to_le_bytes bijective | None | Theoretical | Rust stdlib guarantee |
| T-3 | String::as_bytes deterministic | None | Theoretical | Rust stdlib guarantee |
| T-4 | #[non_exhaustive] semantics | None | Theoretical | Rust language guarantee |
| S-1 | Kani blake3 mock | Low | Proof over input discrimination, not hashing | Proptest defense-in-depth catches blake3 issues |
| S-2 | Proptest AST generator | Low | False negatives | Kani exhaustive exploration covers bounded space |
| B-1 | String ≤ 256 bytes | Low | Misses >256B edge case | Proptest catches statistically |
