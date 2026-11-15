# Typed command queue capacity refusal

## Approved scope

User approval authorizes migration of the existing five public constructor return types to `RuntimeResult<Self>` while retaining their names and arguments: `Shard::new`, `Shard::new_with_journal`, `Shard::new_with_journal_and_artifact_store`, `Runtime::new`, and `Runtime::new_for_tests_and_benchmarks_only`. Propagate `RuntimeResult` through the two existing crate-private shard factories and all crate-owned callers, including function pointers and CLI status propagation.

## Behavior

Reject only `command_queue_capacity` outside `1..=MAX_COMMAND_QUEUE_CAPACITY`, using the existing `CommandQueueCapacityExceeded { capacity, max }` error before queue/shard-vector allocation. Runtime validates before allocating the shard vector. Keep the real construction path, shared quota, supplied journal/artifact store, and named volatile test factory unchanged. Never clamp, substitute, disable, defer, or hide a failed construction.

Preserve raw-config acceptance and behavior for all other fields, especially `trace_capacity == 0`, `step_budget_per_tick == 0`, and `max_open_gates == 0`. Preserve checked `ShardConfig::new` validation order and exact errors.

## Tests and acceptance

Cover exact refusal at 0 and `MAX + 1` through all five public constructors, with journal nonmutation and no returned runtime/shard. Retain positive construction at 1, `MAX`, and defaults; shared-quota, journal/artifact policy, strict/relaxed modes, and existing trace/step/gate zero behaviors remain covered. Propagate `Result` through all inventoried source callers without dropping assertions, converting errors to optional values, or fabricating success values.

No new public names, error variants, dependencies, config fields, persisted formats, gate changes, or benchmark claims. Source functions remain within 25 raw lines and five parameters; production files remain within 300 lines. Final verification includes the original panic reproducer, affected package tests, and required source/canonical checks. Independent CODE and TEST review are required before landing.
