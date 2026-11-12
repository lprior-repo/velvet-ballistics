# Error Taxonomy: vb-fzgdn

Typed errors must return before mutation unless explicitly marked idempotent success. Hot variants must not require heap-string diagnostics.

## Admission errors
| Variant | Meaning | Mutation |
|---|---|---|
| `InvalidTimerTick` | Tick invalid for profile. | No |
| `TimerDurationTooLarge` | Delay exceeds configured max. | No |
| `ZeroDelayRejected` | Zero delay rejected by policy. | No |
| `TimerDeadlineOverflow` | Checked deadline add overflowed. | No |
| `TimerDeadlineBeforeCurrent` | Absolute deadline invalid under policy. | No |
| `InvalidTimerCapacity` | Capacity config invalid. | No |
| `TimerRegistryFull` | Pending timer/key index full. | No |
| `TimerGenerationExhausted` | Non-wrapping successor unavailable. | No |
| `RunNotFound` | Target run absent. | No |
| `TimerStepMismatch` | Step/kind not timer-registering or mismatched. | No |
| `TimerSlotMissing` | Deadline/timeout slot absent. | No |
| `TimerSlotTypeMismatch` | Slot cannot become numeric timer value. | No |
| `DelayedActionDuplicateConflict` | Same key with divergent payload/deadline/kind/action. | No |

## Fire errors
| Variant | Meaning | Mutation |
|---|---|---|
| `TimerAuthorityMissing` | No pending entry for authority target. | No |
| `TimerAuthorityMismatch` | Full authority comparison failed. | No |
| `TimerNotYetDue` | Fire before `deadline <= current_tick`. | No |
| `TimerAlreadyFired` | Duplicate fire after terminal fired state. | No |
| `TimerCancelled` | Fire after cancellation. | No |
| `DelayedActionQueueFull` | Valid fire cannot enqueue downstream action. | No partial mutation |

## Clock errors
- `ClockWentBackwards`: reject `AdvanceClockTo` below current tick before mutation.
- `ClockTickOutOfRange`: reject profile-horizon violation.

## Existing error mapping
Existing `RuntimeError::InvalidTimerFire` can remain a compatibility facade but internal domain must distinguish stale/mismatch/not-due/missing/cancelled for proof and diagnostics. Existing `QueueFull`/`CommandQueueCapacityExceeded` may map queue failures; registry capacity should be distinguishable.

## Railway rules
Admission and fire errors are fail-closed before registry, journal-success, queue, or run-frame mutation. Identical duplicate admission is success-like idempotency, not an error.
