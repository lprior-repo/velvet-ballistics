# Proof-to-Implementation Input: vb-xi2f.38

## Purpose
Map proof claims to exact Rust source references, independent behavior tests, refinement harness refs, and exact evidence commands. This document is the bridge between proof artifacts and implementation work.

---

## Proof Claim Map

### CC-DIGEST-001: Digest Content-Addressing for Collect

| Proof Claim | Rust Source Ref | Test/Harness Ref | Evidence Command |
|-------------|-----------------|------------------|------------------|
| Collect fields must be hashed | `vb_compile/src/mod_compile_lowering/part_05.rs:158-160` | `verification/kani/collect_field_coverage.rs` | `cargo kani --workspace --no-default-features --features verified` |
| Variable field contribution | `vb_compile/src/mod_compile_lowering/part_05.rs:158-160` | `crates/vb_compile/src/tests/digest_collect_tests.rs::digest_collect_variable_field` | `cargo test -p vb_compile digest_collect_variable_field` |
| Source field contribution | `vb_compile/src/mod_compile_lowering/part_05.rs:158-160` | `crates/vb_compile/src/tests/digest_collect_tests.rs::digest_collect_source_field` | `cargo test -p vb_compile digest_collect_source_field` |
| Pages field contribution | `vb_compile/src/mod_compile_lowering/part_05.rs:158-160` | `crates/vb_compile/src/tests/digest_collect_tests.rs::digest_collect_pages_field` | `cargo test -p vb_compile digest_collect_pages_field` |
| Items field contribution | `vb_compile/src/mod_compile_lowering/part_05.rs:158-160` | `crates/vb_compile/src/tests/digest_collect_tests.rs::digest_collect_items_field` | `cargo test -p vb_compile digest_collect_items_field` |
| Body recursive hashing | `vb_compile/src/mod_compile_lowering/part_05.rs:158-160` | `crates/vb_compile/src/tests/digest_collect_tests.rs::digest_collect_body_recursive` | `cargo test -p vb_compile digest_collect_body_recursive` |

### CC-DIGEST-001a: Collect Field Coverage (Specific)

| Field | Source Ref | Implementation Note |
|-------|------------|-------------------|
| `variable: String` | `part_05.rs:158` | Add: `hasher.update(variable.as_bytes());` |
| `source: String` | `part_05.rs:158` | Add: `hasher.update(source.as_bytes());` |
| `pages: Option<u32>` | `part_05.rs:158` | Add: `pages.map_or(0u32, \|p\| hasher.update(&p.to_le_bytes()));` |
| `items: Option<u32>` | `part_05.rs:158` | Add: `items.map_or(0u32, \|i\| hasher.update(&i.to_le_bytes()));` |
| `body: Vec<StepAst>` | `part_05.rs:158` | Add: body iteration with recursive `digest_step_primitive` call |

### CC-DIGEST-002: Digest Determinism

| Proof Claim | Rust Source Ref | Test Ref | Evidence Command |
|-------------|-----------------|----------|------------------|
| Repeated calls produce identical digest | `vb_compile/src/compile/mod.rs:220-241` | `crates/vb_compile/src/tests/error_variant_tests.rs:762-801` | `cargo test -p vb_compile compute_compiled_digest_determinism` |
| Cross-run determinism | `crates/workspace_tests/tests/vb_kyyf_cross_run_determinism.rs` | N/A | `cargo test -p vb_kyyf_cross_run_determinism` |

### CC-DIGEST-004: Collect Lowering Correctness

| Proof Claim | Rust Source Ref | Verus Ref | Evidence Command |
|-------------|-----------------|-----------|------------------|
| lower_canonical_collect emits 4 nodes | `vb_compile/src/mod_compile_lowering/part_03.rs:159-212` | `verification/verus/collect_lowering.rs` | `cargo verus --workspace` |
| CollectStart.limit = pages.unwrap_or(1) | `vb_compile/src/mod_compile_lowering/part_03.rs:159-212` | `verification/verus/collect_lowering.rs` | `cargo verus --workspace` |
| CollectStart.page_size = items.unwrap_or(1) | `vb_compile/src/mod_compile_lowering/part_03.rs:159-212` | `verification/verus/collect_lowering.rs` | `cargo verus --workspace` |

### CC-DIGEST-005: Digest Mismatch Detection

| Proof Claim | Rust Source Ref | Test Ref | Evidence Command |
|-------------|-----------------|----------|------------------|
| Storage admission rejects mismatched digest | `vb_storage/src/admit.rs` | `crates/vb_storage/tests/vb_core_atomic_admission_red.rs:856` | `cargo test -p vb_storage vb_core_atomic_admission_red::artifact_digest_mismatch` |

### CC-DIGEST-006: No Panic on Collect Digest

| Proof Claim | Rust Source Ref | Harness Ref | Evidence Command |
|-------------|-----------------|-------------|------------------|
| digest_step_primitive never panics for Collect | `vb_compile/src/mod_compile_lowering/part_05.rs:140-161` | `verification/kani/collect_try_from_parts.rs` | `cargo kani --workspace --no-default-features --features verified` |

### CC-DIGEST-007: Property-Based Digest Equality

| Proof Claim | Rust Source Ref | Test Ref | Evidence Command |
|-------------|-----------------|----------|------------------|
| (a == b) -> digest_eq(a,b) | `vb_compile/src/mod_compile_lowering/part_05.rs:140-161` | `crates/vb_compile/src/tests/digest_collect_tests.rs::collect_digest_equality_property` | `cargo test -p vb_compile collect_digest_equality_property` |
| (a != b) -> digest_ne(a,b) when fields differ | `vb_compile/src/mod_compile_lowering/part_05.rs:140-161` | `crates/vb_compile/src/tests/digest_collect_tests.rs::collect_digest_equality_property` | `cargo test -p vb_compile collect_digest_equality_property` |

### H-2: ForEach/Aggregate Field Hashing

| Proof Claim | Rust Source Ref | Harness Ref | Evidence Command |
|-------------|-----------------|-------------|------------------|
| ForEach fields hashed | `vb_compile/src/mod_compile_lowering/part_05.rs:158-160` | `verification/kani/foreach_field_coverage.rs` | `cargo kani --workspace --no-default-features --features verified` |
| Aggregate fields hashed | `vb_compile/src/mod_compile_lowering/part_05.rs:158-160` | `verification/kani/aggregate_field_coverage.rs` | `cargo kani --workspace --no-default-features --features verified` |

### H-4: Lowering Determinism

| Proof Claim | Rust Source Ref | TLA+ Ref | Evidence Command |
|-------------|-----------------|----------|------------------|
| Same Collect -> same 4-node sequence | `vb_compile/src/mod_compile_lowering/part_03.rs:159-212` | `verification/tla/collect_body_model.tla` | `java -jar tla2tools.jar verification/tla/collect_body_model.tla -config verification/tla/collect_body_model.cfg` |

### H-5: Serialization Determinism

| Proof Claim | Rust Source Ref | Test Ref | Evidence Command |
|-------------|-----------------|----------|------------------|
| Same WorkflowParts -> same bytes | `vb_compile/src/mod_compile_core.rs:114-116` | `crates/vb_compile/src/tests/error_variant_tests.rs::postcard_serialization_deterministic` | `cargo test -p vb_compile postcard_serialization_deterministic` |

---

## Required Rust Implementation Fix

### File: `vb_compile/src/mod_compile_lowering/part_05.rs`

**Lines 158-160**: Replace catch-all with explicit Collect match arm:

```rust
// CURRENT (BUGGY)
other => {
    hasher.update(canonical_primitive_name(other).as_bytes());
}

// REQUIRED FIX
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

### File: `vb_compile/src/compile/mod.rs`

**Lines 257-259**: Same fix required (duplicate implementation).

---

## Required Test Files

### New File: `crates/vb_compile/src/tests/digest_collect_tests.rs`
- `digest_collect_variable_field` — property test for variable field contribution
- `digest_collect_source_field` — property test for source field contribution
- `digest_collect_pages_field` — property test for pages field contribution
- `digest_collect_items_field` — property test for items field contribution
- `digest_collect_body_recursive` — property test for body recursive hashing
- `collect_digest_equality_property` — equality property test

### New/Update: `verification/kani/collect_field_coverage.rs`
- `kani_collect_different_pages_same_digest` — pre-fix bug reproduction
- `kani_collect_field_coverage` — post-fix correctness proof

### New: `verification/kani/foreach_field_coverage.rs`
- `kani_foreach_field_coverage` — ForEach field coverage proof

### New: `verification/kani/aggregate_field_coverage.rs`
- `kani_aggregate_field_coverage` — Aggregate field coverage proof

---

## Execution Evidence Commands

| Obligation | Command | Expected Output |
|------------|---------|-----------------|
| PO-002 | `cargo kani --workspace --no-default-features --features verified 2>&1 \| tee verification/kani/kani_collect_field_coverage.log` | Kani proves harness; `kani::cover!` statements pass |
| PO-013 | `cargo kani --workspace --no-default-features --features verified 2>&1 \| tee verification/kani/kani_collect_try_from_parts.log` | Kani proves no-panic for arbitrary Collect |
| PO-015 | `cargo kani --workspace --no-default-features --features verified 2>&1 \| tee verification/kani/kani_foreach_field_coverage.log` | Kani proves ForEach field coverage |
| PO-016 | `cargo kani --workspace --no-default-features --features verified 2>&1 \| tee verification/kani/kani_aggregate_field_coverage.log` | Kani proves Aggregate field coverage |
| PO-003..007 | `cargo test -p vb_compile digest_collect_ -- --nocapture` | All 5 proptest cases pass |
| PO-014 | `cargo test -p vb_compile collect_digest_equality_property -- --nocapture` | Equality property passes |
| PO-009 | `cargo test -p vb_compile compute_compiled_digest_determinism -- --nocapture` | Determinism test passes |
| PO-011 | `cargo verus --workspace 2>&1 \| tee verification/verus/verus_collect_lowering.log` | Verus proves lowering lemma |
| PO-001, PO-008, PO-012, PO-017 | `java -jar tla2tools.jar verification/tla/collect_body_model.tla -config verification/tla/collect_body_model.cfg` | TLC: all invariants hold |
| PO-012b | `cargo test -p vb_storage vb_core_atomic_admission_red::artifact_digest_mismatch -- --nocapture` | Integration test passes |

---

## GOD RULE Enforcement

All Kani harnesses MUST use `kani::any::<StepPrimitive::Collect>()` and NOT hardcoded dummy data.

**Invalid (GOD RULE violation)**:
```rust
let collect = StepPrimitive::Collect {
    variable: "x".to_string(),
    source: "list".to_string(),
    pages: Some(10),
    items: Some(5),
    body: vec![],
};
```

**Valid (GOD RULE compliant)**:
```rust
#[kani::proof]
fn kani_collect_field_coverage() {
    let collect = kani::any::<StepPrimitive::Collect>();
    // ... test digest contribution ...
}
```
