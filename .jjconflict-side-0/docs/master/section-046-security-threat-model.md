---
section: 46
title: "Security Threat Model"
parent: velvet-ballistics-MASTER.md
---

## 46. Security Threat Model

Actors:

```text
operator
local OS user
AI agent
compiler process
runtime process
IPC client
action worker
secret provider
Fjall database
artifact store
```

Untrusted inputs:

```text
SDK source tokens
artifact bytes from disk
IPC frames
action outputs
persisted records during recovery
operator-supplied input payloads
mock fixtures
AI repair patches
```

Mitigations:

```text
macro grammar restriction
artifact digest validation
schema version validation
bounded payloads
capability grant checks
secret handle boundary
outbox/inbox side-effect discipline
idempotency certificates
replay divergence detection
storage record checksums/digests
single-writer database lock
compile-fail tests for forbidden SDK behavior
```

No v1 claim of distributed security, multi-tenant isolation, remote authentication, or complete side-channel secrecy.

---

