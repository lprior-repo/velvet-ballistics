STATUS: implemented

Holzmann reference files read:
- `nasa-jpl-standards.md`
- `runtime-performance-architecture.md`
- `zero-cost-abstractions.md`

Bead artifacts read:
- `.beads/vb-qi37.3.1/codebase-map.md`
- `.beads/vb-qi37.3.1/contract.md`
- `.beads/vb-qi37.3.1/test-plan.md`
- `.beads/vb-qi37.3.1/test-plan-review.md`
- `.beads/vb-qi37.3.1/red-phase.md`

Implementation summary:
- `collect_start` now treats an initial page that exhausts the source as complete: it writes the page, removes any active state for the active `(RunId, collector_slot)`, and jumps to `done` without retaining pagination state.
- `collect_next` now checks `cursor > source_items.len()` before the exhausted-page path, so corrupted cursor state fails closed with `collect cursor beyond source items` instead of completing.
- Compile blockers directly needed for targeted verification were repaired in `vb_storage` and `vb_runtime` without forbidden production Rust constructs.

Constraint evidence:
- No `unsafe`, `unwrap`, `expect`, `panic`, `todo`, `unimplemented`, or `dbg` added to modified production Rust.
- State remains caller-owned through existing `CollectStates`; no global mutable collect state was introduced.
- Failure paths remain typed `Result` errors.

Commands run:
- `cargo nextest run -p vb_runtime collect_start_exact_page_limit_finishes_without_active_pagination_state && cargo nextest run -p vb_runtime collect_next_returns_cursor_beyond_source_error_when_cursor_is_one_above_source_len` — passed after direct compile blocker repairs; each test passed individually.
- `cargo nextest run -p vb_runtime collect_start_exact_page_limit_finishes_without_active_pagination_state collect_next_returns_cursor_beyond_source_error_when_cursor_is_one_above_source_len` — passed; 2 tests run, 2 passed, 1344 skipped.
- `rtk cargo fmt --check` — passed with no output.
- `rtk cargo check -p vb_runtime` — passed; warnings only from existing workspace metadata/duplicate Makepad package notices.

Known residual blockers/risks:
- Full `moon ci` was not run in this State 6 pass; targeted verification was used per instruction.
- Existing test modules still emit pre-existing warnings under nextest compile; source package check passed.
