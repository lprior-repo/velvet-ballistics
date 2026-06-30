# Proof Strategy — Section 16 Symbolic Diagnostic Codes

**Bead**: vb-xi2f.10
**Phase**: State 4 — Proof Planning
**Date**: 2026-05-24
**Strategy Version**: 1.0.0

---

## 1. Strategy Summary

**Classification**: P1 diagnostic infrastructure. Pure functional core with zero concurrency, zero unsafe, and zero temporal behavior. The entire diagnostic code system is a const-validated data mapping.

**Primary Lanes**: Kani (bounded model checking of registry invariants, const assertions, deterministic behavior) + proptest (exhaustive enumeration of error variants, property-based testing of the registry/code bijection).

**Defense-in-Depth Lanes**: cargo-fuzz (hostile input deserialization boundary), cargo-mutants (mutation resistance verification for the existing extensive test suite), moon ci gauntlet (aggregate CI gate).

**Not Applicable Lanes**: TLA+ (no temporal workflows, queues, retries, leases, or distributed protocols), Verus (invariants are const-validated data consistency, not algorithmic correctness needing inductive proofs), Flux (simple newtypes with const validation, not refinement-type predicates), Loom (no concurrency, atomics, channels, or locks), Miri (no unsafe, FFI, raw pointers, or provenance).

**Waiver Candidates**: Zero-allocation proof (PS-010) requires allocator stubbing in Kani — non-behavior perf invariant, candidate for waiver if stubbing proves infeasible.

---

## 2. Risk Classification

| Risk Tag | Seeds | Primary Lane | Rationale |
|----------|-------|-------------|-----------|
| `invariant` / `contract` | PS-001–PS-019 | Kani | Bounded state space (90 entries, const slice), finite transition systems (parser, codec), arithmetic/index bounds (u16 packing). Kani is the cheapest lane that kills the real risk. |
| `domain` | PS-009, PS-016, PS-019, PS-020 | proptest | Exhaustive enumeration of error variants and registry entries. proptest generates all cases and asserts properties. |
| `parser` / `hostile-input` | PS-006, PS-011, PS-012 | cargo-fuzz + Kani | Codec boundary: hostile input strings must not panic, corrupt state, or bypass validation. Fuzz for crash/security; Kani for bounded correctness. |
| `performance` | PS-010 | Kani (waiver candidate) | Zero-allocation invariant is non-behavior. Kani stub for alloc verifies no allocation path is reached. |
| `release` / `public-api` | PS-002, PS-011, PS-020 | proptest | Backward compatibility and stable API surface require property-based regression tests. |
| `gap` | PS-008, PS-020 | Kani + proptest | New functionality (YamlError code()) and promoted test constants need both bounded verification and property testing. |

---

## 3. Coverage Approach By Seed

### 3.1 SymbolicCode Construction & Validation (PS-001)
- **Kani**: Verify for all registered strings, from_static(s).is_some(); for all unregistered strings, from_static(s).is_none().
- **proptest**: Generate arbitrary &str values, assert from_static only succeeds for registry strings.

### 3.2 Registry Invariants (PS-002, PS-003, PS-013, PS-014)
- **Kani**: Verify at const-eval time: no duplicate symbolic names, no duplicate numeric codes, all non-zero, all category/high-byte matching.
- **proptest**: Runtime defense-in-depth: enumerate registry, assert uniqueness and bijection.

### 3.3 Error Type Code Methods (PS-004, PS-005, PS-008, PS-009, PS-019)
- **Kani**: For every error variant, code() returns a SymbolicCode in the registry.
- **proptest**: Exhaustive enumeration of all 58+ ValidationError, 20 YamlError, 30+ CompileError, 46 CoreError, 25+ RuntimeError, 28 JournalError variants. Assert uniqueness and registration.

### 3.4 Parser/Codec Boundary (PS-006, PS-011, PS-012)
- **Kani**: Verify is_supported_code() accepts all in-use code constants; from_str for all previously supported codes succeeds; from_str for new codes succeeds.
- **proptest**: Generate E-style strings in and out of range, assert parse behavior.
- **cargo-fuzz**: Fuzz serde deserialization of SymbolicCode with arbitrary JSON payloads.

### 3.5 Diagnostic Record Consistency (PS-007, PS-018)
- **Kani**: Verify Diagnostic::new() constructor never produces mismatched symbolic/numeric codes.

### 3.6 Determinism & Purity (PS-017)
- **Kani**: For arbitrary error value, symbolic_code() returns same result on repeated calls; no panic; no side effects.

### 3.7 Section 16 Master Contract Parity (PS-016)
- **proptest**: Define expected mapping from master document; assert CODE_REGISTRY contains exact matches.

### 3.8 diag_codes.rs Promotion (PS-020)
- **proptest**: For each promoted constant, assert it exists in registry with correct symbolic name.

---

## 4. Defense-in-Depth Layering

```
Layer 0 (Compile-time): const assertions in CODE_REGISTRY
  ├── No duplicate symbolic names
  ├── No duplicate numeric codes
  ├── All numeric codes non-zero
  └── Category matches high byte

Layer 1 (Kani): bounded verification
  ├── from_static validation correctness
  ├── Registry bijection proofs
  ├── Error variant code() property proofs
  ├── Parser/codec bounded-correctness
  └── Diagnostic constructor invariant

Layer 2 (proptest): property-based testing
  ├── Exhaustive error variant enumeration
  ├── Registry completeness against master contract
  ├── Serialization round-trip
  ├── Backward compatibility regression
  └── diag_codes.rs/registry synchronization

Layer 3 (cargo-fuzz): adversarial input testing
  └── JSON deserialization of SymbolicCode

Layer 4 (cargo-mutants): mutation resistance
  └── Existing test suite quality verification

Layer 5 (moon ci): aggregate CI gate
  └── All layers executed in CI pipeline
```

---

## 5. Obligation Count

| Verifier | Obligations | Seeds Covered |
|----------|------------|---------------|
| Kani | 15 | PS-001–PS-004, PS-006–PS-008, PS-010–PS-015, PS-017–PS-019 |
| proptest | 10 | PS-001–PS-003, PS-005, PS-006, PS-009, PS-012–PS-016, PS-019, PS-020 |
| cargo-fuzz | 1 | PS-012 |
| cargo-mutants | 1 | Cross-cutting (test suite quality) |
| moon ci gauntlet | 1 | Aggregate (all layers) |
| **Total** | **28** | **All 20 seeds** |

---

## 6. Non-Applicable Lanes

| Lane | Reason | Evidence |
|------|--------|----------|
| TLA+ | No temporal behavior: diagnostic codes are pure lookups. No queues, retries, leases, distributed protocols, or state machines over time. | boundary-map.md §2.1: Pure core has no time, no concurrency. workflow-model.md §7: Code resolution is pure and atomic. hazard-analysis.md §5: No concurrency hazards. |
| Verus | Invariants are const-validated data consistency (registry bijection, uniqueness, non-zero), not algorithmic correctness. Kani handles bounded verification; proptest handles enumeration. Verus would add disproportionate engineering cost for P1 diagnostic infrastructure. | codebase-map.md §3: Diagnostic system is const slices + newtypes. type-contracts.md §11: Compile-time const assertions cover uniqueness, non-zero, category matching. |
| Flux | No complex refinement types. SymbolicCode is a simple newtype over &'static str; DiagnosticCode is newtype over u16. Validation is const lookup, not refinement predicates. | type-contracts.md §1, §3: Both types are repr(transparent) newtypes with smart constructors; no typestate or dependent refinement properties. |
| Loom | No concurrency primitives. CODE_REGISTRY is a const static. All code() methods are pure functions with no mutable state. No atomics, channels, locks, or async code in diagnostic modules. | hazard-analysis.md §5: "No shared mutable state. Thread-safe by construction." Rust core modules use forbid(unsafe_code). |
| Miri | No unsafe code in any diagnostic module. No FFI. No raw pointers. No provenance concerns. SymbolicCode wraps &'static str from string literals. | boundary-map.md §2.5: "Diagnostic code infrastructure contains no unsafe code. No FFI." |

---

## 7. Trusted Base Summary

| Trusted Element | Kind | Reason |
|-----------------|------|--------|
| rustc const-eval engine | external_body | CODE_REGISTRY const assertions rely on Rust's const evaluation being correct |
| serde deserialize framework | external_body | Deserialization round-trip testing assumes serde correctness |
| thiserror derive macro | external_body | Error display strings assume thiserror derive generates correct impls |
| Kani alloc stubs (PS-010) | stub | Zero-allocation proof requires allocator stubs to detect heap allocation paths |
| Existing test fixtures | trusted | Behavior tests (834-line compile_error test, all_variants test) are assumed correct and comprehensive |
| Master contract doc | external_body | Section 16 code list in velvet-ballistics-MASTER.md is treated as ground truth for PS-016 |

---

## 8. Waiver Candidates

| ID | Seed | Reason | Kind |
|----|------|--------|------|
| WVR-PS010-ALLOC | PS-010 | Zero-allocation proof requires Kani allocator stubs (kani::stub for alloc functions). If tooling cannot reliably detect all allocation paths, this becomes a non-behavior performance invariant with compensating manual audit evidence. | Non-behavior (performance). Compensating: compile-time check that no String, Vec, or Box appear in SymbolicCode/DiagnosticCode hot path. |

---

## 9. Blockers

None. All required verifier tooling is available (Kani, proptest, cargo-fuzz, cargo-mutants, moon ci) in the workspace.

---

## 10. Gap Coverage

| Gap ID | Covered By |
|--------|-----------|
| GAP-1 (no unified symbolic code type) | PO-001, PO-002, PO-016 |
| GAP-2 (ValidationError no code() method) | PO-003, PO-017 |
| GAP-3 (YamlError no code support) | PO-006 |
| GAP-4 (incomplete numeric code range) | PO-004, PO-018 |
| GAP-5 (no cross-crate registry) | PO-002, PO-019 |
| GAP-6 (diag_codes.rs test-only) | PO-025 |
