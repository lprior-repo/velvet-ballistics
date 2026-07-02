# Boundary Map — vb-y9d3v

## Pure/Core Boundaries

| Boundary | Files | Contract |
| --- | --- | --- |
| Action DTO and key functions | `crates/vb_core/src/action.rs` | `ActionTicket` is public untrusted data until canonical/fresh checks pass; idempotency key computation is deterministic and total over ID inputs. |
| Shard helper validation | `crates/vb_runtime/src/shard/helpers.rs` | Pure-ish state readers must validate attempt bounds, current attempt freshness, node kind/action equality, retry policy, and checked retry increments without side effects except where function name explicitly records. |
| Completion preflight | `crates/vb_runtime/src/shard/lifecycle/chunk_003.rs` | Converts untrusted ticket/output into `ActionCompletionPreflight`; no shard mutation or journal append before success. |
| Timer wheel | `crates/vb_runtime/src/shard/timer_wheel.rs` | Maintains dual indexes and generation tokens; stale entries cannot resume a run. |

## Imperative Shell Boundaries

| Boundary | Files | Contract |
| --- | --- | --- |
| Shard state mutation | `crates/vb_runtime/src/shard/transitions.rs`, `chunk_001.rs` | Owns `runs`, `runtime_states`, counters, trace, journals, frame release, and pending timers. Must call pure/preflight checks before mutation. |
| Journal append | `RuntimeJournalEvent::*` call sites in shard lifecycle | Authority-invalid events must not be journaled. Completion journal precedes frame mutation for accepted output. |
| Direct/API/IPC completion ingress | Runtime public and command paths leading to `handle_action_completion`/`handle_action_failure` | Treat incoming tickets as hostile DTOs. Do not trust attempt/capacity/key from callers. |
| Storage/recovery | journal/snapshot replay paths | Must preserve scheduled action authority and prevent replay from duplicating non-idempotent side effects. |

## Async/Concurrency Boundary

- Runtime execution is shard-owned and synchronous until suspension.
- External completions/failures can arrive after cancellation, terminal removal, retry replacement, or timer replacement; generation fences must handle reordering.
- No per-step async task may mutate `RunState` directly; all authority checks occur at shard boundary.

## Parser/Serialization Boundary

- `ActionTicket` serialized in journal/IPC is not trusted when re-entering live mutation logic.
- Postcard-encoded completion payload length must be validated before journal append and frame mutation.
- No JSON/YAML/HTTP may enter runtime core.

## Trusted/Untrusted Data Classification

| Data | Trusted after | Notes |
| --- | --- | --- |
| `ActionTicket` from handler/API/IPC | `FreshActionAuthority` construction | Public struct fields are forgeable. |
| `ActionOutputReady` | completion preflight success | Value is handle-only but encoded length/taint/slot must be checked. |
| `ActionFailure` | failure authority + retry/error validation | Retryable flag alone does not grant retry authority. |
| `TimerEntry` from deadline queue | generation equality check with run index | Old entries can remain in deadline queue after replacement/cancel. |
| Retry policy slot | successful `retry_policy_after_action` | Slot value must be `I64`, positive, and convertible to `u16`. |

## Forbidden Boundary Shortcuts

- Do not accept future action attempts because they are within `capacity`.
- Do not derive freshness from idempotency key alone; key omits attempt/capacity.
- Do not append `ActionFailed`/`ActionCompletedEnvelope` before authority validation.
- Do not let timer deadline order override generation freshness.
- Do not reuse prior rejected proof artifacts as closure without fresh-main wiring and review.
