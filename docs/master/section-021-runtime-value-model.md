---
section: 21
title: "Runtime Value Model"
parent: velvet-ballistics-MASTER.md
---

## 21. Runtime Value Model

Hot runtime values are compact and handle-based.

```rust
pub enum Value {
    Null,
    Bool(bool),
    I64(i64),
    F64(FiniteF64),
    Ref(ValueId),
}

pub enum ValueCell {
    Symbol(Box<str>),
    List(Box<[Value]>),
    Object(Box<[(SymbolId, Value)]>),
    Blob(BlobDigest),
}
```

One per-run value arena is preferred over separate symbol/list/object/blob arenas for recovery simplicity. Arena appends are journaled or snapshotted so handles are recoverable.

Durable history must never contain meaningless process-local handles without the arena state needed to hydrate them.

Language equality is semantic and store-aware:

```rust
pub fn value_eq(a: Value, b: Value, arena: &ValueArena) -> CoreResult<bool>;
```

Rust derived `PartialEq` on handle values is not the language equality rule.

---

