# Error Taxonomy — Digest Coverage of `for_each` Semantics

**Bead:** vb-xi2f.28  
**State:** 3 (rust-contract)  
**Date:** 2026-05-25  
**Status:** DRAFT

---

## 1. Error Categories (Railway Model)

The digest computation (`canonical_digest`) is an **infallible** function — it returns `WorkflowDigest`, not `Result<WorkflowDigest, _>`. Therefore, all errors in this taxonomy are *design-time errors* (bugs/deficiencies), not *runtime errors* that can be caught and handled.

### 1.1 Missing Field Hash Errors

| Code | Error | Severity | Detection | Manifestation |
|---|---|---|---|---|
| **ER-FE-01** | `ForEach.input` not hashed | **P0 - CRITICAL** | Test/proof | `canonical_digest` unchanged when input expression changes |
| **ER-FE-02** | `ForEach.variable` not hashed | **P0 - CRITICAL** | Test/proof | `canonical_digest` unchanged when variable name changes |
| **ER-FE-03** | `ForEach.at_once` not hashed | **P0 - CRITICAL** | Test/proof | `canonical_digest` unchanged when concurrency limit changes |
| **ER-FE-04** | `ForEach.body` not recursively hashed | **P0 - CRITICAL** | Test/proof | `canonical_digest` unchanged when body step content changes |
| **ER-FE-05** | Any field in catch-all primitives not hashed | **P1 - HIGH** | Test/proof | Out of scope for this bead; affects collect, reduce, repeat, etc. |

### 1.2 Duplicate Code Divergence Errors

| Code | Error | Severity | Detection | Manifestation |
|---|---|---|---|---|
| **ER-FE-10** | `digest_step_primitive` in `compile/mod.rs` not updated | **P0 - CRITICAL** | Test/proof | Programmatic compilation path produces different digest from lowering path |
| **ER-FE-11** | `digest_step_primitive` in `mod_compile_lowering/part_05.rs` not updated | **P0 - CRITICAL** | Test/proof | Lowering path produces different digest from programmatic path |
| **ER-FE-12** | Two copies hash ForEach fields in different orders | **P0 - CRITICAL** | Test/proof | Cross-path digest mismatch for identical source |
| **ER-FE-13** | Two copies use different canonical representations (e.g., different at_once encoding) | **P0 - CRITICAL** | Test/proof | Cross-path digest mismatch for identical source |

### 1.3 Non-Deterministic Hashing Errors

| Code | Error | Severity | Detection | Manifestation |
|---|---|---|---|---|
| **ER-FE-20** | Non-deterministic hash input (e.g., pointer addresses, timestamps) | **P0 - CRITICAL** | Kani/proptest | Same source produces different digests across runs |
| **ER-FE-21** | Hash influenced by non-source data (e.g., memory layout, iteration order of HashSet) | **P0 - CRITICAL** | Kani/proptest | Non-reproducible digests |
| **ER-FE-22** | Platform-dependent encoding (e.g., native endianness for non-u32 fields) | **P1 - HIGH** | Cross-platform CI | Different digests on big-endian vs little-endian systems |

### 1.4 Hash Collision / Boundary Errors

| Code | Error | Severity | Detection | Manifestation |
|---|---|---|---|---|
| **ER-FE-30** | Field boundary ambiguity: `hash("a" + "b") == hash("ab" + "")` | **P0 - CRITICAL** | Test/proof | False hash collision: semantically different inputs produce identical hashes |
| **ER-FE-31** | Missing field delimiter: `hash("input:a")` vs. input value starting with `":"` | **P2 - LOW** | Analysis only | Extremely unlikely ambiguous boundary (YAML identifiers do not contain `":"`) |
| **ER-FE-32** | BLAKE3 collision (cryptographic) | **P3 - NEGLIGIBLE** | N/A | Astronomical probability; BLAKE3's 128-bit security target makes this impractical |

### 1.5 Test/Proof Deficiency Errors

| Code | Error | Severity | Detection | Manifestation |
|---|---|---|---|---|
| **ER-FE-40** | No test verifies that changing ForEach.input changes digest | **P1 - HIGH** | Test gap analysis | Regression: future code changes could reintroduce the gap |
| **ER-FE-41** | No test verifies both compilation paths produce identical digests | **P1 - HIGH** | Test gap analysis | Duplicate code can silently diverge |
| **ER-FE-42** | Determinism tests use only name-level differences, not field-level | **P2 - MEDIUM** | Test gap analysis | Partial coverage; field-level aliasing goes undetected |

---

## 2. Railway Error Flow (Design-Time)

```
                    ┌──────────────────┐
                    │   Source YAML    │
                    └──────┬───────────┘
                           │
                           ▼
              ┌────────────────────────┐
              │  canonical_digest()    │
              │  (infallible pipeline) │
              └──────┬─────────────────┘
                     │
                     ├── [ERROR CLASS: Missing Field Hash]
                     │   └── digest produced but incomplete
                     │       → Silent aliasing (P0)
                     │
                     ├── [ERROR CLASS: Duplicate Divergence]
                     │   └── digest produced but inconsistent across paths
                     │       → Cross-path mismatch (P0)
                     │
                     ├── [ERROR CLASS: Non-Deterministic]
                     │   └── digest produced but unreproducible
                     │       → Run-to-run mismatch (P0)
                     │
                     └── [SUCCESS: Complete, deterministic digest]
```

**Key observation:** None of these errors can be caught at the `canonical_digest` return site because the function is infallible. They must be caught at **design time** (type contracts), **test time** (sensitivity tests), or **proof time** (Kani/proptest verification).

---

## 3. Error Prevention Strategy

| Error Class | Prevention Mechanism | Verification Method |
|---|---|---|
| Missing Field Hash | Type contract (exhaustive match on ForEach fields) | Proptest: random ForEach variations → digest uniqueness |
| Duplicate Divergence | Identical implementation in both copies (or single source of truth) | Cross-path integration test: same source through both paths → same digest |
| Non-Deterministic | No non-source data in hash input | Kani: bounded verification of deterministic execution |
| Field Boundary Ambiguity | Canonical delimiters | Unit test: adjacent-field collision vectors |
