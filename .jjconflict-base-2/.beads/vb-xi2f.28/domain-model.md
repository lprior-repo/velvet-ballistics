# Domain Model — Digest Coverage of `for_each` Semantics

**Bead:** vb-xi2f.28  
**State:** 3 (rust-contract)  
**Date:** 2026-05-25  
**Status:** DRAFT

---

## 1. Ubiquitous Language

| Term | Definition |
|---|---|
| **WorkflowDigest** | A 32-byte BLAKE3 content hash that uniquely identifies a workflow's semantic content. Typed as `WorkflowDigest([u8; 32])` with `from_bytes()`/`as_bytes()`. Lives in `vb_core::ids`. |
| **canonical_digest** | The pure function `canonical_digest(source: &WorkflowSource) -> WorkflowDigest` that computes a deterministic BLAKE3 hash over the workflow source YAML structure content. The result is stored in `WorkflowParts.digest` and returned by `CompiledWorkflow.digest()`. |
| **digest_step_primitive** | The dispatch function `digest_step_primitive(hasher: &mut blake3::Hasher, primitive: &StepPrimitive)` that feeds primitive-specific field content into the accumulator hasher. |
| **StepPrimitive::ForEach** | The YAML AST variant representing a parallel fan-out construct: `ForEach { variable: String, input: String, at_once: Option<u32>, body: Vec<StepAst> }`. |
| **ForEach digest sensitivity** | The property that the canonical digest MUST change when any ForEach field (`variable`, `input`, `at_once`, `body`) changes. This is a sub-property of digest *completeness*. |
| **Digest completeness** | The property that every semantically significant field of every step primitive is hashed by `digest_step_primitive`. The absence of this property creates *digest coverage gaps*. |
| **Digest determinism** | The property that identical source produces identical digest across compilations, machines, and process restarts. |
| **Digest distinctness** | The property that semantically different sources produce different digests (modulo BLAKE3 collision resistance). |
| **Digest coverage gap** | A field or set of fields in a step primitive that does NOT contribute to the canonical digest but SHOULD. The current gap: all ForEach fields except the primitive name string. |
| **compute_compiled_digest** | A separate byte-level digest `compute_compiled_digest(source: &[u8]) -> WorkflowDigest` that hashes the entire postcard-serialized `WorkflowParts` byte array. This is artifact-level, not source-level. It IS sensitive to ForEach fields (since they are embedded in the IR) but operates at a different abstraction layer. |

---

## 2. Entities & Value Objects

### 2.1 Core Entity: WorkflowDigest

```
WorkflowDigest([u8; 32])
  - INVARIANT: from_bytes(blake3::hash(x)) uniquely identifies x (collision-resistant)
  - Derives: Debug, Clone, Copy, PartialEq, Eq, Hash
  - Serialized via serde as 32-byte array
```

`WorkflowDigest` is a **value object** with structural equality. It wraps a raw `[u8; 32]` behind a `#[repr(transparent)]` newtype. No semantic operations beyond construction and byte access.

### 2.2 Core Entity: canonical_digest Function

```
canonical_digest: &WorkflowSource → WorkflowDigest
  - Pure function (no I/O, no randomness, no time)
  - Deterministic: f(x) = f(x) always
  - Composed from:
      1. hasher.update(version)
      2. hasher.update(name)
      3. hasher.update(trigger fields)
      4. for each step: hasher.update(step.id); digest_step_primitive(step.primitive)
      5. finalize → [u8; 32] → WorkflowDigest
```

### 2.3 Aggregate: StepPrimitive::ForEach

```
ForEach {
    variable: String,      // Loop variable name (e.g., "item")
    input: String,         // Input collection expression (e.g., "items_list")
    at_once: Option<u32>,  // Max concurrency limit; None defaults to 1 in lowering
    body: Vec<StepAst>,    // Body steps executed per item
}
```

**Digest-relevant fields:**
- `variable`: MUST be hashed (name → byte representation)
- `input`: MUST be hashed (expression → byte representation)
- `at_once`: MUST be hashed as u32 bytes in canonical representation (0 when None, or the actual value when Some)
- `body`: MUST be recursively hashed — each body `StepAst` contributes its `id` and `digest_step_primitive`

**Fields NOT digest-relevant (not present in ForEach AST):**
- `item_slot`: this is a compilation artifact (SlotIdx), not part of source. Derived from canonical layout during lowering.
- `done_target` / `body_step_idx` / `next_step_idx`: these are compilation artifacts computed from canonical step offsets.

---

## 3. Domain Invariants

| ID | Invariant | Category | Impact if violated |
|---|---|---|---|
| **INV-FE-01** | `canonical_digest` is deterministic | Determinism | Same source compiles to different digests across runs/processes |
| **INV-FE-02** | Changing `ForEach.variable` changes `canonical_digest` | Sensitivity | Two workflows with same inputs/body but different variable names produce identical digests |
| **INV-FE-03** | Changing `ForEach.input` changes `canonical_digest` | Sensitivity | Two workflows iterating over different collections produce identical digests |
| **INV-FE-04** | Changing `ForEach.at_once` changes `canonical_digest` | Sensitivity | Two workflows with different concurrency limits produce identical digests |
| **INV-FE-05** | Changing `ForEach.body` (any text in any body step) changes `canonical_digest` | Sensitivity | Two workflows with different body logic produce identical digests |
| **INV-FE-06** | Both copies of `canonical_digest` (compile/mod.rs and part_05.rs) produce identical results for identical input | Duplicate equivalence | Different compilation paths produce different digests for same source |

---

## 4. Illegal States Made Representable (Current)

The current code does NOT make the following illegal states unrepresentable:

1. **ForEach digest aliasing:** `canonical_digest(wf1) == canonical_digest(wf2)` where wf1 and wf2 differ only in `ForEach.input` — this is currently representable because `digest_step_primitive` only hashes the name `"for_each"`.

2. **Silent ForEach changes:** Any modification to ForEach fields (input, variable, at_once, body) produces zero digest change. The digest does not reflect the modification.

3. **Duplicate code divergence:** If `compile/mod.rs::digest_step_primitive` and `mod_compile_lowering/part_05::digest_step_primitive` are updated inconsistently, the two compilation paths produce different digests for the same source.

---

## 5. Domain Decisions

| Decision | Choice | Rationale |
|---|---|---|
| **DD-01: Hash scope** | This bead covers ForEach only. Other primitives with identical catch-all gaps (collect, reduce, repeat, parallel, wait, ask) are out of scope. | Bead scope is explicit. Identical fix pattern applies to all. |
| **DD-02: Digest level** | Fix applies to `canonical_digest` (source-level). `compute_compiled_digest` (artifact-level) is already correct. | `canonical_digest` is what's stored in `WorkflowParts.digest` and used for admission/recovery. |
| **DD-03: Both copies** | Both copies of `canonical_digest` in `compile/mod.rs` and `mod_compile_lowering/part_05.rs` must be updated. | Both are in-use code paths. Failing to update one creates a divergent digest. |
| **DD-04: Canonical representation** | `at_once` is hashed as zero when `None`, and as the u32 value in little-endian bytes when `Some(v)`. Auxiliary fields are *not* included (item_slot, body_step, done_target — these are compilation artifacts, not source fields). | Canonical digest hashes source content, not derived compilation artifacts. |
| **DD-05: Body recursion** | ForEach body steps are recursively hashed by calling `digest_step_primitive` on each body `StepAst`. | Bodies are nested workflows; the digest must reflect their full content. |
