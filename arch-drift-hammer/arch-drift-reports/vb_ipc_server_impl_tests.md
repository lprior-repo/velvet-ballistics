# Architectural Drift Report: `vb_ipc/src/server/impl_tests.rs`

**File:** `crates/vb_ipc/src/server/impl_tests.rs`
**Date:** 2026-05-29
**Analyst:** architectural-drift agent

---

## Summary

| Metric | Value |
|--------|-------|
| **Total Lines** | 2480 |
| **Test Count** | 88 |
| **Size Category** | MASSIVE — exceeds 300-line threshold by 8.27× |
| **Drift Status** | 🔴 SEVERE ARCHITECTURAL DRIFT |

---

## Findings

### 1. File Size Violation
- **Threshold:** <300 lines (per architectural-drift skill)
- **Actual:** 2480 lines
- **Violation:** 8.27× over limit

### 2. Test Distribution
The file contains 88 #[test] functions covering:
- `bind` behavior (3 tests)
- `poll_once` / `poll_once_with_resolver` (8 tests)
- `serve_ipc` dispatch (4 tests)
- Client lifecycle (6 tests)
- Frame parsing/error handling (10 tests)
- Serialization roundtrips (31 tests)
- Private helper coverage via `impl_` module (26 tests)

### 3. Structural Concerns
- **Helper bloat:** `read_exact_timeout` (20 lines), `build_frame` (17 lines), `make_runtime` (4 lines), `make_client` (3 lines), `temp_socket_path` (3 lines) are defined inline
- **RAII guards:** `CleanupPath` and `CleanupDir` are 8 lines each, appear at end of file
- **Repeated pattern:** 20+ tests follow identical boilerplate: create server, create client, accept, send frame, poll, read response
- **Section headers:** 15 section dividers adding noise without logic

### 4. DDD Cohesion
- Single file tests multiple bounded contexts (server bind lifecycle, frame parsing, serialization, error display)
- `impl_tests.rs` name is generic; file tests `impl_.rs` but also `dispatch.rs`, `error.rs`, and cross-cutting IPC protocol behavior

---

## Recommendations

### Priority 1 — Split by Domain Concern
```
server/
├── impl_tests.rs              # Keep: bind/poll_once/serve_ipc core (target: ~300 lines, ~20 tests)
├── impl_tests_bind.rs         # New: bind-specific behavior (3 tests)
├── impl_tests_poll.rs         # New: poll_once variants (8 tests)  
├── impl_tests_dispatch.rs     # New: serve_ipc + resolver (4 tests)
├── impl_tests_client_lifecycle.rs  # New: connect/disconnect/reconnect (6 tests)
├── impl_tests_frame_parsing.rs # New: invalid magic, version, garbage, partial frames (6 tests)
├── impl_tests_serialization.rs # New: ALL roundtrip tests (31 tests) — THIS ALONE IS ~600 lines
└── impl_tests_error_display.rs # New: IpcServerError Display impl (10 tests)
```

### Priority 2 — Extract Shared Helpers
Move to `server/test_helpers.rs`:
- `temp_socket_path`
- `make_runtime`
- `make_client`
- `build_frame`
- `read_exact_timeout`
- `CleanupPath`
- `CleanupDir`

### Priority 3 — Reduce Serialization Redundancy
31 near-identical roundtrip tests should use a macro:
```rust
macro_rules! define_roundtrip {
    ($name:ident, $variant:expr) => {
        #[test]
        fn $name() {
            let original = $variant;
            let encoded = postcard::to_allocvec(&original).expect("encode");
            let decoded: IpcResponse = postcard::from_bytes(&encoded).expect("decode");
            assert_eq!(decoded, original);
        }
    };
}
```

---

## Risk Assessment

| Risk | Level | Notes |
|------|-------|-------|
| Maintenance burden | 🔴 Critical | 2480-line files resist review; bug risk high |
| Test isolation | 🟡 Moderate | Shared helpers + RAII guards create implicit coupling |
| CI parallelization | 🟡 Moderate | Single file cannot be parallelized against other test files |
| Coverage clarity | 🟢 Low | Well-organized sections; intent is clear |

---

## Verdict

**ACCEPT** with mandatory refactor. The test quality is high (good coverage, clear naming, proper cleanup), but the file size is a structural violation. The file must be shredded into ≤300-line chunks aligned with DDD bounded contexts before this codebase can be considered architecturally compliant.
