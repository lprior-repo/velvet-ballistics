# Type Contracts — vb-xi2f.34: Finish Digest Semantics

**Bead**: vb-xi2f.34
**Date**: 2026-05-24
**Rust Contract Agent**: rust-contract

---

## Existing Types Under Contract

### `WorkflowDigest` (`vb_core::ids`)

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct WorkflowDigest([u8; 32]);
```

**Smart Constructors**:
- `from_bytes(bytes: [u8; 32]) -> Self` — accepts any 32 bytes; no validation needed (hash output is always 32 bytes).
- `as_bytes(self) -> [u8; 32]` — accessor.

**Contract**:
- `Eq` is structural byte equality.
- `Hash` delegates to the inner `[u8; 32]`.
- No sentinel or zero value — the zero digest is a valid digest.
- Used as an opaque identity token; consumers MUST NOT interpret individual bytes.

**Illegal states prevented**: None currently. A 32-byte `[u8; 32]` wrapper is sound. The only risk is using a digest from the wrong computation function.

**Contract gap**: The type does not distinguish between a structural digest (from `canonical_digest()`) and a raw digest (from `compute_compiled_digest()`). Both produce `WorkflowDigest`. This is a domain-level distinction that must be enforced by the caller, not the type system.

---

### `ScalarValue` (`vb_yaml::ast`)

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ScalarValue {
    String(String),
    Integer(i64),
}
```

**Contract for digest**:
- `String(s)`: The UTF-8 bytes `s.as_bytes()` are the hash input.
- `Integer(i)`: The 8-byte little-endian encoding `i.to_le_bytes()` is the hash input.
- Future variants: The `#[non_exhaustive]` attribute requires downstream (compile-layer) match arms to handle unknown variants. In `digest_step_primitive`, the `_` arm falls through to `"unsupported"`.

**Hash collision analysis**:
- `String("42")` → bytes `[52, 50]` (two bytes)
- `Integer(42)` → bytes `[42, 0, 0, 0, 0, 0, 0, 0]` (eight bytes)
- Different length + content → no collision possible between these variants.
- `String("")` → empty byte slice `[]`. This is valid and produces a well-defined hash input.
- `Integer(0)` → bytes `[0; 8]`. Different from `String("")`.
- `Integer(i)` and `Integer(-i)` following Two's Complement will produce different LE bytes (e.g., `-1` = `[255, 255, 255, 255, 255, 255, 255, 255]`).

**Contract gap**: The `_` arm in `digest_step_primitive` produces `"unsupported"` for all future variants. This means two different future `ScalarValue` variants (e.g., hypothetical `Bool(true)` and `Bool(false)`) would produce identical hash inputs. This is a deliberate design choice for forward compatibility but represents a latent collision hazard.

**Refinement**: A compile-time exhaustiveness check (or test) SHOULD verify that all current `ScalarValue` variants are explicitly handled in `digest_step_primitive`'s inner match. Currently only `String` and `Integer` exist, and both are handled. When a new variant is added, `#[non_exhaustive]` will force the outer `StepPrimitive` match to recompile, but the inner `ScalarValue` match in `digest_step_primitive` will silently fall through to `_`.

---

### `StepPrimitive::Finish` (`vb_yaml::ast`)

```rust
pub enum StepPrimitive {
    // ...
    Finish { result: ScalarValue },
    // ...
}
```

**Contract**:
- `Finish` is terminal: it MUST be the last step in the workflow.
- `result: ScalarValue` is the output slot selector.
- In the digest: `Finish` is identified by the prefix `b"finish"`, followed by the variant-specific encoding of `result`.

**Illegal states prevented by type**:
- `Finish` without a result is impossible (the `result` field is mandatory).
- `Finish` with multiple results is impossible (single field).

**Illegal states NOT prevented by type**:
- `Finish` at a non-terminal position — validated at compile time, not by type.
- `Finish` with an invalid output name — validated by `canonical_finish_slot()` returning `Err(UnknownOutputName)`.

---

### `WorkflowParts` (`vb_core::workflow`)

```rust
pub struct WorkflowParts {
    pub name: Box<str>,
    pub digest: WorkflowDigest,      // <-- structural digest
    pub nodes: Box<[CompiledNode]>,
    pub expressions: Box<[ExprProgram]>,
    pub accessors: Box<[AccessorProgram]>,
    pub constants: Box<[ConstValue]>,
    pub slot_count: u16,
    pub symbols_count: u32,
    pub entry: StepIdx,
    pub resource_contract: ResourceContract,
    pub step_names: Box<[Box<str>]>,
}
```

**Type contract for `digest` field**:
- Set once during construction; never mutated.
- MUST be the output of `canonical_digest(&WorkflowSource)`.
- Passed through `vb_validate::shared::validate()` unchanged.
- Passed through `CompiledWorkflow::try_from_parts()` unchanged.
- Retrieved via `CompiledWorkflow::digest()`.

**Illegal states prevented**: `digest` is a required field — a `WorkflowParts` without a digest cannot exist.

**Contract gap**: Nothing in the type system prevents constructing a `WorkflowParts` with an arbitrary `WorkflowDigest` that does not match the source. The correct digest MUST be computed by the caller (`compile_source` in either path). Currently no cross-validation checks that `WorkflowParts.digest` matches a re-computation from the source.

---

### `CompiledNodeKind::Finish` (`vb_core::workflow`)

```rust
pub enum CompiledNodeKind {
    // ...
    Finish { result: SlotIdx },
    CollectFinish { collector_slot: SlotIdx },
    ReduceFinish { accumulator: SlotIdx },
    RepeatFinish { result: SlotIdx },
    // ...
}
```

**Contract**:
- `result: SlotIdx` is validated by `validate_parts()` at construction: `validate_slot(*result, parts.slot_count)`.
- The `SlotIdx` is resolved from `ScalarValue` by `canonical_finish_slot()` during lowering.
- At runtime, the finish node terminates execution and returns the value in slot `result`.

**Type safety**: `SlotIdx` is a newtype over `u16`, preventing confusion with `StepIdx`, `ConstIdx`, etc.

---

## Duplicate Code Contract

### Canonical Path

- **Location**: `crates/vb_compile/src/mod_compile_lowering/part_05.rs`
- **Functions**: `canonical_digest()` (line 116), `digest_step_primitive()` (line 140)
- **Status**: ACTIVE — used by `compile_source()` in `part_01.rs`

### Legacy Path

- **Location**: `crates/vb_compile/src/compile/mod.rs`
- **Functions**: `canonical_digest()` (line 220), `digest_step_primitive()` (line 243)
- **Status**: LEGACY — used by proptest helpers only

### Structural Differences (present in source)

| Feature | Canonical Path (`part_05.rs`) | Legacy Path (`mod.rs`) | Risk |
|---|---|---|---|
| Trigger `_` arm | `_ => hasher.update(b"unknown")` | Absent (non-exhaustive match) | Legacy won't compile if new trigger variant added |
| `Finish` ScalarValue `_` arm | `_ => hasher.update(b"unsupported")` | Absent (non-exhaustive inner match) | Legacy won't compile if new ScalarValue variant added |
| `Set` arm | Present (lines 145-149) | Present (lines 245-249) | Identical (low risk) |
| `Finish` arm | Present (lines 150-156) | Present (lines 250-255) | Near-identical (canonical has `_` arm) |
| `other` arm | `canonical_primitive_name(other)` | `canonical_primitive_name(other)` | Identical |
| Function visibility | `pub(super)` | Private (`fn`) | Different visibility |

### Consolidation Contract

**CLAIM-1: Behavioral equivalence (current state)**
For the current set of `ScalarValue` variants (`String`, `Integer`) and `TriggerAst` variants (`Manual`, `Schedule`, `Event`, `Webhook`), both paths produce identical digests for identical inputs.

**CLAIM-2: Future-proof divergence risk**
When a new `ScalarValue` variant is added:
- The canonical path silently produces `"unsupported"` in the hash (lossy but compiles).
- The legacy path fails to compile (non-exhaustive match error).
- This is a safe divergence: the legacy path breaks loudly, forcing update.

When a new `TriggerAst` variant is added:
- The canonical path silently produces `"unknown"` in the hash (lossy but compiles).
- The legacy path fails to compile (non-exhaustive match error).
- This is a safe divergence: the legacy path breaks loudly, forcing update.

**CLAIM-3: Consolidation obligation**
If the legacy path is retained, a test MUST verify that both `canonical_digest()` functions produce identical output for all valid inputs. If the legacy path is removed, no test is needed.

---

## Parser Boundary Contract

### `WorkflowSource` (the AST)

- `version()` → `&str` — workflow language version. Hashed as UTF-8 bytes.
- `name()` → `&str` — workflow name. Hashed as UTF-8 bytes.
- `trigger()` → `TriggerAst` — trigger type + params. Hashed as discriminator + params.
- `steps()` → `&[StepAst]` — ordered step list. Each step's `id` and `primitive` are hashed.

**Contract**: The YAML parser (`vb_yaml`) is responsible for producing a valid `WorkflowSource`. The digest computation trusts the parser — it does not re-validate AST structure. All validation of step ordering, finish position, etc., happens in the lowering phase, after digest computation.

---

## Type-Level Gaps (Illegal States Still Representable)

| Gap | Description | Mitigation |
|---|---|---|
| GAP-1: Digest/IR mismatch | `WorkflowParts.digest` can be set to any `WorkflowDigest`, not necessarily one computed from the source | Cross-validation test or re-computation check |
| GAP-2: `ScalarValue` hashCode ambiguity | `_` arm produces `"unsupported"` for all future variants | Exhaustiveness test; consider making it a compile error |
| GAP-3: Duplicate digest functions | Two implementations of `canonical_digest()` can diverge | Consolidate to one function; or equivalence test |
| GAP-4: `canonical_primitive_name` bugs | `Together` → `"parallel"`, `Aggregate` → `"aggregate"` | Not a digest concern (Finish bypasses this function), but a correctness concern |
