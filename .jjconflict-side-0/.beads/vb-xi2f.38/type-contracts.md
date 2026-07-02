# Type Contracts: Digest Covers Collect Semantics (vb-xi2f.38)

## Contract: WorkflowDigest

### Type Definition
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(transparent)]
pub struct WorkflowDigest([u8; 32]);
```

### Construction
- `WorkflowDigest::from_bytes(bytes: [u8; 32]) -> Self` — wraps raw bytes; caller guarantees valid BLAKE3 output
- `WorkflowDigest::from_bytes(hasher.finalize().into())` — standard construction path

### Behavioral Contract
- **Pure**: same input bytes ALWAYS produce same `WorkflowDigest`
- **Content-addressed**: `digest_a == digest_b` implies `digest_a` bytes == `digest_b` bytes (modulo BLAKE3 collision)
- **No nil digest**: `WorkflowDigest` has no special "null" value semantics; all 32-byte values are valid

### Smart Constructor
```rust
impl WorkflowDigest {
    pub const fn from_bytes(bytes: [u8; 32]) -> Self { Self(bytes) }
    pub const fn as_bytes(self) -> [u8; 32] { self.0 }
}
```

---

## Contract: digest_step_primitive

### Current (Buggy) Signature
```rust
fn digest_step_primitive(hasher: &mut blake3::Hasher, primitive: &vb_yaml::ast::StepPrimitive)
```

### Behavioral Contract (Current Bug)
**INCORRECT**: The function uses `canonical_primitive_name` for `Collect`, which only hashes the static string `"collect"`.

**REQUIRED CORRECT BEHAVIOR**: For `StepPrimitive::Collect`, the hasher MUST incorporate:
1. `b"collect"` — the primitive name tag
2. `variable.as_bytes()` — loop variable name string
3. `source.as_bytes()` — source expression string
4. `pages.map_or(0u32, |p| p).to_le_bytes()` — page limit (0 if None)
5. `items.map_or(0u32, |i| i).to_le_bytes()` — page size (0 if None)
6. For each `StepAst` in `body`:
   - `step.id.as_bytes()`
   - `digest_step_primitive(hasher, &step.primitive)` — recursive digest of body step primitives

### Scope of Fix
Two locations with identical (buggy) implementations:
- `vb_compile/src/mod_compile_lowering/part_05.rs` lines 140–161
- `vb_compile/src/compile/mod.rs` lines 243–261

### Contract for Collect (Required Fix)
```rust
vb_yaml::ast::StepPrimitive::Collect { variable, source, pages, items, body } => {
    hasher.update(b"collect");
    hasher.update(variable.as_bytes());
    hasher.update(source.as_bytes());
    if let Some(p) = pages {
        hasher.update(&p.to_le_bytes());
    } else {
        hasher.update(&0u32.to_le_bytes());
    }
    if let Some(i) = items {
        hasher.update(&i.to_le_bytes());
    } else {
        hasher.update(&0u32.to_le_bytes());
    }
    for step in body {
        hasher.update(step.id.as_bytes());
        digest_step_primitive(hasher, &step.primitive);
    }
}
```

---

## Contract: canonical_primitive_name

### Signature
```rust
fn canonical_primitive_name(primitive: &vb_yaml::ast::StepPrimitive) -> &'static str
```

### Behavioral Contract
Returns the canonical string name for each `StepPrimitive` variant:
| Variant | Return Value |
|---------|--------------|
| `Set` | `"set"` |
| `Save` | `"save"` |
| `Do` | `"do"` |
| `Choose` | `"choose"` |
| `ForEach` | `"for_each"` |
| `Together` | `"parallel"` |
| `Collect` | `"collect"` |
| `Aggregate` | `"aggregate"` |
| `Repeat` | `"repeat"` |
| `Wait` | `"wait"` |
| `Ask` | `"ask"` |
| `Finish` | `"finish"` |

**Note**: This function is used for IR emission metadata, NOT for digest computation. Digest computation MUST use `digest_step_primitive` which handles per-field hashing for `Set` and `Finish` already, and must be extended for `Collect` and other variants.

---

## Contract: canonical_digest

### Signature
```rust
pub(super) fn canonical_digest(source: &vb_yaml::ast::WorkflowSource) -> WorkflowDigest
```

### Behavioral Contract
Computes a content-addressed digest of the entire workflow source:
1. Hash `source.version()` as bytes
2. Hash `source.name()` as bytes
3. Hash trigger variant + trigger-specific data
4. For each step: hash `step.id` + `digest_step_primitive(hasher, &step.primitive)`
5. Return `WorkflowDigest::from_bytes(hasher.finalize().into())`

### Invariant
```text
digest_a == digest_b  ⟹  canonical_digest computes identical hasher state from identical source
```

**Critical Bug**: Step 4 does NOT fully hash `Collect` fields; only primitive name is hashed. This breaks the content-addressing invariant.

---

## Contract: StepPrimitive::Collect

### Type Definition
```rust
Collect {
    variable: String,       // Loop variable name
    source: String,          // Source expression
    pages: Option<u32>,     // Maximum pages (optional)
    items: Option<u32>,     // Items per page (optional)
    body: Vec<StepAst>,     // Body steps
}
```

### Validation Contract (vb_validate)
- `pages` and `items` must be `Some(n)` where `n >= 1` if present
- `body` must contain at least one step
- `source` must evaluate to a list type at runtime
- Failure: `ValidationError::InvalidCollect`

### Digest Contract (vb_compile)
- ALL five fields MUST contribute to the workflow digest
- Two `Collect` primitives with identical `variable`, `source`, `pages`, `items`, and recursively identical `body` MUST produce the same digest contributions
- Any field difference MUST produce different digest contributions

---

## Contract: CompiledNodeKind::CollectStart/Page/Finish

### CollectStart
```rust
CollectStart {
    source: SlotIdx,      // Slot holding the list to collect from
    limit: u32,           // Max pages (= pages.unwrap_or(1))
    page_size: u32,      // Items per page (= items.unwrap_or(1))
    body: StepIdx,        // First body step index
    done: StepIdx,        // Step to jump to when done
}
```

### CollectPage
```rust
CollectPage {
    collector_slot: SlotIdx,  // Slot holding the collector/list state
    body: StepIdx,            // Body step to execute per page
    done: StepIdx,            // Step to jump to when all pages consumed
}
```

### CollectFinish
```rust
CollectFinish {
    collector_slot: SlotIdx,  // Slot holding final collected list
}
```

### Digest Contract
These IR nodes are derived from the YAML `Collect` primitive. The source digest MUST reflect the YAML primitive fields so that:
- Different `Collect` parameters → different IR node contents → different source digest
- Re-compiling same YAML → same IR → same source digest

---

## Error Taxonomy (Relevant to Digest)

| Error Kind | Type | Digest Impact |
|------------|------|---------------|
| `ValidationError::InvalidCollect` | Semantic | Rejected before digest computation |
| `CompileErrors` | Compilation | Compilation fails; no digest emitted |
| `WorkflowError::EmptyNodes` | IR Validation | Compilation succeeds but validation fails; digest still computed |
| `ArtifactDigestMismatch` (storage test) | Admission | Same artifact bytes must produce same artifact digest |

---

## Illegal States Made Unrepresentable

1. **Identical digests for different Collect params**: With the fix, this becomes impossible because `variable`, `source`, `pages`, `items`, and `body` all contribute to the hasher state
2. **Collect with empty body**: `parse_collect` requires at least one body step; `ValidationError::InvalidCollect`
3. **Collect with zero pages/items**: `vb_validate` rejects `pages: 0` or `items: 0`
4. **Collect with duplicate step IDs in body**: Validated elsewhere in the pipeline

---

## Boundary: Pure Core vs. Imperative Shell

- `canonical_digest` and `digest_step_primitive` are **pure**: no I/O, no storage, no time, no randomness
- They operate only on in-memory AST types (`WorkflowSource`, `StepPrimitive`)
- The imperative shell is at the YAML parser boundary and the serialization boundary for `compute_compiled_digest`
