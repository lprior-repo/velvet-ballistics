# Domain Model Review: vb-te1i Binary IPC

## Domain Shape

### Protocol Constants (constants.rs)
```
IPC_MAGIC:     u32 = 0x5642_4C54  // "VBLT" little-endian
IPC_VERSION:   u16 = 1
IPC_HEADER_LEN: usize = 24
```

Wire layout (bytes):
```
[0..3]   magic:        u32 LE
[4..5]   version:      u16 LE
[6..7]   command:      u16 LE  (1..=16)
[8..9]   flags:        u16 LE
[10..11] reserved:      u16 LE  (must be 0)
[12..19] correlation:   u64 LE
[20..23] payload_len:  u32 LE
```

### Command Domain (commands.rs)
16 concrete commands, each with a distinct `u16` ID:

| ID | Variant | Request Payload | Response Payload |
|---|---|---|---|
| 1 | `SubmitRun` | `SubmitRunPayload` | `RunSummary` |
| 2 | `SubmitRunInline` | `SubmitRunInlinePayload` | `RunSummary` |
| 3 | `CancelRun` | `CancelRunPayload` | `()` |
| 4 | `InspectRun` | `InspectRunPayload` | `RunState` |
| 5 | `ListEvents` | `ListEventsPayload` | `Vec<JournalEvent>` |
| 6 | `AnswerAsk` | `AnswerAskPayload` | `()` |
| 7 | `CompleteAction` | `CompleteActionPayload` | `()` |
| 8 | `FailAction` | `FailActionPayload` | `()` |
| 9 | `DrainTrace` | `DrainTracePayload` | `Vec<TraceRecord>` |
| 10 | `Health` | `()` | `HealthResponse` |
| 11 | `Shutdown` | `()` | `()` |
| 12 | `ListRuns` | `()` | `Vec<RunSummary>` |
| 13 | `GetMetrics` | `()` | `RuntimeMetrics` |
| 14 | `GetWorkflowGraph` | `GetWorkflowGraphPayload` | `WorkflowGraph` |
| 15 | `GetTaintReport` | `GetTaintReportPayload` | `TaintReport` |
| 16 | `VerifyWorkflow` | `VerifyWorkflowPayload` | `VerificationResult` |

### Error Domain (error.rs)
14 error variants, grouped:

**Decode/invalid-frame group** (runtime code: `IPC_FRAME_INVALID`):
- `InvalidMagic { actual: u32 }`
- `UnsupportedVersion { actual: u16 }`
- `UnknownCommand(u16)`
- `ReservedNonZero { actual: u16 }`
- `PayloadLengthMismatch { header: usize, actual: usize }`
- `HeaderDecodeFailed`
- `PayloadDecodeFailed`
- `ResponseDecodeFailed`

**Payload size group** (runtime code: `IPC_PAYLOAD_TOO_LARGE`):
- `PayloadTooLarge { actual: usize, limit: usize }`
- `PayloadLengthOutOfRange { actual: u32 }`

**Queue group** (runtime code: `QUEUE_FULL`):
- `Full`
- `Disconnected`

**Encode group** (no runtime code):
- `HeaderEncodeFailed`
- `PayloadEncodeFailed`

### Bounded Types (bounded.rs)
- `MaxPayloadBytes(NonZeroUsize)` — default 1 MiB, enforced in decode before payload read
- `QueueCapacity(NonZeroUsize)` — SPSC ingress queue capacity
- `BoundedPayload(Bytes)` — post-check payload wrapper; `.bytes().len() <= max.get()` invariant

### Frame Types (frame_types.rs)
- `IpcFrameHeader` — 24-byte decoded header with `command`, `flags`, `correlation`, `payload_len`
- `IpcFrame` — header + `BoundedPayload`; built via `IpcFrame::new` after header decode + size agreement check

---

## Type Invariants

| Type | Invariant |
|---|---|
| `IpcFrameHeader` | `payload_len <= MaxPayloadBytes::DEFAULT.get()` after decode |
| `BoundedPayload` | `self.bytes().len() <= max_payload.get()` always |
| `QueueCapacity` | Always constructed from `NonZeroUsize` |
| `MaxPayloadBytes` | Always constructed from `NonZeroUsize` |
| `IpcCommand` | Exactly `1..=16`; `from_u16` is total on this range |
| `IpcError::Full` / `Disconnected` | Queue-only errors; never returned from decode |

---

## Key Design Decisions

1. **Magic checked before version** — InvalidMagic is the first gate, cheapest rejection.
2. **Decode before allocation** — The 24-byte header is read-only; no heap allocation until `IpcFrame::new` with the actual payload.
3. **Reserved field enforced** — Protocol extensibility hook; non-zero is an error.
4. **Queue capacity is caller-configured** — `MemoryIngress::new(capacity: QueueCapacity)`; not hardcoded.
5. **Diagnostic codes are stable** — Every error variant maps to a fixed `DiagnosticCode` (0x3001–0x300E).
6. **Correlation is u64** — Fits any reasonable correlation ID; preserved end-to-end.

---

## Review Assessment

**Status**: ALIGNED

The domain model is clean:
- Finite set of commands (16)
- Closed error taxonomy (14)
- Bounded numeric types with explicit overflow checks (`u32_to_usize`)
- Decode-before-allocate ordering enforced structurally
- No inheritance, no associated type generics beyond serialization

No type-level issues found. All invariants are expressible as Verus `spec` fns or `invariant` blocks.
