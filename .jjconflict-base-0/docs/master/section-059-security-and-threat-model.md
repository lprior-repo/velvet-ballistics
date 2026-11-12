---
section: 59
title: "Security and Threat Model"
parent: velvet-ballistics-MASTER.md
---

## 59. Security and Threat Model


### Trusted Components

Compiled IR, Fjall database, runtime engine.

### Untrusted Inputs

Workflow YAML source, IPC client payloads, action outputs, persisted bytes during recovery.

### Threat Model

| Threat | Mitigation |
|--------|-----------|
| Malformed YAML | Strict parser, typed validation errors |
| Malformed IPC frames | Magic validation, length bounds, typed IPC errors, fuzz coverage |
| Oversized payloads | Bounded frames, bounded queues, typed `PayloadTooLarge` |
| Non-idempotent replay | `ActionReplayTracker` blocks re-execution, `Idempotency` policy |
| Digest tampering | BLAKE3 digests on source, IR, blobs. Mismatch → typed error, no silent continue |
| Secret leak via diagnostics | Taint tracking on action outputs. No raw secret values in hot state |
| Local privilege escalation | Unix socket permissions. No authentication in v1 |
| DoS via resource exhaustion | Bounded queues, bounded retries, bounded expression stacks, bounded trace rings |
| Storage corruption | Fjall WAL replay, snapshot recovery, typed storage errors |

---
