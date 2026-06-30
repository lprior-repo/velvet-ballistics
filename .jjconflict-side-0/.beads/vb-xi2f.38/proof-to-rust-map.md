# Proof-to-Rust Map: vb-xi2f.38

**Bead**: vb-xi2f.38
**Title**: P1: digest covers collect semantics
**State**: 7 (proof-to-implementation bridge)
**Generated**: 2026-05-24

---

## CRITICAL DEFECT SUMMARY

| Location | Lines | Bug |
|----------|-------|-----|
| `crates/vb_compile/src/mod_compile_lowering/part_05.rs` | 158-160 | Catch-all `other =>` for `Collect` only hashes primitive name |
| `crates/vb_compile/src/compile/mod.rs` | 257-259 | Same bug in duplicate `digest_step_primitive` implementation |

**Current buggy code** (both locations):
```rust
other => {
    hasher.update(canonical_primitive_name(other).as_bytes());
}
```

**Required fix** (to be implemented by holzman-rust in state 11):
```rust
vb_yaml::ast::StepPrimitive::Collect { variable, source, pages, items, body } => {
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

---

## PROOF CLAIM → RUST SOURCE MAPPING

### CC-DIGEST-001: Digest Content-Addressing for Collect

| Proof Claim | Rust Source Ref | Behavior Test Ref | Evidence Command |
|-------------|-----------------|------------------|------------------|
| Collect fields must be hashed | `part_05.rs:158-160`, `compile/mod.rs:257-259` | `verification/kani/collect_field_coverage.rs` | `cargo kani -p vb_compile --harness kani_collect_field_coverage` |
| Variable field contribution | `part_05.rs:158-160`, `compile/mod.rs:257-259` | `crates/vb_compile/src/tests/digest_collect_tests.rs::digest_collect_variable_field` | `cargo test -p vb_compile digest_collect_variable_field` |
| Source field contribution | `part_05.rs:158-160`, `compile/mod.rs:257-259` | `crates/vb_compile/src/tests/digest_collect_tests.rs::digest_collect_source_field` | `cargo test -p vb_compile digest_collect_source_field` |
| Pages field contribution | `part_05.rs:158-160`, `compile/mod.rs:257-259` | `crates/vb_compile/src/tests/digest_collect_tests.rs::digest_collect_pages_field` | `cargo test -p vb_compile digest_collect_pages_field` |
| Items field contribution | `part_05.rs:158-160`, `compile/mod.rs:257-259` | `crates/vb_compile/src/tests/digest_collect_tests.rs::digest_collect_items_field` | `cargo test -p vb_compile digest_collect_items_field` |
| Body recursive hashing | `part_05.rs:158-160`, `compile/mod.rs:257-259` | `crates/vb_compile/src/tests/digest_collect_tests.rs::digest_collect_body_recursive` | `cargo test -p vb_compile digest_collect_body_recursive` |

### CC-DIGEST-001a: Collect Field Coverage (Specific)

| Field | Source Ref | Implementation Fix |
|-------|------------|-------------------|
| `variable: String` | `part_05.rs:158` | Add: `hasher.update(variable.as_bytes());` |
| `source: String` | `part_05.rs:158` | Add: `hasher.update(source.as_bytes());` |
| `pages: Option<u32>` | `part_05.rs:158` | Add: `pages.map_or(0u32, \|p\| hasher.update(&p.to_le_bytes()));` |
| `items: Option<u32>` | `part_05.rs:158` | Add: `items.map_or(0u32, \|i\| hasher.update(&i.to_le_bytes()));` |
| `body: Vec<StepAst>` | `part_05.rs:158` | Add: body iteration with recursive `digest_step_primitive` call |

### CC-DIGEST-002: Digest Determinism

| Proof Claim | Rust Source Ref | Test Ref | Evidence Command |
|-------------|-----------------|----------|------------------|
| Repeated calls produce identical digest | `compile/mod.rs:220-241` | `crates/vb_compile/src/tests/error_variant_tests.rs:762-801` | `cargo test -p vb_compile compute_compiled_digest_determinism` |
| Cross-run determinism | `compile/mod.rs:220-241` | `crates/workspace_tests/tests/vb_kyyf_cross_run_determinism.rs` | `cargo test -p vb_kyyf_cross_run_determinism` |

### CC-DIGEST-004: Collect Lowering Correctness

| Proof Claim | Rust Source Ref | Verus Ref | Evidence Command |
|-------------|-----------------|-----------|------------------|
| lower_canonical_collect emits 4 nodes | `crates/vb_compile/src/mod_compile_lowering/part_03.rs:159-212` | `verification/verus/collect_lowering.rs` | `cargo verus --workspace` |
| CollectStart.limit = pages.unwrap_or(1) | `crates/vb_compile/src/mod_compile_lowering/part_03.rs:159-212` | `verification/verus/collect_lowering.rs` | `cargo verus --workspace` |
| CollectStart.page_size = items.unwrap_or(1) | `crates/vb_compile/src/mod_compile_lowering/part_03.rs:159-212` | `verification/verus/collect_lowering.rs` | `cargo verus --workspace` |

### CC-DIGEST-005: Digest Mismatch Detection

| Proof Claim | Rust Source Ref | Test Ref | Evidence Command |
|-------------|-----------------|----------|------------------|
| Storage admission rejects mismatched digest | `crates/vb_storage/src/admit.rs` | `crates/workspace_tests/tests/vb_ssei_verification_admission_acceptance.rs` | `cargo test -p workspace_tests vb_ssei_verification_admission_acceptance::test_admission_rejects_when_ir_digest_mismatches_artifact` |

### CC-DIGEST-006: No Panic on Collect Digest

| Proof Claim | Rust Source Ref | Harness Ref | Evidence Command |
|-------------|-----------------|-------------|------------------|
| digest_step_primitive never panics for Collect | `part_05.rs:140-161` | `verification/kani/collect_try_from_parts.rs` | `cargo kani -p vb_compile --harness kani_collect_try_from_parts` |

### CC-DIGEST-007: Property-Based Digest Equality

| Proof Claim | Rust Source Ref | Test Ref | Evidence Command |
|-------------|-----------------|----------|------------------|
| (a == b) -> digest_eq(a,b) | `part_05.rs:140-161` | `crates/vb_compile/src/tests/digest_collect_tests.rs::collect_digest_equality_property` | `cargo test -p vb_compile collect_digest_equality_property` |
| (a != b) -> digest_ne(a,b) when fields differ | `part_05.rs:140-161` | `crates/vb_compile/src/tests/digest_collect_tests.rs::collect_digest_equality_property` | `cargo test -p vb_compile collect_digest_equality_property` |

### H-2: ForEach/Aggregate Field Hashing

| Proof Claim | Rust Source Ref | Harness Ref | Evidence Command |
|-------------|-----------------|-------------|------------------|
| ForEach fields hashed | `part_05.rs:158-160` | `verification/kani/foreach_field_coverage.rs` | `cargo kani -p vb_compile --harness kani_foreach_field_coverage` |
| Aggregate fields hashed | `part_05.rs:158-160` | `verification/kani/aggregate_field_coverage.rs` | `cargo kani -p vb_compile --harness kani_aggregate_field_coverage` |

### H-4: Lowering Determinism

| Proof Claim | Rust Source Ref | TLA+ Ref | Evidence Command |
|-------------|-----------------|----------|------------------|
| Same Collect -> same 4-node sequence | `crates/vb_compile/src/mod_compile_lowering/part_03.rs:159-212` | `verification/tla/collect_body_model.tla` | `java -jar tla2tools.jar verification/tla/collect_body_model.tla -config verification/tla/collect_body_model.cfg` |

### H-5: Serialization Determinism

| Proof Claim | Rust Source Ref | Test Ref | Evidence Command |
|-------------|-----------------|----------|------------------|
| Same WorkflowParts -> same bytes | `crates/vb_compile/src/mod_compile_core.rs:114-116` | `crates/vb_compile/src/tests/error_variant_tests.rs::postcard_serialization_deterministic` | `cargo test -p vb_compile postcard_serialization_deterministic` |

---

## COLLECT TYPE DEFINITION

From `crates/vb_yaml/src/ast/types.rs:207-218`:

```rust
/// Paginated collection loop.
Collect {
    /// Loop variable name.
    variable: String,
    /// Source expression.
    source: String,
    /// Maximum pages (optional).
    pages: Option<u32>,
    /// Items per page (optional).
    items: Option<u32>,
    /// Body steps.
    body: Vec<StepAst>,
},
```

---

## DUAL BUG LOCATIONS

Both functions must be fixed identically:

### Location 1: `crates/vb_compile/src/mod_compile_lowering/part_05.rs:158-160`

```rust
// BUG: catch-all only hashes primitive name
other => {
    hasher.update(canonical_primitive_name(other).as_bytes());
}

// FIX: explicit Collect arm
vb_yaml::ast::StepPrimitive::Collect { variable, source, pages, items, body } => {
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

### Location 2: `crates/vb_compile/src/compile/mod.rs:257-259`

Identical bug, identical fix required.

---

## GOD RULE COMPLIANCE (Kani Harnesses)

All Kani harnesses in `verification/kani/collect_field_coverage.rs`:

| Harness | kani::any Used | Production Code Called | Status |
|---------|----------------|----------------------|--------|
| `kani_collect_different_pages_different_digest` | Yes (lines 54-85) | Yes (line 115-116 imports `digest_step_primitive`) | COMPLIANT |
| `kani_collect_different_source_different_digest` | Yes | Yes (line 149-150) | COMPLIANT |
| `kani_collect_different_variable_different_digest` | Yes | Yes (line 181-182) | COMPLIANT |
| `kani_collect_different_items_different_digest` | Yes | Yes (line 213-214) | COMPLIANT |

**FINDING-001 repair verified**: Harness no longer defines local `digest_primitive()` copy. It imports and calls `vb_compile::mod_compile_lowering::part_05::digest_step_primitive` directly (line 29 of harness).

---

## VERIFICATION LANE STATUS

| Lane | Obligation | Artifact | Status |
|------|-----------|----------|--------|
| Kani | PO-002, PO-020 | `verification/kani/collect_field_coverage.rs` | COMPLIANT (GOD RULE 2 fixed) |
| Kani | PO-013 | `verification/kani/collect_try_from_parts.rs` | BLOCKED_TOOLING |
| Kani | PO-015 | `verification/kani/foreach_field_coverage.rs` | BLOCKED_TOOLING |
| Kani | PO-016 | `verification/kani/aggregate_field_coverage.rs` | BLOCKED_TOOLING |
| Proptest | PO-003..007, PO-014 | `crates/vb_compile/src/tests/digest_collect_tests.rs` | PENDING |
| Proptest | PO-009, PO-010, PO-018 | `crates/vb_compile/src/tests/error_variant_tests.rs` | PENDING |
| TLA+ | PO-001, PO-008, PO-012, PO-017 | `verification/tla/collect_body_model.tla` | BLOCKED_TOOLING |
| Verus | PO-011 | `verification/verus/collect_lowering.rs` | BLOCKED_TOOLING |
| Integration | PO-012b | `crates/workspace_tests/tests/vb_ssei_verification_admission_acceptance.rs` | PENDING |

---

## IMPLEMENTATION ORDER FOR HOLZMAN-RUST (STATE 11)

1. **Fix Location 1**: `crates/vb_compile/src/mod_compile_lowering/part_05.rs:158-160`
2. **Fix Location 2**: `crates/vb_compile/src/compile/mod.rs:257-259`
3. **Verify no other catch-all arms** for `Collect`, `ForEach`, `Aggregate` in same functions
4. **Run tests**: `cargo test -p vb_compile digest_collect_ -- --nocapture`
5. **Run Kani** (if tooling available): `cargo kani -p vb_compile --harness kani_collect_field_coverage`

---

## BOUNDS AND ASSUMPTIONS

| Bound | Value | Source |
|-------|-------|--------|
| Collect.variable length | 0..64 chars | Kani harness BoundedString |
| Collect.source length | 0..64 chars | Kani harness BoundedString |
| Collect.pages | 0..100 | Obligation PO-002 |
| Collect.items | 0..1000 | Obligation PO-002 |
| Collect.body steps | 0..8 | Obligation PO-002 |
| Workflow steps | 1..20 | TLA+ model |
| Step ID length | 1..64 chars | TLA+ model |
