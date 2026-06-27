---
section: 5
title: "Architecture in One Page"
parent: velvet-ballistics-MASTER.md
---

## 5. Architecture in One Page

```text
Rust SDK DSL source
  -> macro parser
  -> workflow AST
  -> resolver
  -> type checker
  -> effect checker
  -> boundedness analyzer
  -> idempotency verifier
  -> capability/secret verifier
  -> taint checker
  -> durability checker
  -> numeric IR
  -> accepted artifact
  -> runtime admission
  -> durable history
  -> replay-derived frame
  -> side-effect outbox / completion inbox
  -> incident and replay reports
```

The compiler knows almost everything. The runtime knows almost nothing.

Runtime hot state consists of:

```text
RunId
ArtifactDigest
ProgramCounter / StepIdx
SlotIdx
ExprIdx
ActionId
ValueId
SeqNo
RunStatus
bounded frame arrays
bounded value arena
bounded trace ring
bounded outbox/inbox queues
```

Cold metadata consists of:

```text
source spans
workflow names
step names
action names
schema names
repair hints
diagnostic messages
agent context
human-readable reports
```

No cold metadata is required to execute a transition.

---

