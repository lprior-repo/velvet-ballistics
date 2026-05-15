# Domain Model Review - vb-qi37.2.5

STATUS: READY_FOR_INDEPENDENT_REVIEW

## Model Boundaries
- Core boundedness model: `WholeWorkflowBudget`, `BoundednessPolicy`, `StepBudget`, `ValueStore`, `ResourceContract`.
- Runtime shell: `run_until_blocked`/`drive_deterministic` plus caller-provided workflow/run/store.
- Validation shell: `ResourceContract::validate` and nested verifier diagnostics from `vb-qi37.2.4`.
- Out of scope: `vb_runtime` generated chunk build failure; classified as `DEFERRED_GLOBAL`.

## Illegal States To Exclude
- Unbounded adversarial input sizes before allocation.
- Uncapped `ValueStore::new()` used as evidence for cap enforcement.
- Step-budget tests that assert timeout/process kill instead of `StepBudgetExhausted` or typed error.
- Nested composition tests whose expected growth is implicit rather than encoded as finite parameters.
- Failure evidence that depends on panic, OOM, or whole-workspace unrelated build failure.

## Type Model Findings
- `StepBudget` correctly hides `remaining`; construction clamps at `MAX_STEP_BUDGET`, and `try_take` owns the monotonic transition.
- `ValueStore::with_max_slots` encodes finite arena capacity; `max_arena_entries == 0` represents uncapped and must not be used for boundedness rejection evidence.
- `BudgetError` variants are semantic enough for adversarial assertions; tests must assert exact variants and `actual > limit` where available.
- `EngineSignal::StepBudgetExhausted` is a normal bounded terminal slice signal, not an error.
- Nested composition requires static budget admission evidence before runtime execution; runtime step budget alone is insufficient to prove whole-workflow boundedness.

## Review Risks
- Existing `checked_len_to_u64` uses `unwrap_or(u64::MAX)`; this should be covered by tests/proofs as saturating evidence, not treated as a panic risk.
- `vb-qi37.2.4` final nested-composition diagnostics may change; downstream test-writer must bind to the landed public API.
- State 3 cannot approve its own contract; `contract-verification-review.md` must be produced by an independent reviewer.

## Decision
- Domain model is adequate for contract planning.
- Required follow-up: independent contract verification review before test planning consumes this contract.
