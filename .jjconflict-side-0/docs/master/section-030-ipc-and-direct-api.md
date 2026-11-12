---
section: 30
title: "IPC and Direct API"
parent: velvet-ballistics-MASTER.md
---

## 30. IPC and Direct API

Fastest ingress is direct Rust API. External local ingress uses binary IPC.

Command registry is generated from `contracts/ipc_commands.ron`.

Required commands:

```text
InstallArtifact
SubmitRun
CancelRun
InspectRun
ListEvents
AnswerAsk
CompleteAction
FailAction
ReplayRun
IncidentReport
DrainTrace
Health
Shutdown
```

Forbidden in IPC runtime core:

```text
HTTP
JSON routing
text command protocol
runtime SDK source submission
runtime YAML submission
unbounded payloads
unbounded response pages
blocking producer admission
```

---

