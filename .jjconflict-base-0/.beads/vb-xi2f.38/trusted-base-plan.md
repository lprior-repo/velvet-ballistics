# Trusted Base Plan: vb-xi2f.38

## Purpose
Document all explicitly trusted surfaces, assumed bounds, model reductions, and stub justifications for the proof obligations in vb-xi2f.38. This provides a machine-readable record of what the proof depends on without proof.

---

## Trusted Surfaces (Trusted Without Proof)

### T1: BLAKE3-256 Hash Function
- **What**: `blake3::Hasher` — 256-bit cryptographic hash
- **Why trusted**: BLAKE3 is a vetted, widely-used hash function with 256-bit output. No custom crypto.
- **Evidence**: Standard library assumption; collision resistance is computational
- **Boundary**: `digest_step_primitive` and `compute_compiled_digest` use BLAKE3

### T2: postcard::serialize Determinism
- **What**: `postcard::serialize::<WorkflowParts>` produces deterministic CBOR bytes
- **Why trusted**: postcard is a well-tested serde-based serializer; CBOR encoding is deterministic for the types in `WorkflowParts`
- **Evidence**: `crates/vb_compile/src/tests/error_variant_tests.rs` lines 762–801 test determinism
- **Boundary**: `compute_compiled_digest` serialization step

### T3: String::as_bytes Deterministic UTF-8
- **What**: `String::as_bytes()` returns deterministic UTF-8 byte representation
- **Why trusted**: Rust String is guaranteed to store valid UTF-8; `as_bytes()` is a direct view
- **Evidence**: Rust language guarantee
- **Boundary**: All string field hashing in `digest_step_primitive`

### T4: u32::to_le_bytes Deterministic Encoding
- **What**: `u32::to_le_bytes()` produces deterministic little-endian byte representation
- **Why trusted**: Rust integer encoding is specified; little-endian is deterministic for fixed-width integers
- **Evidence**: Rust language guarantee
- **Boundary**: `pages: Option<u32>` and `items: Option<u32>` serialization in `Collect` digest

### T5: Vec<StepAst> Iteration Determinism
- **What**: Iterating `Vec<StepAst>` via `for step in body` visits elements in insertion order
- **Why trusted**: Rust Vec guarantees iteration in insertion order for types without custom iterators
- **Evidence**: Rust std library guarantee
- **Boundary**: Body recursive hashing in `digest_step_primitive` for `Collect`

### T6: Step ID Uniqueness Validation
- **What**: Workflow validation rejects duplicate step IDs before digest computation
- **Why trusted**: `vb_validate::shared::validate_id_uniqueness` is a validation gate
- **Evidence**: Delivery scope references `vb_validate/src/gates.rs` for duplicate ID checks
- **Boundary**: Digest computation assumes unique step IDs

### T7: Bounded Workflow Compilation Limits
- **What**: Workflow compilation enforces maximum limits on steps, body size, string lengths
- **Why trusted**: Compilation pipeline rejects workflows exceeding limits via `CompileErrors`
- **Evidence**: Compilation limits are enforced by the parser and validator
- **Boundary**: All proof obligations bound to "≤ N steps", "≤ M body steps", "≤ L string length"

---

## Explicitly Assumed Bounds

### B1: Bounded Workflow Steps
- **Bound**: 1 ≤ `WorkflowSource.steps().len()` ≤ 20
- **Evidence**: Compilation pipeline limits; TLA+ model uses `1..20` bound
- **Used in**: PO-001, PO-008, PO-017

### B2: Bounded Collect Body Steps
- **Bound**: 0 ≤ `Collect.body.len()` ≤ 10
- **Evidence**: Compilation pipeline limits; Kani harness uses `0..8` (conservative)
- **Used in**: PO-001, PO-002, PO-007, PO-011, PO-012, PO-017

### B3: Bounded String Lengths
- **Bound**: 0 ≤ `Collect.variable.len()` ≤ 64, 0 ≤ `Collect.source.len()` ≤ 64
- **Evidence**: Kani Shrinkable implementation bounds; TLA+ model uses `0..256`
- **Used in**: PO-002, PO-003, PO-004, PO-013, PO-015, PO-016

### B4: Bounded Collect Pages
- **Bound**: 0 ≤ `Collect.pages.unwrap_or(0)` ≤ 100
- **Evidence**: Kani harness uses `0..100`; TLA+ model uses `0..100`
- **Used in**: PO-002, PO-005, PO-012

### B5: Bounded Collect Items
- **Bound**: 0 ≤ `Collect.items.unwrap_or(0)` ≤ 1000
- **Evidence**: Kani harness uses `0..1000`; TLA+ model uses `0..1000`
- **Used in**: PO-002, PO-006, PO-012

### B6: Bounded Step ID Length
- **Bound**: 1 ≤ `step.id.len()` ≤ 64
- **Evidence**: Validation limits; TLA+ model uses `1..64`
- **Used in**: PO-008

### B7: Bounded ForEach Body
- **Bound**: 0 ≤ `ForEach.body.len()` ≤ 8
- **Evidence**: Kani harness uses `0..8`
- **Used in**: PO-015

### B8: Bounded Aggregate Body
- **Bound**: 0 ≤ `Aggregate.body.len()` ≤ 8
- **Evidence**: Kani harness uses `0..8`
- **Used in**: PO-016

---

## Model Reductions

### R1: Single-Threaded Digest Computation
- **Reduction**: `digest_step_primitive` is modeled as atomic sequential execution
- **Justification**: No concurrent calls to `digest_step_primitive`; pure sequential function
- **Not modeled**: Any concurrent interleavings (Loom not applicable)

### R2: No I/O in Digest Path
- **Reduction**: `canonical_digest` and `digest_step_primitive` are modeled as pure functions
- **Justification**: No storage, network, file I/O, randomness, or time in digest computation
- **Not modeled**: Side channels, timing attacks (not in scope)

### R3: Deterministic Option Serialization
- **Reduction**: `Option<u32>` is serialized as `0u32.to_le_bytes()` for `None`, `p.to_le_bytes()` for `Some(p)`
- **Justification**: Contract specifies this deterministic mapping; no undefined behavior
- **Not modeled**: Alternative serialization strategies

### R4: Depth-First Body Hashing
- **Reduction**: Body steps are hashed depth-first (each step's primitive is fully hashed before moving to next step)
- **Justification**: Implementation iterates `for step in body { hasher.update(step.id); digest_step_primitive(hasher, &step.primitive); }`
- **Not modeled**: Breadth-first or parallel hashing strategies

---

## Stub Justifications

### S1: kani::any::<StepPrimitive::Collect>() Generates Well-Formed Collect
- **Stub**: Arbitrary generation of `StepPrimitive::Collect`
- **Justification**: Kani Shrinkable trait bounds the generated values to parser-valid ranges
- **Risk if wrong**: Proof结论 could be unsound (showing digest differences for out-of-range values that never occur)
- **Mitigation**: Shrinkable bounds match compilation limits

### S2: proptest::arbitrary::StepPrimitiveCollect Generates Well-Formed Collect Pairs
- **Stub**: Property-based generation of Collect pairs
- **Justification**: Proptest arbitrary implementation is bounded by the same compilation limits
- **Risk if wrong**: Property test结论 could miss edge cases outside valid input space
- **Mitigation**: Arbitrary implementation is reviewed alongside proof obligations

### S3: canonical_primitive_name Returns Correct Static Strings
- **Stub**: `canonical_primitive_name` returns correct primitive name strings
- **Justification**: Verified by existing Kani harness `kani_canonical_name.rs` (vb-xi2f.16)
- **Risk if wrong**: Digest would use wrong type tags, breaking content-addressing
- **Mitigation**: Separate Kani harness proves canonical_primitive_name correctness

---

## Waived Obligations (None)

No behavior-affecting waivers are proposed for vb-xi2f.38. All verifier lanes are applicable.

---

## Accumulated Trusted Base

| ID | Description | Evidence | Used in |
|----|-------------|----------|---------|
| T1 | BLAKE3-256 | Standard library | All digest obligations |
| T2 | postcard determinism | error_variant_tests.rs:762-801 | PO-010, PO-018 |
| T3 | String::as_bytes | Rust guarantee | All string hashing |
| T4 | u32::to_le_bytes | Rust guarantee | PO-005, PO-006 |
| T5 | Vec iteration order | Rust guarantee | PO-007, PO-017 |
| T6 | Step ID uniqueness | Validation gate | PO-008 |
| T7 | Compilation limits | CompileErrors | All obligations |
| B1-B8 | Explicit bounds | Compilation limits | All obligations |
| R1-R4 | Model reductions | Code inspection | TLA+ model |
| S1-S3 | Stub justifications | Existing verification | Kani/Proptest |

---

## Summary

The trusted base for vb-xi2f.38 consists of:
- **7 trusted surfaces** (standard library, vetted crates, language guarantees)
- **8 explicitly assumed bounds** (compilation limits, parser limits)
- **4 model reductions** (sequential, pure, deterministic Option, depth-first)
- **3 stub justifications** (arbitrary generation, canonical_primitive_name)

No waivers are proposed. All dependencies are either standard library, vetted crates, or explicitly bounded by compilation limits.
