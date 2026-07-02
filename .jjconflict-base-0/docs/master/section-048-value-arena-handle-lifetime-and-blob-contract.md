---
section: 48
title: "Value Arena, Handle Lifetime, and Blob Contract"
parent: velvet-ballistics-MASTER.md
---

## 48. Value Arena, Handle Lifetime, and Blob Contract


### Arena Types

| Arena | Storage type | Deduplication | Growth |
|-------|-------------|---------------|--------|
| Symbol | `Vec<Box<str>>` | None — same string yields different `SymbolId` on each insert | Append-only |
| List | `Vec<Box<[SlotValue]>>` | None | Append-only |
| Object | `Vec<(Box<[(SymbolId, SlotValue)]>, IndexMap<SymbolId, SlotValue>)>` | Duplicate keys: later value wins | Append-only |
| Blob | `Vec<Box<[u8]>>` | None | Append-only |

### Handle Validity

A handle (`SymbolId`, `ListId`, `ObjectId`, `BlobId`) is valid if `id.as_usize() < arena.len()`. No generational indices. Handles are `Copy`. Handle validity lasts for the lifetime of the `ValueStore` — handles are not valid across different `ValueStore` instances.

### Object Field Lookup

Objects use a dual representation: primary `Box<[(SymbolId, SlotValue)]>` for serialization order, secondary `IndexMap<SymbolId, SlotValue>` for O(1) field lookup. Field order is insertion order.

### Blob Size vs Envelope

`ResourceContract.max_blob_bytes` is `u64` (default 16 MiB). Envelope `payload_len_u32` is `u32` (max ~4 GiB). **v1 design decision**: logical blobs are capped at `u32::MAX` bytes. No blob chunking in v1. A blob exceeding `u32::MAX` is rejected at admission.

### No GC in v1

Blobs, symbols, lists, and objects are write-once, read-many. No deletion, TTL, or garbage collection. Long-running servers must manage storage externally or restart.

---
