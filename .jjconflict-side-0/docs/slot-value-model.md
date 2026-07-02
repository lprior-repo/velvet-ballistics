# Slot Value Model

Runtime values live in numeric slots. Known fields must be flattened into slots by the compiler instead of using dynamic object maps in the hot path.

## Current Values

`SlotValue` currently supports:

```text
Null
Bool(bool)
I64(i64)
F64(FiniteF64)
Symbol(SymbolId)   — Interned symbol handle
List(ListId)       — Runtime list arena handle
Object(ObjectId)   — Runtime object arena handle
Blob(BlobId)       — Runtime blob arena/storage handle
```

All variants are `Copy` (no heap allocation in the enum itself).

`SlotValue::is_true` returns true only for `Bool(true)`.

## Taint

Each slot has a parallel `Taint` marker:

```text
Clean
Secret
DerivedFromSecret
```

`Copy` preserves taint. `SetConst` writes clean values.

## Bounds

All slot access uses checked slice access. Out-of-bounds reads/writes return typed errors. No unchecked indexing or string slicing is allowed.

## Value Handles

All non-scalar values (`Symbol`, `List`, `Object`, `Blob`) are handles into runtime arenas. Handles are Copy and compact. Hot known fields compile to slots; object/list/blob values are for cold or opaque payload boundaries only.
