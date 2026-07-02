# Domain Model — vb-xi2f.34: Finish Digest Semantics

**Bead**: vb-xi2f.34
**Date**: 2026-05-24
**Rust Contract Agent**: rust-contract

---

## Ubiquitous Language

| Term | Definition |
|---|---|
| **WorkflowDigest** | A 32-byte blake3 structural hash that fingerprints a compiled workflow's source semantics. Two workflows with equivalent source semantics MUST produce identical digests; workflows that differ in any digest-covered field MUST produce different digests. |
| **Structural Digest** | Hash computed from typed AST fields (`WorkflowSource`), NOT from raw source bytes. Equivalent source text that normalizes differently in the AST (e.g., numeric `1` vs `1.0`) is out of scope for this contract — only AST-level changes matter. |
| **Raw Digest** | Hash of raw source bytes via `compute_compiled_digest()`. Distinct contract; NOT the digest stored in `CompiledWorkflow.digest`. Used for compiled artifact identity but not for semantic fingerprinting. |
| **Digest-Covered Fields** | The set of AST fields that MUST appear in the structural digest. Currently: `version`, `name`, `trigger` (type + params), each step's `id`, and each step's primitive (type-specific fields). |
| **Finish Primitive** | The terminal step in a compiled workflow. Its `result: ScalarValue` field selects the output slot returned to the caller. |
| **ScalarValue** | A `#[non_exhaustive]` enum representing a compile-time scalar literal: `String(String)` or `Integer(i64)`. |
| **Hash Discriminator** | A typed prefix or encoding that prevents distinct `ScalarValue` variants from producing identical hash inputs. E.g., `Integer(42)` hashes differently from `String("42")`. |
| **Canonical Path** | The active compilation route: `mod_compile_lowering::compile_source()` → `part_05::canonical_digest()`. |
| **Legacy Path** | The duplicate route in `compile/mod.rs::compile_source()` → `mod.rs::canonical_digest()`. Present in code but not the primary flow. |

---

## Entities & Value Objects

### Entity: Digest Computation Context

Owns the blake3 hasher and the deterministic feed sequence. Stateless — pure function from `WorkflowSource` → `WorkflowDigest`.

### Value Object: `WorkflowDigest`

```rust
#[repr(transparent)]
pub struct WorkflowDigest([u8; 32]);
```

- Identifies `CompiledWorkflow` at compile time.
- Immutable after construction.
- Equality is byte-equality.
- Serialized as part of `WorkflowParts` (Postcard) for persistence/recovery.

### Value Object: `ScalarValue`

```rust
#[non_exhaustive]
pub enum ScalarValue {
    String(String),
    Integer(i64),
}
```

- `#[non_exhaustive]` — future variants MUST be added.
- Used in `Finish { result: ScalarValue }` and no other step primitive.
- `String` variant: an output name reference resolved via output map.
- `Integer` variant: a direct slot index.

### Aggregate Root: `WorkflowSource`

The parsed YAML AST. Contains ordered steps, each with `id: String` and `primitive: StepPrimitive`. The digest is computed from this aggregate.

### Aggregate Root: `CompiledWorkflow`

The validated, bounded IR. Holds `digest: WorkflowDigest` at field level. The digest is set once during construction and never modified.

---

## Commands

| Command | Precondition | Postcondition |
|---|---|---|
| `canonical_digest(source)` | `source` is a valid parsed `WorkflowSource` | Returns a deterministic `WorkflowDigest` covering version, name, trigger, step IDs, and step primitives |
| `digest_step_primitive(hasher, primitive)` | `hasher` is mutable, `primitive` is a parsed `StepPrimitive` | Hasher state is updated with discriminator + type-specific fields |
| `canonical_finish_slot(result, outputs)` | `result: ScalarValue`, `outputs` maps String names to SlotIdx | Returns the resolved `SlotIdx` for the finish result |
| `lower_finish(id, slot, builder)` | `id: StepIdx`, `slot: SlotIdx` validated | Creates and pushes a `CompiledNodeKind::Finish { result: slot }` |

---

## Core Invariants

### INV-1: Digest is deterministic
`canonical_digest(source)` always returns the same digest for structurally identical `WorkflowSource` values.

### INV-2: Digest is sensitive to all covered fields
Any change to a digest-covered field (version, name, trigger, step id, step primitive discriminator or value) MUST produce a different digest. The digest function MUST NOT silently ignore any covered field.

### INV-3: Finish result is digest-covered
The `result: ScalarValue` field of the `Finish` primitive MUST be hashed into the digest. Changing the finish result value or type MUST produce a different digest.

### INV-4: Hash discrimination by ScalarValue variant
- `ScalarValue::String(s)` → `hasher.update(b"finish"); hasher.update(s.as_bytes())`
- `ScalarValue::Integer(i)` → `hasher.update(b"finish"); hasher.update(&i.to_le_bytes())`
- Other variants → `hasher.update(b"finish"); hasher.update(b"unsupported")`

The encoding of `String` vs `Integer` produces different byte sequences for the same logical value (e.g., `"42"` vs `42i64`), ensuring variant-level discrimination.

### INV-5: Digest is computed before IR lowering
`canonical_digest()` takes `WorkflowSource` (AST), not compiled IR. The digest does not depend on slot layout, constant pool, or bytecode — only on the source AST.

### INV-6: Digest survives round-trip validation
A digest computed from a `WorkflowSource` that compiles successfully MUST match the digest in the resulting `CompiledWorkflow.digest()` field.

### INV-7: Single source of truth for digest computation
There MUST be exactly one canonical implementation of the digest algorithm. Any duplicate implementation represents a divergence risk.

---

## Forbidden States

1. **Digest unchanged after finish result value change**: A workflow with `finish: "output_a"` and a workflow with `finish: "output_b"` MUST NOT produce the same digest.
2. **Digest unchanged after finish result type change**: `finish: "42"` (String) and `finish: 42` (Integer) MUST NOT produce the same digest.
3. **Digest unchanged after finish step ID change**: Renaming the finish step's `id` field MUST change the digest (step IDs are hashed at line 134 of `part_05.rs`).
4. **Legacy/canonical divergence**: The legacy `compile/mod.rs::canonical_digest()` and the canonical `mod_compile_lowering/part_05.rs::canonical_digest()` MUST produce identical digests for the same input.
5. **Silent fallthrough on unknown ScalarValue**: When a new `ScalarValue` variant is added, the `_ => hasher.update(b"unsupported")` arm produces a lossy hash that does not encode the variant's value. This is a valid compile-time behavior but MUST be documented and tested.
6. **Digest computed from raw bytes mistaken for structural digest**: `compute_compiled_digest()` (raw blake3 of source bytes) MUST NOT be confused with `canonical_digest()` (structural hash).

---

## Open Domain Questions

1. **Should the `_` arm in `digest_step_primitive` reject unknown ScalarValue variants at compile time instead of silently producing `"unsupported"`?** Currently it silently produces a hash that does not distinguish different unsupported values. A compile error would force the developer to explicitly handle new variants.

2. **Should `canonical_primitive_name()` return correct names for `Together` ("parallel" → "together") and `Aggregate` ("aggregate" → "reduce")?** These are known bugs but do not affect digest computation for `Finish` since `Finish` has its own match arm.

3. **Should the legacy path (`compile/mod.rs`) be removed?** It duplicates `canonical_digest()` and `digest_step_primitive()` with subtle differences (missing `_` arm in `digest_step_primitive`, missing `_` arm in trigger match). Codebase-map.md notes it is "used by proptest helpers but not by the main compilation flow."
