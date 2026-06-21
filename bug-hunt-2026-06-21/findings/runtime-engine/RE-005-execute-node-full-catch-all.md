# RE-005: `execute_node_full` catch-all `_ => handle_core_step_once` silently routes unhandled node kinds to the core fallback

- **Severity**: Low
- **Category**: bug
- **Location**: `crates/vb_runtime/src/engine/execute.rs:205`
- **Confidence**: confirmed

## Description

`execute_node_full` is the dispatcher for every `CompiledNodeKind`. It explicitly handles 24 variants and falls through `_ => handle_core_step_once(plan, run, store)` for the rest. Because `CompiledNodeKind` is `#[non_exhaustive]` (assumed from the existing defensive `_` arms elsewhere in the codebase), adding a new variant that requires runtime-specific handling will silently route to `vb_core::engine::step_once`, which will probably fail with `InvalidCompiledWorkflow` or fall through again with no diagnostic.

## Evidence

`crates/vb_runtime/src/engine/execute.rs:37-206` ends with:

```rust
        CompiledNodeKind::ErrorHandler {
            body: handler_body, ..
        } => handle_error_handler(run, *handler_body),
        _ => handle_core_step_once(plan, run, store),
    }
}
```

The dispatcher has explicit arms for `ForEach*`, `Together*`, `Collect*`, `Reduce*`, `Repeat*`, `WaitUntil`, `WaitEvent`, `Ask`, `AskResume`, `Do`, `RetryCheck`, `ErrorHandler`. Every other `CompiledNodeKind` (including `Nop`, `Jump`, `Finish`, `SlotAssign`, `Branch`, etc.) falls through. The set of unhandled variants is implicit and changes silently as `vb_core` adds new node kinds.

## Adversarial Check

1. *"Catch-all is fine because the core step handles all primitives."* — True *today*, but the runtime engine adds behavior (`EvidenceCollector`, retry-aware tickets, capability gating) that the core step does not have. A new node kind that the runtime should treat specially (e.g., `EmitMetric`, `Trace`) would silently bypass the runtime layer.
2. *"The compiler-error if a variant is unmatched is enough."* — `#[non_exhaustive]` defeats this; rustc will not error on the missing arm because the `_` catches it.
3. *"This is intentional extensibility."* — Then it should be documented as such, and the call should be `_ => handle_core_step_once(plan, run, store).with_node_kind_hint(node.kind)`, or similar, so the core handler knows it is the fallback.

Severity Low: not a current bug, but a structural trap. The first time someone adds a `CompiledNodeKind::AwaitYield` or `CompiledNodeKind::Observe` and forgets to handle it here, the runtime will silently misbehave.

## Suggested Fix

Either:

(a) Replace the `_` with an explicit list of every primitive the core handler supports, so adding a new variant outside that list produces a compile error.

(b) Keep the `_` but make `handle_core_step_once` log the node kind it received, so a missed variant is observable in traces.

(c) Add a `#[non_exhaustive]` reminder comment near the `_` arm: "If you add a `CompiledNodeKind` variant that needs runtime-layer handling, add an explicit arm here. The `_` exists only for the core-primitive fallback set."

Option (a) is the strongest. The exhaustive match pattern is exactly what `non_exhaustive` enums are designed to opt out of *across crate boundaries*; inside the same workspace the dispatcher can still be exhaustive.
