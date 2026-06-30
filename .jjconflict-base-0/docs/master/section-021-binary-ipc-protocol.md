---
section: 21
title: "Binary IPC Protocol"
parent: velvet-ballistics-MASTER.md
---

## 21. Binary IPC Protocol


Fastest ingress is direct in-process Rust API. External local process ingress uses binary IPC.

Frame wire format:

```text
magic:       u32 = 0x56424C54  # VBLT
version:     u16
command:     u16
flags:       u16
reserved:    u16
correlation: u64
payload_len: u32
payload:     postcard-encoded bytes
```

Required IPC commands:

```text
SubmitRun
SubmitRunInline
CancelRun
InspectRun
ListEvents
AnswerAsk
CompleteAction
FailAction
DrainTrace
Health
Shutdown
```

Architectural decision: IPC v1 has exactly the 11 supported command identifiers
listed above. Their wire IDs are `1..=11` in that order. Every other `u16`
command value is reserved for future protocol versions and must decode as a
typed `UnknownCommand(value)`/equivalent and be rejected by dispatch. This
reserved range explicitly includes the former semantic query/verification IDs
`12..=16` (`ListRuns`, `GetMetrics`, `GetWorkflowGraph`, `GetTaintReport`, and
`VerifyWorkflow`); they are not supported IPC v1 commands unless a future master
contract revision assigns them explicit wire IDs and acceptance evidence.

Forbidden on IPC:

```text
HTTP ingress
JSON routing
unbounded channels
blocking producer admission
text command protocol
runtime YAML submission without prior compile/validation
```

IPC decoder requirements:

1. Validate magic before allocation.
2. Validate payload length against configured maximum before reading payload.
3. Decode Postcard into typed payloads only.
4. Return typed IPC errors for malformed frames.
5. Fuzz arbitrary bytes.

---
