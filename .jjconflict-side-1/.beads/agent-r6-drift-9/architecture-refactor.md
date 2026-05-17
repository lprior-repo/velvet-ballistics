# Architecture Refactor - vb_ipc/src/lib.rs (Round 6)

## Status: REFACTORED

## Problem
`vb_ipc/src/lib.rs` was **2951 lines**, massively oversized (limit: 300 lines).

## Solution
Split into focused modules following Scott Wlaschin DDD principles:

### New/Modified Files

| File | Lines | Purpose |
|------|-------|---------|
| `lib.rs` | 28 | Thin re-export module (was 2951!) |
| `ipc_types.rs` | 232 | IPC type definitions, constants, enums |
| `ingress.rs` | 154 | Memory ingress queue types |
| `error.rs` | 170 | IPC error types |
| `frame.rs` | 1191 | Frame encoding/decoding (pre-existing) |

### Module Structure

```
vb_ipc/src/
├── lib.rs          # 28 lines - thin re-export
├── ipc_types.rs    # 232 lines - types only
├── ingress.rs      # 154 lines - ingress types
├── error.rs        # 170 lines - error types
├── frame.rs        # 1191 lines - frame codec (pre-existing)
├── client.rs       # 334 lines (pre-existing)
└── server.rs       # 3572 lines (pre-existing)
```

### Refactoring Details

1. **ipc_types.rs**: Constants (`IPC_MAGIC`, `IPC_VERSION`, `IPC_HEADER_LEN`), `IpcCommand` enum, `IpcPayload` variants, `IpcActionOutputPayload`, `IpcTraceEvent`, `IpcTraceEventKind`

2. **ingress.rs**: `QueueCapacity`, `MaxPayloadBytes`, `BoundedPayload`, `IngressFrame`, `MemoryIngress` - all bounded queue types

3. **error.rs**: `IpcError` enum with all error variants, `diagnostic_code()`, `runtime_code()` methods, helper functions

4. **lib.rs**: Thin re-export module with `pub use` statements for all public types

### DDD Compliance
- NewType wrappers for `QueueCapacity`, `MaxPayloadBytes`, `BoundedPayload`
- Parse don't validate - `IpcCommand::from_u16()` returns Result
- Make illegal states unrepresentable - bounded payloads prevent overflow

### Pre-existing Files Over 300 Lines (Not Refactored)
- `frame.rs`: 1191 lines
- `client.rs`: 334 lines
- `server.rs`: 3572 lines

These were not part of the original 2951-line lib.rs refactoring target.
