---
section: 50
title: "IPC Transport, Backpressure, and Error Codes"
parent: velvet-ballistics-MASTER.md
---

## 50. IPC Transport, Backpressure, and Error Codes


### Transport

- Socket type: Unix stream socket.
- Max concurrent clients: 256.
- Read chunk: 4096 bytes.
- Backpressure: Bounded command queue (`ArrayQueue`). Queue full → `IpcError::Full` (E3001).
- Per-connection: Non-blocking writes with writable-event polling via `mio`.
- Pipelining: Not supported in v1 — one command per connection, response before next command.
- Shutdown: `Shutdown` acknowledged. Pending runs are not forcibly cancelled.

### IpcResponse Variants

```text
AcceptedRun { run_id: u64 }
Healthy
ShuttingDown
TraceCount { count: u32 }
Events { events: Vec<IpcTraceEvent> }
Inspected { run_id: u64 }
BadRequest
PayloadError { diagnostic: u16, message: String }
CommandPayloadMismatch
WorkflowResolutionRequired
WorkflowResolutionUnsupported
WorkflowDigestMismatch
CountOutOfRange { actual: usize, limit: u32 }
FrameError { message: String }
RuntimeError { message: String }
```

### IpcError Variants with Diagnostic Codes

```text
E3001  Full
E3002  Disconnected
E3003  PayloadTooLarge { actual, limit }
E3004  InvalidMagic { actual }
E3005  UnsupportedVersion { actual }
E3006  UnknownCommand(u16)
E3007  ReservedNonZero { actual }
E3008  PayloadLengthMismatch { header, actual }
E3009  HeaderEncodeFailed
E300A  HeaderDecodeFailed
E300B  PayloadLengthOutOfRange { actual }
E300C  PayloadEncodeFailed
E300D  PayloadDecodeFailed
E300E  ResponseDecodeFailed
```

---
