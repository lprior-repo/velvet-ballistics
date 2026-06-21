# CF-014: Deprecated `as_u64` / `as_u32` methods still live in production id types

- **Severity**: Info
- **Category**: simplification
- **Location**: `crates/vb_core/src/ids/storage_ids.rs:49`, `crates/vb_core/src/ids/workflow_ids.rs:114`, `:193`, `:202`
- **Confidence**: confirmed

## Description

`BlobId::as_u64`, `RunId::as_u64`, `SeqNo::as_u64`, and
`WorkflowId::as_u32` are all marked `#[deprecated]` but still ship in the
production id types. Each is a thin wrapper over the canonical `.get()`
accessor. Per the AGENTS.md "source lint is zero tolerance" rule and
Holzman's "no `as` numeric casts" rule, the `as_u*` family should be
removed entirely now that `.get()` is the documented replacement.

## Evidence

```rust
impl BlobId {
    #[deprecated(since = "0.1.0", note = "Use .get() instead")]
    pub fn as_u64(self) -> u64 {
        self.0
    }
}
```

(`crates/vb_core/src/ids/storage_ids.rs:48-53`)

Same pattern at `workflow_ids.rs:114-117` (`RunId::as_u64`),
`:193-196` (`SeqNo::as_u64`), `:202-205` (`WorkflowId::as_u32`).

## Adversarial Check

A defender might say "they're deprecated, what's the harm?" But deprecated
production methods still compile, still get called (especially by tests
and downstream crates), and still violate the `as_*` naming convention
that the codebase has clearly chosen to retire. The `0.1.0` deprecation
version suggests these have been pending removal for a while.

## Suggested Fix

Delete the four deprecated methods. If any caller (this crate or
downstream) breaks, switch the call site to `.get()`.
