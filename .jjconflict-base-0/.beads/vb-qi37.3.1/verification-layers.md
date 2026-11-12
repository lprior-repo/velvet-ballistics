# Verification Layers: vb-qi37.3.1 — Collect State Isolation

## Boundary

- **Verified kernel**: `CollectStates` table operations (`collect.rs:34-443`) and `RunState.collect_states` ownership chain (`lifecycle.rs:121-439`, `drive.rs:47-127`, `execute.rs:43-280`)
- **Lean contract projection**: `lean-contract.md` — structural HashMap key isolation; waived by code review + unit tests
- **Runtime shell**: `Shard` run lifecycle, `drive_deterministic_full`, `execute_node_full` collect dispatch
- **External systems excluded from formal proof**: Fjall (storage layer), external action callbacks, timer wheel

## Layer Assignment

| Clause | Verification Layer | Tool | Evidence |
|--------|-------------------|------|----------|
| P1 (upsert key) | `code-review` + `unit` | Code inspection + `collect_states_independent_entries_per_run` | `collect.rs:46`, `collect_tests.rs:2507` |
| P2 (find page filter) | `code-review` + `unit` | Code inspection + `collect_states_find_returns_none_for_wrong_page` | `collect.rs:52-62`, `collect_tests.rs:1464` |
| P3 (caller supplies state) | `code-review` | No global collect state; `#![forbid(unsafe_code)]` | `lifecycle.rs:127`, `lifecycle.rs:436` |
| P4 (hydration identity) | `code-review` | `validate_hydrated_identity` rejects mismatch | `collect.rs:138-148` |
| P5 (per-run ownership) | `code-review` | `RunState::new` creates fresh `CollectStates` | `lifecycle.rs:127` |
| Q1 (upsert isolation) | `unit` | `collect_states_independent_entries_per_run` | `collect_tests.rs:2507` |
| Q2 (lookup non-interference) | `unit` | `collect_states_find_returns_none_for_wrong_run_id` | `collect_tests.rs:1486` |
| Q3 (capture non-interference) | `unit` | `collect_states_find_returns_none_for_wrong_run_id` (same key) | `collect_tests.rs:1486` |
| Q4 (remove non-interference) | `unit` | `collect_states_remove_nonexistent_is_noop` + key isolation | `collect_tests.rs:2553` |
| Q5 (collect_next isolation) | `code-review` | `collect_next` uses `run.run_id()` statically | `collect.rs:230-264` |
| Q6 (missing state fails closed) | `unit` | `collect_next_cursor_at_item_count_goes_to_done` | `collect_tests.rs:2627` |
| Q7 (hydration mismatch fails closed) | `code-review` | `validate_hydrated_identity` | `collect.rs:138-148` |
| Q8 (runtime ownership persists) | `code-review` | `take_run_state`/`keep_run` per RunId key | `lifecycle.rs:409-413`, `lifecycle.rs:450` |
| Q9 (evidence extras run-local) | `code-review` | `drive_deterministic_full` uses `run.run_id()` | `drive.rs:98` |
| I1 (key uniqueness) | `code-review` | HashMap invariant | `collect.rs:35` |
| I2 (embedded identity) | `code-review` | upsert uses state fields as key | `collect.rs:46` |
| I3 (page-match) | `code-review` | find filter on current_page | `collect.rs:60-61` |
| I4 (no global state) | `static-scan` | `#![forbid(unsafe_code)]` + code review | runtime crate-wide |
| I5 (per-run shard ownership) | `code-review` | Shard.runs per RunId key | `types.rs:186` |
| I6 (durable identity) | `code-review` | postcard encode + validate | `collect.rs:101-107` |

## Verification Statistics

| Layer | Clause Count | Notes |
|-------|-------------|-------|
| `code-review` | 14 | Structural properties proven by code inspection |
| `unit` | 6 | Existing tests in `collect_tests.rs` |
| `static-scan` | 1 | `#![forbid(unsafe_code)]` enforces no global state |
| `lean` | 0 | Waived — structural HashMap property (see `lean-contract.md`) |
| `kani` | 0 | Waived — no numeric/index bounds to verify beyond HashMap API |
| `miri` | 0 | Waived — no unsafe, no UB-sensitive paths |
| `proptest` | 0 | Waived — structural property, not data-driven |
| `fuzz` | 0 | Waived — no external byte-input boundary |
| `loom` | 0 | Waived — single-threaded shard, no concurrency |
| `mutation` | 0 | Waived — verification-only bead, no new code |

## Canonical Gate

- `moon ci` — all tests must pass
- `cargo test -p vb_runtime -- collect` — collect isolation tests
- `cargo clippy -p vb_runtime` — no warnings

## Waivers Summary

| Clause(s) | Reason | Compensating Evidence |
|-----------|--------|----------------------|
| All Lean | Structural HashMap key property; already unit-tested | `collect_states_independent_entries_per_run`, `collect_states_find_returns_none_for_wrong_run_id` |
| All Kani | No numeric bounds or indexing to verify beyond HashMap API | Code review confirms no unchecked indexing in collect primitive |
| All Miri | No unsafe code in collect primitive | `#![forbid(unsafe_code)]` |
| All proptest/fuzz | No generated input boundary; isolation is structural | Unit tests with explicit RunId/SlotIdx values |
| All loom | Single-threaded shard; no concurrent collect state access | `Shard` is `!Sync` by design |
