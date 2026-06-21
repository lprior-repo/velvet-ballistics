# CF-013: `numeric_id!` macro duplicated across four ids/* files

- **Severity**: Info
- **Category**: simplification
- **Location**: `crates/vb_core/src/ids/workflow_ids.rs:19`, `symbol_ids.rs:11`, `storage_ids.rs:10`, `index_ids.rs:12`
- **Confidence**: confirmed

## Description

The `numeric_id!` macro is textually duplicated in four ids/* files with
identical bodies (debug-derive list, `new`, accessor, `FromStr`). Any
future addition (e.g. a `const fn checked_add`) has to be applied four
times.

## Evidence

Each of these four files declares its own `macro_rules! numeric_id`:

- `crates/vb_core/src/ids/workflow_ids.rs:19-50`
- `crates/vb_core/src/ids/symbol_ids.rs:11-42`
- `crates/vb_core/src/ids/storage_ids.rs:10-41`
- `crates/vb_core/src/ids/index_ids.rs:12-43`

`checked_index!` is similarly duplicated between `workflow_ids.rs:56` and
`index_ids.rs:45`.

## Adversarial Check

One might argue "macros are scoped per-module, so each file needs its
own." But `macro_rules!` can be `#[macro_export]`-ed from a single
location, or moved into a shared `ids/macros.rs` submodule. The
duplication is purely historical. As written, adding a method to all id
types requires touching four files, which is exactly the kind of friction
that leads to drift (e.g. `StepIdx::checked_add` exists on
`workflow_ids.rs:140` and `SlotIdx::checked_add` on line 166, but
`ConstIdx::checked_add` lives separately in `index_ids.rs:70`, with
slightly different placement — already a sign of drift).

## Suggested Fix

Hoist `numeric_id!` and `checked_index!` to `ids/macros.rs` (or
`ids/mod.rs`) and `pub(crate) use` it from each submodule.
