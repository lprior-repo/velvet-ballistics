---
section: 51
title: "Digest Canonicalization and Schema Versioning"
parent: velvet-ballistics-MASTER.md
---

## 51. Digest Canonicalization and Schema Versioning


### Required Digests

```text
workflow_source_digest = BLAKE3(raw source bytes)
compiled_digest       = BLAKE3(canonical compiled artifact payload)
action_abi_digest     = BLAKE3(canonical sorted action contracts)
policy_digest         = BLAKE3(canonical resource/durability/runtime policy)
payload_digest        = BLAKE3(postcard payload bytes)
```

### Canonical Ordering for Stable Digests

- Symbol IDs in definition order (index-based).
- Constant pool in index order.
- Accessor table in index order.
- Compiled nodes in `StepIdx` order.
- Object fields in insertion order (no sorting).
- Action contracts sorted by `ActionId`.
- `ResourceContract` fields in struct field order, encoded via Postcard.

### Libraries

`blake3 = "1"` and `crc32c = "0.6"` are required workspace dependencies for envelope digests and header checksums.

---
