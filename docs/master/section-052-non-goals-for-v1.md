---
section: 52
title: "Non-Goals for v1"
parent: velvet-ballistics-MASTER.md
---

## 52. Non-Goals for v1

Not v1:

```text
YAML workflow authoring
arbitrary Rust workflow execution
generated Rust workflow execution
HTTP runtime ingress
JSON runtime protocol
distributed orchestration
leader election
quorum writes
remote SaaS control plane
native UI
visual graph authoring
unverified extension execution
full control-flow taint proof
proof that external providers honor idempotency keys
multi-tenant sandboxing
```

Future work may add adapters or UI, but they must consume typed artifacts from the compiler/runtime and cannot become a second source of truth.

---

