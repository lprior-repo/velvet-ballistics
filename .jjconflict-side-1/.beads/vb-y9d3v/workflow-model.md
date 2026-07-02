# Workflow Model — vb-y9d3v

## Action Scheduling and Completion State Machine

```text
DeterministicRunning
  -- execute_do / build ticket(attempt=1, capacity=retry_policy.max) --> AwaitingActionSignal
  -- shard.await_action / normalize engine ticket, record current attempt, journal ActionScheduledTicket --> AwaitingExternalAction
  -- external completion with FreshActionAuthority + valid payload --> CompletionPreflighted
  -- journal ActionCompletedEnvelope --> CompletionJournaled
  -- write slot + mark succeeded + advance pc --> DeterministicRunning or Terminal

AwaitingExternalAction
  -- external failure with FreshActionAuthority + RetryAuthority available --> RetryScheduled
  -- external failure with FreshActionAuthority + no retry + handler --> ErrorHandlerRunning
  -- external failure with FreshActionAuthority + no retry + no handler --> RunFailed

RetryScheduled
  -- runtime records current attempt n+1 and schedules/journals ticket --> AwaitingExternalAction
```

## Legal Transitions

| From | Event | Guard | To | Mutation allowed |
| --- | --- | --- | --- | --- |
| `DeterministicRunning` | `RuntimeSignal::AwaitingAction(ticket)` | `ticket.attempt == 1`, `capacity > 0`, Do node exists | `AwaitingExternalAction` | record scheduled attempt; append `ActionScheduledTicket` |
| `AwaitingExternalAction` | completion | fresh action authority; canonical key; output slot/taint/size valid | `CompletionPreflighted` | no mutation yet except local preflight allocation |
| `CompletionPreflighted` | journal append succeeds | journal accepts bounded encoded payload | `CompletionJournaled` | append only |
| `CompletionJournaled` | frame apply | checked slot write and step transition succeed | `DeterministicRunning`/`Terminal` | frame/trace/counters/pc |
| `AwaitingExternalAction` | retryable failure | fresh action authority; retry metadata exists; current < max | `RetryScheduled` | current attempt advances exactly one generation |
| `AwaitingExternalAction` | nonretry/no retry left with handler | fresh action authority; error handler found | `ErrorHandlerRunning` | mark failed, write optional failure slot, set pc |
| `AwaitingExternalAction` | nonretry/no retry left without handler | fresh action authority; no handler | `RunFailed` | failure path cleanup after journal |
| `AwaitingExternalAction` | stale/lower attempt | `incoming < current` | `AwaitingExternalAction` | none |
| `AwaitingExternalAction` | future attempt | `incoming > current` | `AwaitingExternalAction` | none |
| `AwaitingExternalAction` | invalid key/wrong action/wrong step | guard fails | `AwaitingExternalAction` | none |

## Completion Preflight Ordering

The completion path must preserve this order:

1. locate live run;
2. validate action authority (attempt, state, node, action);
3. validate canonical key;
4. resolve action contract;
5. validate input/output slots;
6. reject taint downgrade;
7. encode output and validate declared length;
8. reject contract/resource byte overflow;
9. validate action outcome against contract;
10. append completion journal;
11. mutate frame/trace and resume driving.

No frame/journal/trace mutation may occur before steps 1-9 all pass.

## Failure and Retry Ordering

The failure path must preserve this order:

1. locate live run;
2. if retryable, derive capacity from retry metadata without granting future authority;
3. validate action authority exactly against current attempt;
4. if retryable metadata exists, validate retry policy and current attempt;
5. advance to next attempt only via runtime-owned retry transition;
6. append `ActionFailed` only after invalid/stale/future checks pass;
7. drive retry, handler, or terminal failure.

Current fresh-main code validates inside `apply_action_failure_to_state` before `ActionFailed` append, which matches non-mutation intent for invalid tickets; downstream must verify future attempts are included in invalid checks.

## Timer State Machine

```text
NoTimer
  -- schedule wait/ask timer --> TimerPending(generation=1)
TimerPending(g)
  -- replace timer --> TimerPending(g+1)
TimerPending(g)
  -- cancel --> NoTimer
TimerPending(g)
  -- fire current entry(g) --> TimerFiredFresh -> resume wait/ask
TimerPending(g+1) or NoTimer
  -- fire stale entry(g) --> StaleTimerIgnored
```

## Terminal Fences

- `finish_run` removes pending timers, marks terminal, appends `RunFinished`, releases frame, and discards journal sequence.
- After terminal removal, action completion/failure and timer fires for the run have no live authority and must fail/ignore without mutation.
- Recovery/replay must not re-execute side effects for already scheduled non-idempotent actions.
