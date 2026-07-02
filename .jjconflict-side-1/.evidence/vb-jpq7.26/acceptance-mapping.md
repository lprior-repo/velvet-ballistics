# vb-jpq7.26 TLA bounded overflow acceptance mapping

## Model bounds

- `specs/tla/BudgetArithmetic.tla`: exact Rust `u64` values modeled as four bounded 16-bit limbs (`0..65535`); `u16`, `u32`, and `u64` field ceilings represented by `MaxU16Word`, `MaxU32Word`, and `MaxU64Word`.
- `specs/RetryFSM.tla`: retry attempt bound constrained to `1..MAX_U16`; attempts remain in `0..MAX_U16` and cannot silently exceed configured retry budget.
- `specs/LifecycleJournal.tla`: journal capacity bounded by `MaxJournalLen`; command set bounded to one in-flight command; answer domain bounded to `0..MaxAnswer`.

## Typed overflow/resource-exhaustion transitions

- Budget add overflow yields `last_result = [tag |-> "Err", error |-> "Overflow"]` and moves runtime status nondeterministically into `"Suspended"` or `"Failed"`; subtract underflow yields `"Underflow"` with unchanged usage.
- Retry exhaustion now increments the final allowed attempt and records `last_error = "RetryExhausted"`; retryable running states require `actionAttempts < maxAttempts`, preventing silent saturation at the ceiling.
- Lifecycle journal capacity exhaustion records `journal_status = "JournalFull"` and preserves the journal/state in the TLA model. Production volatile runtime journaling now enforces an explicit event capacity and returns typed `RuntimeError::JournalFull { capacity }` without overwriting or dropping existing entries.

## TLC evidence

- `.evidence/vb-jpq7.26/logs/BudgetArithmetic-tlc.log`: `tlc -metadir .evidence/vb-jpq7.26/metadir/BudgetArithmetic -config specs/tla/BudgetArithmetic.cfg specs/tla/BudgetArithmetic.tla`.
- `.evidence/vb-jpq7.26/logs/RetryFSM-tlc.log`: `tlc -metadir .evidence/vb-jpq7.26/metadir/RetryFSM -config specs/RetryFSM.cfg specs/RetryFSM.tla`.
- `.evidence/vb-jpq7.26/logs/LifecycleJournal-tlc.log`: `tlc -metadir .evidence/vb-jpq7.26/metadir/LifecycleJournal -config specs/LifecycleJournal.cfg specs/LifecycleJournal.tla`.

## Liveness stance

PO-TLA-VB-JPQ7-26-004 is safety/deadlock-only. No liveness property is claimed for vb-jpq7.26 because the acceptance criterion is bounded arithmetic/resource-exhaustion safety: overflow/exhaustion must become typed error/suspend/fail states, and TLC must show no invariant violation or deadlock under finite bounds. Liveness/progress of lifecycle terminalization remains out of scope for this repair because a full journal is an intentional terminal resource-exhaustion condition. The changed configs therefore check invariants with TLC deadlock checking enabled and do not list `PROPERTY` rows.

## Rust implementation surface mapping

### PO-TLA-VB-JPQ7-26-001 — bounded aggregate budget arithmetic

| TLA artifact | Rust surface |
|---|---|
| `specs/tla/BudgetArithmetic.tla::usage` | `crates/vb_core/src/budget.rs:289` `AggregateResourceUsage` fields |
| `BudgetArithmetic.tla::TryAdd` | `crates/vb_core/src/budget.rs:442` `AggregateResourceUsage::try_add_budget` |
| `BudgetArithmetic.tla::TrySubtract` | `crates/vb_core/src/budget.rs:506` `AggregateResourceUsage::try_subtract_budget` |
| `BudgetArithmetic.tla::AddWord` / `AddResult` overflow | `crates/vb_core/src/budget.rs:778` `add_dim`; `:784` `checked_add`; `:785` `AggregateBudgetError::Overflow` |
| `BudgetArithmetic.tla::SubWord` / `SubResult` underflow | `crates/vb_core/src/budget.rs:788` `sub_dim`; `:794` `checked_sub`; `:795` `AggregateBudgetError::Underflow` |
| `BudgetArithmetic.tla::last_result.error` | `crates/vb_core/src/budget.rs:316` `AggregateBudgetError`, `:347` `Overflow`, `:353` `Underflow` |

### PO-TLA-VB-JPQ7-26-002 — bounded retry exhaustion is typed

| TLA artifact | Rust surface |
|---|---|
| `specs/RetryFSM.tla::runs`, `framePC`, `stepState` | `crates/vb_runtime/src/shard/types.rs` shard/run command state and `crates/vb_runtime/src/trace.rs:169` `TraceEvent` |
| `RetryFSM.tla::actionAttempts`, `maxAttempts` | `crates/vb_runtime/src/shard/helpers.rs:174` `validate_retry_attempt`; `:202` retry policy extraction; `crates/vb_core/src/action.rs` `RetryPolicy` / action contract retry fields |
| `RetryFSM.tla::ActionFailed` | `crates/vb_runtime/src/runtime.rs:343` `Runtime::fail_action`; `crates/vb_runtime/src/shard/impl_parts/chunk_001.rs:216` `ShardCommand::ActionFailed`; `:219` `ShardCommand::RuntimeActionFailed` |
| `RetryFSM.tla::last_error = "RetryExhausted"` | `crates/vb_runtime/src/shard/lifecycle/chunk_001.rs:423` `TraceEvent::ActionFailed`; `:428` `RuntimeJournalEvent::ActionFailed`; retry exhaustion is surfaced as failed action/journal state, not counter saturation |

### PO-TLA-VB-JPQ7-26-003 — bounded lifecycle journal full path does not drop/overwrite

Reviewer disposition: IMPLEMENTATION MAPPING UPDATED / PENDING EXTERNAL PROOF-REVIEW.

The external proof-reviewer correctly found that this model previously overclaimed executable Rust closure. Child bead `vb-wgq3` implements the production volatile journal closure: `VolatileRuntimeJournal` stores an explicit `capacity`, checks it before append, and returns `RuntimeError::JournalFull { capacity }` without mutating `events` when full. This updates PO-TLA-VB-JPQ7-26-003 from abstract-only to production-mapped evidence, but `vb-jpq7.26` remains open until external proof-reviewer re-approval.

| TLA artifact | Rust surface |
|---|---|
| `specs/LifecycleJournal.tla::journal`, `MaxJournalLen`, `CanAppend` | `crates/vb_runtime/src/journal/chunk_001.rs` `VolatileRuntimeJournal { events, capacity }`, `DEFAULT_CAPACITY`, `with_capacity(NonZeroUsize)`, and `append` capacity check before mutation. |
| `LifecycleJournal.tla::JournalFull` / `JournalFullReject` | `crates/vb_runtime/src/error/mod.rs` `RuntimeError::JournalFull { capacity }`; `crates/vb_runtime/src/journal/chunk_001.rs` returns that typed error when `events.len() >= capacity`. |
| `LifecycleJournal.tla::ResourceExhaustionDoesNotOverwrite` | `crates/vb_runtime/src/journal/tests/chunk_003.rs` `volatile_runtime_journal_returns_journal_full_and_preserves_entries_when_capacity_is_reached` and `volatile_runtime_journal_snapshots_remain_stable_after_full_append_rejection`. |
| `LifecycleJournal.tla::commands` | Runtime command queues remain separately bounded by `ShardCommandQueue`; PO-TLA-VB-JPQ7-26-003 maps only the full journal append preservation path. |

### PO-TLA-VB-JPQ7-26-004 — evidence/liveness stance

The model-checking obligation for this bead is invariant/deadlock evidence only. TLC deadlock checking is enabled by default in all three configs; no symmetry reduction is used; no temporal `PROPERTY` is claimed.

## Proof-reviewer non-vacuity checklist

- TypeOK checked for all three active bounded models/configs.
- Each config checks semantic invariants beyond TypeOK, including `ResourceExhaustionDoesNotOverwrite` with previous-snapshot variables for LifecycleJournal.
- Deadlock checking is enabled for the TLC runs; no `CHECK_DEADLOCK FALSE` remains in the changed configs.
- Bounds are explicit and finite; no symmetry reduction is used.
- Resource exhaustion is represented as typed error/suspend/fail/status transitions rather than unbounded `Nat` or silent saturation.
