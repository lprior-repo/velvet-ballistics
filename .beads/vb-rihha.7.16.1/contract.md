# Contract: full-domain checked aggregate-budget addition harness

## Scope

Repair only the Kani harness `aggregate_usage_try_add_budget_no_overflow_symbolic` in `crates/vb_core/src/budget/tests_and_verification.rs`. The real operation under verification remains `AggregateResourceUsage::try_add_budget(&AggregateResourceBudget)`. Production implementation, public API, serialized data, and existing separate overflow harness remain unchanged.

## Inputs and domain

The harness quantifies over fieldwise `kani::any()` values for the complete `AggregateResourceUsage` and `AggregateResourceBudget` structs. The property imposes no assumptions that exclude a successful input or any overflow input. Existing Kani proof bounds and flags remain unchanged.

## Result contract

For every arbitrary input pair, independently evaluate each checked addition in the same documented field order as the API contract: `max_steps_executable`, `max_action_tickets`, `max_parallel_in_flight`, `max_gather_pages`, `max_gather_items`, `max_result_bytes`, `max_total_slots_written`, `max_timer_entries`, `max_trace_events`, `max_active_runs` (usage plus one), `max_queue_depth`, `max_journal_batch_bytes`, `max_ipc_payload_bytes`, `max_blob_bytes`, `max_input_bytes`, `max_step_budget_per_tick`, `max_transitions_per_tick`, `max_open_gates`, `max_signal_subscriptions`, `max_incident_emissions`, `max_transitions`, `max_journal_bytes_total`, `max_value_store_entries`, and `max_cost_units`.

If all checked additions succeed, production returns `Ok` and every returned field equals the corresponding independent checked sum. If any addition overflows, production returns `AggregateBudgetError::Overflow` naming the first overflowing field in the listed order. No other error variant is valid for this operation. The expected arithmetic must not call production `add_dim`, `sub_dim`, or `try_add_budget` to construct the expected result.

## Retained behavior and evidence

The full result comparison implies every original exact successful sum claim on its successful subdomain. The old partial bounds never guaranteed success for omitted dimensions; retain their valid successful claims without repeating the false universal Ok claim. A sibling Kani-only oracle module may hold independent checked arithmetic to keep each source file below 300 lines. Keep the separate `aggregate_usage_try_add_budget_overflow_symbolic` harness unchanged. Preserve all other cases, names, flags, and source bytes outside this harness.

The focused Kani command must pass the repaired harness. Focused native budget tests and repository source formatting/lint checks must pass. Preserve the original failing Kani logs and concrete playback unchanged as RED evidence. Freeze the human-authored candidate and changed-path/hash manifest for independent CODE, TEST, and proof review; this contract does not claim verifier-wide proof closure.
