# Waiver Candidates: vb-xi2f.38

## Status: No Waivers Proposed

No behavior-affecting waivers are proposed for vb-xi2f.38. All proof obligations are addressed by applicable verifier lanes.

---

## Rationale

| Lane | Applicable | Obligations | Justification |
|------|------------|-------------|---------------|
| TLA+ | Yes | PO-001, PO-008, PO-008b, PO-012, PO-017 | Formal invariant coverage for digest properties |
| Verus | Yes | PO-011 | Lowering correctness; natural fit for Verus pre/post conditions |
| Kani | Yes | PO-002, PO-013, PO-015, PO-016, PO-020 | Bounded exhaustive checking; GOD RULE enforcement |
| Proptest | Yes | PO-003, PO-004, PO-005, PO-006, PO-007, PO-009, PO-010, PO-014, PO-018 | Property-based testing for broad input space |
| Integration | Yes | PO-012b | End-to-end storage admission test |
| Flux | No | — | Not applicable: digest equality is not a refinement/type-state property |
| Loom | No | — | Not applicable: no concurrency in digest computation |
| Miri | No | — | Not applicable: no unsafe code in digest_step_primitive |
| Fuzz | No | — | Not applicable: digest function is not a parser/security boundary |

---

## Non-Applicable Lane Justifications

### Flux RS
- **Risk**: Digest coverage equality property
- **Why not applicable**: Flux RS is designed for refinement types where a constructor or function enforces invariants. The digest coverage property (a == b ⟺ digest_eq(a, b)) is an equality property, not a numeric/data predicate or constructor-enforced invalid-state exclusion.
- **Alternative**: Kani (bounded exhaustive) + Proptest (property-based) provide stronger coverage for this property.

### Loom
- **Risk**: Concurrency in digest computation
- **Why not applicable**: `digest_step_primitive` is a single-threaded pure sequential function with no atomics, channels, locks, async, or concurrent interleavings. There is no concurrency in the digest computation path.
- **Alternative**: Not needed.

### Miri
- **Risk**: Undefined behavior in digest computation
- **Why not applicable**: `digest_step_primitive` contains no unsafe code: uses only safe `blake3::Hasher`, `String`, `Vec`, `Option<u32>`. No raw pointers, `MaybeUninit`, or FFI.
- **Alternative**: Not needed.

### cargo-fuzz
- **Risk**: Adversarial input at trust boundary
- **Why not applicable**: `canonical_digest` takes a validated `WorkflowSource` (well-typed AST), not raw bytes. There is no parser, no protocol frame, no untrusted input boundary. Digest function is a deterministic pure function.
- **Alternative**: Not needed.
