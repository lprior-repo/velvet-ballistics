# Red Phase Report: vb-qi37.3.1

## Files changed

- `crates/vb_runtime/src/collect_tests.rs`
- `.beads/vb-qi37.3.1/red-phase.md`

## Intended failing test commands

- `cargo nextest run -p vb_runtime collect_start_exact_page_limit_finishes_without_active_pagination_state`
- `cargo nextest run -p vb_runtime collect_next_returns_cursor_beyond_source_error_when_cursor_is_one_above_source_len`

## Why failures are expected before implementation

- The approved plan requires a source whose length exactly equals the first-page limit to finish without retaining active pagination state. Current `collect_start` writes a first page and always upserts pagination state for non-empty sources, so the exact-limit red test should fail until completion semantics are implemented.
- The approved plan requires `cursor == source_len + 1` to return `EngineError::InternalInvariantViolation { reason: "collect cursor beyond source items" }`. Current `collect_next` checks `cursor >= item_count` before the cursor-beyond-source branch, so the red test should fail until cursor boundary classification is repaired.

## Red phase scope notes

- Tests are executable Rust tests only; no production functionality was implemented.
- Primitive red tests assert exact control-flow/state outcomes and exact `EngineError` variants/reasons.
- Fuzz, Kani, mutation, primitive table coverage, and broader engine/shard scenarios remain intended follow-up execution gates after this red suite is integrated.
