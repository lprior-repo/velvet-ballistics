# RS-007: `lifecycle.rs` `#![allow(...)]` block disables every Holzman safety lint for production code

- **Severity**: High
- **Category**: correctness / governance violation
- **Location**: `crates/vb_runtime/src/shard/lifecycle.rs:1-130`
- **Confidence**: confirmed

## Description

`shard/lifecycle.rs` opens with a 130-line `#![allow(...)]` attribute block that disables the clippy lints backing the repository's hard engineering rules ("No `unsafe`, `unwrap`, `expect`, `panic`, `todo`, `unimplemented`, `unreachable!`, `assert!` in production source", "No unchecked indexing/slicing, no `as` numeric casts, no unchecked arithmetic"). The `include!` directives at lines 134-137 pull `lifecycle/chunk_003.rs`, `chunk_001.rs`, and `chunk_002.rs` into the same crate-attribute scope, so all of those production files compile with the safety lints off.

## Evidence

Excerpted from the allow block (full list is 130 lines):

```rust
// crates/vb_runtime/src/shard/lifecycle.rs:1-130
#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::ok_expect,
    clippy::as_conversions,
    clippy::arithmetic_side_effects,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::todo,
    clippy::unimplemented,
    …
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    …
    unused_imports,
    dead_code,
    unused_variables
)]
```

The `include!` macro pulls the production chunks into this scope:

```rust
// lifecycle.rs:134-137
include!("lifecycle/chunk_003.rs");
include!("lifecycle/chunk_001.rs");
include!("lifecycle/chunk_002.rs");
```

`AGENTS.md` states: "Source lint is zero tolerance" and "No `unsafe`, `unwrap`, `expect`, `panic`, `todo`, `unimplemented`, `dbg`" and "No unchecked indexing/slicing, no `as` numeric casts, no unchecked arithmetic". This file overrides every one of those rules.

## Adversarial Check

A defender might argue the allow block exists only because of `lifecycle_tests.rs` (included at line 138) and the legacy test helpers. But:

1. Inner-attribute (`#![...]`) scope applies to the *entire module*, not just tests. The `#[cfg(test)] mod tests` at `lifecycle_tests.rs:5` would already gate the tests; the allow block is not needed for them.
2. The `chunk_001_submit.rs` file actually *uses* `#[cfg(test)]` to swap in different function bodies (lines 6-46), proving the production-vs-test split is reachable from the same source file.
3. `unused_imports, dead_code, unused_variables` in the allow block silence real warnings in production chunks, not tests.

Even if the current code happens not to use `unwrap`/`expect`/`panic`, the lint gate is removed, so any future regression in these `include!`-included files will not be caught by CI. The `#![forbid(unsafe_code)]` at line 131 is preserved (good), but every other safety lint is gone.

## Suggested Fix

Remove the `#![allow(...)]` block. Move any test-specific allows into the `#[cfg(test)] mod tests` block. Fix any actual lint failures the un-allow exposes rather than re-silencing them. If a lint truly does not apply, scope the allow to the specific line (`#[allow(clippy::needless_pass_by_value)]` on `handle_action_completion` at `chunk_001_action.rs:6` already does this correctly).
