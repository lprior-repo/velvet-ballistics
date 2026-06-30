# Verifier Lane Matrix: vb-xi2f.38

## Matrix Overview

| Requirement | TLA+ | Verus | Kani | Flux | Loom | Miri | Proptest | Fuzz | Integration |
|-------------|------|-------|------|------|------|------|----------|------|-------------|
| CC-DIGEST-001 (Collect field coverage) | ✅ PO-001 | — | ✅ PO-002 | — | — | — | ✅ PS-003..007 | — | — |
| CC-DIGEST-001a (variable field) | — | — | — | — | — | — | ✅ PS-003 | — | — |
| CC-DIGEST-001a (source field) | — | — | — | — | — | — | ✅ PS-004 | — | — |
| CC-DIGEST-001a (pages field) | — | — | — | — | — | — | ✅ PS-005 | — | — |
| CC-DIGEST-001a (items field) | — | — | — | — | — | — | ✅ PS-006 | — | — |
| CC-DIGEST-001a (body recursive) | ✅ PO-001 | — | ✅ PO-002 | — | — | — | ✅ PS-007 | — | — |
| CC-DIGEST-001b (step ID coverage) | ✅ PO-008 | — | — | — | — | — | ✅ PS-008 | — | — |
| CC-DIGEST-001c (trigger coverage) | ✅ PS-008 | — | — | — | — | — | ✅ PS-009 | — | — |
| CC-DIGEST-002 (determinism) | ✅ PS-009 | — | — | — | — | — | ✅ PO-009 | — | — |
| CC-DIGEST-003 (artifact digest) | ✅ PS-010 | — | — | — | — | — | ✅ PO-010 | — | — |
| CC-DIGEST-004 (lowering) | ✅ PO-012 | ✅ PO-011 | — | — | — | — | — | — | — |
| CC-DIGEST-005 (mismatch detection) | — | — | — | — | — | — | — | — | ✅ PS-012 |
| CC-DIGEST-006 (no panic) | — | — | ✅ PO-013 | — | — | — | — | — | — |
| CC-DIGEST-007 (equality property) | — | — | ✅ PS-014 | — | — | — | ✅ PO-014 | — | — |
| H-1 (Collect fields) | ✅ PO-001 | — | ✅ PO-002, PO-020 | — | — | — | ✅ PS-002..007 | — | — |
| H-2 (ForEach/Aggregate) | — | — | ✅ PO-015, PO-016 | — | — | — | ✅ PS-015, PS-016 | — | — |
| H-4 (lowering determinism) | ✅ PO-017 | — | — | — | — | — | ✅ PS-017 | — | — |
| H-5 (serialization) | — | — | — | — | — | — | ✅ PO-018 | — | — |
| H-6 (pagination state) | — | — | — | — | — | — | ✅ PS-019 | — | ✅ PS-019 |
| H-9 (GOD RULE) | — | — | ✅ PO-013, PO-020 | — | — | — | — | — | — |

## Lane Details

### TLA+ (Temporal/State Machine Verification)
- **Applicability**: CC-DIGEST-001, CC-DIGEST-001b, CC-DIGEST-002, CC-DIGEST-003, CC-DIGEST-004, H-4, H-1
- **Tool**: TLC model checker with `tla2tools.jar`
- **Bounds**: Bounded workflow steps (≤ 20), bounded body steps (≤ 10), bounded string lengths (≤ 256)
- **Invariants**: `CollectDigestCoverage`, `StepIdCoverage`, `LoweringDeterminism`
- **Not applicable to**: Flux-native properties, Rust-local invariants cheaper to test

### Verus (Rust Formal Verification)
- **Applicability**: CC-DIGEST-004 (lowering correctness)
- **Tool**: `cargo verus` with Verus verifier
- **Bounds**: Bounded `Vec<StepAst>` body iteration
- **Proof**: `lemma_lower_canonical_collect_emits_4_nodes` with pre/post conditions
- **Not applicable to**: Digest coverage (not a refinement/type-state property naturally)

### Kani (Bounded Model Checking)
- **Applicability**: H-1, H-2, CC-DIGEST-001, CC-DIGEST-006, CC-DIGEST-007, H-9
- **Tool**: `cargo kani` with `#[kani::proof]` and `kani::any()`
- **Bounds**: Bounded Collect fields, bounded body Vec (≤ 8 steps), bounded String (≤ 64 chars)
- **Key harnesses**: `collect_field_coverage.rs`, `collect_try_from_parts.rs`
- **GOD RULE enforcement**: All harnesses use `kani::any::<Collect>()` not hardcoded data

### Flux (Refinement Types)
- **Applicability**: NOT APPLICABLE — digest coverage is not naturally a refinement/type-state property
- **Evidence**: Digest equality is an equality property, not a numeric/data predicate or constructor-enforced invalid-state exclusion

### Loom (Concurrency Testing)
- **Applicability**: NOT APPLICABLE — digest computation is single-threaded, no concurrent interleavings
- **Evidence**: `digest_step_primitive` is a pure sequential function; no atomics, channels, locks, or async

### Miri (Undefined Behavior Detection)
- **Applicability**: NOT APPLICABLE — `digest_step_primitive` contains no unsafe code
- **Evidence**: Uses only safe `blake3::Hasher`, `String`, `Vec`, `Option<u32>` — no raw pointers, `MaybeUninit`, or FFI

### Proptest (Property-Based Testing)
- **Applicability**: CC-DIGEST-001a, CC-DIGEST-001b, CC-DIGEST-002, CC-DIGEST-003, CC-DIGEST-007, H-1, H-2, H-4, H-5, H-6
- **Tool**: `cargo test` with `proptest::proptest`
- **Bounds**: Input space bounded by workflow compilation limits
- **Key properties**: Field differential digest equality, cross-run determinism, lowering determinism

### Fuzz (cargo-fuzz)
- **Applicability**: NOT APPLICABLE — digest function is deterministic pure function, not a parser or security boundary
- **Evidence**: `canonical_digest` takes validated `WorkflowSource`, not raw bytes; no frame parsing or untrusted input

### Integration Test
- **Applicability**: CC-DIGEST-005, H-6
- **Evidence**: `vb_core_atomic_admission_red.rs` line 856 for digest mismatch; runtime pagination for H-6

---

## Risk-to-Lane Mapping

| Risk | Severity | Primary Lane | Secondary Lane | Tertiary |
|------|----------|-------------|---------------|----------|
| H-1: Collect fields not hashed | CRITICAL | Kani | Proptest | TLA+ |
| H-2: ForEach/Aggregate same bug | HIGH | Kani | Proptest | — |
| H-9: Hardcoded harness data | CRITICAL | Kani | — | — |
| CC-DIGEST-002: Non-determinism | HIGH | Proptest | TLA+ | — |
| CC-DIGEST-004: Lowering semantics | MEDIUM | Verus | TLA+ | — |
| CC-DIGEST-005: Digest mismatch | MEDIUM | Integration test | — | — |
| H-4: Lowering non-determinism | MEDIUM | TLA+ | Proptest | — |
| H-5: Serialization non-det | MEDIUM | Proptest | — | — |
| H-6: Pagination state | MEDIUM | Proptest | Integration | — |

---

## Blocked Tooling

None. All required tools (Kani, Verus, TLA+/TLC, Proptest) are available in the environment.
