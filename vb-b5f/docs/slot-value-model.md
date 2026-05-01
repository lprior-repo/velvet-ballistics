# Slot Value Model

Runtime values live in numeric slots. Known fields must be flattened into slots by the compiler instead of using dynamic object maps in the hot path.

## Current Values

`SlotValue` currently supports:

```text
Null
Bool(bool)
I64(i64)
Text(Box<str>)
Bytes(bytes::Bytes)
```

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

## Future Values

Future phases may add finite numeric values, symbols, lists, objects, and blob arena references. Hot known fields still compile to slots; object values are for cold or opaque payload boundaries only.
