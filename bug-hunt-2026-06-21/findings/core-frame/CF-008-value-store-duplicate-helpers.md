# CF-008: Massive code duplication across `value_store.rs` and `value_store/*.rs`

- **Severity**: Medium
- **Category**: simplification
- **Location**: `crates/vb_core/src/value_store.rs:339-421` vs. `crates/vb_core/src/value_store/{symbols,lists,objects,blobs,id_gen,validation}.rs`
- **Confidence**: confirmed

## Description

At least eleven functions and one struct are defined multiple times across
the value_store module:

| Symbol | Copies |
|---|---|
| `next_symbol_id` | `value_store.rs:339`, `value_store/symbols.rs:5`, `value_store/id_gen.rs:12` |
| `next_list_id` | `value_store.rs:347`, `value_store/lists.rs:5`, `value_store/id_gen.rs:20` |
| `next_object_id` | `value_store.rs:353`, `value_store/objects.rs:29`, `value_store/id_gen.rs:26` |
| `next_blob_id` | `value_store.rs:361`, `value_store/blobs.rs:5`, `value_store/id_gen.rs:34` |
| `validate_list_len` | `value_store.rs:367`, `value_store/lists.rs:11`, `value_store/validation.rs:9` |
| `validate_symbol_len` | `value_store.rs:377`, `value_store/symbols.rs:13`, `value_store/validation.rs:19` |
| `validate_blob_len` | `value_store.rs:387`, `value_store/blobs.rs:11`, `value_store/validation.rs:29` |
| `validate_object_len` | `value_store.rs:397`, `value_store/objects.rs:37`, `value_store/validation.rs:39` |
| `symbol_index` | `value_store.rs:407`, `value_store/symbols.rs:23`, `value_store/id_gen.rs:40` |
| `list_index` | `value_store.rs:411`, `value_store/lists.rs:21`, `value_store/id_gen.rs:44` |
| `object_index` | `value_store.rs:415`, `value_store/objects.rs:47`, `value_store/id_gen.rs:48` |
| `blob_index` | `value_store.rs:419`, `value_store/blobs.rs:21`, `value_store/id_gen.rs:52` |
| `ObjectField` struct | `value_store.rs:14-41`, `value_store/objects.rs:6-27`, `value_store/types.rs:7-32` |

## Evidence

`diff` between `value_store.rs:339-345` and `value_store/symbols.rs:5-11`
yields no semantic differences. Same for every other row in the table.

## Adversarial Check

A defender might say "the `pub` versions in `value_store/{symbols,lists,...}.rs`
exist for re-use outside the module, while the `pub(super)`/private versions
in `value_store.rs` are for internal use." But that just means the module
boundary is wrong: the internal call sites in `value_store.rs` (lines 94,
105, 120, 136, 165, 175, 184, 191, 199, 207, 218, 223, 228, 238, 255, 266,
269) could just as easily call the `pub` versions. As written, any change
to the limit constants or error variant has to be applied in three places
simultaneously, which is a maintenance hazard. Worse, the three `ObjectField`
definitions can drift out of sync, producing silent type-equivalence bugs
if anyone ever `impl`s a method on one but not the others.

## Suggested Fix

Pick one canonical location per symbol (likely `value_store/{symbols,lists,...}.rs`
with `pub(super)` visibility), `pub use` it from `value_store.rs`, and
delete the duplicates. Unify the three `ObjectField` definitions into one.
