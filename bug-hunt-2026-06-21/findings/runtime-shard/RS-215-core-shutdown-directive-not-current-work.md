# RS-215-core-shutdown-directive-not-current-work: `Shutdown` is documented to drain work but `completes_current_work` returns false

- **Severity**: Medium
- **Category**: bug
- **Location**: `crates/vb_runtime/src/shard/directive.rs:86`
- **Confidence**: confirmed

## Description
`ShardDirective::Shutdown` is documented as a graceful drain-and-stop directive, but `completes_current_work` excludes it. Any caller using this predicate to decide whether to finish queued/current work will treat shutdown as non-draining.

## Evidence
```rust
56:     /// Drain all remaining commands and shut down the shard.
57:     ///
58:     /// The shard processes all queued commands to completion, then transitions
59:     /// to a shut-down state. Returns `Ok(false)` to indicate the shard is dead.
60:     Shutdown,
...
77:     /// Returns true if this directive completes current work before stopping.
...
84:     /// - `Shutdown`: Processes remaining commands then stops.
85:     #[must_use]
86:     pub fn completes_current_work(&self) -> bool {
87:         matches!(self, Self::Suspend | Self::Barrier | Self::Migrate { .. })
88:     }
```

The variant-level contract and method-level documentation both say shutdown drains work. The implementation returns `false` for `Shutdown`.

## Adversarial Check
This is not just stale documentation: the method name and docs define a behavioral predicate, and `Shutdown` is the clearest case of a directive that completes remaining work before stopping. Excluding it creates a direct contradiction inside the public API.

## Suggested Fix
Include `Self::Shutdown` in the match. If the intended semantics are immediate stop, update the variant documentation and separate graceful shutdown from hard stop with distinct directive variants.
