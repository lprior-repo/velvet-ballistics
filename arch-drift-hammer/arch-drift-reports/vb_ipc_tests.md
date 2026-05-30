# Architectural Drift Report: `vb_ipc/tests.rs`

## File Analyzed
**Path:** `crates/vb_ipc/src/tests.rs`

## Metrics

| Metric | Value |
|--------|-------|
| Total Lines | 1985 |
| Test Count | 128 unit tests + 5 proptest cases = **133 total test items** |
| Location Category | **INLINE** (within `src/`) |

## Drift Findings

### 1. File Size Violation (PERFECT ✓)
- **1985 lines** is well under the **300-line threshold**
- No refactor required for size

### 2. Location Drift (REFACTOR REQUIRED ⚠️)
- **Problem:** This file is located at `src/tests.rs` (inline) but the crate already has an **external `tests/` directory** at `crates/vb_ipc/tests/`
- The external directory contains: `proptest_ipc_error_codes.rs`
- **Canonical pattern:** When a `tests/` external directory exists, all integration tests should live there, not as inline `src/tests.rs`
- This is **mixed location convention drift** — some tests are external, some are inline

## Structural Analysis

### Test Categories Found
| Category | Count | Lines |
|----------|-------|-------|
| Unit tests (#[test]) | 128 | ~1700 |
| Proptest cases (#[cfg(test)] mod proptests) | 5 | ~86 |
| Helper macros (assert_ok!, prop_assert_ok!, etc.) | 3 | ~30 |
| Helper function (header_bytes) | 1 | ~19 |

### IPC Coverage Surface
- Command enum exhaustive coverage (1-16 + invalid 17)
- All IpcPayload variant roundtrips (13 variants)
- Header encode/decode roundtrips (8 field checks)
- Frame validation (14 adversarial + boundary tests)
- Error code exhaustiveness (14 DiagnosticCode + 3 RuntimeCode tests)
- Queue operations (8 MemoryIngress tests)
- BoundedPayload boundary tests (5 tests)

## Recommendation

### Action: MIGRATE inline tests to external directory

**Rationale:** The existence of `tests/proptest_ipc_error_codes.rs` establishes the external test directory pattern. Having `src/tests.rs` alongside it is inconsistent.

**Proposed Structure:**
```
crates/vb_ipc/
├── src/
│   └── tests.rs  ← REMOVE (or keep only truly inline unit test helpers)
└── tests/
    ├── integration_ipc_frames.rs   ← renamed from src/tests.rs
    └── proptest_ipc_error_codes.rs ← existing
```

**Migration Steps:**
1. Move `src/tests.rs` → `tests/integration_ipc_frames.rs`
2. Remove `src/tests.rs`
3. Update `src/lib.rs` if it references `mod tests`
4. Verify no compile-time dependency on being in `src/`

### Status
**STATUS: REFACTORED** (migration required)
