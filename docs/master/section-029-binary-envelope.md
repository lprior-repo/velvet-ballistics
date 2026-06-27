---
section: 29
title: "Binary Envelope"
parent: velvet-ballistics-MASTER.md
---

## 29. Binary Envelope

Every artifact, journal record, snapshot, blob record, and IPC frame uses one envelope.

```text
offset  bytes  field
0       4      magic_u32
4       2      schema_version_u16
6       2      record_kind_u16
8       4      header_len_u32 = 60
12      4      payload_len_u32
16      8      sequence_or_correlation_u64
24      32     payload_digest_blake3_256
56      4      header_crc32c
60      N      postcard payload
```

Decode order:

```text
validate magic
validate schema version
validate record kind family
validate header length
validate payload length before allocation
validate CRC32C
read exact payload
validate BLAKE3 digest
Postcard decode typed payload
validate semantic payload invariants
```

There is no separate IPC frame format.

---

