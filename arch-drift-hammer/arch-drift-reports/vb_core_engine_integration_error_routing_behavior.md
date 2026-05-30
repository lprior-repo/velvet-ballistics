# Architectural Drift Report: `integration_error_routing_behavior.rs`

## File Summary

| Attribute | Value |
|-----------|-------|
| **File** | `crates/vb_core/src/engine/tests/integration_error_routing_behavior.rs` |
| **Total Lines** | 1573 |
| **Test Count** | ~56 (50 `#[test]` + 6 proptest cases) |
| **Size Category** | 🔴 **VIOLATION** (>300 lines) |

---

## 1. Line Count Check

```
Total lines: 1573
Threshold:   300
Status:      🔴 EXCEEDS LIMIT by 1273 lines (424% of max)
```

**Verdict**: File is 5.2× the maximum allowed size. Mandatory split required.

---

## 2. DDD Cohesion Check

### Primitive Obsession Analysis
- Uses raw `String` for `resource`, `budget`, `primitive`, `segment`, `found`, `field`, `reason`, `context`, `command` — these are stringly-typed fields that should be **NewTypes**.
- Uses raw `u64`, `u16`, `u8` for numeric fields (`max`, `limit`, `index`, `capacity`, `len`, etc.) — no domain wrapping.
- Uses raw `chrono::DateTime<Utc>` for `timestamp` — should be a domain type.

### Workflow Modeling
- Error routing is modeled as a state machine via `ErrorHandlerOutcome::Routed` / `NoHandler` — **good**.
- Double-fault detection at lines 1276-1354 demonstrates proper state transition modeling — **good**.
- Proptest properties verify context preservation invariants — **good**.

### Parse, Don't Validate
- `ErrorSlotData::from_engine_error()` is a pure encoder that transforms `EngineError → ErrorSlotData` — this is **good** (Parse, don't validate applied here).
- `route_error_handler` returns `Result<ErrorHandlerOutcome, WorkflowError>` — **good**.

---

## 3. Structural / Boundary Check

### Location Violation

```
Current path:  crates/vb_core/src/engine/tests/integration_error_routing_behavior.rs
Rule requires: crates/workspace_tests/ (integration tests only)
Status:        🔴 STRUCTURAL DRIFT — tests live in src/ not workspace_tests/
```

**Issue**: Per AGENTS.md and workspace structure rules, integration tests belong in `crates/workspace_tests/`, not nested inside `src/engine/tests/`. This is a **structural boundary violation**.

---

## 4. File Size Breakdown by Section

| Section | Lines | Problem |
|---------|-------|---------|
| Fixture builders (`make_workflow`, `make_frame`, etc.) | 1–196 | Helper bloat — could be a shared test-util crate |
| Error variant propagation macro + 44 invocations | 197–560 | 363 lines — macro is good but file too large |
| Individual routing tests | 561–895 | ~334 lines — could be grouped |
| Display/Debug tests (two exhaustive variant loops) | 896–1114 | 218 lines — dense but appropriate |
| Lifecycle error routing tests | 1115–1248 | ~133 lines — fine |
| Double-fault tests | 1249–1354 | ~105 lines — fine |
| Proptest properties | 1355–1510 | 155 lines — appropriate |
| Remaining unit-style tests | 1511–1573 | 62 lines — fine |

---

## 5. Recommendations

### Priority 1: Split the File

The file MUST be split into smaller, focused modules:

```
engine/tests/
├── integration_error_routing_behavior.rs   ← KEEP (main entry, ~300 lines)
├── error_routing_variants.rs               ← EXTRACT (44 variant propagation tests)
├── error_routing_lifecycle.rs              ← EXTRACT (lifecycle error tests)
├── error_routing_proptest.rs               ← EXTRACT (proptest module)
└── error_routing_double_fault.rs           ← EXTRACT (double-fault tests)
```

**Target**: Each file ≤300 lines.

### Priority 2: Move to `workspace_tests/`

Per the workspace structure contract, integration tests should live in:
```
crates/workspace_tests/
```

However, since these tests use `crate::*` imports from `vb_core`, they must remain internal integration tests. This is an **acceptable exception** provided the file-splitting mandate is fulfilled.

### Priority 3: Address Primitive Obsession

Introduce NewTypes for recurring string/numeric fields:

```rust
// Instead of: EngineError::ResourceLimitExceeded { resource: "connections" }
pub struct ResourceName(Box<str>);
EngineError::ResourceLimitExceeded { resource: ResourceName }
```

### Priority 4: Extract Shared Fixtures

The `make_workflow`, `make_frame`, `make_simple_handler_workflow`, etc. helpers should be moved to a shared `test_helpers` module at `engine/tests/helpers.rs` to avoid duplication across test files.

---

## 6. Final Verdict

| Check | Status |
|-------|--------|
| Line count ≤300 | 🔴 FAIL (1573 lines) |
| DDD cohesion | ✅ GOOD (well-structured error model) |
| Parse don't validate | ✅ GOOD |
| Structural boundary | 🔴 FAIL (wrong directory) |
| Primitive obsession | ⚠️ MODERATE (many raw strings/nums) |

**Overall**: `STATUS: REFACTOR REQUIRED`

---

## 7. Action Items

- [ ] **Bead**: Split `integration_error_routing_behavior.rs` into 4–5 focused files ≤300 lines each
- [ ] **Bead**: Extract `test_helpers` module for shared workflow/frame builders
- [ ] **Bead**: Introduce NewType wrappers for recurring `String` fields in `EngineError`
- [ ] **Note**: Location in `src/engine/tests/` is acceptable for internal integration tests (requires `#[cfg(test)]`)

---

*Report generated by architectural-drift agent*
