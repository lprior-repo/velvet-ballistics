# ARCHITECTURAL DRIFT HAMMER REPORT

**File Attacked:** `crates/vb_validate/src/diag_render.rs`
**Original Lines:** 638
**Line Limit:** 300
**Violation:** 338 LINE OVERAGE (112.7% over limit)

---

## EXECUTIVE SUMMARY

This file is a SINGLE RESPONSIBILITY PRINCIPLE crime scene. It concatenates:
- Diagnostic rendering logic (lines 1–375)
- Inline tests (lines 377–638)

That's **260 lines of tests** suffocated inside implementation code, contributing 40.8% of the file's mass.

---

## VIOLATION 1: LINE COUNT (<300 RULE)

| Section | Lines | Status |
|---------|-------|--------|
| `diagnostic_from_error` | 14–19 | 6 lines |
| `diagnostic_fallback_symbolic` | 21–33 | 13 lines |
| `diagnostic_from_parts` | 35–51 | 17 lines |
| `error_code` | 53–57 | 5 lines |
| `error_diagnostic_parts` | 59–375 | **317 lines** |
| `render_tests` module | 377–638 | **262 lines** |
| **TOTAL** | | **638 lines** |

The `error_diagnostic_parts` function alone (317 lines) exceeds the 300-line limit. It is a flat match statement with no decomposition.

---

## VIOLATION 2: PRIMITIVE OBSESSION (Scott Wlaschin DDD)

### A. String Type Abuse

The `ValidationError` enum uses raw `String` for domain identifiers. Every `format!` call in `error_diagnostic_parts` admits this:

| Field | Raw Type | Domain Meaning |
|-------|----------|----------------|
| `field` | `String` | Field identifier |
| `version` | `String` | Version string |
| `id` | `String` | Step/action ID |
| `reference` | `String` | Variable reference |
| `secret` | `String` | Secret name |
| `trigger` | `String` | Trigger type |
| `file` | `String` | File path |
| `chain` | `String` | Slot dependency chain |
| `name` | `String` | Capability name |
| `resource` | `String` | Resource limit name |
| `detail` | `String` | Constraint violation detail |
| `step` | `String` | Step identifier |

**Refactor Required:** Each of these should be a NewType:
- `FieldName(String)` — validated field identifier
- `VersionString(String)` — semver-compatible version
- `StepId(String)` — validated step identifier
- `Reference(String)` — `$`-prefixed reference
- `SecretName(String)` — secret identifier
- `TriggerType(String)` — trigger kind
- `FilePath(String)` — file reference
- `DependencyChain(String)` — slot chain representation
- `CapabilityName(String)` — capability identifier
- `ResourceLimitName(String)` — resource limit name

### B. Integer Primitive Abuse

These raw integer types are used without domain wrapping:

| Field | Raw Type | Domain Meaning |
|-------|----------|----------------|
| `expr_index` | `i32` | Expression index |
| `accessor_index` | `i32` | Accessor index |
| `slot` | `i32` | Slot number |
| `slot_count` | `u32` | Slot count |
| `node_index` | `i32` | Node index |
| `action_id` | `i32` | Action ID |
| `capability_index` | `usize` | Capability array index |
| `declared` | `i32` | Declared stack depth |
| `limit` | `i32` | Stack limit |
| `computed` | `i32` | Computed value |
| `depth` | `u32` | Path depth |
| `max` | `u32` | Maximum allowed |
| `symbol` | `i32` | Symbol index |
| `symbols_count` | `i32` | Symbol count |
| `node_count` | `u32` | Node count |
| `source_node` | `i32` | Source node index |
| `label` | `String` | Loop label |
| `first_index` | `usize` | First occurrence index |
| `duplicate_index` | `usize` | Duplicate occurrence index |
| `segment_index` | `i32` | Path segment index |
| `from_node` | `i32` | Source node |
| `to_node` | `i32` | Target node |
| `len` | `usize` | String length |
| `expected` | `u32` | Expected version |
| `actual` | `u32` | Actual version |

**Refactor Required:** Each index/count type should be a NewType:
- `ExprIndex(i32)`
- `AccessorIndex(i32)`
- `SlotIndex(i32)` / `SlotCount(u32)`
- `NodeIndex(i32)` / `NodeCount(u32)`
- `ActionId(i32)`
- `CapabilityIndex(usize)`
- `StackDepth(i32)`
- `PathDepth(u32)`
- `SymbolIndex(i32)` / `SymbolCount(i32)`
- `StepOffset(i32)`
- `SegmentIndex(i32)`
- `StringLength(usize)`
- `SchemaVersion(u32)`

---

## VIOLATION 3: SINGLE RESPONSIBILITY / FUNCTION LENGTH

`error_diagnostic_parts` (lines 59–375) is **317 lines** of pattern matching. This should be decomposed into:

1. A `SchemaErrorRenderer` struct with a `render` method
2. A `ReferenceErrorRenderer` struct
3. A `ControlFlowErrorRenderer` struct  
4. A `GateErrorRenderer` struct
5. A `CapabilityErrorRenderer` struct

Each renderer handles its own error family, matching the code family groupings in `diag_codes.rs` (E01xx schema, E02xx reference, E03xx control-flow, E04xx type/taint, E05xx gate, E06xx contract).

---

## VIOLATION 4: INLINE TESTS IN IMPLEMENTATION

The `render_tests` module (lines 377–638) is **262 lines** of test code embedded in the implementation file. Per workspace conventions:

- Tests belong in `crates/workspace_tests/` or a sibling `tests/` directory
- Or in a `diag_render/tests/` subdirectory
- The implementation file should be ≤300 lines

**Required Split:**
```
diag_render.rs          →  ~280 lines (implementation only)
diag_render/tests/      →  ~260 lines (render_tests moved here)
```

---

## REQUIRED REFACTORING PLAN

### Phase 1: NewType Definitions (new file: `types.rs`)

Create domain-specific newtypes for ALL primitives:

```rust
// Schema domain
pub struct FieldName(String);
pub struct VersionString(String);
pub struct StepId(String);

// Reference domain  
pub struct Reference(String);
pub struct SecretName(String);

// Gate domain
pub struct ExprIndex(i32);
pub struct AccessorIndex(i32);
pub struct SlotIndex(i32);
pub struct SlotCount(u32);
pub struct NodeIndex(i32);
pub struct NodeCount(u32);
// ... etc
```

### Phase 2: Renderer Decomposition

Split `error_diagnostic_parts` by error family:

```
diag_render/
├── lib.rs           re-exports
├── render.rs        diagnostic_from_error, error_code, diagnostic_from_parts, diagnostic_fallback_symbolic
├── schema.rs        SchemaErrorRenderer — handles E01xx variants
├── reference.rs     ReferenceErrorRenderer — handles E02xx variants
├── control_flow.rs  ControlFlowErrorRenderer — handles E03xx variants
├── type_taint.rs     TypeTaintErrorRenderer — handles E04xx variants
└── gate.rs          GateErrorRenderer — handles E05xx and E06xx variants
```

### Phase 3: Test Extraction

Move `render_tests` module to `diag_render/tests/render_tests.rs`.

---

## ARCHITECTURAL CONTRACT STATUS

| Rule | Status |
|------|--------|
| <300 lines per file | **VIOLATED** (638 lines, 112.7% over) |
| No primitive obsession | **VIOLATED** (14 String fields, 25+ integer fields) |
| Single responsibility | **VIOLATED** (one 317-line match) |
| Tests separated | **VIOLATED** (262 inline test lines) |
| Parse don't validate | **COMPLIANT** (ValidationError is exhaustive, rendering is pure) |

---

## RECOMMENDATION

**STATUS: REFACTOR REQUIRED**

This file MUST be refactored before landing. The hammer is dropped.

**Next Steps:**
1. Create `crates/vb_validate/src/diag_render/types.rs` with all NewType wrappers
2. Create `crates/vb_validate/src/diag_render/schema.rs` with `SchemaErrorRenderer`
3. Create `crates/vb_validate/src/diag_render/reference.rs` with `ReferenceErrorRenderer`
4. Create `crates/vb_validate/src/diag_render/control_flow.rs` with `ControlFlowErrorRenderer`
5. Create `crates/vb_validate/src/diag_render/gate.rs` with `GateErrorRenderer`
6. Collapse `diag_render.rs` to ≤300 lines (renderer delegation only)
7. Move `render_tests` to `crates/vb_validate/src/diag_render/tests/`
8. Update `crates/vb_validate/src/diag_render/mod.rs` with re-exports

---

*Arch drift hammer delivered by architectural-drift agent.*
*Report generated: 2026-05-29*
