# Architectural Drift Refactor - vb_ipc/src/client.rs

## Summary
Refactored `vb_ipc/src/client.rs` (334 lines → 5 modules, all ≤300 lines).

## Original Problem
- File `client.rs` exceeded 300-line limit at 334 lines
- All implementation (connection, request building, response parsing) mixed with error types and tests

## Refactor Split

| File | Lines | Responsibility |
|------|-------|----------------|
| `client_conn.rs` | 116 | IpcClient struct, connect, send_raw, recv_response_header, recv_response_payload, recv_response, send_command, health, shutdown |
| `client_error.rs` | 29 | IpcClientError enum |
| `client_request.rs` | 15 | send_command free function |
| `client_response.rs` | 6 | recv_response free function (re-export) |
| `client.rs` | 202 | Re-exports, tests |

## Module Structure
```
vb_ipc/src/
├── client.rs        # Re-exports + tests (202 lines)
├── client_conn.rs   # Connection handling + IpcClient impl (116 lines)
├── client_error.rs  # IpcClientError enum (29 lines)
├── client_request.rs # Request building (15 lines)
└── client_response.rs # Response parsing re-export (6 lines)
```

## Verification
- `cargo check -p vb_ipc`: ✓ Compiles
- `cargo test -p vb_ipc`: ✓ 293 tests pass
- All files ≤300 lines: ✓

## DDD Compliance
- No primitive obsession introduced (using proper types from crate root)
- No unsafe, unwrap, expect, panic: ✓
- Single responsibility per module: ✓
