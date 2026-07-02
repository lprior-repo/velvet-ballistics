# Type Contracts — Digest Coverage of `for_each` Semantics

**Bead:** vb-xi2f.28  
**State:** 3 (rust-contract)  
**Date:** 2026-05-25  
**Status:** DRAFT

---

## 1. Input Type Contracts

### 1.1 `canonical_digest` Input

```
canonical_digest(source: &WorkflowSource) -> WorkflowDigest

PRE-CONDITIONS:
  - source is a valid, fully-parsed WorkflowSource
  - source.version() returns a non-empty version string
  - source.name() returns a non-empty name string
  - source.steps() returns a Vec<StepAst> (may be empty)
  - For each StepAst: step.id is non-empty, step.primitive is a valid variant

POST-CONDITIONS:
  - Result is a deterministic WorkflowDigest
  - Result changes iff any hashed field changes
  - Result is independent of: machine word size, process identity, time-of-day, memory layout
```

### 1.2 `digest_step_primitive` Input

```
digest_step_primitive(hasher: &mut blake3::Hasher, primitive: &StepPrimitive)

PRE-CONDITIONS:
  - hasher is a blake3 accumulator in an arbitrary (non-finalized) state
  - primitive is a valid StepPrimitive variant

POST-CONDITIONS:
  - hasher state has been updated with primitive-specific field content
  - For ForEach: hasher absorbs "for_each", variable, input, at_once (canonical), and recursively body steps
  - The update is purely additive; no fields are removed or reset
```

---

## 2. Output Type Contract: WorkflowDigest

```
WorkflowDigest([u8; 32])

TYPE CONTRACT:
  - #[repr(transparent)] over [u8; 32]
  - PartialEq, Eq, Hash — structural equality on the byte array
  - Copy — cheap to pass by value
  - from_bytes(bytes: [u8; 32]) -> Self — infallible constructor
  - as_bytes(self) -> [u8; 32] — infallible accessor
  - No semantic validation (any 32-byte array is a valid digest)
  - Serialized as 32-byte sequence via serde (no length prefix)

INVARIANTS:
  - Equality implies byte-level equality (no aliasing)
  - No two semantically different workflows may produce the same digest
    (enforced by BLAKE3 collision resistance + complete field hashing)
```

---

## 3. ForEach Field Hashing Contract

### 3.1 Required Fields

Each ForEach field MUST be hashed with a delimiter for unambiguous framing:

```
digest_for_each(hasher, foreach: &StepPrimitive::ForEach):
    hasher.update(b"for_each")             // discriminant
    hasher.update(b"variable:")            // field delimiter
    hasher.update(foreach.variable.as_bytes())
    hasher.update(b"input:")               // field delimiter
    hasher.update(foreach.input.as_bytes())
    hasher.update(b"at_once:")             // field delimiter
    match foreach.at_once {
        None    => hasher.update(&0u32.to_le_bytes()),       // canonical: None → 0
        Some(v) => hasher.update(&v.to_le_bytes()),
    }
    hasher.update(b"body:")                // field delimiter
    for body_step in &foreach.body {
        hasher.update(body_step.id.as_bytes())
        digest_step_primitive(hasher, &body_step.primitive)
    }
```

### 3.2 Field Delimiter Rationale

Field delimiters prevent hash collisions from field-boundary ambiguity. Without delimiters:
- `variable="ab" input="c"` and `variable="a" input="bc"` produce identical hasher input `"ab" + "c" == "a" + "bc"`.
- Delimiters (`":"`, `"\n"`, or fixed-length encodings) disambiguate boundaries.

The byte string `b":"` is used as a delimiter because field values (variable names, input expressions) are drawn from YAML identifiers which never contain `":"` as part of a valid field name.

### 3.3 at_once Canonical Representation

| Source Value | Canonical Hash Input |
|---|---|
| `None` (field absent) | `0u32.to_le_bytes() = [0,0,0,0]` |
| `Some(0)` | `0u32.to_le_bytes() = [0,0,0,0]` |
| `Some(n)` (n > 0) | `n.to_le_bytes()` |

Note: `None` and `Some(0)` produce identical hash input. This is intentional — the lowering phase treats both as "limit 1" — so the digest reflects semantic equivalence.

### 3.4 Body Step Hashing Contract

```
For each body StepAst in ForEach.body:
    1. hasher.update(step.id.as_bytes())
    2. digest_step_primitive(hasher, &step.primitive)
    3. (Recursive: body steps themselves may contain nested primitives)
```

Body step order matters: hashing steps in a different order produces a different digest.

---

## 4. Non-Exhaustiveness Contract

The bead scope covers **ForEach only**. The following StepPrimitive variants have the **same** digest coverage gap and are explicitly **out of scope**:

| Primitive | Current Coverage | Out-of-scope fields |
|---|---|---|
| Collect | Name only | variable, source, pages, items, body |
| Aggregate (reduce) | Name only | variable, input, initial, body |
| Repeat | Name only | max_attempts, body |
| Together (parallel) | Name only | branches (label + steps) |
| Wait | Name only | event, timeout |
| Ask | Name only | prompt, timeout |
| Choose | Name only | branches, otherwise |
| Do | Name only | action, input |
| Save | Name only | value |

**Type contract implication:** The `digest_step_primitive` function must remain *extensible* so future beads can add field hashing for these primitives without restructuring the dispatch.

---

## 5. Duplicate Code Contract

Two copies of `digest_step_primitive` exist:
1. `crates/vb_compile/src/mod_compile_lowering/part_05.rs:140-162`
2. `crates/vb_compile/src/compile/mod.rs:243-261`

**Contract:** Both copies MUST remain behaviorally identical for all StepPrimitive variants. Any update to one MUST be mirrored in the other.

**Current risk:** The two copies have minor differences in:
- `canonical_primitive_name` mapping: part_05 uses `"parallel"`, mod.rs uses `"parallel"` (actually: part_05 uses `"together"`, let me check)... Let me re-check the code... from the source:
  - part_05.rs line 106: `Together { .. } => "together"`
  - mod.rs line 210: `Together { .. } => "parallel"`
  
  This is a pre-existing naming inconsistency but does not affect the ForEach fix.

**Contract requires:** After the ForEach fix, both copies must hash the same ForEach fields in the same canonical order with the same canonical representations.
