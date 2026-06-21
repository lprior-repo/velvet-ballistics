# CF-005: `ValueStore::insert_symbol` does not intern — duplicate bytes get distinct IDs

- **Severity**: Critical
- **Category**: correctness
- **Location**: `crates/vb_core/src/value_store.rs:92`
- **Confidence**: confirmed

## Description

The docstring promises: "Inserts an **interned** symbol and returns its
deterministic insertion ID." The implementation does not intern at all —
it pushes the bytes onto the arena and returns a fresh ID for every call,
so `insert_symbol("foo")` called twice produces two distinct `SymbolId`s
pointing at two copies of `"foo"`. Downstream code that compares
`SymbolId`s for identity (the entire point of interning) will conclude
the two calls produced unrelated symbols.

## Evidence

```rust
/// Inserts an interned symbol and returns its deterministic insertion ID.
pub fn insert_symbol(&mut self, value: impl Into<Box<str>>) -> CoreResult<SymbolId> {
    let value = value.into();
    validate_symbol_len(value.len())?;
    self.check_arena_cap()?;
    let id = next_symbol_id(self.symbols.len())?;
    self.symbols.push(value);          // <-- unconditional push, no dedup
    Ok(id)
}
```

(`crates/vb_core/src/value_store.rs:91-99`)

There is no `HashMap<Box<str>, SymbolId>` lookup, no reverse index, no
deduplication of any kind. The behavior contradicts both the docstring and
the `SlotValue::Symbol(SymbolId)` field comment in
`value/slot.rs:24-25` ("Interned symbol handle").

`insert_blob` (`value_store.rs:163-170`) has the same shape and also does
not deduplicate, but blob interning is not promised by the docstring, so
it is a separate (lower-severity) finding.

## Adversarial Check

A defender might claim "interning is the caller's responsibility; this
function just inserts." Then the docstring is wrong and should say "push"
or "allocate", not "intern". But every consumer of `SymbolId` — the
`Choose`/`ChooseSlot`/`BuildObject` evaluators, the `object_field`
lookup keyed by `SymbolId` — treats symbol equality as content equality.
Without deduplication at insertion, two `SymbolId`s that *should* be
equal (same bytes) will compare unequal, breaking object field lookup
(`value_store.rs:237-247`) and producing silent misses. This is exactly
the "symbol intern returning different IDs for same bytes" finding the
task brief lists as Critical.

## Suggested Fix

Maintain a `HashMap<Box<str>, SymbolId>` reverse index. On insert, look
up the bytes; if present, return the existing id; otherwise allocate and
insert. Cost: one `HashMap` lookup per intern, which is the standard
interning tradeoff.
