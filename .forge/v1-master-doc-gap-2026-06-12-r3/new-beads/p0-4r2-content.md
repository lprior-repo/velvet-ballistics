P0-4r2 runtime-action-mock-arms: Add explicit match arms for github.issue.create, ai.classify_ticket, http.request in ActionRegistry::dispatch_generic (NO new trait)

# Verification excerpts (read-before-write)

## Master doc §19 (line 876-1005)
- The static dispatch is `pub fn dispatch_action(action: ActionId, input: ActionInput) -> ActionResult<ActionOutcome>` matching on `ActionId`. There is NO `ActionExecutor` trait in §19. `ActionOutcome` is `enum { Ready, Suspended, Failed }`. Action names are resolved to numeric `ActionId` at compile time. Runtime dispatches by `ActionId` only.

## Master doc §75 (line 4317-4324)
- Real action names: `github.issue.create` (action_id=7), `ai.classify_ticket` (action_id=12). http.request is an additional external action not enumerated in master doc but cited by user spec.

## crates/vb_runtime/src/action.rs (212 lines total)
- Line 122-136: `ActionRegistry::dispatch(&self, input: &ActionInput, contract: &ActionContract) -> ActionResult<ActionOutcome>` — already implemented.
- Line 182-194: `fn dispatch_generic(input: &ActionInput, contract: &ActionContract) -> ActionResult<ActionOutcome>` — table-driven path. Returns `Ok(ActionOutcome::Suspended(ticket))`.
- NO `ActionExecutor` trait exists.

# Scope (verified, no fabrication)

Replace the body of `dispatch_generic` at lines 182-194 of `crates/vb_runtime/src/action.rs` so it first matches on the 3 action names (resolved from `ActionRegistry::resolve_by_name` or by ActionId lookup), and for each of the 3 actions returns a `MockMarker` payload variant indicating which mock executor would handle it (current behavior: `ActionOutcome::Suspended(ticket)` with a typed `MockMarker` enum field on the ticket that distinguishes `GitHubIssueCreate`, `AiClassifyTicket`, `HttpRequest`). For all other actions, fall through to the current `dispatch_generic` path.

The trait can come later as v2; v1 uses direct match arms.

# Acceptance test

Write a unit test in `crates/vb_runtime/src/action/tests.rs` that:
1. Registers a contract for `github.issue.create` with `ActionId::new(7)`.
2. Calls `ActionRegistry::dispatch` with input matching that contract.
3. Asserts the outcome is `ActionOutcome::Suspended(ticket)` where the ticket carries `MockMarker::GitHubIssueCreate`.

# Anti-hallucination guards

- DO NOT add a new `ActionExecutor` trait.
- DO NOT add 3 mock actions named `Echo`, `ComputeHash`, `Delay` — these are fabricated.
- DO NOT add a new file `crates/vb_runtime/src/action/mocks.rs` — the match arms go INSIDE `dispatch_generic` at lines 182-194.
- Use the REAL action names from master §75: `github.issue.create`, `ai.classify_ticket`, `http.request`.

# Kani harness

`#[cfg(kani)]` harness at `crates/vb_runtime/src/kani/action_dispatch.rs`:
```rust
#[cfg(kani)]
mod proof {
    use super::*;
    use vb_core::action::{ActionContract, ActionInput, ActionOutcome, ActionTicket, Idempotency};
    use vb_core::ids::{ActionId, RunId, SlotIdx, StepIdx};
    use crate::action::ActionRegistry;
    use kani::Arbitrary;

    impl Arbitrary for ActionId {
        fn any() -> Self {
            let id: u16 = kani::any();
            kani::assume(id < 100);  // bound registry
            ActionId::new(id)
        }
    }

    impl Arbitrary for RunId { fn any() -> Self { RunId::new(kani::any()) } }
    impl Arbitrary for SlotIdx { fn any() -> Self { SlotIdx::new(kani::any::<u16>()) } }
    impl Arbitrary for StepIdx { fn any() -> Self { StepIdx::new(kani::any::<u16>()) } }

    #[kani::proof]
    fn dispatch_never_panics() {
        let registry = ActionRegistry::new();
        let action: ActionId = kani::any();
        let run: RunId = kani::any();
        let step: StepIdx = kani::any();
        let slot: SlotIdx = kani::any();
        let ticket = ActionTicket { run, step, seq: 0, action, attempt: 1, idempotency_key: 0, capacity: 1 };
        let input = ActionInput { run, step, action, input: slot, ticket };
        // No contract registered — expect UnknownAction Err, no panic.
        let _ = registry.dispatch(&input, &contract_for(action));
    }
}
```

# Kani `Arbitrary` impl note

The harness constrains `ActionId::any()` to `< 100` (the registry is bounded at 65_535 by MAX_REGISTERED_ACTIONS, but 100 is enough to exercise the dispatch path).
