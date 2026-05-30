# Architectural Drift Report: `vb_ipc/src/server/mod.rs`

**File**: `crates/vb_ipc/src/server/mod.rs`  
**Total Lines**: 194 (PASSES - under 300)  
**Analysis Date**: 2026-05-29  
**Status**: `PERFECT` (line count) / `DRIFT DETECTED` (DDD cohesion)

---

## 1. Line Count Check

| Metric | Value | Threshold | Status |
|--------|-------|-----------|--------|
| Total lines | 194 | 300 | ✅ PASS |

---

## 2. DDD Cohesion Analysis

### 2.1 Module Role
This file serves as the **IPC server boundary module**. It defines:
- The `IpcServer` struct (infrastructure entity)
- The `ClientConnection` struct (infrastructure entity)
- The `IpcResponse` enum (domain response model)
- The `WorkflowResolver` trait (domain interface)
- The `WorkflowResolutionError` enum (domain error type)
- Public re-exports from submodules

### 2.2 Cohesion Violations

#### VIOLATION 1: `IpcResponse` God Object (HIGH SEVERITY)
The `IpcResponse` enum contains **22 variants** handling unrelated concerns:

```rust
pub enum IpcResponse {
    AcceptedRun { run_id: u64 },       // Run lifecycle
    Healthy,                           // Health check
    ShuttingDown,                      // Lifecycle
    TraceCount { count: u32 },         // Trace query
    Events { events: Vec<IpcTraceEvent> }, // Trace query
    Inspected { run_id: u64 },        // Run inspection
    BadRequest,                        // Protocol error
    PayloadError { diagnostic: u16, message: String }, // Protocol error
    CommandPayloadMismatch,            // Protocol error
    WorkflowResolutionRequired,        // Workflow resolution
    WorkflowResolutionUnsupported,     // Workflow resolution
    WorkflowDigestMismatch,           // Workflow resolution
    CountOutOfRange { actual: usize, limit: u32 }, // Protocol error
    FrameError { message: String },    // Protocol error
    RuntimeError { message: String },  // Runtime error
    RunList { runs: Vec<RunSummary> }, // Run query
    Metrics(crate::RuntimeMetrics),    // Metrics
    VerifyWorkflow { result: crate::VerificationResult }, // Verification
    TaintReport { ... },               // Security analysis
    WorkflowGraph { nodes, edges },    // Workflow graph
}
```

**Problem**: This enum violates the Single Responsibility Principle. It combines:
- Run lifecycle responses
- Protocol error responses
- Workflow resolution responses
- Metrics/verification responses
- Security analysis responses

**Scott Wlaschin DDD**: Should be split into bounded contexts:
- `RunResponse` (lifecycle + inspection)
- `ProtocolResponse` (errors)
- `WorkflowResponse` (resolution + verification)
- `TelemetryResponse` (traces + metrics)

#### VIOLATION 2: Primitive Obsession (MEDIUM SEVERITY)
- `run_id: u64` should be `RunId` newtype
- `count: u32` should be `EventCount` newtype
- `diagnostic: u16` should be `DiagnosticCode` newtype
- `actual/limit` in `CountOutOfRange` use raw `usize`/`u32`

#### VIOLATION 3: Infrastructure/Domain Mixing (MEDIUM SEVERITY)
The module mixes:
- **Domain**: `IpcResponse`, `WorkflowResolver`, `WorkflowResolutionError`
- **Infrastructure**: `mio::Poll`, `mio::net::UnixListener`, `mio::Events`, `std::collections::HashMap`

DDD best practice: Infrastructure should be in `impl_` submodule, domain types should be isolated.

---

## 3. Identified Violations Summary

| # | Violation | Severity | Type | Location |
|---|-----------|----------|------|----------|
| 1 | `IpcResponse` has 22 variants (God Object) | HIGH | DDD Cohesion | Lines 57-114 |
| 2 | Primitive obsession for IDs | MEDIUM | DDD | Lines 61, 71, 85 |
| 3 | Infrastructure types in domain module | MEDIUM | DDD Layering | Lines 40-54 |
| 4 | Re-exports leak implementation details | LOW | Encapsulation | Lines 140-143 |

---

## 4. DDD Smell Assessment

**Overall Smell**: `MEDIUM-HIGH`

The file is well under the 300-line limit, but the `IpcResponse` enum is a classic God Object anti-pattern. The enum handles too many distinct concerns that should be separated into distinct bounded contexts.

**Recommended Refactoring**:
1. Split `IpcResponse` into:
   - `RunResponse`: `AcceptedRun`, `Inspected`, `RunList`
   - `ProtocolResponse`: `BadRequest`, `PayloadError`, `CommandPayloadMismatch`, `CountOutOfRange`, `FrameError`
   - `WorkflowResponse`: `WorkflowResolutionRequired`, `WorkflowResolutionUnsupported`, `WorkflowDigestMismatch`, `VerifyWorkflow`, `WorkflowGraph`
   - `TelemetryResponse`: `TraceCount`, `Events`, `Metrics`
   - `SystemResponse`: `Healthy`, `ShuttingDown`, `RuntimeError`

2. Wrap primitives in newtypes:
   ```rust
   pub struct RunId(u64);
   pub struct EventCount(u32);
   pub struct DiagnosticCode(u16);
   ```

---

## 5. Priority Assessment

| Aspect | Priority | Rationale |
|--------|----------|-----------|
| Line Count | **NONE** | File is 194 lines, well under 300 |
| DDD Cohesion | **MEDIUM** | God Object enum needs splitting |
| Primitive Types | **LOW** | Not blocking, but reduces type safety |

**Overall Priority**: `MEDIUM` - DDD refactoring recommended but not blocking.

---

## 6. Submodule Analysis

The file correctly declares submodules:
- `dispatch`, `error`, `handlers`, `helpers`, `impl_`, `ticket`, `trace`

The actual `IpcServer` implementation is correctly delegated to `impl_`.rs (275 lines).

---

## 7. Conclusion

| Metric | Result |
|--------|--------|
| Lines | 194 ✅ |
| DDD Cohesion | DRIFT DETECTED ⚠️ |
| Priority | MEDIUM |
| Action Required | Refactor `IpcResponse` enum, wrap primitives |

**STATUS**: `DRIFT DETECTED` - The file passes line count but has significant DDD cohesion issues centered on the `IpcResponse` God Object enum.
