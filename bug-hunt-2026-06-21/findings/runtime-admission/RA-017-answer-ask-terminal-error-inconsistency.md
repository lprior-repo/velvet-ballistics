# RA-017: `answer_ask` returns `RunNotFound` for terminal runs while `complete_action_with_output` returns `InvalidActionCompletion`

- **Severity**: Info
- **Category**: correctness (API consistency)
- **Location**: `crates/vb_runtime/src/runtime/runtime_control.rs:137-183`
- **Confidence**: confirmed

## Description

Three enqueue-style façade methods perform a terminal-runs probe before enqueueing, but they return different error variants for the same situation: `complete_action_with_output` and `fail_action` return `InvalidActionCompletion` for cancelled/killed runs, while `answer_ask` returns `RunNotFound`.

## Evidence

`complete_action_with_output` (lines 137-157):

```rust
if shard.terminal_runs_contains(ticket.run) {
    match shard.terminal_outcome_get(ticket.run) {
        Some(crate::shard::TerminalOutcome::Cancelled)
        | Some(crate::shard::TerminalOutcome::Killed) => {
            return Err(RuntimeError::InvalidActionCompletion);
        }
        _ => {}
    }
}
```

`fail_action` (lines 160-172): identical — returns `InvalidActionCompletion`.

`answer_ask` (lines 177-183):

```rust
pub fn answer_ask(&self, answer: crate::shard::AskAnswer) -> RuntimeResult<()> {
    let shard = self.shard_for(answer.ticket.run)?;
    if shard.terminal_runs_contains(answer.ticket.run) {
        return Err(RuntimeError::RunNotFound);
    }
    shard.enqueue(ShardCommand::AskAnswered { answer })
}
```

Additionally, `answer_ask` returns `RunNotFound` for *any* terminal outcome (including `Completed` / `Failed`), while `complete_action_with_output` only rejects `Cancelled` / `Killed` (passing through `Completed` / `Failed` for IPC re-entry). The docstring on `complete_action_with_output` (lines 143-146) explicitly justifies the asymmetry: "Naturally-completed runs are accepted here so that IPC re-entry scenarios produce RunNotFound at tick time instead of InvalidActionCompletion at enqueue time." `answer_ask` does not honor the same re-entry policy.

## Adversarial Check

One could argue `AskAnswer` and `ActionCompleted` are semantically different — an ask is a synchronous request/response while an action completion is asynchronous, so the re-entry semantics legitimately differ. But the *error variant* returned should not differ for the same logical condition ("this run is cancelled"). A caller that catches `RunNotFound` may attempt to retry or rebuild state, while `InvalidActionCompletion` is a terminal rejection. Returning the wrong variant causes the caller to follow the wrong recovery path. The asymmetry between "any terminal outcome" and "only Cancelled/Killed" is also unjustified by the docstring.

## Suggested Fix

Standardize on `InvalidActionCompletion` for the three methods, restricted to `Cancelled | Killed` outcomes (matching `complete_action_with_output`'s IPC re-entry carve-out). If `answer_ask` legitimately wants to reject natural completion too, add a dedicated `RuntimeError::AskRunNotAsking` variant or document the policy inline.
