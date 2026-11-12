# Contract: Digest Covers Collect Semantics (vb-xi2f.38)

## Contract Clause: Digest Content-Addressing

**Clause ID**: CC-DIGEST-001
**Requirement**: `canonical_digest(source_a) == canonical_digest(source_b)` if and only if workflow sources `source_a` and `source_b` are semantically identical for all execution behavior.

**Type**: Invariant on `canonical_digest`

### Formal Statement
```text
∀ a, b ∈ WorkflowSource:
  digest(a) = digest(b)  ⟺  ∀ collect ∈ a.steps ∧ ∀ collect' ∈ b.steps:
    collect.variable = collect'.variable  ∧
    collect.source    = collect'.source    ∧
    collect.pages     = collect'.pages     ∧
    collect.items     = collect'.items     ∧
    body_digest(collect.body) = body_digest(collect'.body)
```

Where `body_digest` recursively applies the same digest function to body steps.

### Sub-Clause: Collect Field Coverage
**Clause ID**: CC-DIGEST-001a

`StepPrimitive::Collect` digest contributions MUST include:
1. The string `"collect"` as a type tag
2. `variable: String` — loop variable name
3. `source: String` — source expression
4. `pages: Option<u32>` — page limit (0 if None)
5. `items: Option<u32>` — items per page (0 if None)
6. `body: Vec<StepAst>` — each step's id + primitive digest

### Sub-Clause: Step ID Coverage
**Clause ID**: CC-DIGEST-001b

For each step in `source.steps`, the digest MUST incorporate:
1. `step.id.as_bytes()` — the step identifier
2. `digest_step_primitive(hasher, &step.primitive)` — the primitive content

### Sub-Clause: Trigger Coverage
**Clause ID**: CC-DIGEST-001c

The digest MUST incorporate:
- For `TriggerAst::Manual`: `b"manual"`
- For `TriggerAst::Schedule { cron }`: `b"schedule"` + `cron.as_bytes()`
- For `TriggerAst::Event { event_type }`: `b"event"` + `event_type.as_bytes()`
- For `TriggerAst::Webhook`: `b"webhook"`

---

## Contract Clause: Digest Determinism

**Clause ID**: CC-DIGEST-002
**Requirement**: `canonical_digest` is a pure function. For any `WorkflowSource` value, repeated calls with the same value MUST return bit-for-bit identical `WorkflowDigest`.

**Formal Statement**:
```text
∀ source ∈ WorkflowSource, ∀ i ∈ ℕ:
  canonical_digest(source) = canonical_digest(source)  [idempotent]
```

**Evidence Requirement**: `crates/vb_compile/src/tests/error_variant_tests.rs` lines 762–801 test determinism.

---

## Contract Clause: Artifact Digest Depends on Source Digest

**Clause ID**: CC-DIGEST-003
**Requirement**: The artifact digest (BLAKE3 of serialized `WorkflowParts`) MUST be a function of the source digest embedded in `WorkflowParts.digest`.

**Formal Statement**:
```text
let source_a = WorkflowSource(...)
let digest_a = canonical_digest(source_a)
let parts_a = compile(source_a).to_parts()
let artifact_a = serialize(parts_a)
let comp_digest_a = BLAKE3(artifact_a)

then  comp_digest_a = f(digest_a, other_ir_components)
```

**Note**: This is not a pure function requirement — artifact digest depends on IR structure — but the IR is derived deterministically from the source.

---

## Contract Clause: Collect Lowering Preserves Semantic Identity

**Clause ID**: CC-DIGEST-004
**Requirement**: `lower_canonical_collect` MUST emit IR nodes that faithfully represent the `Collect` primitive's semantics. The source digest reflects the YAML fields; the IR reflects the lowered form.

**Formal Statement**:
```text
∀ collect ∈ StepPrimitive::Collect:
  let ir_nodes = lower_canonical_collect(collect)
  then  ir_nodes[0].kind = CollectStart { source, limit, page_size, body, done }
        ir_nodes[2].kind = CollectPage { collector_slot, body, done }
        ir_nodes[3].kind = CollectFinish { collector_slot }
```

Where `limit = collect.pages.unwrap_or(1)` and `page_size = collect.items.unwrap_or(1)`.

---

## Contract Clause: Digest Mismatch Detection

**Clause ID**: CC-DIGEST-005
**Requirement**: Storage admission MUST detect when submitted artifact bytes produce a different digest than claimed.

**Formal Statement**:
```rust
fn admit_artifact(claimed_digest: WorkflowDigest, bytes: &[u8]) -> Result<(), StorageError> {
    let computed = compute_compiled_digest(bytes);
    if computed != claimed_digest {
        return Err(ArtifactDigestMismatch);  // fail-closed
    }
    Ok(())
}
```

**Evidence**: `crates/vb_storage/tests/vb_core_atomic_admission_red.rs` line 856.

---

## Contract Clause: No Panic on Collect Digest

**Clause ID**: CC-DIGEST-006
**Requirement**: `digest_step_primitive` MUST NOT panic when passed any valid `StepPrimitive::Collect`.

**Formal Statement**:
```text
∀ primitive ∈ StepPrimitive::Collect:
  ∀ variable ∈ String, ∀ source ∈ String,
  ∀ pages ∈ Option<u32>, ∀ items ∈ Option<u32>,
  ∀ body ∈ Vec<StepAst>:
    let hasher = blake3::Hasher::new();
    digest_step_primitive(&mut hasher, &primitive)  // must not panic
```

**Evidence**: Kani harness proving panic-freedom for arbitrary `Collect`.

---

## Contract Clause: Property-Based Digest Equality

**Clause ID**: CC-DIGEST-007
**Requirement**: Two `Collect` primitives with identical fields MUST produce identical digest contributions. Two `Collect` primitives with different fields MUST produce different digest contributions.

**Formal Statement**:
```text
∀ a, b ∈ StepPrimitive::Collect:
  a = b  ⟹  digest_primitive(a) = digest_primitive(b)
  a ≠ b  ⟹  digest_primitive(a) ≠ digest_primitive(b)
```

**Proof Approach**: Property test with `proptest` generating pairs of `Collect` primitives and asserting:
1. `a == b → digest_eq(a, b)`
2. `a != b → digest_ne(a, b)` (when fields differ in `variable`, `source`, `pages`, `items`, or body content)

---

## Type-Level Obligations

### TO-1: Collect Variant Must Have Explicit Match Arm
```rust
// CURRENT (BUG)
StepPrimitive::Collect { .. } => {
    hasher.update(canonical_primitive_name(other).as_bytes());
}

// REQUIRED FIX
StepPrimitive::Collect { variable, source, pages, items, body } => {
    hasher.update(b"collect");
    hasher.update(variable.as_bytes());
    hasher.update(source.as_bytes());
    pages.map_or(0u32, |p| hasher.update(&p.to_le_bytes()));
    items.map_or(0u32, |i| hasher.update(&i.to_le_bytes()));
    for step in body {
        hasher.update(step.id.as_bytes());
        digest_step_primitive(hasher, &step.primitive);
    }
}
```

### TO-2: Recursive Body Digest
```rust
// Body steps must be digested recursively
for step in body {
    hasher.update(step.id.as_bytes());
    digest_step_primitive(hasher, &step.primitive);
}
```

### TO-3: Option<u32> Serialization for pages/items
```rust
// Must serialize Option<u32> deterministically
if let Some(p) = pages {
    hasher.update(&p.to_le_bytes());
} else {
    hasher.update(&0u32.to_le_bytes());
}
```

---

## Evidence Requirements

| Clause | Evidence Location |
|--------|-------------------|
| CC-DIGEST-001 | Kani harness with `kani::any::<StepPrimitive::Collect>()` |
| CC-DIGEST-001a | TLA+ invariant: `CollectDigestCoverage` |
| CC-DIGEST-002 | Proptest: `digest_determinism` |
| CC-DIGEST-004 | Verus: `lemma_lower_canonical_collect_emits_4_nodes` |
| CC-DIGEST-005 | Integration test: `vb_core_atomic_admission_red` |
| CC-DIGEST-006 | Kani: `kani_collect_try_from_parts` |
| CC-DIGEST-007 | Proptest: `collect_digest_equality_property` |

---

## Open Questions

1. Should `ForEach`, `Aggregate`, `Together`, `Choose`, `Repeat` also have explicit match arms, or is the catch-all acceptable for some variants?
2. Should `Collect.body` be hashed in a depth-first manner (current approach) or breadth-first?
3. Should step `condition`, `name`, `with`, `retry`, `on_error`, `then` fields also contribute to the digest?
