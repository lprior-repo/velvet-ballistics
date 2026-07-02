# Error Taxonomy — vb-y9d3v

## Railway Error Categories

| Category | Condition | Preferred typed error | Existing acceptable error |
| --- | --- | --- | --- |
| Missing run | Completion/failure/timer references run absent from live shard state | `RunNotFound` | `RunNotFound` |
| Terminal run authority | Run already finished/failed/cancelled and no live state exists | `RunNotFound` or `InvalidActionCompletion` | `RunNotFound` |
| Invalid attempt zero | `attempt == 0` | `AttemptBeyondMax { attempt: 0, max: capacity }` | existing `AttemptBeyondMax` |
| Invalid capacity zero | `capacity == 0` | `AttemptBeyondMax { attempt, max: 0 }` | existing `AttemptBeyondMax` |
| Over-capacity attempt | `attempt > capacity` | `AttemptBeyondMax { attempt, max: capacity }` | existing `AttemptBeyondMax` |
| Stale/lower attempt | `attempt < current` | `StaleAttempt { incoming, current }` | existing `StaleAttempt` |
| Future attempt | `attempt > current` while within capacity | `FutureAttempt { incoming, current }` | `InvalidActionCompletion` until variant exists |
| No scheduled authority | current attempt missing or zero for step | `InvalidActionCompletion` | existing `InvalidActionCompletion` |
| Wrong step state | step is not `Running` | `InvalidActionCompletion` | existing `InvalidActionCompletion` |
| Wrong action node/id | step missing, not `Do`, or action mismatch | `InvalidActionCompletion` | existing `InvalidActionCompletion` |
| Noncanonical key | key differs from canonical `(run, seq, action)` | `InvalidActionCompletion` | existing `InvalidActionCompletion` |
| Output slot mismatch | ready output slot differs from Do output | `InvalidActionCompletion` | existing `InvalidActionCompletion` |
| Taint downgrade | output/failure taint less restrictive than required | `ActionTaintDowngrade { required, supplied }` | existing variant |
| Encoded length mismatch | declared output length not actual Postcard length | `ActionOutputLengthMismatch { declared, actual }` | existing variant |
| Contract output too large | encoded output exceeds action contract | `ActionOutputTooLarge { size, max }` | existing variant |
| Resource output too large | encoded/blob output exceeds resource contract | `ActionOutputBlobTooLarge { size, max }` | existing variant |
| Retry metadata absent | retry requested but no retry check follows | `UnsupportedOperation { operation: "retry_metadata_missing" }` | existing variant |
| Retry policy unreadable | retry slot missing/non-i64/out of range/zero | `UnsupportedOperation { operation: ... }` | existing variants |
| Retry exhausted | current attempt already max | `RetryExhausted` or `Ok(false)` outcome | existing control flow |
| Seq overflow on retry | `seq.checked_add(1)` fails | `InternalInvariantViolation { reason: "seq_overflow_on_retry" }` | existing engine error |
| Attempt overflow on retry | `attempt.checked_add(1)` fails | `InternalInvariantViolation { reason: "attempt_overflow_on_retry" }` | existing engine error |
| Timer generation overflow | generation increment overflows | `TimerWheelError::GenerationExhausted` | existing variant |
| Stale timer fire | fired timer entry not current | ignored/no resume | current `fire_expired` removes only current index; downstream must bridge behavior |

## Non-Mutation Error Rule

All authority errors above are pre-mutation errors. On these results, the following must remain unchanged:

- run frame slots, taints, pc, step states, executed counter;
- `action_attempts` except during a validated runtime retry transition;
- pending timer index except during validated schedule/cancel/fire current;
- journal records;
- trace ring;
- runtime state map;
- completed/failed counters.

## Error Surface Recommendation

Add `RuntimeError::FutureAttempt { incoming: u16, current: u16 }` only if public API compatibility permits. If not, use `InvalidActionCompletion` but tests/proofs must still assert semantic rejection of future attempts.
