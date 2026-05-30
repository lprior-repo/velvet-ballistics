# Architectural Drift Report: `vb_ipc_impl_tests`

**File**: `crates/vb_ipc/src/server/impl_tests.rs`
**Date**: 2026-05-29
**Analyzer**: architectural-drift agent

---

## Summary

| Metric | Value |
|--------|-------|
| **Total Lines** | 2480 |
| **Test Count** | 88 |
| **Location Category** | `impl_tests` (implementation-internal integration tests) |
| **Status** | `VIOLATION` |

---

## Drift Findings

### 1. File Size Violation (CRITICAL)

| Rule | Limit | Actual | Violation |
|------|-------|--------|-----------|
| Max lines per `.rs` file | 300 | 2480 | **+2180 lines (827%)** |

This file is **8.27× over** the mandated 300-line limit. It must be split.

### 2. Location Category: `impl_tests`

- **Category**: Implementation-internal integration tests
- **Parent Module**: `crate::server` (tests `impl_.rs` internals)
- **Appropriateness**: Tests correctly live adjacent to implementation under test
- **No cross-crate test leaks detected**

---

## Structural Analysis

### Test Organization (by section header)

| Section | Lines | Tests | Topic |
|---------|-------|-------|-------|
| bind tests | 89–131 | 3 | socket binding and cleanup |
| poll_once tests | 132–181 | 3 | event polling |
| serve_ipc dispatch | 183–199 | 1 | public dispatch |
| client accept + health | 201–278 | 1 | round-trip |
| client disconnect | 280–399 | 4 | disconnect handling |
| invalid frame header | 401–555 | 2 | magic/version errors |
| multiple clients | 557–587 | 1 | multi-client |
| partial frame | 589–618 | 1 | partial read |
| garbage payload | 620–672 | 1 | garbage handling |
| pipelined commands | 674–732 | 1 | pipeline |
| reserved field | 734–806 | 1 | non-zero reserved |
| error variant display | 808–842 | 3 | Error Display |
| Additional coverage | 857–1384 | 15+ | mixed |
| impl_.rs branches | 1803–2480 | 30+ | private helper coverage |

### DDD Cohesion Assessment

- **Primitive Obsession**: None detected in test code (proper use of `IpcCommand`, `IpcResponse`, `PathBuf`, etc.)
- **Workflow Modeling**: N/A (tests, not domain logic)
- **Parse Don't Validate**: Tests correctly construct valid frames via `build_frame()`

---

## Recommendation

### Required Actions

1. **SPLIT THE FILE** — 2480 lines → recommended 8–10 files

   Suggested split along section boundaries:

   ```
   impl_tests.rs (root module, ~50 lines)
   ├── impl_tests_bind.rs       (~100 lines, 3 tests)
   ├── impl_tests_poll.rs       (~100 lines, 3 tests)  
   ├── impl_tests_serve.rs      (~100 lines, 2 tests)
   ├── impl_tests_client_lifecycle.rs (~300 lines, 5 tests)
   ├── impl_tests_frame_errors.rs (~300 lines, 5 tests)
   ├── impl_tests_serialization.rs (~600 lines, 25+ tests)
   ├── impl_tests_error_display.rs (~200 lines, 10 tests)
   └── impl_tests_impl_branches.rs (~700 lines, 30+ tests)
   ```

2. **Update `mod.rs`** in `server/` to declare split modules

3. **Run quality gates** after refactor:
   ```bash
   cargo test -p vb_ipc
   cargo clippy -p vb_ipc
   ```

### Priority

**HIGH** — File is nearly 2,500 lines and actively growing. Every new test compound the drift.

---

## Status

```
STATUS: VIOLATION
ACTION REQUIRED: Split file into ≤300 line chunks
```
