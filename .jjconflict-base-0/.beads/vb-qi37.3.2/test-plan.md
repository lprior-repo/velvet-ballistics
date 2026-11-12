# Test Plan: vb-qi37.3.2 — Collect Cursor Persistence

## Feature Under Test

Collect cursor persistence through Fjall journal and recovery via `hydrate_collect_states_from_recovered_journal`.

## Test Strategy

**Primary strategy**: Unit testing of the pure `CollectStates` persistence/recovery functions.
**Scope**: All contract clauses from `contract.md` must have at least one test or proof.
**Location**: `crates/vb_runtime/src/collect_tests.rs`

## Existing Tests to Leverage

The following existing tests already cover vb-qi37.3.2 contract clauses:

| Test | Lines | Covers |
|------|-------|--------|
| `collect_pagination_extra_round_trips_for_recovery` | 2112-2154 | PP1, PP3, PQ1, PQ2, PQ3 |
| `collect_pagination_extra_rejects_corrupt_bytes` | 2158-2163 | RP4, RQ3 |
| `collect_journal_extra_rejects_corrupt_bytes` | 2166-2172 | RP1, RP4, RQ3 |
| `collect_pagination_extra_recovered_journal_rejects_corrupt_bytes` | 2175-2191 | RP5, RQ3 |
| `collect_pagination_extra_recovered_journal_round_trips_and_resumes_next_page` | 2193-2258 | PQ4, PQ5, PQ6, RP5, RQ2, RQ6 |
| `collect_pagination_extra_rejects_identity_mismatch` | 2262-2270 | RP2, RP3, PI3 |
| `collect_journal_extra_rejects_identity_mismatch` | 2273-2282 | RP1, RP2, RP3, PI3 |
| `collect_pagination_extra_recovered_journal_rejects_identity_mismatch` | 2285-2307 | RP1, RP2, RP3, RP5, PI3, RQ4 |

## Gap Analysis

| Clause | Covered By | Gap? |
|--------|------------|------|
| PP1 | `collect_pagination_extra_round_trips_for_recovery` | No |
| PP2 | Structural proof at `drive.rs:98-100` | No (code review) |
| PP3 | `collect_pagination_extra_round_trips_for_recovery` | No |
| PP4 | Structural proof at `events.rs:214` | No (code review) |
| PQ1 | `collect_pagination_extra_round_trips_for_recovery` | No |
| PQ2 | `collect_pagination_extra_round_trips_for_recovery` | No |
| PQ3 | `collect_pagination_extra_round_trips_for_recovery` | No |
| PQ4 | `collect_pagination_extra_recovered_journal_round_trips_and_resumes_next_page` | No |
| PQ5 | `collect_pagination_extra_recovered_journal_round_trips_and_resumes_next_page` | No |
| PQ6 | `collect_pagination_extra_recovered_journal_round_trips_and_resumes_next_page` | No |
| RP1 | `collect_journal_extra_rejects_corrupt_bytes` | No |
| RP2 | `collect_pagination_extra_rejects_identity_mismatch` | No |
| RP3 | `collect_pagination_extra_rejects_identity_mismatch` | No |
| RP4 | `collect_pagination_extra_rejects_corrupt_bytes` | No |
| RP5 | `collect_pagination_extra_recovered_journal_rejects_corrupt_bytes` | No |
| RQ1 | Implicit in empty event path | No |
| RQ2 | `collect_pagination_extra_recovered_journal_round_trips_and_resumes_next_page` | No |
| RQ3 | `collect_pagination_extra_recovered_journal_rejects_corrupt_bytes` | No |
| RQ4 | `collect_pagination_extra_recovered_journal_rejects_identity_mismatch` | No |
| RQ5 | `collect_pagination_extra_recovered_journal_rejects_identity_mismatch` | No |
| RQ6 | `collect_pagination_extra_recovered_journal_round_trips_and_resumes_next_page` | No |
| PI1 | `collect_pagination_extra_recovered_journal_round_trips_and_resumes_next_page` | No |
| PI2 | Structural proof at `drive.rs:98-100` | No (code review) |
| PI3 | `collect_pagination_extra_recovered_journal_rejects_identity_mismatch` | No |
| PI4 | Structural proof at `collect.rs:133` | No (code review) |

## Additional Tests Required

No additional tests are required. All 25 contract clauses are covered by existing tests or structural code review.

## Test Execution

```bash
# Run all collect tests
cargo test -p vb_runtime collect_

# Run specific persistence/recovery tests
cargo test -p vb_runtime collect_pagination_extra_recovered
cargo test -p vb_runtime collect_journal_extra
```

## Integration with Broader Testing

- **Storage integration**: Tests at `collect_tests.rs` use `vb_storage::FjallJournal::open` to verify Fjall persistence
- **Recovery integration**: `hydrate_collect_states_from_recovered_journal` is exercised in recovery scenarios
- **No additional integration tests required**: The existing test suite covers the full persistence cycle

## Acceptance Criteria

| Criterion | Evidence |
|-----------|----------|
| All 25 contract clauses covered | `collect_tests.rs` lines 2112-2307 + structural proofs |
| Happy path: cursor round-trips correctly | `collect_pagination_extra_recovered_journal_round_trips_and_resumes_next_page` |
| Error path: corrupt bytes rejected | `collect_pagination_extra_recovered_journal_rejects_corrupt_bytes` |
| Error path: identity mismatch rejected | `collect_pagination_extra_recovered_journal_rejects_identity_mismatch` |
| Recovery: cursor resumes correctly | `collect_pagination_extra_recovered_journal_round_trips_and_resumes_next_page` |
