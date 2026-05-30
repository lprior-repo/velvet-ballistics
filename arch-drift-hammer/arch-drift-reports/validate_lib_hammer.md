# Architectural Drift Report: `vb_validate/src/lib.rs`

**File**: `/home/lewis/src/velvet-ballistics/crates/vb_validate/src/lib.rs`
**Total Lines**: 471
**Violation**: EXCEEDS 300-LINE LIMIT BY 57%

**Date**: 2026-05-29
**Enforcer**: architectural-drift agent

---

## Executive Summary

The `lib.rs` file is a **God Object** entry point that centralizes 50+ validation error variants with raw primitives instead of domain types. It violates:

1. **Hard Limit**: 471 lines (limit: 300)
2. **Primitive Obsession**: 50+ variants with `String`, `usize`, `u32` instead of typed values
3. **Single Responsibility**: Error definitions, code mapping, trait impl all in one file
4. **DDD Violation**: No value objects for domain concepts (FieldName, Version, Identifier, Reference, etc.)

---

## VIOLATION 1: File Size (471 > 300)

**Severity**: CRITICAL

The file is 157% of the allowed size. Lines 1-12 are lint directives, lines 104-471 are error enum + impl. The file cannot be justified under any architecture as coherent.

**Blast Radius**: Every crate importing `vb_validate` pays the coupling tax.

---

## VIOLATION 2: Primitive Obsession in ValidationError (Lines 104-384)

### A. String-Caked Variants (should be typed domain values)

| Variant | Raw Field | Should Be |
|---------|-----------|-----------|
| `MissingRequiredField { field: String }` | `String` | `FieldName` |
| `InvalidVersion { version: String }` | `String` | `Version` |
| `InvalidId { id: String }` | `String` | `Identifier` |
| `ReservedId { id: String }` | `String` | `Identifier` |
| `DuplicateId { id: String }` | `String` | `Identifier` |
| `UnknownReference { reference: String }` | `String` | `Reference` |
| `FutureReference { reference: String }` | `String` | `Reference` |
| `SecretNotDeclared { secret: String }` | `String` | `SecretName` |
| `UnreachableStep { step: String }` | `String` | `StepName` |
| `LimitRequired { resource: String }` | `String` | `ResourceType` |
| `LimitExceeded { resource: String }` | `String` | `ResourceType` |
| `UnsupportedTrigger { trigger: String }` | `String` | `TriggerType` |
| `SlotReferenceOutOfRange { context: String }` | `String` | `SlotContext` |
| `LoopBodyStepOutOfRange { label: String }` | `String` | `StepLabel` |
| `NodeKindConstraintViolation { detail: String }` | `String` | `ConstraintDetail` |
| `SlotDependencyCycle { chain: String }` | `String` | `SlotChain` |
| `CueVetFailed { file: String }` | `String` | `FilePath` |
| `VersionMonotonicityBreach { file: String, expected: String, actual: String }` | `String` | `FilePath`, `Version`, `Version` |
| `CapabilityNameInvalid { name: String }` | `String` | `CapabilityName` |
| `CapabilityDuplicate { name: String }` | `String` | `CapabilityName` |

**Count**: 20+ variants with untyped strings

### B. Untyped Index/Count Fields

| Variant | Raw Fields | Should Be |
|---------|-----------|-----------|
| `ExpressionStackExceeded { declared: usize, limit: usize }` | `usize, usize` | `StackDepth, StackLimit` |
| `ExpressionStackMismatch { expr_index: usize, declared: usize, computed: usize }` | `usize, usize, usize` | `ExprIndex, StackDepth, StackDepth` |
| `AccessorSlotOutOfRange { accessor_index: usize, slot: usize, slot_count: usize }` | `usize, usize, usize` | `AccessorIndex, SlotIndex, SlotCount` |
| `AccessorPathInvalid { accessor_index: usize, segment_index: usize }` | `usize, usize` | `AccessorIndex, SegmentIndex` |
| `AccessorPathTooDeep { accessor_index: usize, depth: usize, max: usize }` | `usize, usize, usize` | `AccessorIndex, PathDepth, MaxDepth` |
| `AccessorSymbolOutOfBounds { accessor_index: usize, segment_index: usize, symbol: u32, symbols_count: u32 }` | `usize, usize, u32, u32` | `AccessorIndex, SegmentIndex, SymbolIndex, SymbolCount` |
| `LoopBodyStepOutOfRange { step: usize, node_count: usize, source_node: usize }` | `usize, usize, usize` | `StepIndex, NodeCount, NodeIndex` |
| `ActionContractMissing { action_id: usize, node_index: usize }` | `usize, usize` | `ActionId, NodeIndex` |
| `ActionContractOrphan { action_id: usize }` | `usize` | `ActionId` |
| `CapabilityNameEmpty { action_id: usize, capability_index: usize }` | `usize, usize` | `ActionId, CapabilityIndex` |
| `CapabilityNameTooLong { action_id: usize, capability_index: usize, len: usize, max: usize }` | `usize, usize, usize, usize` | `ActionId, CapabilityIndex, Length, MaxLength` |
| `CapabilityNameInvalid { action_id: usize, capability_index: usize }` | `usize, usize` | `ActionId, CapabilityIndex` |
| `CapabilityActionMismatch { contract_action_id: usize, capability_action_id: usize, capability_index: usize }` | `usize, usize, usize` | `ActionId, ActionId, CapabilityIndex` |
| `CapabilityDuplicate { action_id: usize, first_index: usize, duplicate_index: usize }` | `usize, usize, usize` | `ActionId, CapabilityIndex, CapabilityIndex` |
| `NonDeterministicPath { from_node: usize, to_node: usize }` | `usize, usize` | `NodeIndex, NodeIndex` |

**Count**: 15+ variants with untyped indices

---

## VIOLATION 3: God Object Anti-Pattern

The `ValidationError` enum attempts to cover **6 orthogonal concern domains**:

1. **Schema validation** (E01xx): duplicate keys, unknown fields, missing required fields
2. **Reference validation** (E02xx): unknown references, future references, secrets
3. **Control flow** (E03xx): invalid then-target, cycles, unreachable steps
4. **Type/taint/limits** (E04xx): type mismatches, resource limits, triggers
5. **Gate verification** (E05xx): expression stack, accessor paths, slot references
6. **Contract discovery** (E06xx): missing schema version, cue vet, monotonicity

**Scott Wlaschin DDD Principle**: Each domain should have its own error types, not one monolithic enum.

---

## VIOLATION 4: Massive Match Expression (Lines 392-462)

```rust
pub fn code(&self) -> SymbolicCode {
    let s: &'static str = match self {
        // 70+ arms covering all variants
    };
    SymbolicCode::from_static(s).unwrap_or(SymbolicCode::INTERNAL_INVARIANT)
}
```

This 70-line match is a maintenance burden. The `#[error(...)]` attributes already define the string codes; this duplicates them manually.

**Should**: Derive `Code` via a proc macro or use `strum` to generate the mapping.

---

## VIOLATION 5: Crate Sprawl

The `vb_validate` crate has **44 entries** in its `src/` directory:

```
control_flow.rs, diag_codes.rs, diag_convert.rs, diag_render.rs, 
fact_table.rs, forward_ref.rs, gate_07_stack.rs, gate_08_accessor.rs, 
gate_08_verus_proof.rs, gate_09_slots.rs, gate_10_node.rs, gate_11_loop.rs, 
gate_12_14_15.rs, gate_13_cycles.rs, gate_tests.rs, gates.rs, gates/, 
idempotency_contract.rs, kani_*, red_phase_proptest.rs, ref_unit_tests.rs, 
ref_validate.rs, references_tests.rs, references.rs, schema_doc.rs, 
schema_fields.rs, schema_id.rs, schema_tests.rs, schema.rs, schema/, 
secret_leak.rs, shared.rs, taint_prop.rs, type_check.rs, type_sigs.rs, 
type_taint_tests.rs, type_taint.rs
```

This suggests the library grew organically rather than being designed with clear boundaries.

---

## Required Refactors

### 1. Split the 471-line lib.rs into:

```
vb_validate/src/
├── lib.rs           # < 100 lines: re-exports only
├── error/
│   ├── mod.rs       # ValidationError enum (no impl, just enum + docs)
│   ├── schema.rs    # Schema errors (E01xx)
│   ├── reference.rs # Reference errors (E02xx)  
│   ├── control.rs   # Control flow errors (E03xx)
│   ├── type_.rs      # Type/taint/limit errors (E04xx)
│   ├── gate.rs      # Gate verification errors (E05xx)
│   └── contract.rs  # Contract discovery errors (E06xx)
└── types/
    ├── field_name.rs
    ├── version.rs
    ├── identifier.rs
    ├── reference.rs
    ├── secret_name.rs
    ├── step_name.rs
    ├── resource_type.rs
    ├── accessor_index.rs
    ├── slot_index.rs
    └── ... (one file per value object)
```

### 2. Create domain value objects:

```rust
// types/identifier.rs
pub struct Identifier(Box<str>);

impl Identifier {
    pub fn new(s: impl Into<Box<str>>) -> Self { ... }
    pub fn as_str(&self) -> &str { &self.0 }
}
```

### 3. Derive symbolic codes via macro instead of manual match

### 4. Reduce `ValidationError` to a sum type over domain-specific error enums

---

## Risk Assessment

| Issue | Severity | Remediation Effort |
|-------|----------|---------------------|
| 471-line file | CRITICAL | High (require module split) |
| Primitive obsession | HIGH | Medium (add ~20 value types) |
| God Object enum | HIGH | High (sum type decomposition) |
| Manual code() match | MEDIUM | Low (proc macro) |

---

## Recommendations

1. **Immediate**: Split `lib.rs` into `error/` and `types/` submodules
2. **Short-term**: Introduce `Identifier`, `Version`, `Reference`, `FieldName` value objects
3. **Long-term**: Extract each error family into its own crate (`vb_validate_schema`, etc.)
4. **Automated**: Add CI gate enforcing `<=300 line limit` per file

---

*Report generated by architectural-drift enforcer*
