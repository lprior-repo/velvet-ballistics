---
section: 43
title: "Property, Fuzz, and Replay Tests"
parent: velvet-ballistics-MASTER.md
---

## 43. Property, Fuzz, and Replay Tests

Required property tests:

```text
same source -> same source digest
same workflow definition -> same IR digest
same policy -> same policy digest
action manifests sorted by ActionId -> stable ABI digest
key expression canonicalization stable
idempotency key rejects forbidden ingredients
boundedness analysis conservative under nesting
snapshot + tail replay == full replay
history replay deterministic
indexes rebuild from history
semantic equality store-aware
budget counters never underflow/overflow
```

Required fuzz targets:

```text
artifact_decode
journal_record_decode
ipc_frame_decode
postcard_payload_family
sdk_macro_parser
expression_parser
key_expression_parser
action_manifest_decode
policy_decode
```

Required replay/crash tests:

```text
crash after RunAccepted before ack
crash after ActionScheduled before dispatch
crash after dispatch before completion
crash after completion before frame mutation
duplicate completion same digest
duplicate completion different digest
non-idempotent pending reconciliation
strict durability persists before ack
journaled durability bounded loss documented
```

---

